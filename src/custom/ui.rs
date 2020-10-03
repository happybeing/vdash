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
		DashViewMain::DashVault => draw_vault_dash(f, &mut app.dash_state, &mut app.monitors),
		DashViewMain::DashDebug => debug_draw_dashboard(f, &mut app.dash_state, &mut app.monitors),
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
			draw_vault(f, chunks[0], dash_state, &mut monitor);
			draw_timeline(f, chunks[1], dash_state, &mut monitor);
			draw_bottom_panel(f, chunks[2], dash_state, &logfile, &mut monitor);
			return;
		}
	}

	draw_debug_window(f, size, dash_state);
}

fn draw_vault<B: Backend>(f: &mut Frame<B>, area: Rect, dash_state: &mut DashState, monitor: &mut LogMonitor) {
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
	draw_vault_storage(f, chunks[1], dash_state, monitor);
}

fn draw_vault_stats<B: Backend>(f: &mut Frame<B>, area: Rect, monitor: &mut LogMonitor) {
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
		&monitor.metrics.activity_puts.to_string(),
	);

	push_metric(
		&mut items,
		&"ERRORS".to_string(),
		&monitor.metrics.activity_errors.to_string(),
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

	let heading = format!("Vault {:>2} Status", monitor.index + 1);
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

fn draw_vault_storage<B: Backend>(f: &mut Frame<B>, area: Rect, _dash_state: &mut DashState, monitor: &mut LogMonitor) {
	let total_string = format_size(monitor.chunk_store.total_used, 1);
	let limit_string = match &monitor.chunk_store_fsstats {
		Some(fsstats) => {
			let chunk_store_limit = fsstats.free_space();
			format_size(chunk_store_limit, 1).to_string()
		},
		None => {
			"unknown".to_string()
		}
	};

	let heading = format!("Vault {:>2} Chunk Store:  {:>9} of {} limit", monitor.index+1, &total_string, &limit_string);
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
	
	if monitor.chunk_store.chunk_store_stats.len() < 1 {
		return;
	}

	let columns = Layout::default()
		.direction(Direction::Horizontal)
		.margin(1)
		.constraints(
			[
				Constraint::Length(27),
				Constraint::Min(12),
			]
			.as_ref(),
		)
		.split(area);

		let mut label_items = Vec::<ListItem>::new();
		push_storage_subheading(&mut label_items, &"Chunks".to_string());
		let mut gauges_column = columns[1];
		gauges_column.height = 1;
		
		// One gauge gap for heading, and an extra gauge so the last one drawn doesn't expand to the bottom
		let constraints = vec![Constraint::Length(1); monitor.chunk_store.chunk_store_stats.len() + 2];
		let gauges = Layout::default()
			.direction(Direction::Vertical)
			.constraints(constraints.as_ref())
			.split(columns[1]);

		// Metrics with label + gauge
		let mut next_gauge: usize = 1;	// Start after the heading
		for stat in monitor.chunk_store.chunk_store_stats.iter() {
			// For labels column
			push_storage_metric(
				&mut label_items,
				&stat.spec.ui_name,
				&format_size(stat.space_used, 1)
			);

			// Gauge2s column
			let gauge = Gauge2::default()
				.block(Block::default())
				.gauge_style(Style::default().fg(Color::Yellow))
				.ratio(ratio(stat.space_used, monitor.chunk_store.total_used));
			f.render_widget(gauge, gauges[next_gauge]);
			next_gauge += 1;
		}

		push_storage_subheading(&mut label_items, &"".to_string());
		push_storage_subheading(&mut label_items, &"Device".to_string());

		push_storage_metric(
			&mut label_items,
			&"Total Chunks".to_string(),
			&total_string
		);
		
		push_storage_metric(
			&mut label_items,
			&"Space Free".to_string(),
			&limit_string
		);
		
		
		// Render labels
		let labels_widget = List::new(label_items).block(
			Block::default()
				.borders(Borders::NONE))
		);
		f.render_widget(labels_widget, columns[0]);

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
