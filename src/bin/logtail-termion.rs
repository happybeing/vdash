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

#![recursion_limit = "1024"] // Prevent select! macro blowing up

use std::io;

///! forks of logterm customise the files in src/custom
#[path = "../custom/mod.rs"]
pub mod custom;
use self::custom::app::{set_main_view, App, DashViewMain};
use self::custom::ui::draw_dashboard;

#[macro_use]
extern crate log;
extern crate env_logger;

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

use std::{
	io::{Error, ErrorKind},
	time::{Duration, SystemTime, UNIX_EPOCH},
};

use chrono::Utc;

use futures::{
	future::FutureExt, // for `.fuse()`
	pin_mut,
	select,
};

use tokio::stream::StreamExt;

#[tokio::main]
pub async fn main() -> std::io::Result<()> {
	env_logger::init();
	info!("Started");

	match terminal_main().await {
		Ok(()) => (),
		Err(e) => println!("{}", e),
	}
	Ok(())
}

async fn terminal_main() -> std::io::Result<()> {
	let mut app = match App::new().await {
		Ok(app) => app,
		Err(e) => {
			return Err(e);
		}
	};

	let mut events = Events::new();

	// Terminal initialization
	info!("Intialising terminal (termion backend)");
	let stdout = io::stdout().into_raw_mode()?;
	let stdout = MouseTerminal::from(stdout);
	let stdout = AlternateScreen::from(stdout);
	let backend = TermionBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;

	// Use futures of async functions to handle events
	// concurrently with logfile changes.
	info!("Processing started");

	let mut start = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.expect("Time went backwards");
	let mut next_update = start - Duration::from_secs(2);
	loop {
		if next_update < SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.expect("Time went backwards") {
			terminal.draw(|f| draw_dashboard(f, &mut app))?;
			next_update += Duration::from_secs(1);
		}

		let events_future = events.rx.recv().fuse();
		let logfiles_future = app.logfiles.next().fuse();
		pin_mut!(events_future, logfiles_future);

		select! {
			(e) = events_future => {
				match e {
					Some(Event::Input(input)) => {
						match input {
							// For debugging, ~ sends a line to the debug_window
							Key::Char('~') => app.dash_state._debug_window(format!("Event::Input({:#?})", input).as_str()),

							Key::Char('q')|
							Key::Char('Q') => return Ok(()),
						// Key::Char('s')|
						// Key::Char('S') => app.set_main_view(DashViewMain::DashSummary),
							Key::Char('v')|
							Key::Char('V') => set_main_view(DashViewMain::DashVault, &mut app),

							Key::Char('+')|
							Key::Char('i')|
							Key::Char('I') => app.scale_timeline_up(),
							Key::Char('-')|
							Key::Char('o')|
							Key::Char('O') => app.scale_timeline_down(),
	
							Key::Down => app.handle_arrow_down(),
							Key::Up => app.handle_arrow_up(),
							Key::Right|
							Key::Char('\t') => app.change_focus_next(),
							Key::Left => app.change_focus_previous(),

							Key::Char('g') => set_main_view(DashViewMain::DashDebug, &mut app),
								_ => {},
						};
						match terminal.draw(|f| draw_dashboard(f, &mut app)) {
							Ok(_) => {},
							Err(e) => {
								error!("terminal.draw() '{:#?}'", e);
								return Err(e);
							}
						};
					}

					Some(Event::Tick) => {
						trace!("Event::Tick");
						app.update_timelines(Some(Utc::now()));
						match terminal.draw(|f| draw_dashboard(f, &mut app)) {
							Ok(_) => {},
							Err(e) => {
								error!("terminal.draw() '{:#?}'", e);
								return Err(e);
							}
						};
						trace!("Event::Tick DONE");
					}

					None => (),
				}
			},
			(line) = logfiles_future => {
				trace!("logfiles_future line");
				match line {
					Some(Ok(line)) => {
						let source_str = line.source().to_str().unwrap();
						let source = String::from(source_str);
						app.dash_state._debug_window(format!("{}: {}", source, line.line()).as_str());

						match app.get_monitor_for_file_path(&source) {
							Some(monitor) => {
								trace!("APPENDING: {}", line.line());
								monitor.append_to_content(line.line())?;
								if monitor.is_debug_dashboard_log {
									app.dash_state._debug_window(line.line());
								}
							},
							None => (),
						}
					},
					Some(Err(e)) => {
						app.dash_state._debug_window(format!("logfile error: {:#?}", e).as_str());
						error!("logfiles error '{:#?}'", e);
						return Err(e)
					},
					None => {
						app.dash_state._debug_window(format!("logfile error: None").as_str());
						()
					}
				}
			},
		}
	}
}
