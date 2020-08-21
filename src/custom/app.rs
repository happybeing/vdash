///! Application logic
///!
///! Edit src/custom/app.rs to create a customised fork of logtail-dash

use std::fs::File;

use crate::shared::util::{StatefulList};  

pub struct LogMonitor {
  pub index: usize,
  pub content: StatefulList<String>,
  pub logfile:  String,  

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
      content: StatefulList::with_items(vec![]),
    }
  }

  pub fn load_logfile(&mut self) -> std::io::Result<()> {
    use std::io::{BufRead, BufReader};

    let f = File::open(self.logfile.to_string());
    let f = match f {
      Ok(file) => file,
      Err(_e) => return Ok(()),  // It's ok for a logfile not to exist yet
    };

    let f = BufReader::new(f);

    for line in f.lines() {
        let line = line.expect("Unable to read line");
        self.process_line(&line);
    }

    Ok(())
  }

  pub fn process_line(&mut self, text: &str) {
    // TODO parse and update metrics
    self.append_to_content(text);
  }

  pub fn append_to_content(&mut self, text: &str) {
    self.content.items.push(text.to_string());
    if self.content.items.len() > self.max_content {
      self.content.items = self.content.items.split_off(self.content.items.len() - self.max_content);
    }
  }

  fn _reset_metrics(&mut self) {}
}

pub enum DashViewMain {DashHorizontal, DashVertical}

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
    DashVertical { active_view: 0, }
  }
}
