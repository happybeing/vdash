///! Terminal based interface and dashboard
///!
use std::collections::HashMap;

use super::app::{DashState, LogMonitor, DEBUG_WINDOW_NAME};
use crate::custom::opt::{get_app_name, get_app_version};

use ratatui::{
	layout::Rect,
	style::{Color, Modifier, Style},
	text::Line,
	widgets::{Block, Borders, List, ListItem},
	Frame,
};

use super::ui_node::draw_logfile;

pub fn draw_debug_dash(
	f: &mut Frame,
	_dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	for (logfile, monitor) in monitors.iter_mut() {
		if monitor.is_debug_dashboard_log {
			draw_logfile(f, f.size(), logfile, monitor);
		}
	}
}

pub fn draw_debug_window(f: &mut Frame, area: Rect, dash_state: &mut DashState) {
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
			ListItem::new(vec![Line::from(s.clone())])
				.style(Style::default().fg(Color::Black).bg(Color::White))
		})
		.collect();

	let debug_window_widget = List::new(items)
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title(format!("{} v{} - {}", get_app_name(), get_app_version(), String::from(DEBUG_WINDOW_NAME))),
			)
		.highlight_style(highlight_style);

	f.render_stateful_widget(
		debug_window_widget,
		area,
		&mut dash_state.debug_window_list.state,
	);
}
