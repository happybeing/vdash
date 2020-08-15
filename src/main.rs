//! safe-dash is a learning Rust project which aspires to be a SAFE Network vault montoring dashboard
//!
//! Displays and updates a dashboard based on one or more logfiles
//!
//! Usage:
//!     safe-dash /path/to/file1 [/path/to/file2 ...]
//!
//! The files could be present or not, and the dashboard will monitor each file
//! and use the lines from each file to provide telemetry for that logfile
//! in the dashboard.
//! 
//! Keyboard commands: '?' or 'h' to get help, or 'q' to quit.

#![recursion_limit="256"] // Prevent select! macro blowing up

use std::{error::Error, io};
use std::collections::HashMap;

use linemux::MuxedLines;
use tokio::stream::StreamExt;

mod event;
use crate::event::{Event, Events};

mod util;
use crate::util::{StatefulList};

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
  backend::TermionBackend,
  layout::{Constraint, Corner, Direction, Layout},
  style::{Color, Modifier, Style},
  text::{Span, Spans, Text},
  widgets::{Widget, Block, BorderType, Borders, List, ListItem},
  Terminal,
};

type TuiTerminal = tui::terminal::Terminal<TermionBackend<termion::screen::AlternateScreen<termion::input::MouseTerminal<termion::raw::RawTerminal<std::io::Stdout>>>>>;

use futures::{
  future::FutureExt, // for `.fuse()`
  pin_mut,
  select,
};

struct LogMonitor {
  index: usize,
  logfile:  String,  
  max_content: usize, // Limit number of lines in content
  content: StatefulList<String>,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);

impl LogMonitor {
  pub fn new(f: String) -> LogMonitor {
    let index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);
    LogMonitor {
      index,
      logfile: f,
      max_content: 100,
      content: StatefulList::with_items(vec!["test string".to_string()]),
    }
  }

  pub fn append_to_content(&mut self, text: &str) {
    self.content.items.push(text.to_string());
    if self.content.items.len() > self.max_content {
      self.content.items = self.content.items.split_off(self.content.items.len() - self.max_content);
    }
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
  if args.is_empty() {
    let command_path = std::env::current_exe().unwrap();
    println!("Usage: {} logfile1 [logfile2 ...]", command_path.file_name().unwrap().to_string_lossy());
    println!();
    println!("A dashboard to display the last few lines of one or more logfiles.");
    return Ok(());
  }

  let mut dash_state = DashState::new();

  let events = Events::new();
  let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
  let mut logfiles = MuxedLines::new()?;

  for f in args {
    let monitor = LogMonitor::new(f.to_string());
    monitors.insert(f.to_string(), monitor);
    logfiles.add_file(&f).await?;
  }

  // Terminal initialization
  let stdout = io::stdout().into_raw_mode()?;
  let stdout = MouseTerminal::from(stdout);
  let stdout = AlternateScreen::from(stdout);
  let backend = TermionBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;


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
                Key::Char('q')|
                Key::Char('Q') => return Ok(()),
                Key::Char('s')|
                Key::Char('S') => dash_state.main_view = DashViewMain::DashSummary,
                Key::Char('d')|
                Key::Char('D') => dash_state.main_view = DashViewMain::DashDetail,
                Key::Down => monitors.get_mut("/var/log/auth.log").unwrap().content.next(),
                Key::Up => monitors.get_mut("/var/log/auth.log").unwrap().content.previous(),
              _ => {},
              }
          }
          
          Ok(Event::Tick) => {
            draw_dashboard(&mut terminal, &dash_state, &mut monitors).unwrap();
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

fn draw_dashboard(
  terminal: &mut TuiTerminal, 
  dash_state: &DashState, 
  monitors: &mut HashMap<String, 
  LogMonitor>)
  -> std::io::Result<()> {

  match dash_state.main_view {
    DashViewMain::DashSummary => draw_dash_summary(terminal, dash_state, monitors),
    DashViewMain::DashDetail => draw_dash_detail(terminal, dash_state, monitors),
  }
}

fn draw_dash_summary(
  terminal: &mut TuiTerminal, 
  dash_state: &DashState, 
  monitors: &mut HashMap<String, 
  LogMonitor>)
  -> std::io::Result<()> {
  
  let constraints = make_percentage_constraints(monitors.len());

  terminal.draw(|f| {
    let size = f.size();
    let block = Block::default()
    .borders(Borders::ALL)
    .title("SAFE Vault Monitor: SUMMARY ")
    .border_type(BorderType::Rounded);
    f.render_widget(block, size);
    
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .margin(1)
      .constraints(constraints.as_ref())
      .split(size);

    for (logfile, monitor) in monitors.iter_mut() {
      monitor.content.state.select(Some(monitor.content.items.len()-1));
      let items: Vec<ListItem> = monitor.content.items.iter().map(|s| {
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
      f.render_stateful_widget(monitor_widget,chunks[monitor.index], &mut monitor.content.state);
    }
  })
}

fn draw_dash_detail(
  terminal: &mut TuiTerminal, 
  dash_state: &DashState, 
  monitors: &mut HashMap<String, 
  LogMonitor>)
  -> std::io::Result<()> {
  
  let constraints = make_percentage_constraints(monitors.len());
  terminal.draw(|f| {
    let size = f.size();
    let block = Block::default()
      .borders(Borders::ALL)
      .title("SAFE Vault Monitor:  DETAIL ")
      .border_type(BorderType::Rounded);
    f.render_widget(block, size);

    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .margin(1)
      .constraints(constraints.as_ref())
      .split(size);

    for (logfile, monitor) in monitors.iter_mut() {
      monitor.content.state.select(Some(monitor.content.items.len()-1));
      let items: Vec<ListItem> = monitor.content.items.iter().map(|s| {
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
      f.render_stateful_widget(monitor_widget,chunks[monitor.index], &mut monitor.content.state);
    }
  })
}

fn make_percentage_constraints(count: usize) -> Vec<Constraint> {
  let percent = 100 / count as u16;
  let mut constraints = Vec::new();
  let mut total_percent = 0;

  for i in 1..count+1 {
    total_percent += percent;

    let next_percent = if i == count && total_percent < 100 
      { 100 - total_percent } else { percent };

    constraints.push(Constraint::Percentage(next_percent));
  }
  constraints
}
