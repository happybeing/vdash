//! safe-dash is a learning Rust project which aspires to be a SAFE Network vault montoring dashboard
//!
//! Builds a dashboard based on one or more logfiles
//!
//! Usage:
//!     safe-dash /path/to/file1 [/path/to/file2 ...]
//!
//! The files could be present or not, and the dashboard will monitor each file
//! and use the lines from each file to provide telemetry for that logfile
//! in the dashboard.

#![recursion_limit="256"] // Prevent select! macro blowing up

use std::{error::Error, io};
use std::collections::HashMap;

use linemux::MuxedLines;
use tokio::stream::StreamExt;

mod event;
use crate::event::{Event, Events};

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Widget, Block, BorderType, Borders, List, ListItem},
    Terminal,
};

use futures::{
  future::FutureExt, // for `.fuse()`
  pin_mut,
  select,
};

struct LogMonitor {
  index: usize,
  logfile:  String,  
  content: Vec<String>,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);

impl LogMonitor {
  pub fn new(f: String) -> LogMonitor {
    let index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);
    LogMonitor {
      index,
      logfile: f,
      content: vec!["test string".to_string()],
    }
  }

  pub fn append_to_content(&mut self, text: &str) {
    self.content.push(text.to_string());
  }
}

enum DashViewMain {DashSummary, DashDetail}

struct DashState {
  main_view: DashViewMain,

  // For DashViewMain::dashDetail
  dash_detail: DashDetail,
}

impl DashState {
  pub fn new() -> DashState { 
    DashState {
      main_view: DashViewMain::DashSummary, 
      dash_detail: DashDetail::new(),
    }
  }
}

struct DashDetail {
  active_view: usize,
}

impl DashDetail {
  pub fn new() -> Self { 
    DashDetail { active_view: 0, }
  }
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut dash_state = DashState::new();

    let events = Events::new();
    let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
    let mut logfiles = MuxedLines::new()?;

    for f in args {
      let monitor = LogMonitor::new(f.to_string());
      monitors.insert(f.to_string(), monitor);
      logfiles.add_file(&f).await?;
    }

    // Use futures of async functions to handle events
    // concurrently with logfile changes.
    loop {
      let events_future = next_event(&events).fuse();
      let logfiles_future = logfiles.next().fuse();
      pin_mut!(events_future, logfiles_future);
    
      select! {
        (e) = events_future => {
          match e {
            Ok(Event::Input(input)) => {
                match input {
                  Key::Char('q') => return Ok(()),
                  Key::Char('s')|
                  Key::Char('S') => dash_state.main_view = DashViewMain::DashSummary,
                  Key::Char('d')|
                  Key::Char('D') => dash_state.main_view = DashViewMain::DashDetail,
                  _ => {},
                }
            }
            
            Ok(Event::Tick) => {
              draw_dashboard(&dash_state, &monitors).unwrap();
            }
    
            Err(error) => {
              println!("{}", error);
            }
          }
        },
        (line) = logfiles_future => {
          match line {
            Some(Ok(line)) => {
              let source_str = line.source().to_str().unwrap();
              let source = String::from(source_str);
        
              match monitors.get_mut(&source) {
                None => (),
                Some(monitor) => monitor.append_to_content(line.line())
              }
            },
            Some(Err(e)) => panic!("{}", e),
            None => (),
          }
        },
      }
    }
}

use std::sync::mpsc;

async fn next_event(events: &Events) -> Result<Event<Key>, mpsc::RecvError> {
  events.next()
}

fn draw_dashboard(dash_state: &DashState, monitors: &HashMap<String, LogMonitor>) -> std::io::Result<()> {
  match dash_state.main_view {
    DashViewMain::DashSummary => draw_dash_summary(dash_state, monitors),
    DashViewMain::DashDetail => draw_dash_detail(dash_state, monitors),
  }
}

fn draw_dash_summary(dash_state: &DashState, monitors: &HashMap<String, LogMonitor>) -> std::io::Result<()> {
  // Terminal initialization
  let stdout = io::stdout().into_raw_mode()?;
  let stdout = MouseTerminal::from(stdout);
  let stdout = AlternateScreen::from(stdout);
  let backend = TermionBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // TODO provide a constraint *per* monitor
  let columns_percent = 100 / monitors.len() as u16;
  terminal.draw(|f| {
    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(100)].as_ref())
      .split(f.size());

      let size = f.size();
      let block = Block::default()
          .borders(Borders::ALL)
          .title("safe-dash SAFE vault montoring dashboard == SUMMARY == ")
          .border_type(BorderType::Rounded);
      f.render_widget(block, size);

    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .margin(1)
      .constraints([Constraint::Percentage(columns_percent), Constraint::Percentage(50)].as_ref())
      .split(size);

      for (logfile, monitor) in monitors.iter() {
        let items: Vec<ListItem> = monitor.content.iter().map(|s| {
            ListItem::new(vec![Spans::from(s.clone())]).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

        let monitor_widget = List::new(items)
          .block(Block::default().borders(Borders::ALL).title(logfile.clone()))
          .highlight_style(
              Style::default()
                  .bg(Color::LightGreen)
                  .add_modifier(Modifier::BOLD),
          );
        f.render_widget(monitor_widget,chunks[monitor.index]);
      }
  })
}

fn draw_dash_detail(dash_state: &DashState, monitors: &HashMap<String, LogMonitor>) -> std::io::Result<()> {
  // Terminal initialization
  let stdout = io::stdout().into_raw_mode()?;
  let stdout = MouseTerminal::from(stdout);
  let stdout = AlternateScreen::from(stdout);
  let backend = TermionBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;

  // TODO provide a constraint *per* monitor
  let columns_percent = 100 / monitors.len() as u16;
  terminal.draw(|f| {
    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(100)].as_ref())
      .split(f.size());

      let size = f.size();
      let block = Block::default()
          .borders(Borders::ALL)
          .title("safe-dash SAFE vault montoring dashboard == DETAIL == ")
          .border_type(BorderType::Rounded);
      f.render_widget(block, size);

    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .margin(1)
      .constraints([Constraint::Percentage(columns_percent), Constraint::Percentage(50)].as_ref())
      .split(size);

      for (logfile, monitor) in monitors.iter() {
        let items: Vec<ListItem> = monitor.content.iter().map(|s| {
            ListItem::new(vec![Spans::from(s.clone())]).style(Style::default().fg(Color::Black).bg(Color::White))
        })
        .collect();

        let monitor_widget = List::new(items)
          .block(Block::default().borders(Borders::ALL).title(logfile.clone()))
          .highlight_style(
              Style::default()
                  .bg(Color::LightGreen)
                  .add_modifier(Modifier::BOLD),
          );
        f.render_widget(monitor_widget,chunks[monitor.index]);
      }
  })
}

