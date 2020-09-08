///! Application logic
///!
///! Edit src/custom/app.rs to create a customised fork of logtail-dash
use linemux::MuxedLines;
use std::collections::HashMap;

use chrono::{DateTime, FixedOffset};
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use structopt::StructOpt;
use tempfile::NamedTempFile;

use crate::custom::opt::Opt;
use crate::shared::util::StatefulList;

pub struct App {
	pub opt: Opt,
	pub dash_state: DashState,
	pub monitors: HashMap<String, LogMonitor>,
	pub logfiles: MuxedLines,
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
		let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
		let mut logfiles = MuxedLines::new()?;
		let mut parser_output: Option<tempfile::NamedTempFile> = if opt.debug_parser {
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
			if opt.debug_parser && monitor.index == 0 {
				if let Some(named_file) = parser_output {
					monitor.metrics.debug_logfile = Some(named_file);
					parser_output = None;
					dash_state.debug_ui = true;
				}
			}
			if opt.ignore_existing {
				monitors.insert(f.to_string(), monitor);
			} else {
				match monitor.load_logfile() {
					Ok(()) => {
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

		Ok(App {
			opt,
			dash_state,
			monitors,
			logfiles,
		})
	}
}

pub struct LogMonitor {
	pub index: usize,
	pub content: StatefulList<String>,
	pub logfile: String,
	pub metrics: VaultMetrics,
	pub metrics_status: StatefulList<String>,
	max_content: usize, // Limit number of lines in content
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
		if self.content.items.len() > self.max_content {
			self.content.items = self
				.content
				.items
				.split_off(self.content.items.len() - self.max_content);
		}
		Ok(())
	}

	// Some logfile lines are too numerous to include so we ignore them
	// Returns true if the line is to be processed
	fn line_filter(&mut self, line: &str) -> bool {
		if line.contains("quinn-") && line.contains("connection.rs:") {
			return false;
		}
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
	pub most_recent: Option<DateTime<FixedOffset>>,
	pub agebracket: VaultAgebracket,
	pub adults: usize,
	pub elders: usize,
	pub activity_gets: u64,
	pub activity_muts: u64,
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

			// Timelines
			activity_history: Vec::<ActivityEntry>::new(),
			log_history: Vec::<LogEntry>::new(),
			most_recent: None,

			// Counts
			category_count: HashMap::new(),
			activity_gets: 0,
			activity_muts: 0,
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
		self.activity_muts = 0;
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
			self.activity_gets += 1;
		} else if entry.activity.starts_with("Mut") {
			self.activity_muts += 1;
		} else {
			self.activity_other += 1;
		}
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
	pub debug_ui: bool,

	// For DashViewMain::DashVertical
	dash_vertical: DashVertical,
}

impl DashState {
	pub fn new() -> DashState {
		DashState {
			main_view: DashViewMain::DashHorizontal,
			dash_vertical: DashVertical::new(),
			debug_ui: false,
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
