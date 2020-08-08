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

use std::collections::HashMap;

use linemux::MuxedLines;
use tokio::stream::StreamExt;

// use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
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
}

impl LogMonitor {
  pub fn new(f: String) -> LogMonitor {
    LogMonitor {
      logfile: f,
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

    while let Some(Ok(line)) = lines.next().await {
      println!("({}) {}", line.source().display(), line.line());
    }

    Ok(())
}

