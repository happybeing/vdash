///! Application logic
///!
///! Edit src/custom/app.rs to create a customised fork of logtail-dash
use linemux::MuxedLines;
use std::collections::HashMap;

use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc, Duration};
use structopt::StructOpt;
use tempfile::NamedTempFile;

use crate::custom::timelines::{MinMeanMax, get_duration_text};
use crate::custom::app_timelines::{AppTimelines, TIMESCALES, APP_TIMELINES};
use crate::custom::app_timelines::{STORAGE_COST_TIMELINE_KEY, EARNINGS_TIMELINE_KEY, PUTS_TIMELINE_KEY, GETS_TIMELINE_KEY, CONNECTIONS_TIMELINE_KEY, RAM_TIMELINE_KEY, ERRORS_TIMELINE_KEY};
use crate::custom::opt::{Opt, MIN_TIMELINE_STEPS};
use crate::shared::util::StatefulList;

pub const SAFENODE_BINARY_NAME: &str = "safenode";
pub static SUMMARY_WINDOW_NAME: &str = "Summary of Monitored Nodes";
pub static HELP_WINDOW_NAME: &str = "Help";
pub static DEBUG_WINDOW_NAME: &str = "Debug Window";

use std::sync::Mutex;
lazy_static::lazy_static! {
	pub static ref DEBUG_LOGFILE: Mutex<Option<NamedTempFile>> =
		Mutex::<Option<NamedTempFile>>::new(None);
}

#[macro_export]
macro_rules! debug_log {
	($message:expr) => {
		unsafe {
			debug_log($message);
		}
	};
}
pub use crate::debug_log;

pub unsafe fn debug_log(message: &str) {
	// --debug-window - prints parser results for a single logfile
	// to a temp logfile which is displayed in the adjacent window.
	match &(*DEBUG_LOGFILE.lock().unwrap()) {
		Some(f) => {
			use std::io::Seek;
			if let Ok(mut file) = f.reopen() {
				file.seek(std::io::SeekFrom::End(0)).unwrap();
				writeln!(file, "{}", message).unwrap();
			}
		}
		None => (),
	};
}

pub struct App {
	pub opt: Opt,
	pub dash_state: DashState,
	pub monitors: HashMap<String, LogMonitor>,
	pub logfile_with_focus: String,
	pub logfiles: MuxedLines,
}

impl App {
	pub async fn new() -> Result<App, std::io::Error> {
		let mut opt = Opt::from_args();

		if opt.files.is_empty() {
			println!("{}: no logfile(s) specified.", Opt::clap().get_name());
			return exit_with_usage("missing logfiles");
		}

		if opt.timeline_steps < MIN_TIMELINE_STEPS {
			println!(
				"Timeline steps number is too small, minimum is {}",
				MIN_TIMELINE_STEPS
			);
			return exit_with_usage("invalid parameter");
		}

		let mut dash_state = DashState::new();
		dash_state.debug_window = opt.debug_window;
		if opt.debug_dashboard {
			dash_state.main_view = DashViewMain::DashDebug;
		}

		let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
		let mut logfiles = MuxedLines::new()?;
		let mut debug_logfile_name = String::new();

		let mut debug_logfile: Option<tempfile::NamedTempFile> = if opt.debug_window {
			opt.files = opt.files[0..1].to_vec();
			let named_file = NamedTempFile::new()?;
			let path = named_file.path();
			let path_str = path
				.to_str()
				.ok_or_else(|| Error::new(ErrorKind::Other, "invalid path"))?;
			opt.files.push(String::from(path_str));
			debug_logfile_name = String::from(path_str);
			Some(named_file)
		} else {
			None
		};

		println!("Loading {} files...", opt.files.len());
		let mut first_logfile = String::new();
		for f in &opt.files {
			println!("file: {}", f);
			if first_logfile.is_empty() {
				first_logfile = f.to_string();
			}
			let mut monitor = LogMonitor::new(&opt, f.to_string(), opt.lines_max);
			if opt.debug_window && monitor.index == 0 {
				if let Some(named_file) = debug_logfile {
					*DEBUG_LOGFILE.lock().unwrap() = Some(named_file);
					debug_logfile = None;
				}
			}
			if opt.ignore_existing {
				dash_state.logfile_names.push(f.to_string());
				monitors.insert(f.to_string(), monitor);
			} else {
				match monitor.load_logfile(&mut dash_state) {
					Ok(()) => {
						dash_state.logfile_names.push(f.to_string());
						monitors.insert(f.to_string(), monitor);
					}
					Err(e) => {
						println!("...failed: {}", e);
						return Err(e);
					}
				}
			}

			match logfiles.add_file(&f).await {
				Ok(_) => (),
				Err(e) => {
					println!("ERROR: {}", e);
					println!(
						"Note: it is ok for the file not to exist, but the file's parent directory must exist."
					);
					return Err(e);
				}
			}
		}

		let activate_debug_dashboard = opt.debug_dashboard;
		let mut app = App {
			opt,
			dash_state,
			monitors,
			logfile_with_focus: first_logfile.clone(),
			logfiles,
		};
		app.update_timelines(&Utc::now());
		app.dash_state.update_summary_window(&mut app.monitors);

		if !first_logfile.is_empty() {
			app.dash_state.dash_node_focus = first_logfile.clone();
		}

		if activate_debug_dashboard {
			app.set_logfile_with_focus(debug_logfile_name);
		} else {
			app.set_logfile_with_focus(first_logfile);
		}
		Ok(app)
	}

	pub fn update_timelines(&mut self, now: &DateTime<Utc>) {
		for (_monitor_file, monitor) in self.monitors.iter_mut() {
			monitor.metrics.update_timelines(now);
		}
	}

	pub fn update_chunk_store_stats(&mut self) {
		for (_monitor_file, monitor) in self.monitors.iter_mut() {
			monitor.update_chunk_store_fsstats();
		}
	}

	pub fn get_monitor_for_file_path(&mut self, logfile: &String) -> Option<&mut LogMonitor> {
		let mut monitor_for_path = None;
		for (monitor_file, monitor) in self.monitors.iter_mut() {
			if monitor_file.eq(logfile) {
				monitor_for_path = Some(monitor);
				break;
			}
			use std::env::current_dir;
			if let Ok(current_dir) = current_dir() {
				let logfile_path = Path::new(logfile.as_str());
				if current_dir.join(monitor_file).eq(&logfile_path) {
					monitor_for_path = Some(monitor);
					break;
				}
			}
		}
		return monitor_for_path;
	}

	pub fn get_debug_dashboard_logfile(&mut self) -> Option<String> {
		for (_logfile, monitor) in self.monitors.iter_mut() {
			if monitor.is_debug_dashboard_log {
				return Some(monitor.logfile.clone());
			}
		}
		None
	}

	pub fn get_logfile_with_focus(&mut self) -> Option<String> {
		match (&mut self.monitors).get_mut(&self.logfile_with_focus) {
			Some(monitor) => Some(monitor.logfile.clone()),
			None => None,
		}
	}

	pub fn get_monitor_with_focus(&mut self) -> Option<&mut LogMonitor> {
		match (&mut self.monitors).get_mut(&self.logfile_with_focus) {
			Some(monitor) => Some(monitor),
			None => None,
		}
	}

	pub fn set_logfile_with_focus(&mut self, logfile_name: String) {
		match self.get_monitor_with_focus() {
			Some(fading_monitor) => {
				fading_monitor.has_focus = false;
				self.logfile_with_focus = String::new();
			}
			None => (),
		}

		if logfile_name == DEBUG_WINDOW_NAME {
			self.dash_state.debug_window_has_focus = true;
			self.logfile_with_focus = logfile_name.clone();
			return;
		} else {
			self.dash_state.debug_window_has_focus = false;
		}

		if let Some(focus_monitor) = (&mut self.monitors).get_mut(&logfile_name) {
			focus_monitor.has_focus = true;
			self.logfile_with_focus = logfile_name.clone();
		} else {
			error!("Unable to focus UI on: {}", logfile_name);
		};
	}

	pub fn change_focus_next(&mut self) {
		if self.dash_state.main_view == DashViewMain::DashDebug {
			return;
		}

		let mut next_i = 0;
		for (i, name) in self.dash_state.logfile_names.iter().enumerate() {
			if name == &self.logfile_with_focus {
				if i < self.dash_state.logfile_names.len() - 1 {
					next_i = i + 1;
				}
				break;
			}
		}

		if next_i == 0 && self.opt.debug_window && self.logfile_with_focus != DEBUG_WINDOW_NAME {
			self.set_logfile_with_focus(DEBUG_WINDOW_NAME.to_string());
			return;
		}

		let logfile = self.dash_state.logfile_names[next_i].to_string();
		self.set_logfile_with_focus(logfile.clone());

		if let Some(debug_logfile) = self.get_debug_dashboard_logfile() {
			if logfile.eq(&debug_logfile) {
				self.change_focus_next();
			}
		}
	}

	pub fn change_focus_previous(&mut self) {
		if self.dash_state.main_view == DashViewMain::DashDebug {
			return;
		}

		let len = self.dash_state.logfile_names.len();
		let mut previous_i = len - 1;
		for (i, name) in self.dash_state.logfile_names.iter().enumerate() {
			if name == &self.logfile_with_focus {
				if i > 0 {
					previous_i = i - 1;
				}
				break;
			}
		}

		if self.opt.debug_window
			&& previous_i == len - 1
			&& self.logfile_with_focus != DEBUG_WINDOW_NAME
		{
			self.set_logfile_with_focus(DEBUG_WINDOW_NAME.to_string());
			return;
		}

		let logfile = self.dash_state.logfile_names[previous_i].to_string();
		self.set_logfile_with_focus(logfile.clone());

		if let Some(debug_logfile) = self.get_debug_dashboard_logfile() {
			if logfile.eq(&debug_logfile) {
				self.change_focus_previous();
			}
		}
	}

	pub fn change_focus_to(&mut self, logfile_index: usize) {
		if logfile_index < self.dash_state.logfile_names.len() {
			self.set_logfile_with_focus(self.dash_state.logfile_names[logfile_index].clone());
			self.dash_state.main_view = DashViewMain::DashNode;
		}
	}

	pub fn handle_arrow_up(&mut self)   { self.handle_arrow(false); }

	pub fn handle_arrow_down(&mut self) { self.handle_arrow( true); }

	pub fn handle_arrow(&mut self, is_down: bool) {
		let list = match self.dash_state.main_view {
			DashViewMain::DashSummary => { Some(&mut self.dash_state.summary_window_list) }
			DashViewMain::DashNode => {
				if let Some(monitor) = self.get_monitor_with_focus() {
					Some(&mut monitor.content)
				} else if self.opt.debug_window {
					Some(&mut self.dash_state.debug_window_list)
				} else {
					None
				}
			}
			DashViewMain::DashHelp => { None }
			DashViewMain::DashDebug => {
				if self.opt.debug_window {
					Some(&mut self.dash_state.debug_window_list)
				} else {
					None
				}
			}
		};

		if let Some(list) = list {
			do_bracketed_next_previous(list, is_down);
		}
	}

	pub fn preserve_node_selection(&mut self) {
		if self.dash_state.main_view == DashViewMain::DashSummary {
			if let Some(selected_index) = self.dash_state.summary_window_list.state.selected() {
				self.change_focus_to(selected_index);
			}
		} else if self.dash_state.main_view == DashViewMain::DashNode {
			if let Some(monitor) = self.get_monitor_with_focus() {
				let index = Some(monitor.index);
				self.dash_state.summary_window_list.state.select(index);
			}
		}
	}

	pub fn toggle_logfile_area(&mut self) {
		self.dash_state.node_logfile_visible = !self.dash_state.node_logfile_visible;
	}

	pub fn scale_timeline_up(&mut self) {
		if self.dash_state.active_timescale == 0 {
			return;
		}
		self.dash_state.active_timescale -= 1;
	}

	pub fn scale_timeline_down(&mut self) {
		if self.dash_state.active_timescale == TIMESCALES.len()-1 {
			return;
		}
		self.dash_state.active_timescale += 1;
	}

    pub fn top_timeline_next(&mut self) {
        if self.dash_state.top_timeline < APP_TIMELINES.len() {
            self.dash_state.top_timeline += 1;
        }
        else {
            self.dash_state.top_timeline = 0;
        }
    }

    pub fn top_timeline_previous(&mut self) {
        if self.dash_state.top_timeline > 0 {
            self.dash_state.top_timeline -= 1;
        }
        else {
            self.dash_state.top_timeline = APP_TIMELINES.len() - 1;
        }
    }

    // Rotate UI display state through Min, Mean, Max values
    pub fn bump_mmm_ui_mode(&mut self) {
        self.dash_state.bump_mmm_ui_mode();
    }

    pub fn mmm_ui_mode(&mut self) -> &MinMeanMax {
        return self.dash_state.mmm_ui_mode();
    }
}

/// Move selection forward or back without wrapping at start or end
fn do_bracketed_next_previous(list: &mut StatefulList<String>, next: bool) {
	if next {
		if let Some(selected) = list.state.selected() {
			if selected != list.items.len() - 1 {
				list.next();
			}
		} else {
			list.previous();
		}
	} else {
		if let Some(selected) = list.state.selected() {
			if selected != 0 {
				list.previous();
			}
		} else {
			list.previous();
		}
	}
}

fn exit_with_usage(reason: &str) -> Result<App, std::io::Error> {
	println!(
		"Try '{} --help' for more information.",
		Opt::clap().get_name()
	);
	return Err(Error::new(ErrorKind::Other, reason));
}

use fs2::{statvfs, FsStats};

const NODE_INACTIVITY_TIMEOUT_S: i64 = 20;	// Seconds with no log message before node becomes 'inactive'

pub struct LogMonitor {
	pub index: usize,
	pub content: StatefulList<String>,
	max_content: usize, // Limit number of lines in content
	pub has_focus: bool,
	pub logfile: String,
	pub chunk_store_fsstats: Option<FsStats>,
	pub chunk_store_pathbuf: PathBuf,
	pub metrics: NodeMetrics,
	pub metrics_status: StatefulList<String>,
	pub is_debug_dashboard_log: bool,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);

impl LogMonitor {
	pub fn new(opt: &Opt, f: String, max_lines: usize) -> LogMonitor {
		let index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);

		let mut is_debug_dashboard_log = false;
		if let Some(debug_logfile) = &*DEBUG_LOGFILE.lock().unwrap() {
			if let Some(debug_logfile_path) = debug_logfile.path().to_str() {
				is_debug_dashboard_log = f.eq(debug_logfile_path);
			}
		}

		let mut chunk_store_pathbuf = PathBuf::from(&f);
		if chunk_store_pathbuf.pop() {
			chunk_store_pathbuf.push("chunkdb")
		}

		LogMonitor {
			index,
			logfile: f,
			max_content: max_lines,
			chunk_store_fsstats: None,
			chunk_store_pathbuf,
			metrics: NodeMetrics::new(&opt),
			content: StatefulList::with_items(vec![]),
			has_focus: false,
			metrics_status: StatefulList::with_items(vec![]),
			is_debug_dashboard_log,
		}
	}

	pub fn is_node(&self) -> bool { return !self.is_debug_dashboard_log; }

	pub fn update_chunk_store_fsstats(&mut self) {
		self.chunk_store_fsstats = match statvfs(&self.chunk_store_pathbuf) {
			Ok(fsstats) => Some(fsstats),
			Err(_) => None,
		};
	}

	pub fn load_logfile(&mut self, dash_state: &mut DashState) -> std::io::Result<()> {
		use std::io::{BufRead, BufReader};

		let f = File::open(self.logfile.to_string());
		let f = match f {
			Ok(file) => file,
			Err(_e) => return Ok(()), // It's ok for a logfile not to exist yet
		};

		self.update_chunk_store_fsstats();
		let f = BufReader::new(f);

		for line in f.lines() {
			let line = line.expect("Unable to read line");
			self.append_to_content(&line)?;
			if self.is_debug_dashboard_log {
				dash_state._debug_window(&line);
			}
		}

		if self.content.items.len() > 0 {
			self.content
				.state
				.select(Some(self.content.items.len() - 1));
		}

		Ok(())
	}

	pub fn append_to_content(&mut self, text: &str) -> Result<(), std::io::Error> {
		if self.line_filter(&text) {
			self._append_to_content(text)?; // Show in TUI
			if self.is_debug_dashboard_log {
				return Ok(());
			}
			self.metrics.gather_metrics(&text)?;
		}
		Ok(())
	}

	pub fn _append_to_content(&mut self, text: &str) -> Result<(), std::io::Error> {
		self.content.items.push(text.to_string());
		let len = self.content.items.len();
		if len > self.max_content {
			self.content.items = self.content.items.split_off(len - self.max_content);
		} else {
			self.content.state.select(Some(len - 1));
		}
		Ok(())
	}

	// Some logfile lines are too numerous to include so we ignore them
	// Returns true if the line is to be processed
	fn line_filter(&mut self, _line: &str) -> bool {
		true
	}
}

use regex::Regex;
lazy_static::lazy_static! {
	static ref LOG_LINE_PATTERN: Regex =
		Regex::new(r"\[(?P<time_string>[^ ]{27}) (?P<category>[A-Z]{4,6}) (?P<source>.*)\](?P<message>.*)").expect("The regex failed to compile. This is a bug.");
}

#[derive(PartialEq)]
pub enum NodeStatus {
	Started,
	Connecting,
	Connected,
	Stopped,
}

pub fn node_status_as_string(node_status: &NodeStatus) -> String {
	match node_status {
		NodeStatus::Connecting => "Connecting".to_string(),
		NodeStatus::Connected => "Connected".to_string(),
		NodeStatus::Stopped => "Stopped".to_string(),
		NodeStatus::Started => "Started".to_string(),
	}
}

pub struct MmmStat {
	sample_count:	u64,

	pub most_recent:	u64,
	pub total:	u64,
	pub min:	u64,
	pub mean:	u64,
	pub max:	u64,
}

impl MmmStat {
	pub fn new() -> MmmStat {
		MmmStat {
			sample_count:	0,
			most_recent: 	0,
			total: 	0,
			min: 	u64::MAX,
			mean:	0,
			max:	0,
		}
	}

	pub fn add_sample(&mut self, value: u64) {
		self.most_recent = value;
		self.sample_count += 1;
		self.total += value;
		self.mean = self.total / self.sample_count;

		if self.min > value || self.min == u64::MAX {
			self.min = value;
		}
		if self.max < value { self.max = value; }
	}
}

pub struct NodeMetrics {
	pub node_started: Option<DateTime<Utc>>,
	pub running_message: Option<String>,
	pub running_version: Option<String>,
	pub category_count: HashMap<String, usize>,
	pub activity_history: Vec<ActivityEntry>,

	pub app_timelines: AppTimelines,

	pub entry_metadata: Option<LogMeta>,
	pub node_status: NodeStatus,
	pub node_inactive: bool,

	pub activity_gets: MmmStat,
	pub activity_puts: MmmStat,
	pub activity_errors: MmmStat,
	pub storage_payments: MmmStat,
	pub storage_cost: MmmStat,
	pub peers_connected: MmmStat,
	pub memory_used_mb: MmmStat,

	pub used_space: u64,
	pub max_capacity: u64,

	pub system_cpu: f32,
	pub system_memory: f32,
	pub system_memory_used_mb: f32,
	pub system_memory_usage_percent: f32,

	pub interface_name: String,
	pub bytes_received: u64,
	pub bytes_transmitted: u64,
	pub total_mb_received: f32,
	pub total_mb_transmitted: f32,

	pub cpu_usage_percent:	f32,
	pub cpu_usage_percent_max:	f32,
	pub bytes_read: u64,
	pub bytes_written: u64,
	pub total_mb_read: f32,
	pub total_mb_written: f32,

	pub debug_logfile: Option<NamedTempFile>,
	parser_output: String,
}

impl NodeMetrics {
	fn new(opt: &Opt) -> NodeMetrics {
		let mut metrics = NodeMetrics {
			// Start
			node_started: None,
			running_message: None,
			running_version: None,

			// Logfile entries
			activity_history: Vec::<ActivityEntry>::new(),
			entry_metadata: None,

			// A predefined set of Timelines (Sparklines)
			app_timelines: AppTimelines::new(opt),

			// Counts
			category_count: HashMap::new(),
			activity_gets: MmmStat::new(),
			activity_puts: MmmStat::new(),
			activity_errors: MmmStat::new(),

			// Storage Payments
			storage_payments: MmmStat::new(),
			storage_cost: MmmStat::new(),
			peers_connected: MmmStat::new(),

			// State (node)
			node_status: NodeStatus::Stopped,
			node_inactive: false,

			// State (network)

			// Disk use:
			used_space: 0,
			max_capacity: 0,


			system_cpu: 0.0,
			system_memory: 0.0,
			system_memory_used_mb: 0.0,
			system_memory_usage_percent: 0.0,

			interface_name: String::from("unknown"),
			bytes_received: 0,
			bytes_transmitted: 0,
			total_mb_received: 0.0,
			total_mb_transmitted: 0.0,

			memory_used_mb: MmmStat::new(),
			cpu_usage_percent: 0.0,
			cpu_usage_percent_max: 0.0,
			bytes_read: 0,
			bytes_written: 0,
			total_mb_read: 0.0,
			total_mb_written: 0.0,

			// Debug
			debug_logfile: None,
			parser_output: String::from("-"),
		};
		metrics.update_timelines(&Utc::now());
		metrics
	}

	pub fn is_node_active(&self) -> bool {return !self.node_inactive;}

	pub fn get_node_status_string(&mut self) -> String {
		let node_inactive_timeout = Duration::seconds(NODE_INACTIVITY_TIMEOUT_S);

		let mut node_status_string = node_status_as_string(&self.node_status);

		if let Some(metadata) = &self.entry_metadata {
			let idle_time = Utc::now() - metadata.system_time;
			if idle_time > node_inactive_timeout {
				self.node_inactive = true;
				node_status_string = format!("INACTIVE ({})", get_duration_text(idle_time));
			} else {
				self.node_inactive = false;
			}
		}

		return node_status_string;
	}

	fn reset_metrics(&mut self) {
		self.node_status = NodeStatus::Started;
		self.activity_gets = MmmStat::new();
		self.activity_puts = MmmStat::new();
		self.activity_errors = MmmStat::new();
		self.storage_cost = MmmStat::new();
		self.peers_connected = MmmStat::new();
		self.memory_used_mb = MmmStat::new();
	}

	///! Process a line from a SAFE Node logfile.
	///! Use a created LogMeta to update metrics.
	pub fn gather_metrics(&mut self, line: &str) -> Result<(), std::io::Error> {
		self.parser_output = format!("LogMeta::decode_metadata() failed on: {}", line); // For debugging
		// debug_log!(&self.parser_output.clone());

		if let Some(metadata) = LogEntry::decode_metadata(line) {
			self.entry_metadata = Some(metadata);
		}

		if self.entry_metadata.is_none() {
			// debug_log!("gather_metrics() - skipping bec. metadata missing");
			return Ok(());	// Skip until start of first log message
		}

		let entry = LogEntry { logstring: String::from(line) };
		let entry_metadata = self.entry_metadata.as_ref().unwrap().clone();
		let entry_time = entry_metadata.message_time;

		debug_log!(format!("gather_metrics() entry_time: {:?}", entry_time).as_str());

		self.update_timelines(&entry_time);
		self.parser_output = entry_metadata.parser_output.clone();
		self.process_logfile_entry(&entry.logstring, &entry_metadata); // May overwrite self.parser_output

		// --debug-dashboard - prints parser results for a single logfile
		// to a temp logfile which is displayed in the adjacent window.
		debug_log!(&self.parser_output.clone());

		Ok(())
	}

	pub fn update_timelines(&mut self, now: &DateTime<Utc>) {
		self.app_timelines.update_timelines(now);
	}

	///! Return a LogMeta and capture metadata for logfile node start:
	///!	'Running sn_node v0.74.4'
	pub fn parse_start(&mut self, line: &String, entry_metadata: &LogMeta) -> bool {
		let running_prefix = String::from("Running safenode ");

		if line.starts_with(&running_prefix) {
			self.node_status = NodeStatus::Started;
			let message = line.to_string();
			let version = String::from(line[running_prefix.len()..].to_string());
			self.node_started = Some(entry_metadata.message_time);
			self.parser_output = format!(
				"START node {} at {}",
				String::from(version.clone()),
				self.node_started
					.map_or(String::from("None"), |m| format!("{}", m))
			);

			self.running_message = Some(message);
			self.running_version = Some(version);
			self.reset_metrics();
			return true;
		}

		false
	}

	///! Process a logfile entry
	///! Returns true if the line has been processed and can be discarded
	pub fn process_logfile_entry(&mut self, line: &String, entry_metadata: &LogMeta) -> bool {
		return self.parse_data_response(
			&line,
			"Running as Node: SendToSection [ msg: MsgEnvelope { message: QueryResponse { response: QueryResponse::",
		)
		|| self.parse_timed_data(&line, &entry_metadata.message_time)
		|| self.parse_states(&line, &entry_metadata)
		|| self.parse_start(&line, &entry_metadata);
	}

	fn parse_timed_data(&mut self, line: &String, entry_time: &DateTime<Utc>) -> bool {
		if line.contains("Retrieved record from disk") {
			self.count_get(&entry_time);
			self.node_status = NodeStatus::Connected;
			return true;
		} else if line.contains("Wrote record") {
			self.count_put(&entry_time);
			self.node_status = NodeStatus::Connected;
			return true;
		} else if line.contains("Editing Register success") {
			self.count_put(&entry_time);
			self.node_status = NodeStatus::Connected;
			return true;
		} else if line.contains("Cost is now") {
			if let Some(storage_cost) = self.parse_u64("Cost is now ", line) {
				self.count_storage_cost(entry_time, storage_cost);
				self.parser_output = format!("Storage cost: {}", storage_cost);
			};
			return true;
		} else if line.contains("nanos accepted for record") {
			if 	let Some(storage_payment) = self.parse_u64("Payment of NanoTokens(", line) {
				self.count_storage_payment(entry_time, storage_payment);
				self.parser_output = format!("Payment received: {}", storage_payment);
				return true;
			};
		} else if line.contains("connected peers") {
			let mut parser_output = String::from("connected peers:");
			if let Some(peers_connected) = self.parse_u64("now we have #", line) {
				self.count_peers_connected(entry_time, peers_connected);
				parser_output = format!("{} {}", &parser_output, peers_connected);
			};
			self.parser_output = parser_output;
			return true;
		}
		return false;
	}

	///! Update data metrics from a handler response logfile entry
	///! Returns true if the line has been processed and can be discarded
	fn parse_data_response(&mut self, line: &String, pattern: &str) -> bool {
		if let Some(mut response_start) = line.find(pattern) {
			response_start += pattern.len();
			let mut response = "";

			if let Some(response_end) = line[response_start..].find(",") {
				response = line.as_str()[response_start..response_start + response_end]
					.as_ref();
				if !response.is_empty() {
					let activity_entry = ActivityEntry::new(&line, &self.entry_metadata.as_ref().unwrap(), response);
					self.activity_history.push(activity_entry);
					self.parser_output = format!("node activity: {}", response);
				}
			}
			if response.is_empty() {
				self.parser_output = format!("failed to parse_data_response: {}", line);
			};

			return true;
		};
		return false;
	}

	///! Capture state updates from a logfile entry
	///! Returns true if the line has been processed and can be discarded
	fn parse_states(&mut self, line: &String, entry_metadata: &LogMeta) -> bool {
		if entry_metadata.category.eq("ERROR") {
			self.count_error(&entry_metadata.message_time);
		}

		let &content = &line.as_str();

		// Node Status
		if content.contains("Getting closest peers") {
			self.node_status = NodeStatus::Connecting;
			self.parser_output = String::from("Node status: Connecting");
			return true;
		}

		if content.contains("Connected to the Network") {
			self.node_status = NodeStatus::Connected; // Also set by some other matches
			self.parser_output = String::from("Node status: Connected");
			return true;
		}

		if content.contains("Node events channel closed") {
			self.node_status = NodeStatus::Stopped;
			self.parser_output = String::from("Node status: Disconnected");
			return true;
		}

		if content.contains("Skipping ") {
			let mut parser_output = String::from("Connected ({} lag)");
			if let Some(events_skipped) = self.parse_usize("Skipping ", content) {
				parser_output = format!("{} ({})", &parser_output, events_skipped);
			};
			self.parser_output = parser_output;
			return true;
		}

		// Metrics
		if content.contains("sn_logging::metrics") {
			// System
			let mut parser_output = String::from("system_cpu_usage_percent:");
			if let Some(system_cpu) = self.parse_float32("system_cpu_usage_percent\":", content) {
				self.system_cpu = system_cpu;
				parser_output = format!("{} gl_cpu: {}", &parser_output, system_cpu);
			};
			if let Some(system_memory) = self.parse_float32("system_total_memory_mb\":", content) {
				self.system_memory = system_memory;
				parser_output = format!("{} , System Memory: {}", &parser_output, system_memory);
			};
			if let Some(system_memory_used_mb) = self.parse_float32("system_memory_used_mb\":", content) {
				self.system_memory_used_mb = system_memory_used_mb;
				parser_output = format!("{} , System Memory Use (MB): {}", &parser_output, system_memory_used_mb);
			};
			if let Some(system_memory_usage_percent) = self.parse_float32("system_memory_usage_percent\":", content) {
				self.system_memory_usage_percent = system_memory_usage_percent;
				parser_output = format!("{} , System Memory Use (%): {}", &parser_output, system_memory_usage_percent);
			};

			// Networking
			if let Some(interface_name) = self.parse_word("interface_name\":", content) {
				self.interface_name = String::from(interface_name.clone());
				parser_output = format!("{} , interface_name: {}", &parser_output, interface_name);
			};
			if let Some(bytes_received) = self.parse_u64("bytes_received\":", content) {
				self.bytes_received = bytes_received;
				parser_output = format!("{} , bytes_received: {}", &parser_output, bytes_received);
			};
			if let Some(bytes_transmitted) = self.parse_u64("bytes_transmitted\":", content) {
				self.bytes_transmitted = bytes_transmitted;
				parser_output = format!("{} , bytes_transmitted: {}", &parser_output, bytes_transmitted);
			};
			if let Some(total_mb_received) = self.parse_float32("total_mb_received\":", content) {
				self.total_mb_received = total_mb_received;
				parser_output = format!("{} , total_mb_received: {}", &parser_output, total_mb_received);
			};
			if let Some(total_mb_transmitted) = self.parse_float32("total_mb_transmitted\":", content) {
				self.total_mb_transmitted = total_mb_transmitted;
				parser_output = format!("{} , total_mb_transmitted: {}", &parser_output, total_mb_transmitted);
			};

			// Node Resources
			if let Some(cpu_usage_percent) = self.parse_float32("\"cpu_usage_percent\":", content) {

				self.cpu_usage_percent = cpu_usage_percent;
				if cpu_usage_percent > self.cpu_usage_percent_max {
					self.cpu_usage_percent_max = cpu_usage_percent;
				}
				parser_output = format!("{}  cpu: {}, cpu_max {}", &parser_output, cpu_usage_percent, self.cpu_usage_percent_max);
			};
			if let Some(memory_used_mb) = self.parse_float32("\"memory_used_mb\":", content) {
				self.count_memory_used_mb(&entry_metadata.message_time, memory_used_mb as u64);
				parser_output = format!("{} , memory: {}", &parser_output, memory_used_mb);
			};
			if let Some(bytes_read) = self.parse_u64("bytes_read\":", content) {
				self.bytes_read = bytes_read;
				parser_output = format!("{} , bytes_read: {}", &parser_output, bytes_read);
			};
			if let Some(bytes_written) = self.parse_u64("bytes_written\":", content) {
				self.bytes_written = bytes_written;
				parser_output = format!("{} , bytes_written: {}", &parser_output, bytes_written);
			};
			if let Some(total_mb_read) = self.parse_float32("total_mb_read\":", content) {
				self.total_mb_read = total_mb_read;
				parser_output = format!("{} , total_mb_read: {}", &parser_output, total_mb_read);
			};
			if let Some(total_mb_written) = self.parse_float32("total_mb_written\":", content) {
				self.total_mb_written = total_mb_written;
				parser_output = format!("{} , total_mb_written: {}", &parser_output, total_mb_written);
			};

			self.parser_output = parser_output;
			return true;
		}

		// Overall storage use / size
		if let Some(used_space) = self.parse_u64("Used space:", content) {
			self.used_space = used_space;
			self.parser_output = format!("Used space: {}", used_space);
			return true;
		};
		if let Some(max_capacity) = self.parse_u64("Max capacity:", content) {
			self.max_capacity = max_capacity;
			self.parser_output = format!("Max capacity: {}", max_capacity);
			return true;
		};

		false
	}

	fn parse_usize(&mut self, prefix: &str, content: &str) -> Option<usize> {
		if let Some(position) = content.find(prefix) {
			let word: Vec<&str> = content[position + prefix.len()..]
				.trim()
				.splitn(2, |c| c == ' ' || c == ',' || c== '}')
				.collect();
			if word.len() > 0 {
				match word[0].parse::<usize>() {
					Ok(value) => return Some(value),
					Err(_e) => self.parser_output = format!("failed to parse '{}' as usize from: '{}'", word[0], &content[position + prefix.len()..]),
				}
			}
		}
		None
	}

	fn parse_u64(&mut self, prefix: &str, content: &str) -> Option<u64> {
		if let Some(position) = content.find(prefix) {
			let word: Vec<&str> = content[position + prefix.len()..]
				.trim()
				.splitn(2, |c| c == ' ' || c == ',' || c== '}' || c== ')')
				.collect();
			if word.len() > 0 {
				match word[0].parse::<u64>() {
					Ok(value) => return Some(value),
					Err(_e) => self.parser_output = format!("failed to parse '{}' as u64 from: '{}'", word[0], &content[position + prefix.len()..]),
				}
			}
		}
		None
	}

	fn parse_float32(&mut self, prefix: &str, content: &str) -> Option<f32> {
		if let Some(position) = content.find(prefix) {
			let word: Vec<&str> = content[position + prefix.len()..]
				.trim()
				.splitn(2, |c| c == ' ' || c == ',' || c== '}')
				.collect();
			if word.len() > 0 {
				match word[0].parse::<f32>() {
					Ok(value) => return Some(value),
					Err(_e) => self.parser_output = format!("failed to parse '{}' as float from: '{}'", word[0], &content[position + prefix.len()..]),
				}
			}
		}
		None
	}

	fn parse_word(&mut self, prefix: &str, content: &str) -> Option<String> {
		if let Some(start) = content.find(prefix) {
			let word: Vec<&str> = content[start + prefix.len()..]
				.trim_start()
				.splitn(2, |c| c == ' ' || c == ',' || c== '}')
				.collect();
			if word.len() > 0 {
				return Some(word[0].to_string());
			} else {
				self.parser_output = format!("failed to parse word at: '{}'", &content[start..]);
			}
		}
		None
	}

	fn count_get(&mut self, time: &DateTime<Utc>) {
		self.activity_gets.add_sample(1);
		self.apply_timeline_sample(GETS_TIMELINE_KEY, time, 1);
	}

	fn count_put(&mut self, time: &DateTime<Utc>) {
		self.activity_puts.add_sample(1);
		self.apply_timeline_sample(PUTS_TIMELINE_KEY, time, 1);
	}

	fn count_error(&mut self, time: &DateTime<Utc>) {
		self.activity_errors.add_sample(1);
		self.apply_timeline_sample(ERRORS_TIMELINE_KEY, time, 1);
	}

	fn count_storage_payment(&mut self, time: &DateTime<Utc>, storage_payment: u64) {
		self.storage_payments.add_sample(storage_payment);
		self.apply_timeline_sample(EARNINGS_TIMELINE_KEY, time, storage_payment);
	}

	fn count_storage_cost(&mut self, time: &DateTime<Utc>, storage_cost: u64) {
		self.storage_cost.add_sample(storage_cost);
		self.apply_timeline_sample(STORAGE_COST_TIMELINE_KEY, time, storage_cost);
	}

	fn count_peers_connected(&mut self, time: &DateTime<Utc>, connections: u64) {
		self.peers_connected.add_sample(connections);
		self.apply_timeline_sample(CONNECTIONS_TIMELINE_KEY, time, connections);
	}

	fn count_memory_used_mb(&mut self, time: &DateTime<Utc>, memory_used_mb: u64) {
		self.memory_used_mb.add_sample(memory_used_mb);
		self.apply_timeline_sample(RAM_TIMELINE_KEY, time, memory_used_mb);
	}

	fn apply_timeline_sample(&mut self, timeline_key: &str, time: &DateTime<Utc>, value: u64) {
		if let Some(timeline) = self.app_timelines.get_timeline_by_key(timeline_key) {
				timeline.update_value(time, value);
		}
	}
}

///! Node activity for node activity_history
pub struct ActivityEntry {
	pub message: String,
	pub activity: String,
	pub logstring: String,
	pub category: String, // First word, "Running", "INFO", "WARN" etc
	pub message_time: DateTime<Utc>,
	pub source: String,

	pub parser_output: String,
}

impl ActivityEntry {
	pub fn new(line: &String, entry_metadata: &LogMeta, activity: &str) -> ActivityEntry {
		ActivityEntry {
			message: entry_metadata.message.clone(),
			activity: activity.to_string(),
			logstring: line.clone(),
			category: entry_metadata.category.clone(),
			message_time: entry_metadata.message_time,
			source: entry_metadata.source.clone(),

			parser_output: String::from(""),
		}
	}
}


///! Metadata for a logfile line
#[derive(Clone)]
pub struct LogMeta {
	pub category: String, // First word ('INFO', 'WARN' etc.)
	pub message_time: DateTime<Utc>,
	pub system_time: DateTime<Utc>,
	pub source: String,
	pub message: String,

	pub parser_output: String,
}

impl LogMeta {
	pub fn clone(&self) -> LogMeta {
		LogMeta {
			category: self.category.clone(),
			message_time: self.message_time,
			system_time: self.system_time,
			source: self.source.clone(),
			message: self.message.clone(),
			parser_output: self.parser_output.clone(),
		}
	}
}
///! Used to build a history of what is in the log, one LogMeta per line
pub struct LogEntry {
	pub logstring: String,			// One line of raw text from the logfile
}

impl LogEntry {
	///! Decode metadata from logfile line when present. Example input lines:
	///! " INFO 2022-01-15T20:21:02.659471Z [sn/src/node/routing/core/mod.rs:L211]:"
	///! "	 ➤ Writing our latest PrefixMap to disk"
	///! " ERROR 2022-01-15T20:21:07.643598Z [sn/src/node/routing/api/dispatcher.rs:L450]:"
	fn decode_metadata(line: &str) -> Option<LogMeta> {
		if line.is_empty() {
			return None;
		}

		if let Some(captures) = LOG_LINE_PATTERN.captures(line) {
			let category = captures.name("category").map_or("", |m| m.as_str());
			let time_string = captures.name("time_string").map_or("", |m| m.as_str());
			let source = captures.name("source").map_or("", |m| m.as_str());
			let message = captures.name("message").map_or("", |m| m.as_str());
			let time_str: String;

			let time_utc: DateTime<Utc>;

			match DateTime::parse_from_str(time_string, "%+") {
				Ok(time) => {
					time_utc = time.with_timezone(&Utc);
					time_str = format!("{}", time);
				}
				Err(e) => {
					debug_log!(format!("ERROR parsing logfile time: {}", e).as_str());
					return None
				}
			};
			let parser_output = format!(
				"c: {}, t: {}, s: {}, m: {}",
				category, time_str, source, message
			);

			return Some(LogMeta {
				category: String::from(category),
				message_time: time_utc,
				system_time: Utc::now(),
				source: String::from(source),
				message: String::from(message),
				parser_output,
			});
		}
		None
	}
}

///! Active UI at top level
#[derive(PartialEq)]
pub enum DashViewMain {
	DashSummary,
	DashNode,
	DashHelp,
	DashDebug,
}

pub struct DashState {
	pub main_view: DashViewMain,
	pub logfile_names: Vec<String>,
	pub active_timescale: usize,
	pub node_logfile_visible: bool,
	pub dash_node_focus: String,
    pub mmm_ui_mode:   MinMeanMax,
    pub top_timeline: usize,  // Timeline to show at top of UI

	pub summary_window_heading: String,
	pub summary_window_list: StatefulList<String>,
	max_summary_window: usize,

	pub help_status: StatefulList<String>,

	// For --debug-window option
	pub debug_window_list: StatefulList<String>,
	pub debug_window: bool,
	pub debug_window_has_focus: bool,
	max_debug_window: usize,
}

impl DashState {
	pub fn new() -> DashState {

		DashState {
			main_view: DashViewMain::DashSummary,
			logfile_names: Vec::<String>::new(),
			active_timescale: 0,
			node_logfile_visible: true,
			dash_node_focus: String::new(),
			mmm_ui_mode: MinMeanMax::Mean,
            top_timeline: 0,

			summary_window_heading: String::from(""),
			summary_window_list: StatefulList::new(),
			max_summary_window: 1000,

			help_status: StatefulList::with_items(vec![]),

			debug_window: false,
			debug_window_has_focus: false,
			debug_window_list: StatefulList::new(),
			max_debug_window: 100,
		}
	}

	// TODO this regenerates every line. May be worth just updating the line for the updated node/monitor
	pub fn update_summary_window(&mut self, monitors: &mut HashMap<String, LogMonitor>) {
		let current_selection = self.summary_window_list.state.selected();
		self.summary_window_list = StatefulList::new();

		let earnings_heading = format!("Earned ({})", crate::custom::app_timelines::EARNINGS_UNITS_TEXT);

		self.summary_window_heading = format!("{:>4} {:>15}{:>12} {:>11} {:>11} {:>11} {:>11} {:>11} {:>24}",
			String::from("Node"),
			earnings_heading,
			String::from("StoreCost"),
			String::from("PUTS"),
			String::from("GETS"),
			String::from("Errors"),
			String::from("Connections"),
			String::from("MB RAM"),
			String::from("Status"),
		);

		for i in 0..self.logfile_names.len() {
			let filepath = self.logfile_names[i].clone();
			if let Some(monitor) = monitors.get_mut(&filepath) {
				if !monitor.is_debug_dashboard_log {
					let node_status_string = monitor.metrics.get_node_status_string();
					let node_summary = format!("{:>4} {:>15}{:>12} {:>11} {:>11} {:>11} {:>11} {:>11} {:>24}",
						monitor.index + 1,
						monitor.metrics.storage_payments.total.to_string(),
						monitor.metrics.storage_cost.most_recent,
						monitor.metrics.activity_puts.total,
						monitor.metrics.activity_gets.total,
						monitor.metrics.activity_errors.total,
						monitor.metrics.peers_connected.most_recent,
						monitor.metrics.memory_used_mb.most_recent,
						node_status_string
					);
					self.append_to_summary_window(&node_summary);
				}
			}
		}

		self.summary_window_list.state.select(current_selection);
	}

	fn append_to_summary_window(&mut self, text: &str){
		self.summary_window_list.items.push(text.to_string());

		let len = self.summary_window_list.items.len();

		if len > self.max_summary_window {
			self.summary_window_list.items = self
				.summary_window_list
				.items
				.split_off(len - self.max_summary_window);
		} else {
			self.summary_window_list.state.select(Some(len - 1));
		}

	}

	pub fn _debug_window(&mut self, text: &str) {
		self.debug_window_list.items.push(text.to_string());
		let len = self.debug_window_list.items.len();

		if len > self.max_debug_window {
			self.debug_window_list.items = self
				.debug_window_list
				.items
				.split_off(len - self.max_debug_window);
		} else {
			self.debug_window_list.state.select(Some(len - 1));
		}
	}

	pub fn get_active_timescale_name(&self) -> Option<&'static str> {
		return match TIMESCALES.get(self.active_timescale) {
			None => {
				// debug_log!("ERROR getting active timescale name");
				return None;
			}
			Some((name, _)) => Some(name),
		};
	}

    // Rotate UI display state through Min, Mean, Max values
    pub fn bump_mmm_ui_mode(&mut self) {
        match &self.mmm_ui_mode {
            MinMeanMax::Min => self.mmm_ui_mode = MinMeanMax::Mean,
            MinMeanMax::Mean => self.mmm_ui_mode = MinMeanMax::Max,
            MinMeanMax::Max => self.mmm_ui_mode = MinMeanMax::Min,
        }
    }

	pub fn top_timeline_index(&self)  -> usize { return self.top_timeline; }
	pub fn mmm_ui_mode(&self) -> &MinMeanMax { &self.mmm_ui_mode }

}

pub struct DashVertical {
	_active_view: usize,
}

impl DashVertical {
	pub fn new() -> Self {
		DashVertical { _active_view: 0 }
	}
}

pub fn set_main_view(view: DashViewMain, app: &mut App) {
	if app.dash_state.main_view == view {
		return;
	}

	save_focus(app);
	app.dash_state.main_view = view;
	restore_focus(app);
}

pub fn save_focus(app: &mut App) {
	match app.dash_state.main_view {
		DashViewMain::DashSummary => {} // TODO
		DashViewMain::DashHelp => {} // TODO
		DashViewMain::DashNode => {
			if let Some(focus) = app.get_logfile_with_focus() {
				app.dash_state.dash_node_focus = focus;
			}
		}
		DashViewMain::DashDebug => {}
	}
}

pub fn restore_focus(app: &mut App) {
	match app.dash_state.main_view {
		DashViewMain::DashSummary => {} // TODO
		DashViewMain::DashHelp => {} // TODO
		DashViewMain::DashNode => {
			app.set_logfile_with_focus(app.dash_state.dash_node_focus.clone())
		}
		DashViewMain::DashDebug => {
			if let Some(debug_logfile) = app.get_debug_dashboard_logfile() {
				app.set_logfile_with_focus(debug_logfile);
			}
		}
	}
}
