///! Terminal based interface and dashboard
///!
///! Edit src/custom/ui.rs to create a customised fork of logtail-dash
use super::app::{DashState, DashViewMain, LogMonitor};
use super::ui_debug::draw_dashboard as debug_draw_dashboard;
use std::collections::HashMap;

use log;

use tui::{
	backend::Backend,
	layout::{Constraint, Corner, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Span, Spans, Text},
	widgets::{Block, BorderType, Borders, List, ListItem, Widget},
	Frame, Terminal,
};

pub fn draw_dashboard<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	match dash_state.main_view {
		DashViewMain::DashHorizontal => draw_vault_dash(f, dash_state, monitors),
		DashViewMain::DashVertical => draw_vault_dash(f, dash_state, monitors),
		DashViewMain::DashDebug => {
			if (dash_state.debug_ui) {
				debug_draw_dashboard(f, dash_state, monitors);
			} else {
				draw_dash_vertical(f, dash_state, monitors)
			}
		}
	}
}

fn draw_vault_dash<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	// Horizonatal bands:
	let constraints = [
		Constraint::Length(12), // Stats summary and graphs
		Constraint::Length(12), // Timeline
		Constraint::Min(0),     // Logfile tail
	];

	let size = f.size();
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(size);

	let entry = match monitors.into_iter().next() {
		None => return,
		Some(entry) => entry,
	};

	let (logfile, mut monitor) = entry;

	// Stats and Graphs / Timeline / Logfile
	draw_vault(f, chunks[0], &logfile, &mut monitor);
	draw_timeline(f, chunks[1], &logfile, &mut monitor);
	draw_logfile(f, chunks[2], &logfile, &mut monitor);
}

fn draw_vault<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
	// Columns:
	let constraints = [
		Constraint::Length(40), // Stats summary
		Constraint::Min(10),    // Graphs
	];

	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints.as_ref())
		.split(area);

	draw_vault_stats(f, chunks[0], logfile, monitor);
	draw_vault_graphs(f, chunks[1], logfile, monitor);
}

fn draw_vault_stats<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
	// TODO maybe add items to monitor.metrics_status and make items from that as in draw_logfile()
	let mut items = Vec::<ListItem>::new();
	push_subheading(&mut items, &"Vault".to_string());
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
		&monitor.metrics.activity_muts.to_string(),
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

fn draw_vault_graphs<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
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

fn draw_timeline<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
	// TODO draw the timeline!

	let monitor_widget = List::new(Vec::<ListItem>::new())
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title("Timeline (TODO)".to_string()),
		)
		.highlight_style(
			Style::default()
				.bg(Color::LightGreen)
				.add_modifier(Modifier::BOLD),
		);
	f.render_stateful_widget(monitor_widget, area, &mut monitor.content.state);
}

fn draw_logfile<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	logfile: &String,
	monitor: &mut LogMonitor,
) {
	if monitor.content.items.len() > 0 {
		monitor
			.content
			.state
			.select(Some(monitor.content.items.len() - 1));
	}
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
		.highlight_style(
			Style::default()
				.bg(Color::LightGreen)
				.add_modifier(Modifier::BOLD),
		);
	f.render_stateful_widget(logfile_widget, area, &mut monitor.content.state);
}

fn draw_dash_horizontal<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let constraints = make_percentage_constraints(monitors.len());

	let size = f.size();
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(size);

	for (logfile, monitor) in monitors.iter_mut() {
		let len = monitor.content.items.len();
		if len > 0 {
			monitor
				.content
				.state
				.select(Some(monitor.content.items.len() - 1));
		}

		let items: Vec<ListItem> = monitor
			.content
			.items
			.iter()
			.map(|s| {
				ListItem::new(vec![Spans::from(s.clone())])
					.style(Style::default().fg(Color::Black).bg(Color::White))
			})
			.collect();

		let monitor_widget = List::new(items)
			.block(
				Block::default()
					.borders(Borders::ALL)
					.title(logfile.clone()),
			)
			.highlight_style(
				Style::default()
					.bg(Color::LightGreen)
					.add_modifier(Modifier::BOLD),
			);
		f.render_stateful_widget(
			monitor_widget,
			chunks[monitor.index],
			&mut monitor.content.state,
		);
	}
}

fn draw_dash_vertical<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let constraints = make_percentage_constraints(monitors.len());
	let size = f.size();
	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints.as_ref())
		.split(size);

	for (logfile, monitor) in monitors.iter_mut() {
		monitor
			.content
			.state
			.select(Some(monitor.content.items.len() - 1));
		let items: Vec<ListItem> = monitor
			.content
			.items
			.iter()
			.map(|s| {
				ListItem::new(vec![Spans::from(s.clone())])
					.style(Style::default().fg(Color::Black).bg(Color::White))
			})
			.collect();

		let monitor_widget = List::new(items)
			.block(
				Block::default()
					.borders(Borders::ALL)
					.title(logfile.clone()),
			)
			.highlight_style(
				Style::default()
					.bg(Color::LightGreen)
					.add_modifier(Modifier::BOLD),
			);
		f.render_stateful_widget(
			monitor_widget,
			chunks[monitor.index],
			&mut monitor.content.state,
		);
	}
}

fn make_percentage_constraints(count: usize) -> Vec<Constraint> {
	let percent = if count > 0 { 100 / count as u16 } else { 0 };
	let mut constraints = Vec::new();
	let mut total_percent = 0;

	for i in 1..count + 1 {
		total_percent += percent;

		let next_percent = if i == count && total_percent < 100 {
			100 - total_percent
		} else {
			percent
		};

		constraints.push(Constraint::Percentage(next_percent));
	}
	constraints
}
