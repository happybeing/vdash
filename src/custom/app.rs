///! Application logic
///!
///! Edit src/custom/app.rs to create a customised fork of logtail-dash
use linemux::MuxedLines;
use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use std::fs::{File, OpenOptions};
use std::io::{Read, Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use tempfile::NamedTempFile;

use crate::custom::opt::{Opt, MIN_TIMELINE_STEPS};
use crate::shared::util::StatefulList;

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
	pub logfile_names: Vec<String>,
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
		let mut logfile_names = Vec::<String>::new();

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
				logfile_names.push(f.to_string());
				monitors.insert(f.to_string(), monitor);
			} else {
				match monitor.load_logfile(&mut dash_state) {
					Ok(()) => {
						logfile_names.push(f.to_string());
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
			logfile_names,
		};
		app.update_timelines(&Utc::now());

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
		for (i, name) in self.logfile_names.iter().enumerate() {
			if name == &self.logfile_with_focus {
				if i < self.logfile_names.len() - 1 {
					next_i = i + 1;
				}
				break;
			}
		}

		if next_i == 0 && self.opt.debug_window && self.logfile_with_focus != DEBUG_WINDOW_NAME {
			self.set_logfile_with_focus(DEBUG_WINDOW_NAME.to_string());
			return;
		}

		let logfile = self.logfile_names[next_i].to_string();
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

		let len = self.logfile_names.len();
		let mut previous_i = len - 1;
		for (i, name) in self.logfile_names.iter().enumerate() {
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

		let logfile = self.logfile_names[previous_i].to_string();
		self.set_logfile_with_focus(logfile.clone());

		if let Some(debug_logfile) = self.get_debug_dashboard_logfile() {
			if logfile.eq(&debug_logfile) {
				self.change_focus_previous();
			}
		}
	}

	pub fn handle_arrow_up(&mut self) {
		if let Some(monitor) = self.get_monitor_with_focus() {
			do_bracketed_next_previous(&mut monitor.content, false);
		} else if self.opt.debug_window {
			do_bracketed_next_previous(&mut self.dash_state.debug_window_list, false);
		}
	}

	pub fn handle_arrow_down(&mut self) {
		if let Some(monitor) = self.get_monitor_with_focus() {
			do_bracketed_next_previous(&mut monitor.content, true);
		} else if self.opt.debug_window {
			do_bracketed_next_previous(&mut self.dash_state.debug_window_list, true);
		}
	}

	pub fn scale_timeline_up(&mut self) {
		if self.dash_state.active_timeline == 0 {
			return;
		}
		self.dash_state.active_timeline -= 1;
	}

	pub fn scale_timeline_down(&mut self) {
		if self.dash_state.active_timeline == TIMELINES.len()-1 {
			return;
		}
		self.dash_state.active_timeline += 1;
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
		Regex::new(r"(?P<module>\[.*\])* (?P<category>[A-Z]{4,6}) (?P<time_string>[^ ]{27}) (?P<source>\[.*\])(?P<message>.*)").expect("The regex failed to compile. This is a bug.");
}

#[derive(PartialEq)]
pub enum NodeAgebracket {
	Unknown,
	Joining,
	Adult,
	Elder,
}

///! Maintains one or more 'marching bucket' histories for
///! a given metric, each with its own duration and granularity.
///!
///! A BucketSet is used to hold the history of values with
///! a given bucket_duration and maximum number of buckets.
///!
///! A BucketSet begins with a single bucket of fixed
///! duration holding the initial metric value. New buckets
///! are added as time progresses until the number of buckets
///! covers the total duration of the BucketSet. At this
///! point the oldest bucket is removed when a new bucket is
///! added, so that the total duration remains constant and
///! the specified maximum number of buckets is never
///! exceeded.
///!
///! By adding more than one BucketSet, a given metric can be
///! recorded for different durations and with different
///! granularities. E.g. 60 * 1s buckets covers a minute
///! and 60 * 1m buckets covers an hour, and so on.
pub struct TimelineSet {
	name: String,
	bucket_sets: HashMap<&'static str, BucketSet>,
}

pub struct BucketSet {
	pub bucket_time: Option<DateTime<Utc>>,
	pub total_duration: Duration,
	pub bucket_duration: Duration,
	pub max_buckets: usize,
	pub buckets: Vec<u64>,
}

impl TimelineSet {
	pub fn new(name: String) -> TimelineSet {
		TimelineSet {
			name,
			bucket_sets: HashMap::<&'static str, BucketSet>::new(),
		}
	}

	pub fn get_name(&self) -> &String {
		&self.name
	}

	pub fn add_bucket_set(&mut self, name: &'static str, duration: Duration, max_buckets: usize) {
		self.bucket_sets
			.insert(name, BucketSet::new(duration, max_buckets));
	}

	pub fn get_bucket_set(&mut self, bucket_set_name: &str) -> Option<&BucketSet> {
		self.bucket_sets.get(bucket_set_name)
	}

	///! Update all bucket_sets with new current time
	///!
	///! Call significantly more frequently than the smallest BucketSet duration
	fn update_current_time(&mut self, new_time: &DateTime<Utc>) {
		// debug_log!("update_current_time()");
		for (_name, bs) in self.bucket_sets.iter_mut() {
			if let Some(mut bucket_time) = bs.bucket_time {
				let mut end_time = bucket_time + bs.bucket_duration;
				// debug_log!(format!("end_time       : {}", end_time).as_str());

				while end_time.lt(&new_time) {
					// debug_log!("Start new bucket");
					// Start new bucket
					bs.bucket_time = Some(end_time);
					bucket_time = end_time;
					end_time = bucket_time + bs.bucket_duration;

					bs.buckets.push(0);
					if bs.buckets.len() > bs.max_buckets {
						bs.buckets.remove(0);
					}
				}
			} else {
				bs.bucket_time = Some(*new_time);
			}
		}
	}

	fn increment_value(&mut self, time: &DateTime<Utc>) {
		// debug_log!("increment_value()");
		for (_name, bs) in self.bucket_sets.iter_mut() {
			// debug_log!(format!("name       : {}", _name).as_str());
			let mut index = Some(bs.buckets.len() - 1);
			// debug_log!(format!("time       : {}", time).as_str());
			if let Some(bucket_time) = bs.bucket_time {
			// debug_log!(format!("bucket_time: {}", bucket_time).as_str());
				if time.lt(&bucket_time) {
					// Use the closest bucket to this time
					// debug_log!("increment (closest bucket)");
					let time_difference = (bucket_time - *time).num_nanoseconds();
					let bucket_duration = bs.bucket_duration.num_nanoseconds();
					if time_difference.and(bucket_duration).is_some() {
						let buckets_behind = time_difference.unwrap() / bucket_duration.unwrap();
						if buckets_behind as usize >= bs.buckets.len() {
							// debug_log!(format!("increment DISCARDED buckets_behind: {}", buckets_behind).as_str());
							index = None;
						} else {
							// debug_log!(format!("increment INCLUDED buckets_behind: {}", buckets_behind).as_str());
							index = Some(bs.buckets.len() - 1 - buckets_behind as usize);
						}
					}
				}
			}
			if let Some(index) = index {
				// debug_log!(format!("increment index: {}", index).as_str());
				bs.buckets[index] += 1;
			}
		}
	}
}

impl BucketSet {
	pub fn new(bucket_duration: Duration, max_buckets: usize) -> BucketSet {
		BucketSet {
			bucket_duration,
			max_buckets,
			total_duration: bucket_duration * max_buckets as i32,

			bucket_time: None,
			buckets: vec![0; max_buckets],
		}
	}

	pub fn set_bucket_value(&mut self, value: u64) {
		let index = self.buckets.len() - 1;
		self.buckets[index] = value;
	}

	pub fn increment_value(&mut self) {
		let index = self.buckets.len() - 1;
		self.buckets[index] += 1;
	}

	pub fn buckets(&self) -> &Vec<u64> {
		&self.buckets
	}

	pub fn buckets_mut(&mut self) -> &mut Vec<u64> {
		&mut self.buckets
	}
}

pub struct NodeMetrics {
	pub node_started: Option<DateTime<Utc>>,
	pub running_message: Option<String>,
	pub running_version: Option<String>,
	pub category_count: HashMap<String, usize>,
	pub activity_history: Vec<ActivityEntry>,
	pub log_history: Vec<LogEntry>,

	pub puts_timeline: TimelineSet,
	pub gets_timeline: TimelineSet,
	pub errors_timeline: TimelineSet, // TODO add code to collect and display

	pub entry_metadata: Option<LogMeta>,
	pub agebracket: NodeAgebracket,
	pub section_prefix: String,
	pub node_age: usize,
	pub node_name: String,
	pub adults: usize,
	pub elders: usize,
	pub activity_gets: u64,
	pub activity_puts: u64,
	pub activity_errors: u64,

	pub used_space: u64,
	pub max_capacity: u64,

	pub debug_logfile: Option<NamedTempFile>,
	parser_output: String,
}

impl NodeMetrics {
	fn new(opt: &Opt) -> NodeMetrics {
		let mut puts_timeline = TimelineSet::new("PUTS".to_string());
		let mut gets_timeline = TimelineSet::new("GETS".to_string());
		let mut errors_timeline = TimelineSet::new("ERRORS".to_string());
		for timeline in [&mut puts_timeline, &mut gets_timeline, &mut errors_timeline].iter_mut() {
			for i in 0..TIMELINES.len() {
				if let Some(spec) = TIMELINES.get(i) {
					timeline.add_bucket_set(spec.0, spec.1, opt.timeline_steps);
				}
			}
		}

		let mut metrics = NodeMetrics {
			// Start
			node_started: None,
			running_message: None,
			running_version: None,

			// Logfile entries
			activity_history: Vec::<ActivityEntry>::new(),
			log_history: Vec::<LogEntry>::new(),
			entry_metadata: None,

			// Timelines / Sparklines
			puts_timeline,
			gets_timeline,
			errors_timeline,

			// Counts
			category_count: HashMap::new(),
			activity_gets: 0,
			activity_puts: 0,
			activity_errors: 0,

			// State (node)
			agebracket: NodeAgebracket::Unknown,
			section_prefix: String::from(""),
			node_age: 0,
			node_name: String::from(""),

			// State (network)
			adults: 0,
			elders: 0,

			// Disk use:
			used_space: 0,
			max_capacity: 0,

			// Debug
			debug_logfile: None,
			parser_output: String::from("-"),
		};
		metrics.update_timelines(&Utc::now());
		metrics
	}

	pub fn agebracket_string(&self) -> String {
		match self.agebracket {
			NodeAgebracket::Joining => "Joining".to_string(),
			NodeAgebracket::Adult => "Adult".to_string(),
			NodeAgebracket::Elder => "Elder".to_string(),
			NodeAgebracket::Unknown => "Unknown".to_string(),
		}
	}

	fn reset_metrics(&mut self) {
		self.agebracket = NodeAgebracket::Unknown;
		self.section_prefix = String::from("");
		self.node_age = 0;
		self.node_name = String::from("");
		self.adults = 0;
		self.elders = 0;
		self.activity_gets = 0;
		self.activity_puts = 0;
		self.activity_errors = 0;
	}

	///! Process a line from a SAFE Node logfile.
	///! May add a LogMeta to the NodeMetrics::log_history vector.
	///! Use a created LogMeta to update metrics.
	pub fn gather_metrics(&mut self, line: &str) -> Result<(), std::io::Error> {
		// let mut parser_result = format!("LogMeta::decode_metadata() failed on: {}", line); // For debugging

		if let Some(metadata) = LogEntry::decode_metadata(line) {
			self.entry_metadata = Some(metadata);
		}

		if self.entry_metadata.is_none() {
			return Ok(());	// Skip until start of first log message
		}

		let entry = LogEntry { logstring: String::from(line) };
		let entry_metadata = self.entry_metadata.as_ref().unwrap().clone();
		let entry_time = entry_metadata.time;

		self.update_timelines(&entry_time);
		self.parser_output = entry_metadata.parser_output.clone();
		self.process_logfile_entry(&entry.logstring, &entry_metadata); // May overwrite self.parser_output
		self.log_history.push(entry); // TODO Trim log_history

		// --debug-dashboard - prints parser results for a single logfile
		// to a temp logfile which is displayed in the adjacent window.
		debug_log!(&self.parser_output.clone());

		Ok(())
	}

	pub fn update_timelines(&mut self, now: &DateTime<Utc>) {
		for timeline in &mut [
			&mut self.puts_timeline,
			&mut self.gets_timeline,
			&mut self.errors_timeline,
		]
		.iter_mut()
		{
			timeline.update_current_time(&now);
		}
	}

	///! Return a LogMeta and capture metadata for logfile node start:
	///!	'Running safe-node v0.24.0'
	// pub fn parse_start(&mut self, line: &str) -> Option<LogMeta> {
	// 	let running_prefix = String::from("Running sn_node ");

	// 	if line.starts_with(&running_prefix) {
	// 		self.running_message = Some(line.to_string());
	// 		self.running_version = Some(line[running_prefix.len()..].to_string());
	// 		self.node_started = self.entry_metadata.time;
	// 		let parser_output = format!(
	// 			"START at {}",
	// 			self.entry_metadata
	// 				.map_or(String::from("None"), |m| format!("{}", m))
	// 		);

	// 		self.reset_metrics();
	// 		return Some(LogMeta {
	// 			category: String::from("START"),
	// 			time: self.entry_metadata.time,
	// 			source: String::from(""),
	// 			message: line.to_string(),
	// 			parser_output,
	// 		});
	// 	}

	// 	None
	// }

	///! Process a logfile entry
	///! Returns true if the line has been processed and can be discarded
	pub fn process_logfile_entry(&mut self, line: &String, entry_metadata: &LogMeta) -> bool {
		return self.parse_data_response(
			&line,
			"Running as Node: SendToSection [ msg: MsgEnvelope { message: QueryResponse { response: QueryResponse::",
		) || self.parse_gets_and_puts(&line, &entry_metadata.time) || self.parse_states(&line, &entry_metadata);
	}

	///! TODO: Review and update these tests
	fn parse_gets_and_puts(&mut self, line: &String, entry_time: &DateTime<Utc>) -> bool {
		if line.contains("Getting chunk") {
			self.count_get(&entry_time);
			return true;
		} else if line.contains("StoredNewChunk") {
			self.count_put(&entry_time);
			return true;
		} else if line.contains("Editing Register success!") {
			self.count_put(&entry_time);
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
			self.count_error(&entry_metadata.time);
		}


		let &content = &line.as_str();

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

		if let Some(elders) = self.parse_usize("No. of Elders:", content) {
			self.elders = elders;
			self.parser_output = format!("ELDERS: {}", elders);
			return true;
		};

		if let Some(adults) = self.parse_usize("No. of Adults:", content) {
			self.adults = adults;
			self.parser_output = format!("ADULTS: {}", adults);
			return true;
		};

		// TODO: review as things stabilise during Fleming testnets
		// Pre-Fleming testnets code with additions for Fleming T4.1
		if let Some(agebracket) = self
			.parse_word("xNode promoted to ", content)
			.or(self.parse_word("xWe are ", content))
			.or(self.parse_word("xNew RoutingEvent received. Current role:", content))
		{
			self.agebracket = match agebracket.as_str() {
				"Adult" => NodeAgebracket::Adult,
				"Elder" => NodeAgebracket::Elder,
				_ => {
					debug_log!(self.parser_output.as_str());
					NodeAgebracket::Joining
				}
			};
			if self.agebracket != NodeAgebracket::Unknown {
				self.parser_output = format!("Node agebracket: {}", agebracket);
			} else {
				self.parser_output = format!("FAILED to parse agebracket in: {}", content);
			}

			if let Some(section_prefix) = self.parse_word("section prefix:", content) {
				self.parser_output = format!("section prefix: {}", &section_prefix);
				self.section_prefix = section_prefix;
			} else {
				self.parser_output = format!("FAILED to parse section prefix in: {}", content);
			}

			if let Some(node_name) = self.parse_word("node name:", content) {
				self.parser_output = format!("node name: {}", &node_name);
				self.node_name = node_name;
			} else {
				self.parser_output = format!("FAILED to parse node name in: {}", content);
			}

			return true;
		};

		if content.contains("Sending aggregated JoinRequest")
		{
			self.agebracket = NodeAgebracket::Joining;
			self.parser_output = format!("Age-bracket updated to: Joining");
			return true;
		}

		if content.contains("Joined the network") {
			self.agebracket = NodeAgebracket::Adult;
			self.parser_output = format!("Age updated to: Adult");

			if let Some(node_name) = self.parse_word("➤", content) {
				self.parser_output = format!("node name: {}", &node_name);
				self.node_name = node_name;
			}

			return true;
		}

		if content.contains("Relocation: switching from") {
			self.agebracket = NodeAgebracket::Adult;
			self.parser_output = format!("Age updated to: Adult");
			if let Some(new_node_name) = self.parse_word("to", content) {
				self.node_name = new_node_name;
				self.parser_output = format!("New node name: {}", &self.node_name);
			}

			return true;
		}

		if content.contains("PromotedToElder") {
			self.agebracket = NodeAgebracket::Elder;
			self.parser_output = format!("Age updated to: Elder");
			return true;
		}

		if let Some(node_age) = self.parse_usize("Our AGE:", content) {
			self.parser_output = format!("age: {}", node_age);
			self.node_age = node_age;
		} else {
			self.parser_output = format!("FAILED to parse node age in: {}", content);
		}

		false
	}

	fn parse_usize(&mut self, prefix: &str, content: &str) -> Option<usize> {
		if let Some(position) = content.find(prefix) {
			let word: Vec<&str> = content[position + prefix.len()..]
				.trim()
				.splitn(2, |c| c == ' ' || c == ',')
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
				.splitn(2, |c| c == ' ' || c == ',')
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

	fn parse_word(&mut self, prefix: &str, content: &str) -> Option<String> {
		if let Some(start) = content.find(prefix) {
			let word: Vec<&str> = content[start + prefix.len()..]
				.trim_start()
				.splitn(2, |c| c == ' ' || c == ',')
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
		self.activity_gets += 1;
		self.gets_timeline.increment_value(time);
	}

	fn count_put(&mut self, time: &DateTime<Utc>) {
		self.activity_puts += 1;
		self.puts_timeline.increment_value(time);
	}

	fn count_error(&mut self, time: &DateTime<Utc>) {
		self.activity_errors += 1;
		self.errors_timeline.increment_value(time);
	}

	///! TODO
	pub fn parse_logentry_counts(&mut self, entry: &LogMeta) {
		// Categories ('INFO', 'WARN' etc)
		if !entry.category.is_empty() {
			let count = match self.category_count.get(&entry.category) {
				Some(count) => count + 1,
				None => 1,
			};
			self.category_count.insert(entry.category.clone(), count);
		}
	}
}

///! Node activity for node activity_history
pub struct ActivityEntry {
	pub message: String,
	pub activity: String,
	pub logstring: String,
	pub category: String, // First word, "Running", "INFO", "WARN" etc
	pub time: DateTime<Utc>,
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
			time: entry_metadata.time,
			source: entry_metadata.source.clone(),

			parser_output: String::from(""),
		}
	}
}


///! Metadata for a logfile line
#[derive(Clone)]
pub struct LogMeta {
	pub category: String, // First word ('INFO', 'WARN' etc.)
	pub time: DateTime<Utc>,
	pub source: String,
	pub message: String,

	pub parser_output: String,
}

impl LogMeta {
	pub fn clone(&self) -> LogMeta {
		LogMeta {
			category: self.category.clone(),
			time: self.time,
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
				time: time_utc,
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
	DashDebug,
}

lazy_static::lazy_static! {
	pub static ref TIMELINES: std::vec::Vec<(&'static str, Duration)> = vec!(
		("1 second columns", Duration::seconds(1)),
		("1 minute columns", Duration::minutes(1)),
		("1 hour columns", Duration::hours(1)),
		("1 day columns", Duration::days(1)),
		("1 week columns", Duration::days(7)),
		("1 year columns", Duration::days(365)),
	);
}

pub struct DashState {
	pub main_view: DashViewMain,
	pub active_timeline: usize,
	pub dash_node_focus: String,

	// For --debug-window option
	pub debug_window_list: StatefulList<String>,
	pub debug_window: bool,
	pub debug_window_has_focus: bool,
	max_debug_window: usize,
}

impl DashState {
	pub fn new() -> DashState {

		DashState {
			main_view: DashViewMain::DashNode,
			active_timeline: 0,
			dash_node_focus: String::new(),

			debug_window: false,
			debug_window_has_focus: false,
			debug_window_list: StatefulList::new(),
			max_debug_window: 100,
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
