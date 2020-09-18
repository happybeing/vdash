///! Terminal based interface and dashboard
///!
///! Edit src/custom/ui.rs to create a customised fork of logtail-dash
use super::app::{DashState, DashViewMain, LogMonitor, DEBUG_WINDOW_NAME};
use super::ui_debug::draw_dashboard as debug_draw_dashboard;
use std::collections::HashMap;

use log;

use tui::{
	backend::Backend,
	layout::{Constraint, Corner, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Span, Spans, Text},
	widgets::{Block, BorderType, Borders, List, ListItem, Sparkline, Widget},
	Frame, Terminal,
};

pub fn draw_dashboard<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	match dash_state.main_view {
		DashViewMain::DashSummary => {} //draw_summary_dash(f, dash_state, monitors),
		DashViewMain::DashVault => draw_vault_dash(f, dash_state, monitors),
		DashViewMain::DashDebug => {
			if (dash_state.debug_dashboard) {
				debug_draw_dashboard(f, dash_state, monitors);
			}
		}
	}
}

fn draw_vault_dash<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	// Horizonatal bands:
	let constraints = [
		Constraint::Length(12), // Stats summary and graphs
		Constraint::Length(12), // Timeline
		Constraint::Min(0),     // Bottom panel
	];

	let size = f.size();
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(size);

	for mut entry in monitors.into_iter() {
		let (logfile, mut monitor) = entry;
		if monitor.has_focus {
			// Stats and Graphs / Timeline / Logfile
			draw_vault(f, chunks[0], &mut monitor);
			draw_timeline(f, chunks[1], dash_state, &mut monitor);
			draw_bottom_panel(f, chunks[2], dash_state, &logfile, &mut monitor);
			return;
		}
	}

	draw_debug_window(f, size, dash_state);
}

fn draw_bottom_panel<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	dash_state: &mut DashState,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
	if dash_state.debug_window {
		// Vertical split:
		let constraints = [
			Constraint::Percentage(50), // Logfile
			Constraint::Percentage(50), // Debug window
		];

		let chunks = Layout::default()
			.direction(Direction::Horizontal)
			.constraints(constraints.as_ref())
			.split(area);

		draw_logfile(f, chunks[0], &logfile, monitor);
		draw_debug_window(f, chunks[1], dash_state);
	} else {
		draw_logfile(f, area, &logfile, monitor);
	}
}

fn draw_vault<B: Backend>(f: &mut Frame<B>, area: Rect, monitor: &mut LogMonitor) {
	// Columns:
	let constraints = [
		Constraint::Length(40), // Stats summary
		Constraint::Min(10),    // Graphs
	];

	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints.as_ref())
		.split(area);

	draw_vault_stats(f, chunks[0], monitor);
	draw_vault_graphs(f, chunks[1], monitor);
}

fn draw_vault_stats<B: Backend>(f: &mut Frame<B>, area: Rect, monitor: &mut LogMonitor) {
	// TODO maybe add items to monitor.metrics_status and make items from that as in draw_logfile()
	let mut items = Vec::<ListItem>::new();
	let heading = format!("Vault {:>2}", monitor.index + 1);
	push_subheading(&mut items, &heading);
	push_metric(
		&mut items,
		&"Agebracket".to_string(),
		&monitor.metrics.agebracket_string(),
	);

	push_subheading(&mut items, &"".to_string());
	push_metric(
		&mut items,
		&"GETS".to_string(),
		&monitor.metrics.activity_gets.to_string(),
	);

	push_metric(
		&mut items,
		&"PUTS".to_string(),
		&monitor.metrics.activity_puts.to_string(),
	);

	push_metric(
		&mut items,
		&"Other".to_string(),
		&monitor.metrics.activity_other.to_string(),
	);

	push_subheading(&mut items, &"".to_string());
	push_subheading(&mut items, &"Network".to_string());
	push_metric(
		&mut items,
		&"Elders".to_string(),
		&monitor.metrics.elders.to_string(),
	);
	// TODO re-instate when available
	// push_metric(
	// 	&mut items,
	// 	&"Adults".to_string(),
	// 	&monitor.metrics.elders.to_string(),
	// );

	let monitor_widget = List::new(items)
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title("Vault Status".to_string()),
		)
		.highlight_style(
			Style::default()
				.bg(Color::LightGreen)
				.add_modifier(Modifier::BOLD),
		);
	f.render_stateful_widget(monitor_widget, area, &mut monitor.metrics_status.state);
}

fn push_subheading(items: &mut Vec<ListItem>, subheading: &String) {
	items.push(
		ListItem::new(vec![Spans::from(subheading.clone())])
			.style(Style::default().fg(Color::Yellow).bg(Color::Black)),
	);
}

fn push_metric(items: &mut Vec<ListItem>, metric: &String, value: &String) {
	let s = format!("{:<12}: {:>12}", metric, value);
	items.push(
		ListItem::new(vec![Spans::from(s.clone())])
			.style(Style::default().fg(Color::Green).bg(Color::Black)),
	);
}

fn draw_vault_graphs<B: Backend>(f: &mut Frame<B>, area: Rect, monitor: &mut LogMonitor) {
	// TODO draw some graphs!

	let monitor_widget = List::new(Vec::<ListItem>::new())
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title("Vault Metrics (TODO)".to_string()),
		)
		.highlight_style(
			Style::default()
				.bg(Color::LightGreen)
				.add_modifier(Modifier::BOLD),
		);
	f.render_stateful_widget(monitor_widget, area, &mut monitor.content.state);
}

use super::app::{ONE_DAY_NAME, ONE_HOUR_NAME, ONE_MINUTE_NAME, ONE_TWELTH_NAME, ONE_YEAR_NAME};

fn draw_timeline<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	dash_state: &mut DashState,
	monitor: &mut LogMonitor,
) {
	let window_widget = Block::default()
		.borders(Borders::ALL)
		.title(format!("Timeline - {}", &dash_state.active_timeline_name).to_string());
	f.render_widget(window_widget, area);

	// For debugging the bucket state
	//
	// if let Some(b_time) = monitor.metrics.sparkline_bucket_time {
	// 	dash_state._debug_window(format!("sparkline_b_time: {}", b_time).as_str());
	// 	dash_state._debug_window(
	// 		format!(
	// 			"sparkline_b_width: {}",
	// 			monitor.metrics.sparkline_bucket_width
	// 		)
	// 		.as_str(),
	// 	);
	// }

	// let mut i = 0;
	// while i < monitor.metrics.puts_sparkline.len() {
	// 	dash_state._debug_window(
	// 		format!(
	// 			"{:>2}: {:>2} puts, {:>2} gets",
	// 			i, monitor.metrics.puts_sparkline[i], monitor.metrics.gets_sparkline[i]
	// 		)
	// 		.as_str(),
	// 	);
	// 	i += 1;
	// }

	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.margin(1)
		.constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
		.split(area);

	if let Some(bucket_set) = monitor
		.metrics
		.puts_timeline
		.get_bucket_set(&dash_state.active_timeline_name)
	{
		let sparkline = Sparkline::default()
			.block(Block::default().title("PUTS"))
			.data(&bucket_set.buckets())
			.style(Style::default().fg(Color::Yellow));
		f.render_widget(sparkline, chunks[0]);
	};

	if let Some(bucket_set) = monitor
		.metrics
		.gets_timeline
		.get_bucket_set(&dash_state.active_timeline_name)
	{
		let sparkline = Sparkline::default()
			.block(Block::default().title("GETS"))
			.data(&bucket_set.buckets())
			.style(Style::default().fg(Color::Green));
		f.render_widget(sparkline, chunks[1]);
	};
}

pub fn draw_logfile<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
	let highlight_style = match monitor.has_focus {
		true => Style::default()
			.bg(Color::LightGreen)
			.add_modifier(Modifier::BOLD),
		false => Style::default().add_modifier(Modifier::BOLD),
	};

	let items: Vec<ListItem> = monitor
		.content
		.items
		.iter()
		.map(|s| {
			ListItem::new(vec![Spans::from(s.clone())])
				.style(Style::default().fg(Color::Black).bg(Color::White))
		})
		.collect();

	let vault_log_title = format!("Vault Log ({})", logfile);

	let logfile_widget = List::new(items)
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title(vault_log_title.clone()),
		)
		.highlight_style(highlight_style);

	f.render_stateful_widget(logfile_widget, area, &mut monitor.content.state);
}

fn draw_debug_window<B: Backend>(f: &mut Frame<B>, area: Rect, dash_state: &mut DashState) {
	let highlight_style = match dash_state.debug_window_has_focus {
		true => Style::default()
			.bg(Color::LightGreen)
			.add_modifier(Modifier::BOLD),
		false => Style::default().add_modifier(Modifier::BOLD),
	};

	let items: Vec<ListItem> = dash_state
		.debug_window_list
		.items
		.iter()
		.map(|s| {
			ListItem::new(vec![Spans::from(s.clone())])
				.style(Style::default().fg(Color::Black).bg(Color::White))
		})
		.collect();

	let debug_window_widget = List::new(items)
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title(String::from(DEBUG_WINDOW_NAME)),
		)
		.highlight_style(highlight_style);

	f.render_stateful_widget(
		debug_window_widget,
		area,
		&mut dash_state.debug_window_list.state,
	);
}
