//! This app monitors logfiles and displays status in the terminal
//!
//! It is based on logtail-dash, which is a basic logfile dashboard
//! and also a framework for similar apps with customised dashboard
//! displays.
//!
//! Custom apps based on logtail can be created by creating a
//! fork of logtail-dash and modifying the files in src/custom
//!
//! See README for more information.

#![recursion_limit = "1024"] // Prevent select! macro blowing up

#[path = "../custom/mod.rs"]
pub mod custom;
use self::custom::app::{OPT, App, DashViewMain};
use self::custom::ui::draw_dashboard;

#[macro_use]
extern crate log;
extern crate env_logger;

///! logtail and its forks share code in src/
#[path = "../mod.rs"]
pub mod shared;

use crossterm::{
	event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
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

pub enum Event<I> {
	Input(I),
	Tick,
}

use tokio_stream::StreamExt;
use tokio::sync::mpsc;

#[tokio::main]
pub async fn main() -> Result<(), Box<dyn Error>> {
	let (opt_tick_rate, checkpoint_interval, opt_debug_window,
		coingecho_api_key, coinmarketcap_api_key, currency_apiname) = {
		let opt = OPT.lock().unwrap();
		(opt.tick_rate, opt.checkpoint_interval, opt.debug_window,
			opt.coingecko_key.clone(), opt.coinmarketcap_key.clone(), opt.currency_apiname.clone())
	};

	env_logger::init();
	info!("Started");

	let mut app = match App::new().await {
		Ok(app) => app,
		Err(_e) => return Ok(()),
	};

	let mut web_apis = crate::custom::web_requests::WebPriceAPIs::new(coingecho_api_key, coinmarketcap_api_key, &currency_apiname);

	// Terminal initialization
	enable_raw_mode()?;

	let mut stdout = stdout();
	execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
	let backend = CrosstermBackend::new(stdout);
	let mut terminal = Terminal::new(backend)?;
	let mut rx = initialise_events(opt_tick_rate);
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
			match web_apis.handle_web_requests().await {
				Ok(Some(currency_per_token)) => { app.dash_state.currency_per_token = Some(currency_per_token); },
				Ok(None) => {},
				Err(e) => {
					app.dash_state.vdash_status.message(&format!("{}", e), None);
				},
			};
			let prices = custom::app::WEB_PRICES.lock().unwrap();
			if prices.snt_rate.is_some() {
				app.dash_state.currency_per_token = prices.snt_rate;
			}
		}

		let logfiles_future = app.logfiles_manager.linemux_files.next().fuse();
		let events_future = rx.recv().fuse();

		pin_mut!(logfiles_future, events_future);

		select! {
				e = events_future => {
				match e {
					Some(Event::Input(event)) => {
						if !self::custom::ui_keyboard::handle_keyboard_event(&mut app, &event, opt_debug_window).await {
							disable_raw_mode()?;
							execute!(
								terminal.backend_mut(),
								LeaveAlternateScreen,
								DisableMouseCapture
							)?;
							terminal.show_cursor()?;
							return Ok(());
						}
						terminal.draw(|f| draw_dashboard(f, &mut app)).unwrap();
					}

					Some(Event::Tick) => {
						app.update_timelines(&Utc::now());
						app.scan_glob_paths(true, true).await;
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

						let mut checkpoint_result: Result<String, std::io::Error> = Ok("".to_string());
						match app.get_monitor_for_file_path(&source) {
							Some(monitor) => {
								checkpoint_result = monitor.append_to_content(line.line(), checkpoint_interval);
								if monitor.is_debug_dashboard_log {
									app.dash_state._debug_window(line.line());
								} else if app.dash_state.main_view == DashViewMain::DashSummary {
									app.update_summary_window();
								}
							},
							None => {
								app.dash_state._debug_window(format!("NO MONITOR FOR: {}", source).as_str());
							},
						}
						match checkpoint_result {
							Ok(message) => {
								if message.len() > 0 {
									app.dash_state.vdash_status.message(&message, None);
								}
							},
							Err(e) => {
								app.dash_state.vdash_status.message(&e.to_string(), None);
							}
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
						Err(e) => eprintln!("send error: {}", e),

					}
				}
			}
			if last_tick.elapsed() >= tick_rate {
				match tx.send(Event::Tick) {
					Ok(()) => last_tick = Instant::now(),
					Err(e) => eprintln!("send error: {}", e),

				}
			}

			// TODO remove duplicate code!
			if last_tick.elapsed() >= tick_rate {
				match tx.send(Event::Tick) {
					Ok(()) => last_tick = Instant::now(),
					Err(e) => eprintln!("send error: {}", e),
				}
			}
		}
	});
	rx
}

