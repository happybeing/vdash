///! Application logic
///!
///! Edit src/custom/app.rs to create a customised fork of logtail-dash
use linemux::MuxedLines;
use std::collections::HashMap;

use chrono::{DateTime, Duration, FixedOffset, TimeZone};
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use structopt::StructOpt;
use tempfile::NamedTempFile;

use crate::custom::opt::Opt;
use crate::shared::util::StatefulList;

pub static DEBUG_WINDOW_NAME: &str = "Debug Window";

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
			println!(
				"Try '{} --help' for more information.",
				Opt::clap().get_name()
			);
			return Err(Error::new(ErrorKind::Other, "missing logfiles"));
		}
		let mut dash_state = DashState::new();
		dash_state.debug_window = opt.debug_window;
		let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
		let mut logfiles = MuxedLines::new()?;
		let mut name_for_focus = String::new();
		let mut logfile_names = Vec::<String>::new();

		let mut parser_output: Option<tempfile::NamedTempFile> = if opt.debug_dashboard {
			dash_state.main_view = DashViewMain::DashDebug;
			opt.files = opt.files[0..1].to_vec();
			let named_file = NamedTempFile::new()?;
			let path = named_file.path();
			let path_str = path
				.to_str()
				.ok_or_else(|| Error::new(ErrorKind::Other, "invalid path"))?;
			opt.files.push(String::from(path_str));
			Some(named_file)
		} else {
			None
		};
		println!("Loading {} files...", opt.files.len());
		for f in &opt.files {
			println!("file: {}", f);
			let mut monitor = LogMonitor::new(f.to_string(), opt.lines_max);
			if opt.debug_dashboard && monitor.index == 0 {
				if let Some(named_file) = parser_output {
					monitor.metrics.debug_logfile = Some(named_file);
					parser_output = None;
					dash_state.debug_dashboard = true;
				}
			}
			if opt.ignore_existing {
				logfile_names.push(f.to_string());
				monitors.insert(f.to_string(), monitor);
			} else {
				match monitor.load_logfile() {
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

			if name_for_focus.is_empty() {
				name_for_focus = f.to_string();
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

		let mut app = App {
			opt,
			dash_state,
			monitors,
			logfile_with_focus: name_for_focus.clone(),
			logfiles,
			logfile_names,
		};
		app.set_logfile_focus(&name_for_focus);
		Ok(app)
	}

	pub fn get_monitor_with_focus(&mut self) -> Option<(&mut LogMonitor)> {
		match (&mut self.monitors).get_mut(&self.logfile_with_focus) {
			Some(mut monitor) => Some(monitor),
			None => None,
		}
	}

	pub fn set_logfile_focus(&mut self, logfile_name: &String) {
		match self.get_monitor_with_focus() {
			Some(fading_monitor) => {
				fading_monitor.has_focus = false;
				self.logfile_with_focus = String::new();
			}
			None => (),
		}

		if (logfile_name == DEBUG_WINDOW_NAME) {
			self.dash_state.debug_window_has_focus = true;
			self.logfile_with_focus = logfile_name.clone();
			return;
		} else {
			self.dash_state.debug_window_has_focus = false;
		}

		if let Some(focus_monitor) = (&mut self.monitors).get_mut(logfile_name) {
			focus_monitor.has_focus = true;
			self.logfile_with_focus = logfile_name.clone();
		} else {
			error!("Unable to focus UI on: {}", logfile_name);
		};
	}

	pub fn change_focus_next(&mut self) {
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
			self.set_logfile_focus(&DEBUG_WINDOW_NAME.to_string());
			return;
		}

		let new_focus_name = &self.logfile_names[next_i].to_string();
		self.set_logfile_focus(&new_focus_name);
	}

	pub fn change_focus_previous(&mut self) {
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
			self.set_logfile_focus(&DEBUG_WINDOW_NAME.to_string());
			return;
		}
		let new_focus_name = &self.logfile_names[previous_i].to_string();
		self.set_logfile_focus(new_focus_name);
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
}

/// Move selection forward or back without wrapping at start or end
fn do_bracketed_next_previous(list: &mut StatefulList<String>, next: bool) {
	if (next) {
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

pub struct LogMonitor {
	pub index: usize,
	pub content: StatefulList<String>,
	max_content: usize, // Limit number of lines in content
	pub has_focus: bool,
	pub logfile: String,
	pub metrics: VaultMetrics,
	pub metrics_status: StatefulList<String>,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);

impl LogMonitor {
	pub fn new(f: String, max_lines: usize) -> LogMonitor {
		let index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);
		LogMonitor {
			index,
			logfile: f,
			max_content: max_lines,
			metrics: VaultMetrics::new(),
			content: StatefulList::with_items(vec![]),
			has_focus: false,
			metrics_status: StatefulList::with_items(vec![]),
		}
	}

	pub fn load_logfile(&mut self) -> std::io::Result<()> {
		use std::io::{BufRead, BufReader};

		let f = File::open(self.logfile.to_string());
		let f = match f {
			Ok(file) => file,
			Err(_e) => return Ok(()), // It's ok for a logfile not to exist yet
		};

		let f = BufReader::new(f);

		for line in f.lines() {
			let line = line.expect("Unable to read line");
			self.append_to_content(&line)?
		}

		if self.content.items.len() > 0 {
			self
				.content
				.state
				.select(Some(self.content.items.len() - 1));
		}

		Ok(())
	}

	pub fn append_to_content(&mut self, text: &str) -> Result<(), std::io::Error> {
		if self.line_filter(&text) {
			self.metrics.gather_metrics(&text)?;
			self._append_to_content(text)?; // Show in TUI
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
	fn line_filter(&mut self, line: &str) -> bool {
		true
	}
}

use regex::Regex;

lazy_static::lazy_static! {
	// static ref REGEX_ERROR = "The regex failed to compile. This is a bug.";
	static ref LOG_LINE_PATTERN: Regex =
		Regex::new(r"(?P<category>^[A-Z]{4}) (?P<time_string>[^ ]{35}) (?P<source>\[.*\]) (?P<message>.*)").expect("The regex failed to compile. This is a bug.");

	// static ref STATE_PATTERN: Regex =
	//   Regex::new(r"vault.rs .*No. of Elders: (?P<elders>\d+)").expect(REGEX_ERROR);

	// static ref COUNTS_PATTERN: Regex =215

	// Regex::new(r"vault.rs .*No. of Adults: (?P<elders>\d+)").expect(REGEX_ERROR);
}

pub enum VaultAgebracket {
	Unknown,
	Infant,
	Adult,
	Elder,
}

pub struct VaultMetrics {
	pub vault_started: Option<DateTime<FixedOffset>>,
	pub running_message: Option<String>,
	pub running_version: Option<String>,
	pub category_count: HashMap<String, usize>,
	pub activity_history: Vec<ActivityEntry>,
	pub log_history: Vec<LogEntry>,
	pub sparkline_bucket_time: Option<DateTime<FixedOffset>>,
	pub sparkline_width: Duration,
	pub sparkline_bucket_width: Duration,
	pub sparkline_buckets: usize,
	pub puts_sparkline: Vec<u64>,
	pub gets_sparkline: Vec<u64>,
	pub most_recent: Option<DateTime<FixedOffset>>,
	pub agebracket: VaultAgebracket,
	pub adults: usize,
	pub elders: usize,
	pub activity_gets: u64,
	pub activity_puts: u64,
	pub activity_other: u64,

	pub debug_logfile: Option<NamedTempFile>,
	parser_output: String,
}

impl VaultMetrics {
	fn new() -> VaultMetrics {
		VaultMetrics {
			// Start
			vault_started: None,
			running_message: None,
			running_version: None,

			// Logfile entries
			activity_history: Vec::<ActivityEntry>::new(),
			log_history: Vec::<LogEntry>::new(),
			most_recent: None,

			// Timeline / Sparklines
			sparkline_bucket_time: None,
			sparkline_width: Duration::minutes(1),
			sparkline_bucket_width: Duration::seconds(1),
			sparkline_buckets: 60,
			puts_sparkline: vec![0],
			gets_sparkline: vec![0],

			// Counts
			category_count: HashMap::new(),
			activity_gets: 0,
			activity_puts: 0,
			activity_other: 0,

			// State (vault)
			agebracket: VaultAgebracket::Infant,

			// State (network)
			adults: 0,
			elders: 0,

			// Debug
			debug_logfile: None,
			parser_output: String::from("-"),
		}
	}

	pub fn agebracket_string(&self) -> String {
		match self.agebracket {
			VaultAgebracket::Infant => "Infant".to_string(),
			VaultAgebracket::Adult => "Adult".to_string(),
			VaultAgebracket::Elder => "Elder".to_string(),
			VaultAgebracket::Unknown => "Unknown".to_string(),
		}
	}

	fn reset_metrics(&mut self) {
		self.agebracket = VaultAgebracket::Infant;
		self.adults = 0;
		self.elders = 0;
		self.activity_gets = 0;
		self.activity_puts = 0;
		self.activity_other = 0;
	}

	///! Process a line from a SAFE Vault logfile.
	///! May add a LogEntry to the VaultMetrics::log_history vector.
	///! Use a created LogEntry to update metrics.
	pub fn gather_metrics(&mut self, line: &str) -> Result<(), std::io::Error> {
		// For debugging LogEntry::decode()
		let mut parser_result = format!("LogEntry::decode() failed on: {}", line);
		if let Some(mut entry) = LogEntry::decode(line).or_else(|| self.parse_start(line)) {
			if entry.time.is_none() {
				entry.time = self.most_recent;
			} else {
				self.most_recent = entry.time;
			}

			self.parser_output = entry.parser_output.clone();
			self.process_logfile_entry(&entry); // May overwrite self.parser_output
			parser_result = self.parser_output.clone();
			self.log_history.push(entry);

			// TODO Trim log_history
		}

		// --debug-parser - prints parser results for a single logfile
		// to a temp logfile which is displayed in the adjacent window.
		match &self.debug_logfile {
			Some(f) => {
				use std::io::Seek;
				let mut file = f.reopen()?;
				file.seek(std::io::SeekFrom::End(0))?;
				writeln!(file, "{}", &parser_result)?
			}
			None => (),
		};
		Ok(())
	}

	///! Returm a LogEntry and capture metadata for logfile vault start:
	///!    'Running safe-vault v0.24.0'
	pub fn parse_start(&mut self, line: &str) -> Option<LogEntry> {
		let running_prefix = String::from("Running safe-vault ");

		if line.starts_with(&running_prefix) {
			self.running_message = Some(line.to_string());
			self.running_version = Some(line[running_prefix.len()..].to_string());
			self.vault_started = self.most_recent;
			let parser_output = format!(
				"START at {}",
				self
					.most_recent
					.map_or(String::from("None"), |m| format!("{}", m))
			);

			self.reset_metrics();
			return Some(LogEntry {
				logstring: String::from(line),
				category: String::from("START"),
				time: self.most_recent,
				source: String::from(""),
				message: line.to_string(),
				parser_output,
			});
		}

		None
	}

	///! Process a logfile entry
	///! Returns true if the line has been processed and can be discarded
	pub fn process_logfile_entry(&mut self, entry: &LogEntry) -> bool {
		return self.parse_data_response(
			&entry,
			"Responded to our data handlers with: Response { response: Response::",
		) || self.parse_states(&entry);
	}

	///! Update data metrics from a handler response logfile entry
	///! Returns true if the line has been processed and can be discarded
	fn parse_data_response(&mut self, entry: &LogEntry, pattern: &str) -> bool {
		if let Some(mut response_start) = entry.logstring.find(pattern) {
			response_start += pattern.len();
			let mut response = "";

			if let Some(response_end) = entry.logstring[response_start..].find(",") {
				response = entry.logstring.as_str()[response_start..response_start + response_end].as_ref();
				if !response.is_empty() {
					let activity_entry = ActivityEntry::new(entry, response);
					self.update_buckets();
					self.parse_activity_counts(&activity_entry);
					self.activity_history.push(activity_entry);
					self.parser_output = format!("vault activity: {}", response);
				}
			}
			if response.is_empty() {
				self.parser_output = format!("failed to parse_data_response: {}", entry.logstring);
			};

			return true;
		};
		return false;
	}

	///! Capture state updates from a logfile entry
	///! Returns true if the line has been processed and can be discarded
	fn parse_states(&mut self, entry: &LogEntry) -> bool {
		let mut updated = false;
		// TODO re-instate if can count adults using "|^.*vault\.rs.*No.\ of\ Adults:\ (?P<adults>\d+)"
		let re = Regex::new(
			r"(?x)
			 ^.*network_stats\.rs.*Known\ elders:\ *(?P<elders>\d+)
			|^.*vault\.rs.*Initializing\ new\ Vault\ as\ (?P<initas>[[:alpha:]]+)
			|^.*vault\.rs.*Vault\ promoted\ to\ (?P<promoteto>[[:alpha:]]+)",
		)
		.expect("Woops"); // TODO: make the expression a static (see LOG_LINE_PATTERN)

		let captures = match re.captures(entry.logstring.as_str()) {
			Some(captures) => captures,
			None => return false,
		};

		if let Some(elders_str) = captures.name("elders").map(|m| m.as_str()) {
			self.parser_output = format!("ELDERS: {}", elders_str);
			match elders_str.parse::<usize>() {
				Ok(elders) => {
					self.elders = elders;
					return true;
				}
				Err(e) => {
					self.parser_output = format!("Error, invalid elders value '{}'", elders_str);
					return false;
				}
			}
		}

		if let Some(adults_str) = captures.name("adults").map(|m| m.as_str()) {
			self.parser_output = format!("ADULTS: {}", adults_str);
			match adults_str.parse::<usize>() {
				Ok(adults) => {
					self.adults = adults;
					return true;
				}
				Err(e) => {
					self.parser_output = format!("Error, invalid adults value '{}'", adults_str);
					return false;
				}
			}
		}

		if let Some(agebracket) = captures
			.name("initas")
			.or_else(|| captures.name("promoteto"))
			.map(|m| m.as_str())
		{
			self.parser_output = format!("Vault agebracket: {}", agebracket);
			self.agebracket = match agebracket {
				"Infant" => VaultAgebracket::Infant,
				"Adult" => VaultAgebracket::Adult,
				"Elder" => VaultAgebracket::Elder,
				_ => {
					self.parser_output = format!("Error, unkown vault agedbracket '{}'", agebracket);
					VaultAgebracket::Unknown
				}
			};
			return true;
		}

		false
	}

	///! Counts vault activity in categories GET, PUT and other
	pub fn parse_activity_counts(&mut self, entry: &ActivityEntry) {
		if entry.activity.starts_with("Get") {
			self.count_get();
		} else if entry.activity.starts_with("Mut") {
			self.count_put();
		} else {
			self.activity_other += 1;
		}
	}

	fn update_buckets(&mut self) {
		if let Some(mut bucket_time) = self.sparkline_bucket_time {
			if let Some(most_recent) = self.most_recent {
				let mut end_time = bucket_time + self.sparkline_bucket_width;

				while end_time.lt(&most_recent) {
					// Start new bucket
					self.sparkline_bucket_time = Some(end_time);
					bucket_time = end_time;
					end_time = bucket_time + self.sparkline_bucket_width;

					self.gets_sparkline.push(0);
					self.puts_sparkline.push(0);
					if self.gets_sparkline.len() > self.sparkline_buckets {
						self.gets_sparkline.remove(0);
						self.puts_sparkline.remove(0);
					}
				}
			}
		} else {
			self.sparkline_bucket_time = self.most_recent;
		}
	}

	fn count_get(&mut self) {
		self.activity_gets += 1;
		let index = self.gets_sparkline.len() - 1;
		self.gets_sparkline[index] += 1;
	}

	fn count_put(&mut self) {
		self.activity_puts += 1;
		let index = self.puts_sparkline.len() - 1;
		self.puts_sparkline[index] += 1;
	}

	///! TODO
	pub fn parse_logentry_counts(&mut self, entry: &LogEntry) {
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

///! Vault activity for vault activity_history
pub struct ActivityEntry {
	pub activity: String,
	pub logstring: String,
	pub category: String, // First word, "Running", "INFO", "WARN" etc
	pub time: Option<DateTime<FixedOffset>>,
	pub source: String,

	pub parser_output: String,
}

impl ActivityEntry {
	pub fn new(entry: &LogEntry, activity: &str) -> ActivityEntry {
		ActivityEntry {
			activity: activity.to_string(),
			logstring: entry.logstring.clone(),
			category: entry.category.clone(),
			time: entry.time,
			source: entry.source.clone(),

			parser_output: String::from(""),
		}
	}
}

///! Decoded logfile entries for a vault log history
pub struct LogEntry {
	pub logstring: String,
	pub category: String, // First word, "Running", "INFO", "WARN" etc
	pub time: Option<DateTime<FixedOffset>>,
	pub source: String,
	pub message: String,

	pub parser_output: String,
}

impl LogEntry {
	///! Decode vault logfile lines of the form:
	///!    INFO 2020-07-08T19:58:26.841778689+01:00 [src/bin/safe_vault.rs:114]
	///!    WARN 2020-07-08T19:59:18.540118366+01:00 [src/data_handler/idata_handler.rs:744] 552f45..: Failed to get holders metadata from DB
	///!
	pub fn decode(line: &str) -> Option<LogEntry> {
		let mut test_entry = LogEntry {
			logstring: String::from(line),
			category: String::from("test"),
			time: None,
			source: String::from(""),
			message: String::from(""),
			parser_output: String::from("decode()..."),
		};

		if line.is_empty() {
			return None;
		}

		LogEntry::parse_logfile_line(line)
	}

	///! Parse a line of the form:
	///!    INFO 2020-07-08T19:58:26.841778689+01:00 [src/bin/safe_vault.rs:114]
	///!    WARN 2020-07-08T19:59:18.540118366+01:00 [src/data_handler/idata_handler.rs:744] 552f45..: Failed to get holders metadata from DB
	fn parse_logfile_line(line: &str) -> Option<LogEntry> {
		let captures = LOG_LINE_PATTERN.captures(line)?;

		let category = captures.name("category").map_or("", |m| m.as_str());
		let time_string = captures.name("time_string").map_or("", |m| m.as_str());
		let source = captures.name("source").map_or("", |m| m.as_str());
		let message = captures.name("message").map_or("", |m| m.as_str());
		let mut time_str = String::from("None");
		let time = match DateTime::<FixedOffset>::parse_from_rfc3339(time_string) {
			Ok(time) => {
				time_str = format!("{}", time);
				Some(time)
			}
			Err(e) => None,
		};
		let parser_output = format!(
			"c: {}, t: {}, s: {}, m: {}",
			category, time_str, source, message
		);

		Some(LogEntry {
			logstring: String::from(line),
			category: String::from(category),
			time: time,
			source: String::from(source),
			message: String::from(message),
			parser_output,
		})
	}
}

pub enum DashViewMain {
	DashHorizontal,
	DashVertical,
	DashDebug,
}

pub struct DashState {
	pub main_view: DashViewMain,
	pub debug_window: bool,
	pub debug_window_has_focus: bool,
	pub debug_dashboard: bool,
	max_debug_window: usize,

	// For --debug-window option
	pub debug_window_list: StatefulList<String>,

	// For DashViewMain::DashVertical
	dash_vertical: DashVertical,
}

impl DashState {
	pub fn new() -> DashState {
		DashState {
			main_view: DashViewMain::DashHorizontal,
			dash_vertical: DashVertical::new(),
			debug_dashboard: false,
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
	active_view: usize,
}

impl DashVertical {
	pub fn new() -> Self {
		DashVertical { active_view: 0 }
	}
}
