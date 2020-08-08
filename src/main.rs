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

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
    backend::TermionBackend,
    layout::{Constraint, Corner, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Widget, Block, BorderType, Borders, List, ListItem},
    Terminal,
};

struct LogMonitor {
  logfile:  String,  
  index: usize,
}

use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_MONITOR: AtomicUsize = AtomicUsize::new(0);

impl LogMonitor {
  pub fn new(f: String) -> LogMonitor {
    let index = NEXT_MONITOR.fetch_add(1, Ordering::Relaxed);
    LogMonitor {
      logfile: f,
      index,
    }
  }
}

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut monitors: HashMap<String, LogMonitor> = HashMap::new();
    let mut lines = MuxedLines::new()?;

    for f in args {
      let widget = Block::default()
      .title(f.to_string())
      .borders(Borders::LEFT | Borders::RIGHT)
      .border_style(Style::default().fg(Color::White))
      .border_type(BorderType::Rounded)
      .style(Style::default().bg(Color::Black));

      let monitor = LogMonitor::new(f.to_string());
      monitors.insert(f.to_string(), monitor);
      lines.add_file(&f).await?;
    }

    draw_dashboard(&monitors);
    while let Some(Ok(line)) = lines.next().await {
      // println!("({}) {}", line.source().display(), line.line());
      draw_dashboard(&monitors);
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

  let columns_percent = 100 / monitors.len() as u16;
  terminal.draw(|f| {
    let chunks = Layout::default()
      .direction(Direction::Horizontal)
      .constraints([Constraint::Percentage(columns_percent), Constraint::Percentage(50)].as_ref())
      .split(f.size());

      for (logfile, monitor) in monitors.iter() {
      let monitor_list: Vec<ListItem> = vec![ListItem::new("testing...1")];
      let monitor_widget = List::new(monitor_list)
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

