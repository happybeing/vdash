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

use logtail::ui::{draw_dashboard};
use logtail::app::{DashState, LogMonitor, DashViewMain};
use logtail::event::{Event, Events};
use logtail::util::{StatefulList};

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
  backend::Backend,
  backend::TermionBackend,
  layout::{Constraint, Corner, Direction, Layout},
  style::{Color, Modifier, Style},
  text::{Span, Spans, Text},
  widgets::{Widget, Block, BorderType, Borders, List, ListItem},
  Terminal, Frame,
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
    match logfiles.add_file(&f).await {
      Ok(_) => println!("{} done.", &f),
      Err(e) => {
        println!("ERROR: {}", e);
        println!("Note: it is ok for the file not to exist, but the file's parent directory must exist.");
        return Ok(());
      }
    }
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
            terminal.draw(|f| draw_dashboard(f, &dash_state, &mut monitors))?;
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

