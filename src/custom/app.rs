///! Application logic
///!
///! Edit src/custom/app.rs to create a customised fork of logtail-dash
use linemux::MuxedLines;
use std::collections::HashMap;

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
      dash_state.main_view = DashViewMain::DashVertical;
      opt.files = opt.files[0..1].to_vec();
      let named_file = NamedTempFile::new()?;
      let path = named_file.path();
      let path_str = path.to_str().unwrap();
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
          monitor.parser_logfile = Some(named_file);
          parser_output = None;
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
  pub parser_logfile: Option<NamedTempFile>,
  pub metrics: VaultMetrics,
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
      parser_logfile: None,
      metrics: VaultMetrics::new(),
      content: StatefulList::with_items(vec![]),
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
      self.process_line(&line)?
    }

    Ok(())
  }

  pub fn process_line(&mut self, line: &str) -> Result<(), std::io::Error> {
    // For debugging LogEntry::decode()
    let mut parser_result = String::from("");
    if self.metrics.parse_timeline(line) {
      if let Some(log_entry) = self.metrics.timeline.last() {
        parser_result = log_entry.parser_debug.clone()
      }
    }
    // For debugging the metric code
    // let parser_result = self.metrics.get_debug_parser_text();

    // --debug-parser - prints parser results for a single logfile
    // to a temp logfile which is displayed in the adjacent window.
    match &self.parser_logfile {
      Some(f) => {
        use std::io::Seek;
        let mut file = f.reopen()?;
        file.seek(std::io::SeekFrom::End(0))?;
        writeln!(file, "{}", &parser_result)?
      }
      None => (),
    };

    // Show in TUI
    self.append_to_content(line)
  }

  pub fn append_to_content(&mut self, text: &str) -> Result<(), std::io::Error> {
    self.content.items.push(text.to_string());
    if self.content.items.len() > self.max_content {
      self.content.items = self
        .content
        .items
        .split_off(self.content.items.len() - self.max_content);
    }
    Ok(())
  }

  fn _reset_metrics(&mut self) {}
}

use regex::Regex;
use time::Time;

lazy_static::lazy_static! {
  static ref LOG_LINE_PATTERN: Regex =
    Regex::new(r"(?P<category>^[A-Z]{4}) (?P<time_string>[^ ]{35}) (?P<source>\[.*\]) (?P<message>.*)").expect("The regex failed to compile. This is a bug.");
}

///! Decoded logfile entries for a vault timeline metric
pub struct LogEntry {
  pub logstring: String,
  pub category: String, // First word, "Running", "INFO", "WARN" etc
  pub time: Option<Time>,
  pub source: String,
  pub message: String,

  parser_debug: String,
}

impl LogEntry {
  ///! Decode vault logfile lines of the form:
  ///!    Running safe-vault v2.2.23
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
      parser_debug: String::from("decode()..."),
    };

    if line.is_empty() {
      return None;
    }

    LogEntry::parse_info_line(line)
  }

  ///! Parse a line of the form:
  ///!    INFO 2020-07-08T19:58:26.841778689+01:00 [src/bin/safe_vault.rs:114]
  ///!    WARN 2020-07-08T19:59:18.540118366+01:00 [src/data_handler/idata_handler.rs:744] 552f45..: Failed to get holders metadata from DB
  fn parse_info_line(line: &str) -> Option<LogEntry> {
    let captures = LOG_LINE_PATTERN.captures(line)?;

    let category = captures.name("category").map_or("", |m| m.as_str());
    let time_string = captures.name("time_string").map_or("", |m| m.as_str());
    let source = captures.name("source").map_or("", |m| m.as_str());
    let message = captures.name("message").map_or("", |m| m.as_str());

    let parser_debug = format!(
      "c: {}, t: {}, s: {}, m: {}",
      category, time_string, source, message
    );

    Some(LogEntry {
      logstring: String::from(line),
      category: String::from(category),
      time: None,
      source: String::from(source),
      message: String::from(message),
      parser_debug,
    })
  }
}

pub struct VaultMetrics {
  pub vault_started: Option<Time>,
  pub running_message: Option<String>,
  pub running_version: Option<String>,
  pub category_count: HashMap<String, usize>,
  pub timeline: Vec<LogEntry>,
  pub most_recent: Option<Time>,

  parser_debug: String,
}

impl VaultMetrics {
  fn new() -> VaultMetrics {
    VaultMetrics {
      // Start
      vault_started: None,
      running_message: None,
      running_version: None,

      // Timeline
      timeline: Vec::<LogEntry>::new(),
      most_recent: None,

      // Counts
      category_count: HashMap::new(),
      // State

      // Debug
      parser_debug: String::from("-"),
    }
  }

  ///! Start is found when we capture a line beginning with 'Running' such as:
  ///!    'Running safe-vault v0.24.0'
  // pub fn parse_start(&mut self, text: &str) {
  //   match self
  //     .running_message
  //     .as_ref()
  //     .and(self.running_version.as_ref())
  //   {
  //     Some(_) => return,
  //     None => {
  //       let running_prefix = String::from("Running safe-vault ");
  //       if text.starts_with(&running_prefix) {
  //         self.running_message = Some(text.to_string());
  //         self.running_version = Some(text[running_prefix.len()..].to_string());
  //         self.vault_started = Some(self.most_recent);
  //       }
  //     }
  //   }
  //   ()
  // }
  pub fn parse_counts(&mut self, text: &str) {}
  pub fn parse_states(&mut self, text: &str) {}

  ///! Captures entries for timeline displays as Vec<LogEntry>
  ///! Captures most_recent log entry time
  ///! Returns true if an entry was added to self.timeline[]
  pub fn parse_timeline(&mut self, text: &str) -> bool {
    match LogEntry::decode(text) {
      Some(mut entry) => {
        if entry.time.is_none() {
          entry.time = self.most_recent;
        }

        match entry.time {
          Some(t) => self.most_recent = Some(t),
          None => {}
        };

        self.timeline.push(entry);

        // TODO Trim timeline

        true
      }
      None => false,
    }
  }

  pub fn get_debug_parser_text(&mut self) -> &String {
    //   match self
    //     .running_message
    //     .as_ref()
    //     .and(self.running_version.as_ref())
    //   {
    //     Some(v) => format!("Vault Version: {}", v),
    //     None => String::from("-"),
    //   }
    // }
    &self.parser_debug
  }
}

pub enum DashViewMain {
  DashHorizontal,
  DashVertical,
}

pub struct DashState {
  pub main_view: DashViewMain,

  // For DashViewMain::DashVertical
  dash_vertical: DashVertical,
}

impl DashState {
  pub fn new() -> DashState {
    DashState {
      main_view: DashViewMain::DashHorizontal,
      dash_vertical: DashVertical::new(),
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
