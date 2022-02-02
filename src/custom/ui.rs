/// Terminal based interface and dashboard
///
/// Edit src/custom/ui.rs to create a customised fork of logtail-dash

use super::app::{TIMELINES, App, DashState, DashViewMain, LogMonitor, DEBUG_WINDOW_NAME};
use super::ui_debug::draw_dashboard as debug_draw_dashboard;

#[path = "../widgets/mod.rs"]
pub mod widgets;
use self::widgets::sparkline::Sparkline2;
use self::widgets::gauge::Gauge2;
use std::collections::HashMap;

use tui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::{Spans},
	widgets::{Block, Borders, List, ListItem},
	Frame,
};

pub fn draw_dashboard<B: Backend>(f: &mut Frame<B>, app: &mut App) {
	match app.dash_state.main_view {
		DashViewMain::DashSummary => {} //draw_summary_dash(f, dash_state, monitors),
		DashViewMain::DashNode => draw_node_dash(f, &mut app.dash_state, &mut app.monitors),
		DashViewMain::DashDebug => debug_draw_dashboard(f, &mut app.dash_state, &mut app.monitors),
	}
}

fn draw_node_dash<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	// Horizonatal bands:
	let constraints = [
		Constraint::Length(12), // Stats summary and graphs
		Constraint::Length(18), // Timeline
		Constraint::Min(0),     // Bottom panel
	];

	let size = f.size();
	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(size);

	for entry in monitors.into_iter() {
		let (logfile, mut monitor) = entry;
		if monitor.has_focus {
			// Stats and Graphs / Timeline / Logfile
			draw_node(f, chunks[0], dash_state, &mut monitor);
			draw_timeline(f, chunks[1], dash_state, &mut monitor);
			draw_bottom_panel(f, chunks[2], dash_state, &logfile, &mut monitor);
			return;
		}
	}

	draw_debug_window(f, size, dash_state);
}

fn draw_node<B: Backend>(f: &mut Frame<B>, area: Rect, dash_state: &mut DashState, monitor: &mut LogMonitor) {
	// Columns:
	let constraints = [
		Constraint::Length(40), // Stats summary
		Constraint::Min(10),    // Graphs
	];

	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints.as_ref())
		.split(area);

	draw_node_stats(f, chunks[0], monitor);
	draw_node_storage(f, chunks[1], dash_state, monitor);
}

fn draw_node_stats<B: Backend>(f: &mut Frame<B>, area: Rect, monitor: &mut LogMonitor) {
	// TODO maybe add items to monitor.metrics_status and make items from that as in draw_logfile()
	let mut items = Vec::<ListItem>::new();
	push_subheading(&mut items, &"Node".to_string());
	push_metric(
		&mut items,
		&"Role".to_string(),
		&monitor.metrics.agebracket_string(),
	);
	push_metric(
		&mut items,
		&"Age".to_string(),
		&monitor.metrics.node_age.to_string()
	);
	push_metric(
		&mut items,
		&"Name".to_string(),
		&monitor.metrics.node_name,
	);
	push_metric(
		&mut items,
		&"Section".to_string(),
		&monitor.metrics.section_prefix,
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
		&"ERRORS".to_string(),
		&monitor.metrics.activity_errors.to_string(),
	);

	push_subheading(&mut items, &"".to_string());
	// TODO re-instate when available
	// push_subheading(&mut items, &"Network".to_string());
	// push_metric(
	// 	&mut items,
	// 	&"Elders".to_string(),
	// 	&monitor.metrics.elders.to_string(),
	// );
	// push_metric(
	// 	&mut items,
	// 	&"Adults".to_string(),
	// 	&monitor.metrics.elders.to_string(),
	// );

	let heading = format!("Node {:>2} Status", monitor.index + 1);
	let monitor_widget = List::new(items).block(
		Block::default()
			.borders(Borders::ALL)
			.title(heading.to_string()),
	);
	f.render_stateful_widget(monitor_widget, area, &mut monitor.metrics_status.state);
}

fn push_subheading(items: &mut Vec<ListItem>, subheading: &String) {
	items.push(
		ListItem::new(vec![Spans::from(subheading.clone())])
			.style(Style::default().fg(Color::Yellow)),
	);
}

fn push_metric(items: &mut Vec<ListItem>, metric: &String, value: &String) {
	let s = format!("{:<12}: {:>12}", metric, value);
	items.push(
		ListItem::new(vec![Spans::from(s.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
}

// TODO split into two sub functions, one for gauges, one for text strings
fn draw_node_storage<B: Backend>(f: &mut Frame<B>, area: Rect, _dash_state: &mut DashState, monitor: &mut LogMonitor) {
	let used_string = format_size(monitor.metrics.used_space, 1);
	let max_string = format_size(monitor.metrics.max_capacity, 1);
	let device_limit_string = match &monitor.chunk_store_fsstats {
		Some(fsstats) => {
			let chunk_store_limit = fsstats.free_space();
			format_size(chunk_store_limit, 1).to_string()
		},
		None => {
			"unknown".to_string()
		}
	};

	let heading = format!("Node {:>2} Resources - Chunk Store:  {:>9} of {} limit", monitor.index+1, &used_string, &max_string);
	let monitor_widget = List::new(Vec::<ListItem>::new())
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title(heading.clone()),
		)
		.highlight_style(
			Style::default()
				.bg(Color::LightGreen)
				.add_modifier(Modifier::BOLD),
		);
	f.render_stateful_widget(monitor_widget, area, &mut monitor.content.state);

	// Two rows top=gauges / bottom=text
	let rows = Layout::default()
		.direction(Direction::Vertical)
		.margin(1)
		.constraints(
			[
				Constraint::Length(7),
				Constraint::Min(3),
			]
			.as_ref(),
		)
		.split(area);

	// Two columns for label+value | bar
	let columns = Layout::default()
		.direction(Direction::Horizontal)
		.margin(0)
		.constraints(
			[
				Constraint::Length(27),
				Constraint::Min(12),
			]
			.as_ref(),
		)
		.split(rows[0]);

	let mut label_items = Vec::<ListItem>::new();
	push_storage_subheading(&mut label_items, &"Chunks".to_string());
	let mut gauges_column = columns[1];
	gauges_column.height = 1;

	// One gauge gap for heading, and an extra gauge so the last one drawn doesn't expand to the bottom
	let constraints = vec![Constraint::Length(1); 1 + 2];
	let gauges = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(columns[1]);

	push_storage_metric(
		&mut label_items,
		&"Chunk storage".to_string(),
		&format_size(monitor.metrics.used_space, 1)
	);

	let gauge = Gauge2::default()
		.block(Block::default())
		.gauge_style(Style::default().fg(Color::Yellow))
		.ratio(ratio(monitor.metrics.used_space, monitor.metrics.max_capacity));
	f.render_widget(gauge, gauges[1]);

	push_storage_subheading(&mut label_items, &"".to_string());
	push_storage_subheading(&mut label_items, &"Device".to_string());

	push_storage_metric(
		&mut label_items,
		&"Space Avail".to_string(),
		&max_string
	);

	push_storage_metric(
		&mut label_items,
		&"Space Free".to_string(),
		&device_limit_string
	);

	// Render labels
	let labels_widget = List::new(label_items).block(
		Block::default()
			.borders(Borders::NONE)
	);
	f.render_widget(labels_widget, columns[0]);


	let mut text_items = Vec::<ListItem>::new();
	push_storage_subheading(&mut text_items, &"Load".to_string());

	let node_text = format!("{:<13}: CPU {} (MAX {}) Memory {}",
		"Node",
		monitor.metrics.node_cpu,
		monitor.metrics.node_cpu_max,
		monitor.metrics.node_memory
	);
	text_items.push(
		ListItem::new(vec![Spans::from(node_text.clone())])
			.style(Style::default().fg(Color::Blue)),
	);

	let system_text = format!("{:<13}: CPU {} LoadAvg {} {} {}",
		"System",
		monitor.metrics.global_cpu,
		monitor.metrics.load_avg_1,
		monitor.metrics.load_avg_5,
		monitor.metrics.load_avg_15,
	);
	text_items.push(
		ListItem::new(vec![Spans::from(system_text.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
	// Render labels
	let text_widget = List::new(text_items).block(
		Block::default()
			.borders(Borders::NONE)
	);
	f.render_widget(text_widget, rows[1]);
}

// Return string representation in TB, MB, KB or bytes depending on magnitude
fn format_size(bytes: u64, fractional_digits: usize) -> String {
	use::byte_unit::Byte;
	let bytes = Byte::from_bytes(bytes as u128);
	bytes.get_appropriate_unit(false).format(fractional_digits)
}

// Return ratio from two u64
fn ratio(numerator: u64, denomimator: u64) -> f64 {
	let percent = numerator as f64 / denomimator as f64;
	if  percent.is_nan() || percent < 0.0 {
		0.0
	} else if percent > 1.0 {
		1.0
	} else {
		percent
	}
}

fn push_storage_subheading(items: &mut Vec<ListItem>, subheading: &String) {
	items.push(
		ListItem::new(vec![Spans::from(subheading.clone())])
			.style(Style::default().fg(Color::Yellow)),
	);
}

fn push_storage_metric(items: &mut Vec<ListItem>, metric: &String, value: &String) {
	let s = format!("{:<13}:{:>9}", metric, value);
	items.push(
		ListItem::new(vec![Spans::from(s.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
}

fn draw_timeline<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	dash_state: &mut DashState,
	monitor: &mut LogMonitor,
) {
	let active_timeline_name = match TIMELINES.get(dash_state.active_timeline) {
		None => {
			// debug_log!("ERROR getting active timeline name");
			return;
		}
		Some((name, _)) => name,
	};

	let window_widget = Block::default()
		.borders(Borders::ALL)
		.title(format!("Timeline - {}", active_timeline_name).to_string());
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
		.constraints(
			[
				Constraint::Percentage(33),
				Constraint::Percentage(33),
				Constraint::Percentage(33),
			]
			.as_ref(),
		)
		.split(area);

	if let Some(bucket_set) = monitor
		.metrics
		.puts_timeline
		.get_bucket_set(active_timeline_name)
	{
		draw_sparkline(f, chunks[0], &bucket_set.buckets(), &"PUTS", Color::Yellow);
	};

	if let Some(bucket_set) = monitor
		.metrics
		.gets_timeline
		.get_bucket_set(active_timeline_name)
	{
		draw_sparkline(f, chunks[1], &bucket_set.buckets(), &"GETS", Color::Green);
	};

	if let Some(bucket_set) = monitor
		.metrics
		.errors_timeline
		.get_bucket_set(active_timeline_name)
	{
		draw_sparkline(f, chunks[2], &bucket_set.buckets(), &"ERRORS", Color::Red);
	};
}

fn draw_sparkline<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	buckets: &Vec<u64>,
	title: &str,
	fg_colour: tui::style::Color,
	) {

		let sparkline = Sparkline2::default()
		.block(Block::default().title(title))
		.data(buckets_right_justify(
			&buckets,
			area.width,
		))
		.style(Style::default().fg(fg_colour));
	f.render_widget(sparkline, area);
}

// Right justify and truncate (left) a set of buckets to width
fn buckets_right_justify(buckets: &Vec<u64>, width: u16) -> &[u64] {
	let width = width as usize;
	if width < buckets.len() {
		return &buckets[buckets.len() - width..];
	}

	buckets
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

	let node_log_title = format!("Node Log ({})", logfile);

	let logfile_widget = List::new(items)
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title(node_log_title.clone()),
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
