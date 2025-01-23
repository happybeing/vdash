use std::collections::HashMap;

use super::app::{DashState, LogMonitor};
use super::ui::{monetary_string, monetary_string_ant};

use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Line, Span},
	widgets::{Block, List, ListItem},
	Frame,
};

use strfmt::{strfmt, strfmt_builder};

#[derive(Copy, Clone)]
pub enum NodeMetric {
	Index,
	StoragePayments,
	StorageCost,
	Records,
	Puts,
	Gets,
	Errors,
	Peers,
	Memory,
	Status,
}

pub const COLUMN_HEADERS: [(NodeMetric, &str, &str); 10] = [
	//  (node_metric,                   key/heading, format_string)
	(NodeMetric::Index, "Node", "{index:>4} "),
	(
		NodeMetric::StoragePayments,
		"Earnings",
		"{storage_payments:>13} ",
	),
	(NodeMetric::StorageCost, "StoreCost", "{storage_cost:>13} "),
	(NodeMetric::Records, "Records", "{records_stored:>11} "),
	(NodeMetric::Puts, "PUTS", "{puts:>11} "),
	(NodeMetric::Gets, "GETS", "{gets:>11} "),
	(NodeMetric::Errors, "Errors", "{errors:>11} "),
	(NodeMetric::Peers, "Peers", "{connections:>7} "),
	(NodeMetric::Memory, "MB RAM", "{memory:>7} "),
	(NodeMetric::Status, "Status", "  {status:<500} "),
];

pub fn sort_nodes_by_column(
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	use std::cmp::Ordering;

	let sort_by = COLUMN_HEADERS[dash_state.summary_window_heading_selected].0;

	// let logfile_with_focus = dash_state.logfile
	dash_state.logfile_names_sorted.sort_by(|a, b| {
		let mut ordering = Ordering::Equal;
		if let Some(a) = monitors.get(a) {
			if let Some(b) = monitors.get(b) {
				ordering = match sort_by {
					NodeMetric::Index => a.index.cmp(&b.index),
					NodeMetric::StoragePayments => a
						.metrics
						.attos_earned
						.total
						.cmp(&b.metrics.attos_earned.total),
					NodeMetric::StorageCost => a
						.metrics
						.storage_cost
						.most_recent
						.cmp(&b.metrics.storage_cost.most_recent),
					NodeMetric::Records => a.metrics.records_stored.cmp(&b.metrics.records_stored),
					NodeMetric::Puts => a
						.metrics
						.activity_puts
						.total
						.cmp(&b.metrics.activity_puts.total),
					NodeMetric::Gets => a
						.metrics
						.activity_gets
						.total
						.cmp(&b.metrics.activity_gets.total),
					NodeMetric::Errors => a
						.metrics
						.activity_errors
						.total
						.cmp(&b.metrics.activity_errors.total),
					NodeMetric::Peers => a
						.metrics
						.peers_connected
						.most_recent
						.cmp(&b.metrics.peers_connected.most_recent),
					NodeMetric::Memory => a
						.metrics
						.memory_used_mb
						.most_recent
						.cmp(&b.metrics.memory_used_mb.most_recent),
					NodeMetric::Status => a
						.metrics
						.node_status_string
						.cmp(&b.metrics.node_status_string),
				}
			}
		};
		if dash_state.logfile_names_sorted_ascending {
			ordering
		} else {
			ordering.reverse()
		}
	});
}

pub fn format_table_row(dash_state: &DashState, monitor: &mut LogMonitor) -> String {
	let mut row_text = String::from("");

	for i in 0..COLUMN_HEADERS.len() {
		let (metric, _heading, format_string) = &COLUMN_HEADERS[i];
		row_text += &match metric {
            NodeMetric::Index =>            { strfmt!(format_string, index => monitor.index + 1).unwrap() },
            NodeMetric::StoragePayments =>  { strfmt!(format_string, storage_payments  => monetary_string_ant(dash_state, monitor.metrics.attos_earned.total)).unwrap() },
            NodeMetric::StorageCost =>      { strfmt!(format_string, storage_cost => monetary_string(dash_state, monitor.metrics.storage_cost.most_recent)).unwrap() },
            NodeMetric::Records =>          { strfmt!(format_string, records_stored => monitor.metrics.records_stored).unwrap() },
            NodeMetric::Puts =>             { strfmt!(format_string, puts => monitor.metrics.activity_puts.total).unwrap() },
            NodeMetric::Gets =>             { strfmt!(format_string, gets => monitor.metrics.activity_gets.total).unwrap() },
            NodeMetric::Errors =>           { strfmt!(format_string, errors => monitor.metrics.activity_errors.total).unwrap() },
            NodeMetric::Peers =>            { strfmt!(format_string, connections => monitor.metrics.peers_connected.most_recent).unwrap() },
            NodeMetric::Memory =>           { strfmt!(format_string, memory => monitor.metrics.memory_used_mb.most_recent).unwrap() },
            NodeMetric::Status =>           { strfmt!(format_string, status => monitor.metrics.node_status_string.clone()).unwrap() },
        };
	}

	row_text
}

pub fn draw_summary_table_window(
	f: &mut Frame,
	area: Rect,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let constraints = [
		Constraint::Length(1), // Heading
		Constraint::Min(0),    // List
	];

	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(area);

	draw_summary_headings(f, chunks[0], dash_state, monitors);
	draw_summary_rows(f, chunks[1], dash_state, monitors);
}

pub fn initialise_summary_headings(dash_state: &mut DashState) {
	for i in 0..COLUMN_HEADERS.len() {
		let (metric, heading, format_string) = &COLUMN_HEADERS[i];
		dash_state.summary_window_headings.items.push(match metric {
			NodeMetric::Index => strfmt!(format_string, index => *heading).unwrap(),
			NodeMetric::StoragePayments => strfmt!(format_string, storage_payments => *heading).unwrap(),
			NodeMetric::StorageCost => strfmt!(format_string, storage_cost => *heading).unwrap(),
			NodeMetric::Records => strfmt!(format_string, records_stored => *heading).unwrap(),
			NodeMetric::Puts => strfmt!(format_string, puts => *heading).unwrap(),
			NodeMetric::Gets => strfmt!(format_string, gets => *heading).unwrap(),
			NodeMetric::Errors => strfmt!(format_string, errors => *heading).unwrap(),
			NodeMetric::Peers => strfmt!(format_string, connections => *heading).unwrap(),
			NodeMetric::Memory => strfmt!(format_string, memory => *heading).unwrap(),
			NodeMetric::Status => strfmt!(format_string, status => *heading).unwrap(),
		});
	}
}

fn draw_summary_headings(
	f: &mut Frame,
	area: Rect,
	dash_state: &mut DashState,
	_monitors: &mut HashMap<String, LogMonitor>,
) {
	let heading_style = Style::default().fg(Color::White).bg(Color::Black);
	let highlight_style = Style::default()
		.bg(Color::LightGreen)
		.add_modifier(Modifier::BOLD);

	let mut index = 0;
	let spans: Vec<Span> = dash_state
		.summary_window_headings
		.items
		.iter()
		.map(|s| {
			Span::styled(
				s.clone(),
				if dash_state.summary_window_heading_selected != index {
					index += 1;
					heading_style
				} else {
					index += 1;
					highlight_style
				},
			)
		})
		.collect();

	let summary_header_widget = List::new(vec![ListItem::new(vec![Line::from(spans)])])
		.block(Block::default())
		.highlight_style(highlight_style);

	f.render_widget(summary_header_widget, area);
}

fn draw_summary_rows(
	f: &mut Frame,
	area: Rect,
	dash_state: &mut DashState,
	_monitors: &mut HashMap<String, LogMonitor>,
) {
	let highlight_style = Style::default()
		.bg(Color::LightGreen)
		.add_modifier(Modifier::BOLD);

	let items: Vec<ListItem> = dash_state
		.summary_window_rows
		.items
		.iter()
		.map(|s| ListItem::new(vec![Line::from(s.clone())]).style(Style::default().fg(Color::White)))
		.collect();

	let summary_window_widget = List::new(items)
		.block(Block::default())
		.highlight_style(highlight_style);

	f.render_stateful_widget(
		summary_window_widget,
		area,
		&mut dash_state.summary_window_rows.state,
	);
}
