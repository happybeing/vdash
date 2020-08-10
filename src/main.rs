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

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let events = Events::new();
    let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
    let mut logfiles = MuxedLines::new()?;

    for f in args {
      let monitor = LogMonitor::new(f.to_string());
      monitors.insert(f.to_string(), monitor);
      logfiles.add_file(&f).await?;
    }

    loop {
      let e = match events.next() {
        Ok(Event::Input(input)) => {
            if input == Key::Char('q') {
                return Ok(());
            }
        }
        
        Ok(Event::Tick) => {
          draw_dashboard(&monitors).unwrap();
        }

        Err(error) => {
          println!("{}", error);
        }
      };
    }

    draw_dashboard(&monitors).unwrap();
    while let Some(Ok(line)) = logfiles.next().await {
      // println!("({}) {}", line.source().display(), line.line());
      let source_str = line.source().to_str().unwrap();
      let source = String::from(source_str);

      match monitors.get_mut(&source) {
        None => (),
        Some(monitor) => monitor.append_to_content(line.line())
      }
      draw_dashboard(&monitors).unwrap();
    }

    Ok(())
}

fn draw_dashboard(monitors: &HashMap<String, LogMonitor>) -> std::io::Result<()> {
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
          .title("safe-dash SAFE vault montoring dashboard")
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

