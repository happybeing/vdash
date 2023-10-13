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

use crossterm::{
	event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode},
	execute,
	terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::{
	error::Error,
	io::stdout,
	thread,
	time::{Duration, Instant,SystemTime, UNIX_EPOCH},
};

use chrono::Utc;

use ratatui::{backend::CrosstermBackend, Terminal};

use futures::{
	future::FutureExt, // for `.fuse()`
	pin_mut,
	select,
};

enum Event<I> {
	Input(I),
	Tick,
}

use tokio_stream::StreamExt;
use tokio::sync::mpsc;

// RUSTFLAGS="-A unused" cargo run --bin logtail-crossterm --features="crossterm" /var/log/auth.log /var/log/dmesg
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();
	info!("Started");

	let mut app = match App::new().await {
		Ok(app) => app,
		Err(_e) => return Ok(()),
	};

	// Terminal initialization
	enable_raw_mode()?;
	let mut stdout = stdout();
	execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let mut rx = initialise_events(app.opt.tick_rate);
	terminal.clear()?;

	// Use futures of async functions to handle events
	// concurrently with logfile changes.

	let start = SystemTime::now()
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

		let logfiles_future = app.logfiles.next().fuse();
		let events_future = rx.recv().fuse();
		pin_mut!(logfiles_future, events_future);

		select! {
			e = events_future => {
			match e {
				Some(Event::Input(event)) => {
					match event.code {
						// For debugging, ~ sends a line to the debug_window
						KeyCode::Char('~') => app.dash_state._debug_window(format!("Event::Input({:#?})", event).as_str()),

						KeyCode::Char('q')|
						KeyCode::Char('Q') => {
							disable_raw_mode()?;
							execute!(
								terminal.backend_mut(),
								LeaveAlternateScreen,
								DisableMouseCapture
							)?;
							terminal.show_cursor()?;
							break Ok(());
						},
						KeyCode::Char('s')|
						KeyCode::Char('S') => set_main_view(DashViewMain::DashSummary, &mut app),
						KeyCode::Char('h')|
						KeyCode::Char('H')|
						KeyCode::Char('?') => set_main_view(DashViewMain::DashHelp, &mut app),
						KeyCode::Char('n')|
						KeyCode::Char('N') => set_main_view(DashViewMain::DashNode, &mut app),

						KeyCode::Char('+')|
						KeyCode::Char('i')|
						KeyCode::Char('I') => app.scale_timeline_up(),
						KeyCode::Char('-')|
						KeyCode::Char('o')|
						KeyCode::Char('O') => app.scale_timeline_down(),

						KeyCode::Char('l')|
						KeyCode::Char('L') => app.toggle_logfile_area(),

						KeyCode::Char('m')|
						KeyCode::Char('M') => app.bump_mmm_ui_mode(),

						KeyCode::Char('j')|
						KeyCode::Char('J') => app.jump_to_node(),

						KeyCode::Char('t') => app.top_timeline_next(),
						KeyCode::Char('T') => app.top_timeline_previous(),

						KeyCode::Down => app.handle_arrow_down(),
						KeyCode::Up => app.handle_arrow_up(),
						KeyCode::Right|
						KeyCode::Tab => app.change_focus_next(),
						KeyCode::Left => app.change_focus_previous(),

						KeyCode::Char('g') => {
							if app.opt.debug_window { set_main_view(DashViewMain::DashDebug, &mut app); }
						},
						_ => {}
					};
					terminal.draw(|f| draw_dashboard(f, &mut app)).unwrap();
				}

				Some(Event::Tick) => {
					app.update_timelines(&Utc::now());
				// draw_dashboard(&mut f, &dash_state, &mut monitors).unwrap();
				// draw_dashboard(f, &dash_state, &mut monitors)?;
				}

				None => {},
			}
			},

			line = logfiles_future => {
			match line {
				Some(Ok(line)) => {
					trace!("logfiles_future line");
					let source_str = line.source().to_str().unwrap();
					let source = String::from(source_str);
					// app.dash_state._debug_window(format!("{}: {}", source, line.line()).as_str());

					match app.get_monitor_for_file_path(&source) {
						Some(monitor) => {
							monitor.append_to_content(line.line())?;
							if monitor.is_debug_dashboard_log {
								app.dash_state._debug_window(line.line());
							} else if app.dash_state.main_view == DashViewMain::DashSummary {
								app.dash_state.update_summary_window(&mut app.monitors);
							}
						},
						None => {
							app.dash_state._debug_window(format!("NO MONITOR FOR: {}", source).as_str());
						},
					}
				},
				Some(Err(e)) => {
					app.dash_state._debug_window(format!("logfile error: {:#?}", e).as_str());
					panic!("{}", e)
				}
				None => {
					app.dash_state._debug_window(format!("logfile error: None").as_str());
					()
				}
		}
			},
		}
	}
}
type Rx = tokio::sync::mpsc::UnboundedReceiver<Event<crossterm::event::KeyEvent>>;

fn initialise_events(tick_rate: u64) -> Rx {
	let tick_rate = Duration::from_millis(tick_rate);
	let (tx, rx) = mpsc::unbounded_channel(); // Setup input handling

	thread::spawn(move || {
		let mut last_tick = Instant::now();
		loop {
			// poll for tick rate duration, if no events, sent tick event.
			if event::poll(tick_rate - last_tick.elapsed()).unwrap() {
				if let CEvent::Key(key) = event::read().unwrap() {
					match tx.send(Event::Input(key)) {
						Ok(()) => {},
						Err(e) => println!("send error: {}", e),

					}
				}
			}
			if last_tick.elapsed() >= tick_rate {
				match tx.send(Event::Tick) {
					Ok(()) => last_tick = Instant::now(),
					Err(e) => println!("send error: {}", e),

				}
			}

			if last_tick.elapsed() >= tick_rate {
				match tx.send(Event::Tick) {
					Ok(()) => last_tick = Instant::now(),
					Err(e) => println!("send error: {}", e),
				}
			}
		}
	});
	rx
}
