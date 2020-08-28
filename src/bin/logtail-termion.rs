//! This app monitors and logfiles and displays status in the terminal
//!
//! It is based on logtail-dash, which is a basic logfile dashboard
//! and also a framework for similar apps with customised dahsboard
//! displays.
//!
//! Custom apps based on logtail can be created by creating a
//! fork of logtail-dash and modifying the files in src/custom
//!
//! See README for more information.

#![recursion_limit = "256"] // Prevent select! macro blowing up

use std::io;

///! forks of logterm customise the files in src/custom
#[path = "../custom/mod.rs"]
pub mod custom;
use self::custom::app::{App, DashViewMain};
use self::custom::opt::Opt;
use self::custom::ui::draw_dashboard;

///! logtail and its forks share code in src/
#[path = "../mod.rs"]
pub mod shared;
use crate::shared::util::StatefulList;
use shared::event::{Event, Events};

use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};
use tui::{
	backend::Backend,
	backend::TermionBackend,
	layout::{Constraint, Corner, Direction, Layout},
	style::{Color, Modifier, Style},
	text::{Span, Spans, Text},
	widgets::{Block, BorderType, Borders, List, ListItem, Widget},
	Frame, Terminal,
};

type TuiTerminal = tui::terminal::Terminal<
	TermionBackend<
		termion::screen::AlternateScreen<
			termion::input::MouseTerminal<termion::raw::RawTerminal<std::io::Stdout>>,
		>,
	>,
>;

use std::fs::File;
use std::io::{BufRead, BufReader};
use structopt::StructOpt;

use futures::{
	future::FutureExt, // for `.fuse()`
	pin_mut,
	select,
};

use tokio::stream::StreamExt;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
	let mut app = match App::new().await {
		Ok(app) => app,
		Err(e) => return Ok(()),
	};

	let events = Events::new();

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
		let logfiles_future = app.logfiles.next().fuse();
		pin_mut!(events_future, logfiles_future);

		select! {
			(e) = events_future => {
			match e {
				Ok(Event::Input(input)) => {
					match input {
					Key::Char('q')|
					Key::Char('Q') => return Ok(()),
					Key::Char('h')|
					Key::Char('H') => app.dash_state.main_view = DashViewMain::DashHorizontal,
					Key::Char('v')|
					Key::Char('V') => app.dash_state.main_view = DashViewMain::DashVertical,
					Key::Char('D') => app.dash_state.main_view = DashViewMain::DashDebug,
					_ => {},
					}
				}

				Ok(Event::Tick) => {
				terminal.draw(|f| draw_dashboard(f, &app.dash_state, &mut app.monitors))?;
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

				match app.monitors.get_mut(&source) {
					Some(monitor) => monitor.append_to_content(line.line())?,
					None => (),
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
