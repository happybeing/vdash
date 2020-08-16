//! logtail is a logfile monitoring dashboard in the terminal
//!
//! Displays and updates a dashboard based on one or more logfiles
//!
//! Example:
//!   logtail /var/log/auth.log /var/log/kern.log
//! 
//! Press 'v' and 'h' for a vertical or horizontal layout.
//! 
//! See README or try `logtail -h` for more information.

#![recursion_limit="256"] // Prevent select! macro blowing up

use std::io;
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

use std::fs::File;
use std::io::{BufRead, BufReader};

use futures::{
  future::FutureExt, // for `.fuse()`
  pin_mut,
  select,
};

static MAX_CONTENT: &str = "100";

struct LogMonitor {
  index: usize,
  logfile:  String,  
  max_content: usize, // Limit number of lines in content
  content: StatefulList<String>,
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
    let f = File::open(self.logfile.to_string());
    let mut f = match f {
      Ok(file) => file,
      Err(e) => return Ok(()),  // It's ok for a logfile not to exist yet
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

enum DashViewMain {DashHorizontal, DashVertical}

struct DashState {
  main_view: DashViewMain,

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

struct DashVertical {
  active_view: usize,
}

impl DashVertical {
  pub fn new() -> Self { 
    DashVertical { active_view: 0, }
  }
}

use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(about = "Monitor multiple logfiles in the terminal.")]
struct Opt {
  /// Maximum number of lines to keep for each logfile
  #[structopt(short = "l", long, default_value = MAX_CONTENT)]
  lines_max: usize,

  /// Ignore any existing logfile content
  #[structopt(short, long)]
  ignore_existing: bool,

  /// One or more logfiles to monitor
  #[structopt(name = "LOGFILE")]
  files: Vec<String>,
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
  let opt = Opt::from_args();

  if opt.files.is_empty() {
    println!("{}: no logfile(s) specified.", Opt::clap().get_name());
    println!("Try '{} --help' for more information.", Opt::clap().get_name());
    return Ok(());
  }

  let mut dash_state = DashState::new();
  let events = Events::new();
  let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
  let mut logfiles = MuxedLines::new()?;

  println!("Loading...");
  for f in opt.files {
    let mut monitor = LogMonitor::new(f.to_string(), opt.lines_max);
    println!("{}", monitor.logfile);
    if opt.ignore_existing {
        monitors.insert(f.to_string(), monitor);
    } else {
        match monitor.load_logfile() {
        Ok(()) => {monitors.insert(f.to_string(), monitor);},
        Err(e) => {
          println!("...failed: {}", e);
          return Ok(());
        },
      }
    }
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
                Key::Char('h')|
                Key::Char('H') => dash_state.main_view = DashViewMain::DashHorizontal,
                Key::Char('v')|
                Key::Char('V') => dash_state.main_view = DashViewMain::DashVertical,
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
    DashViewMain::DashHorizontal => draw_dash_horizontal(terminal, dash_state, monitors),
    DashViewMain::DashVertical => draw_dash_vertical(terminal, dash_state, monitors),
  }
}

fn draw_dash_horizontal(
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
    .title(" logtail ")
    .border_type(BorderType::Rounded);
    f.render_widget(block, size);
    
    let chunks = Layout::default()
      .direction(Direction::Vertical)
      .margin(1)
      .constraints(constraints.as_ref())
      .split(size);

    for (logfile, monitor) in monitors.iter_mut() {
      let len = monitor.content.items.len();
      if len > 0 {monitor.content.state.select(Some(monitor.content.items.len()-1));}

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

fn draw_dash_vertical(
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
      .title(" logtail ")
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
  let percent = if count > 0 { 100 / count as u16 } else { 0 };
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
