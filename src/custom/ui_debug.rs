///! Terminal based interface and dashboard
///!
///! Edit src/custom/ui.rs to create a customised fork of logtail-dash
use super::app::{DashState, DashViewMain, LogMonitor};
use log;
use std::collections::HashMap;

use tui::{
	backend::Backend,
	layout::{Constraint, Corner, Direction, Layout},
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
	trace!("ui_debug.rs draw_dashboard()");

	match dash_state.main_view {
		DashViewMain::DashSummary => {}
		DashViewMain::DashVault => {}
		DashViewMain::DashDebug => draw_dash_vertical(f, dash_state, monitors),
	}
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
