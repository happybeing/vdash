///! Application logic
//
// TODO consider colouring logfiles using regex's from https://github.com/bensadeh/tailspin

use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::Path;

use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};
use structopt::StructOpt;
use tempfile::NamedTempFile;

use crate::shared::util::StatefulList;

use super::timelines::{MinMeanMax, get_duration_text};
use super::app_timelines::{AppTimelines, TIMESCALES, APP_TIMELINES};
use super::app_timelines::{STORAGE_COST_TIMELINE_KEY, EARNINGS_TIMELINE_KEY, PUTS_TIMELINE_KEY, GETS_TIMELINE_KEY, CONNECTIONS_TIMELINE_KEY, RAM_TIMELINE_KEY, ERRORS_TIMELINE_KEY};
use super::opt::{Opt, MIN_TIMELINE_STEPS};
use super::logfiles_manager::LogfilesManager;
use super::logfile_checkpoints::save_checkpoint;

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

lazy_static::lazy_static! {
	pub static ref OPT: Mutex<Opt> =
		Mutex::<Opt>::new(Opt::from_args());
}

pub struct App {
	pub dash_state: DashState,
	pub monitors: HashMap<String, LogMonitor>,
	pub logfile_with_focus: String,

	pub logfiles_manager: LogfilesManager,
	pub next_glob_scan: Option<DateTime<Utc>>,
}

impl App {
	pub async fn new() -> Result<App, std::io::Error> {
		let (opt_files, opt_globpaths, opt_debug_window, opt_timeline_steps) = {
			let opt = OPT.lock().unwrap();
			(opt.files.clone(), opt.glob_paths.clone(), opt.debug_window, opt.timeline_steps)
		};

		let mut app = App {
			dash_state: DashState::new(),
			monitors: HashMap::new(),
			logfile_with_focus: String::new(),

			logfiles_manager: LogfilesManager::new(opt_globpaths.clone()),
			next_glob_scan: None,
		};

		if opt_files.is_empty() && opt_globpaths.is_empty() {
			eprintln!("{}: no logfile(s) or 'glob' paths provided.", Opt::clap().get_name());
			return exit_with_usage("missing logfiles");
		}

		if opt_timeline_steps < MIN_TIMELINE_STEPS {
			eprintln!(
				"Timeline steps number is too small, minimum is {}",
				MIN_TIMELINE_STEPS
			);
			return exit_with_usage("invalid parameter");
		}

		let mut dash_state = DashState::new();
		dash_state.debug_window = opt_debug_window;
		if opt_debug_window {
			dash_state.main_view = DashViewMain::DashDebug;
		}

		let mut files_to_load = opt_files.clone();

		if opt_debug_window {
			if opt_files.len() == 0 {
				eprint!("For debugging with --debug-window you must specify a logfile path.");
				return exit_with_usage("missing logfile");
			}

			// For debug: only use first logfile, plus one for debug messages
			files_to_load = opt_files[0..1].to_vec();
			let debug_file = NamedTempFile::new()?;
			let path = debug_file.path();
			let path_str = path
				.to_str()
				.ok_or_else(|| Error::new(ErrorKind::Other, "invalid path"))?;
			files_to_load.push(String::from(path_str));
			*DEBUG_LOGFILE.lock().unwrap() = Some(debug_file);
		}

		if files_to_load.len() > 0 {
			app.logfiles_manager.monitor_multi_paths(files_to_load, &mut app.monitors, &mut app.dash_state, false).await;
		}

		app.scan_glob_paths(false, false).await;

		if app.logfiles_manager.logfiles_added.len() > 0 {
			app.logfile_with_focus = app.logfiles_manager.logfiles_added[0].clone();	// Save to give focus
		} else {
			app.dash_state.vdash_status.message(&"No files to monitor, please start a node and try again.".to_string(), None);
			return exit_with_usage("no files to monitor.");
		}

		app.update_timelines(&Utc::now());
		app.update_summary_window();

		if !app.logfile_with_focus.is_empty() {
			app.dash_state.dash_node_focus = app.logfile_with_focus.clone();
		}

		app.set_logfile_with_focus(app.logfile_with_focus.clone());
		app.dash_state.vdash_status.disable_to_console();
		Ok(app)
	}

	pub async fn scan_glob_paths(&mut self, timed: bool, disable_status: bool) {
		if self.logfiles_manager.globpaths.len() == 0 { return; }
		let opt_globs_scan = OPT.lock().unwrap().glob_scan;

		let mut do_scan = !timed;
		if timed && opt_globs_scan > 0 {
			let current_time = Utc::now();
			if let Some(next_glob_scan) = self.next_glob_scan {
				if current_time > next_glob_scan {
					self.next_glob_scan = Some(current_time + Duration::seconds(opt_globs_scan));
					do_scan = true;
				}
			} else {
				self.next_glob_scan = Some(current_time + Duration::seconds(opt_globs_scan));
				do_scan = true;
			}
		}

		if do_scan {
			let opt_glob_paths = OPT.lock().unwrap().glob_paths.clone();
			self.logfiles_manager.scan_multi_globpaths(opt_glob_paths, &mut self.monitors, &mut self.dash_state, disable_status).await;
		}
	}

	pub fn update_timelines(&mut self, now: &DateTime<Utc>) {
		for (_monitor_file, monitor) in self.monitors.iter_mut() {
			monitor.metrics.update_timelines(now);
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
		if logfile_name.len() == 0 { return; }

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
		if self.logfiles_manager.logfiles_added.len() == 0 { return; }

		let opt_debug_window = { let opt = OPT.lock().unwrap(); opt.debug_window };

		if self.dash_state.main_view == DashViewMain::DashDebug {
			return;
		}

		if self.dash_state.main_view == DashViewMain::DashSummary {
			if self.dash_state.summary_window_heading_selected < self.dash_state.summary_window_headings.items.len() - 1 {
				self.dash_state.summary_window_heading_selected += 1;
				self.update_summary_window();
			}
		}

		let mut next_i = 0;
		for (i, name) in self.logfiles_manager.logfiles_added.iter().enumerate() {
			if name == &self.logfile_with_focus {
				if i < self.logfiles_manager.logfiles_added.len() - 1 {
					next_i = i + 1;
				}
				break;
			}
		}

		if next_i == 0 && opt_debug_window && self.logfile_with_focus != DEBUG_WINDOW_NAME {
			self.set_logfile_with_focus(DEBUG_WINDOW_NAME.to_string());
			return;
		}

		let logfile = self.logfiles_manager.logfiles_added[next_i].to_string();
		self.set_logfile_with_focus(logfile.clone());

		if let Some(debug_logfile) = self.get_debug_dashboard_logfile() {
			if logfile.eq(&debug_logfile) {
				self.change_focus_next();
			}
		}
	}

	pub fn change_focus_previous(&mut self) {
		if self.logfiles_manager.logfiles_added.len() == 0 { return; }

		let opt_debug_window = { let opt = OPT.lock().unwrap(); opt.debug_window };

		if self.dash_state.main_view == DashViewMain::DashDebug {
			return;
		}

		if self.dash_state.main_view == DashViewMain::DashSummary {
			if self.dash_state.summary_window_heading_selected > 0 {
				self.dash_state.summary_window_heading_selected -= 1;
				self.update_summary_window();
			}
		}

		let len = self.logfiles_manager.logfiles_added.len();
		let mut previous_i = len - 1;
		for (i, name) in self.logfiles_manager.logfiles_added.iter().enumerate() {
			if name == &self.logfile_with_focus {
				if i > 0 {
					previous_i = i - 1;
				}
				break;
			}
		}

		if opt_debug_window
			&& previous_i == len - 1
			&& self.logfile_with_focus != DEBUG_WINDOW_NAME
		{
			self.set_logfile_with_focus(DEBUG_WINDOW_NAME.to_string());
			return;
		}

		let logfile = self.logfiles_manager.logfiles_added[previous_i].to_string();
		self.set_logfile_with_focus(logfile.clone());

		if let Some(debug_logfile) = self.get_debug_dashboard_logfile() {
			if logfile.eq(&debug_logfile) {
				self.change_focus_previous();
			}
		}
	}

	pub fn change_focus_to(&mut self, logfile_index: usize) {
		if logfile_index < self.logfiles_manager.logfiles_added.len() {
			self.set_logfile_with_focus(self.logfiles_manager.logfiles_added[logfile_index].clone());
			self.dash_state.main_view = DashViewMain::DashNode;
		}
	}

	pub fn handle_arrow_up(&mut self)   { self.handle_arrow(false); }

	pub fn handle_arrow_down(&mut self) { self.handle_arrow( true); }

	pub fn handle_arrow(&mut self, is_down: bool) {
		if self.logfiles_manager.logfiles_added.len() == 0 { return; }

		let opt_debug_window = { let opt = OPT.lock().unwrap(); opt.debug_window };

		let list = match self.dash_state.main_view {
			DashViewMain::DashSummary => { Some(&mut self.dash_state.summary_window_rows) }
			DashViewMain::DashNode => {
				if let Some(monitor) = self.get_monitor_with_focus() {
					Some(&mut monitor.content)
				} else if opt_debug_window {
					Some(&mut self.dash_state.debug_window_list)
				} else {
					None
				}
			}
			DashViewMain::DashHelp => { None }
			DashViewMain::DashDebug => {
				if opt_debug_window {
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
		if self.logfiles_manager.logfiles_added.len() == 0 { return; }

		if self.dash_state.main_view == DashViewMain::DashSummary {
			if let Some(selected_index) = self.dash_state.summary_window_rows.state.selected() {
				let selected_logfile = &self.dash_state.logfile_names_sorted[selected_index];
				if let Some(node_index) = self.logfiles_manager.logfiles_added.iter().position(|s| s == selected_logfile.as_str()) {
					self.change_focus_to(node_index);
				}
			}
		} else if self.dash_state.main_view == DashViewMain::DashNode {
			for index in 0..self.dash_state.logfile_names_sorted.len() {
				if self.dash_state.logfile_names_sorted[index] == self.logfile_with_focus {
					self.dash_state.summary_window_rows.state.select(Some(index));
					break;
				}
			}

			if let Some(monitor) = self.get_monitor_with_focus() {
				let selected_logfile = monitor.logfile.clone();
				if let Some(node_index) = self.dash_state.logfile_names_sorted.iter().position(|s| s == selected_logfile.as_str()) {
					self.dash_state.summary_window_rows.state.select(Some(node_index));
				}
			}
		}
	}

	// TODO this regenerates every line. May be worth just updating the line for the updated node/monitor
	// Needs to be on the app to manage focus for DashSummary and DashNode through sorting of summary table
	pub fn update_summary_window(&mut self) {
		let current_selection = self.dash_state.summary_window_rows.state.selected();

		self.dash_state.summary_window_rows = StatefulList::new();

		// TODO could avoid this repeated copy by ensuring both are modified at the same time
		self.dash_state.logfile_names_sorted = self.logfiles_manager.logfiles_added
			.iter()
			.map(|f| f.clone())
			.collect();

		super::ui_summary_table::sort_nodes_by_column(&mut self.dash_state, &mut self.monitors);

		for i in 0..self.dash_state.logfile_names_sorted.len() {
			let filepath = self.dash_state.logfile_names_sorted[i].clone();
			if let Some(monitor) = self.monitors.get_mut(&filepath) {
				if !monitor.is_debug_dashboard_log {
					monitor.metrics.update_node_status_string();
					let node_summary = super::ui_summary_table::format_table_row(monitor);
					self.append_to_summary_window(&node_summary);
				}
			}
		}

		self.dash_state.summary_window_rows.state.select(current_selection);
	}

	fn append_to_summary_window(&mut self, text: &str){
		self.dash_state.summary_window_rows.items.push(text.to_string());

		let len = self.dash_state.summary_window_rows.items.len();

		if len > self.dash_state.max_summary_window {
			self.dash_state.summary_window_rows.items = self.dash_state
				.summary_window_rows
				.items
				.split_off(len - self.dash_state.max_summary_window);
		} else {
			self.dash_state.summary_window_rows.state.select(Some(len - 1));
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
	eprintln!(
		"Try '{} --help' for more information.",
		Opt::clap().get_name()
	);
	return Err(Error::new(ErrorKind::Other, reason));
}

const NODE_INACTIVITY_TIMEOUT_S: i64 = 20;	// Seconds with no log message before node becomes 'inactive'

pub struct LogMonitor {
	pub index: usize,
	pub content: StatefulList<String>,
	max_content: usize, // Limit number of lines in content
	pub has_focus: bool,
	pub logfile: String,
	pub metrics: NodeMetrics,
	pub metrics_status: StatefulList<String>,
	pub is_debug_dashboard_log: bool,
	pub latest_checkpoint_time: Option<DateTime<Utc>>,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);

fn next_unused_index(monitors: &mut HashMap<String, LogMonitor>) -> usize {
	let mut next_index = 0;

	let mut index_unused = false;
	while !index_unused {
		next_index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);

		index_unused = true;
		for (_logfile, monitor) in monitors.iter()  {
			if next_index == monitor.index { index_unused = false; }
		}
	}

	next_index
}


use super::logfile_checkpoints::LogfileCheckpoint;

impl LogMonitor {
	pub fn new(logfile_path: String) -> LogMonitor {
		let index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);

		let mut is_debug_dashboard_log = false;
		if let Some(debug_logfile) = &*DEBUG_LOGFILE.lock().unwrap() {
			if let Some(debug_logfile_path) = debug_logfile.path().to_str() {
				is_debug_dashboard_log = logfile_path.eq(debug_logfile_path);
			}
		}

		let opt_lines_max = { OPT.lock().unwrap().lines_max };
		LogMonitor {
			index,
			logfile: logfile_path,
			max_content: opt_lines_max,
			metrics: NodeMetrics::new(),
			content: StatefulList::with_items(vec![]),
			has_focus: false,
			metrics_status: StatefulList::with_items(vec![]),
			is_debug_dashboard_log,
			latest_checkpoint_time: None,
		}
	}

	/// Resolve any clash between self.index and index of other monitors which may happen
	/// when mixing creation of new monitors with initialisation by restoring a checkpoint.
	///
	/// For a restored checkpoint the metrics should be set, so if one has metrics and the other
	/// doesn't, the former is treated as older and given the lower index.
	pub fn canonicalise_monitor_index(&mut self, monitors: &mut HashMap<String, LogMonitor>) {
		let existing_index = NEXT_MONITOR.fetch_add(0, Ordering::Relaxed);
		let next_index = next_unused_index(monitors);

		let mut clash_monitor = None;
		for (other_logfile, other) in monitors.iter_mut() {
			if self.index == other.index && &self.logfile != other_logfile {
				clash_monitor = Some(other);
			}
		}

		if let Some(other) = clash_monitor {

			let mut lower_index = self.index;
			let mut higher_index = next_index;
			if lower_index > higher_index {
				lower_index = next_index;
				higher_index = self.index;
			}

			// Default
			self.index = higher_index;
			other.index = lower_index;

			// If we know the earlier of the two metrics, use that to order the index in self and other
			if let Some(self_start_time) = self.metrics.node_started {
				let flip = if let Some(other_start_time) = other.metrics.node_started {
					self_start_time < other_start_time
				} else {
					true
				};
				if flip {
					self.index = lower_index;
					other.index = higher_index;
				}
			}
		} else {
			// next_index not used so restore state to avoid unnecessary increments
			NEXT_MONITOR.store(existing_index, Ordering::Relaxed);
		}
	}

	pub fn is_node(&self) -> bool { return !self.is_debug_dashboard_log; }

	pub fn from_checkpoint(&mut self, checkpoint: &LogfileCheckpoint) {
		self.index = checkpoint.monitor_index;
		self.latest_checkpoint_time = checkpoint.latest_entry_time;
		self.metrics = checkpoint.monitor_metrics.clone();
	}

	pub fn to_checkpoint(&mut self, checkpoint: &mut LogfileCheckpoint) {
		checkpoint.latest_entry_time = self.latest_checkpoint_time;
		checkpoint.monitor_index = self.index;
		checkpoint.monitor_metrics = self.metrics.clone();
	}

	// TODO if speed is an issue look at speeding up:
	// TODO - LogEntry::decode_metadata()
	// TODO - finding first log entry to decode using a bisection search
	pub fn load_logfile_from_time(&mut self, dash_state: &mut DashState, after_time: Option<DateTime<Utc>>) -> std::io::Result<()> {
		if let Some(after_time) = after_time {
			dash_state.vdash_status.message(&format!("loading logfile after time: {}", after_time).to_string(), None);
		}

		use std::io::{BufRead, BufReader};

		let f = File::open(self.logfile.to_string());
		let f = match f {
			Ok(file) => file,
			Err(_e) => return Ok(()), // It's ok for a logfile not to exist yet
		};

		let f = BufReader::new(f);

		for line in f.lines() {
			let line = line.expect("Unable to read line");
			self.append_to_content_from_time(dash_state, &line, after_time)?;
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

	pub fn append_to_content(&mut self, line: &str, checkpoint_interval: u64) -> Result<String, std::io::Error> {
		self.metrics.parser_output = format!("LogMeta::decode_metadata() failed on: {}", line); // For debugging
		// debug_log!(&self.parser_output.clone());

		self.metrics.entry_metadata = LogEntry::decode_metadata(line);

		if self.metrics.entry_metadata.is_none() {
			// debug_log!("gather_metrics() - skipping bec. metadata missing");
			return Ok("".to_string());	// Skip until start of first log message
		}

		self._append_to_content(line)?; // Show in TUI
		if self.is_debug_dashboard_log {
			return Ok("".to_string());
		}

		self.metrics.gather_metrics(&line)?;

		if checkpoint_interval > 0 { 	// Checkpoints disabled by zero interval
			return self.update_checkpoint(checkpoint_interval);
		}

		Ok("".to_string())
	}

	pub fn update_checkpoint(&mut self, checkpoint_interval: u64) -> Result<String, Error> {
		if let Some(metadata) = &self.metrics.entry_metadata {
			if self.latest_checkpoint_time.is_none() {
				return save_checkpoint(self);
			} else {
				if let Some(latest_checkpoint_time) = self.latest_checkpoint_time {
					if latest_checkpoint_time + Duration::seconds(checkpoint_interval as i64) < metadata.message_time {
						return save_checkpoint(self);
					}
				}
			}
		}

		Ok("".to_string())
	}

	pub fn append_to_content_from_time(&mut self, _dash_state: &mut DashState, line: &str, after_time: Option<DateTime<Utc>>) -> Result<(), std::io::Error> {
		self.metrics.parser_output = format!("LogMeta::decode_metadata() failed on: {}", line); // For debugging
		// debug_log!(&self.parser_output.clone());

		if let Some(entry_metadata) = LogEntry::decode_metadata(line) {
			if let Some(after_time) = after_time {
				if !entry_metadata.message_time.gt(&after_time) { return Ok(()); }
			}

			self.metrics.entry_metadata = Some(entry_metadata);
		} else {
			// debug_log!("gather_metrics() - skipping bec. metadata missing");
			if after_time.is_some() { return Ok(()); }
		}

		self._append_to_content(line)?; // Show in TUI
		if self.is_debug_dashboard_log {
			return Ok(());
		}

		self.metrics.gather_metrics(&line)?;

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
}

use regex::Regex;
lazy_static::lazy_static! {
	static ref LOG_LINE_PATTERN: Regex =
		Regex::new(r"\[(?P<time_string>[^ ]{27}) (?P<category>[A-Z]{4,6}) (?P<source>.*)\](?P<message>.*)").expect("The regex failed to compile. This is a bug.");
}

#[derive(PartialEq)]
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub enum NodeStatus {
	Started,
	Connecting,
	Connected,
	#[default]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeMetrics {
	pub node_started: Option<DateTime<Utc>>,
	pub running_message: Option<String>,
	pub running_version: Option<String>,
	pub node_process_id: Option<u64>,
	pub node_peer_id: Option<String>,
	pub category_count: HashMap<String, usize>,

	pub app_timelines: AppTimelines,

	pub entry_metadata: Option<LogMeta>,
	pub node_status: NodeStatus,
	pub node_status_string: String,
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

	pub parser_output: String,
}

impl NodeMetrics {
	pub fn new() -> NodeMetrics {
		let mut metrics = NodeMetrics {
			// Start
			node_started: None,
			running_message: None,
			running_version: None,
			node_process_id: None,
			node_peer_id: None,

			// Logfile entries
			entry_metadata: None,

			// A predefined set of Timelines (Sparklines)
			app_timelines: AppTimelines::new(),

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
			node_status_string: String::from(""),
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
			parser_output: String::from("-"),
		};
		metrics.update_timelines(&Utc::now());
		metrics
	}

	pub fn is_node_active(&self) -> bool {return !self.node_inactive;}

	pub fn update_node_status_string(&mut self) {
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

		self.node_status_string = node_status_string;
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
	///!	'Running safenode v0.98.32'
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

		let process_id_prefix = "Node (PID: ";
		if line.contains(&process_id_prefix) {
			self.node_process_id = self.parse_u64(process_id_prefix, line);
			let process_id = match &self.node_process_id {
				Some(process_id) => process_id.to_string(),
				None => String::from("unknown")
			};

			if let Some(peer_id) = self.parse_string("PeerId: ", line) {
				self.parser_output = format!("Node pid: {} peer_id: {}", String::from(process_id.clone()), peer_id);
				self.node_peer_id = Some(peer_id);
			}
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
		} else if line.contains("Wrote record") || line.contains("ValidSpendRecordPutFromNetwork") {
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
			if 	let Some(storage_payment) = self.parse_u64("payment of NanoTokens(", line) {
				self.count_storage_payment(entry_time, storage_payment);
				self.parser_output = format!("Payment received: {}", storage_payment);
				return true;
			};
		} else if line.contains("PeersInRoutingTable") {
			let mut parser_output = String::from("connected peers:");
			if let Some(peers_connected) = self.parse_u64("PeersInRoutingTable(", line) {
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

	///! Update data metrics from a handler response logfile entry
	///! Returns true if the line has been processed and can be discarded
	fn parse_string(&mut self, prefix: &str, line: &String) -> Option<String> {
		let mut string = "";
		if let Some(mut string_start) = line.find(prefix) {
			string_start += prefix.len();

			if let Some(string_end) = line[string_start..].find("\"") {
				string = line.as_str()[string_start..string_start + string_end].as_ref()
				} else {
				string = line.as_str()[string_start..].as_ref()
			}
			if string.is_empty() {
				self.parser_output = format!("failed to parse string after {} in: {}", prefix, line);
			}
		};

		if string.len() > 0 { Some(String::from(string)) } else { None }
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

///! Metadata for a logfile line
#[derive(Clone, Debug, Serialize, Deserialize)]
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
#[derive(Clone)]
#[derive(Copy)]
pub enum DashViewMain {
	DashSummary,
	DashNode,
	DashHelp,
	DashDebug,
}

pub struct DashState {
	pub vdash_status: StatusMessage,
	pub main_view: DashViewMain,
	pub previous_main_view: DashViewMain,
	pub logfile_names_sorted: Vec<String>,
	pub logfile_names_sorted_ascending: bool,

	pub active_timescale: usize,
	pub node_logfile_visible: bool,
	pub dash_node_focus: String,
    pub mmm_ui_mode:   MinMeanMax,
    pub top_timeline: usize,  // Timeline to show at top of UI

	pub summary_window_heading: String,	// TODO delete in favour of...
	pub summary_window_headings: StatefulList<String>,
	pub summary_window_heading_selected: usize,
	pub summary_window_rows: StatefulList<String>,
	max_summary_window: usize,

	pub help_status: StatefulList<String>,

	// For --debug-window option
	pub debug_window_list: StatefulList<String>,
	pub debug_window: bool,
	pub debug_window_has_focus: bool,
	max_debug_window: usize,
}

const UI_STATUS_DEFAULT_MESSAGE: &str = "Press '?' for Help";
const UI_STATUS_DEFAULT_DURATION_S: i64 = 5;
use super::ui_status::StatusMessage;

impl DashState {
	pub fn new() -> DashState {

		let mut new_dash = DashState {
			vdash_status: StatusMessage::new(&String::from(UI_STATUS_DEFAULT_MESSAGE), &Duration::seconds(UI_STATUS_DEFAULT_DURATION_S)),

			main_view: DashViewMain::DashSummary,
			previous_main_view: DashViewMain::DashSummary,
			logfile_names_sorted: Vec::<String>::new(),	// Sorted by column
			logfile_names_sorted_ascending: true,

			active_timescale: 0,
			node_logfile_visible: true,
			dash_node_focus: String::new(),
			mmm_ui_mode: MinMeanMax::Mean,
            top_timeline: 0,

			summary_window_heading: String::from(""),
			summary_window_headings: StatefulList::new(),
			summary_window_heading_selected: 0,
			summary_window_rows: StatefulList::new(),
			max_summary_window: 1000,

			help_status: StatefulList::with_items(vec![]),

			debug_window: false,
			debug_window_has_focus: false,
			debug_window_list: StatefulList::new(),
			max_debug_window: 100,
		};
		super::ui_summary_table::initialise_summary_headings(&mut new_dash);
		new_dash
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

	app.dash_state.previous_main_view = app.dash_state.main_view;
	save_focus(app);
	app.dash_state.main_view = view;
	restore_focus(app);
}

pub fn save_focus(app: &mut App) {
	match app.dash_state.main_view {
		DashViewMain::DashHelp => {}

		DashViewMain::DashSummary|
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
		DashViewMain::DashHelp => {}

		DashViewMain::DashSummary|
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
