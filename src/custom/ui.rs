/// Terminal based interface and dashboard
///
/// Edit src/custom/ui.rs to create a customised fork of logtail-dash

use chrono::Utc;

use super::app::{App, DashState, DashViewMain, LogMonitor,
	SUMMARY_WINDOW_NAME,
	HELP_WINDOW_NAME,
	DEBUG_WINDOW_NAME
};

use super::ui_summary::draw_dashboard as draw_summary_dash;
use super::ui_help::draw_dashboard as draw_help_dash;
use super::ui_debug::draw_dashboard as debug_draw_dashboard;

use structopt::StructOpt;
use crate::custom::opt::Opt;

#[path = "../widgets/mod.rs"]
pub mod widgets;
use self::widgets::sparkline::Sparkline2;
use self::widgets::gauge::Gauge2;
use std::collections::HashMap;

use ratatui::{
	backend::Backend,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	text::Line,
	widgets::{Block, Borders, List, ListItem},
	Frame,
};

use crate::custom::timelines::{get_duration_text, get_min_buckets_value, get_max_buckets_value};

pub fn draw_dashboard<B: Backend>(f: &mut Frame<B>, app: &mut App) {
	match app.dash_state.main_view {
		DashViewMain::DashSummary => draw_summary_dash(f, &mut app.dash_state, &mut app.monitors),
		DashViewMain::DashHelp => draw_help_dash(f, &mut app.dash_state, &mut app.monitors),
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

	draw_summary_window(f, size, dash_state, monitors);
	draw_help_window(f, size, dash_state);
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

	let mut node_title_text = String::from(super::app::SAFENODE_BINARY_NAME);
	if let Some(node_running_version) = &monitor.metrics.running_version {
		node_title_text = format!("{} {}", &node_title_text, node_running_version);
	};
	push_subheading(&mut items, &node_title_text);

	let mut node_uptime_txt = String::from("Start time unknown");
	if let Some(node_start_time) = monitor.metrics.node_started {
		node_uptime_txt = get_duration_text(Utc::now() - node_start_time);
	}
	push_metric(
		&mut items,
		&"Node Uptime".to_string(),
		&node_uptime_txt,
	);

	push_metric(
		&mut items,
		&"Status".to_string(),
		&monitor.metrics.get_node_status_string(),
	);

	push_subheading(&mut items, &"".to_string());
	let storage_payments_txt = format!("{}{}",
		monitor.metrics.storage_payments.to_string(),
		crate::custom::app_timelines::EARNINGS_UNITS_TEXT,
	);
	push_metric(&mut items,
		&"Earnings".to_string(),
		&storage_payments_txt);

	let chunk_fee_txt = format!("{} ({}-{}){}",
		monitor.metrics.storage_cost.to_string(),
		monitor.metrics.storage_cost_min.to_string(),
		monitor.metrics.storage_cost_max.to_string(),
		crate::custom::app_timelines::STORAGE_COST_UNITS_TEXT,
	);
	push_metric(&mut items,
	&"Storage Cost".to_string(),
	&chunk_fee_txt);

	push_subheading(&mut items, &"".to_string());
	push_metric(
		&mut items,
		&"PUTS".to_string(),
		&monitor.metrics.activity_puts.to_string(),
	);

	push_metric(
		&mut items,
		&"GETS".to_string(),
		&monitor.metrics.activity_gets.to_string(),
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
		ListItem::new(vec![Line::from(subheading.clone())])
			.style(Style::default().fg(Color::Yellow)),
	);
}

fn push_metric(items: &mut Vec<ListItem>, metric: &String, value: &String) {
	let s = format!("{:<12}: {:>12}", metric, value);
	items.push(
		ListItem::new(vec![Line::from(s.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
}

// TODO split into two sub functions, one for gauges, one for text strings
fn draw_node_storage<B: Backend>(f: &mut Frame<B>, area: Rect, _dash_state: &mut DashState, monitor: &mut LogMonitor) {
	let used_string = format_size(monitor.metrics.used_space, 1);
	let max_string = format_size(monitor.metrics.max_capacity, 1);
	// let device_limit_string = match &monitor.chunk_store_fsstats {
	// 	Some(fsstats) => {
	// 		let chunk_store_limit = fsstats.free_space();
	// 		format_size(chunk_store_limit, 1).to_string()
	// 	},
	// 	None => {
	// 		"unknown".to_string()
	// 	}
	// };

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
				Constraint::Length(2),	// Rows for storage gauges
				Constraint::Min(8),		// Rows for other metrics
			]
			.as_ref(),
		)
		.split(area);

	// Storage: two columns for label+value | bar
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

	let mut storage_items = Vec::<ListItem>::new();
	push_storage_subheading(&mut storage_items, &"Storage".to_string());
	let mut gauges_column = columns[1];
	gauges_column.height = 1;

	// One gauge gap for heading, and an extra gauge so the last one drawn doesn't expand to the bottom
	let constraints = vec![Constraint::Length(1); 1 + 2];
	let gauges = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.split(columns[1]);

	push_storage_metric(
		&mut storage_items,
		&"Chunk storage".to_string(),
		&format_size(monitor.metrics.used_space, 1)
	);

	let gauge = Gauge2::default()
		.block(Block::default())
		.gauge_style(Style::default().fg(Color::Yellow))
		.ratio(ratio(monitor.metrics.used_space, monitor.metrics.max_capacity));
	f.render_widget(gauge, gauges[1]);

	// TODO lobby to re-instate in node logfile
	// push_storage_metric(
	// 	&mut storage_items,
	// 	&"Space Avail".to_string(),
	// 	&max_string
	// );

	// push_storage_metric(
	// 	&mut storage_items,
	// 	&"Space Free".to_string(),
	// 	&device_limit_string
	// );

	let storage_text_widget = List::new(storage_items).block(
		Block::default()
			.borders(Borders::NONE)
	);
	f.render_widget(storage_text_widget, columns[0]);

	let mut text_items = Vec::<ListItem>::new();
	// push_storage_subheading(&mut text_items, &"".to_string());
	push_storage_subheading(&mut text_items, &"Network".to_string());

	const UPDATE_INTERVAL: u64 = 5;	// Match value in s from maidsafe/safe_network/sn_logging/metrics.rs

	let current_rx_text = format!("{:9} B/s",
		monitor.metrics.bytes_written / UPDATE_INTERVAL,
	);

	push_storage_metric(
		&mut text_items,
		&"Current Rx".to_string(),
		&current_rx_text
	);

	let current_tx_text = format!("{:9} B/s",
		monitor.metrics.bytes_read / UPDATE_INTERVAL,
	);

	push_storage_metric(
		&mut text_items,
		&"Current Tx".to_string(),
		&current_tx_text
	);

	let total_rx_text = format!("{:<13}: {:.0} / {:.0} MB",
		"Total Rx",
		monitor.metrics.total_mb_read,
		monitor.metrics.total_mb_received,
	);

	text_items.push(
		ListItem::new(vec![Line::from(total_rx_text.clone())])
			.style(Style::default().fg(Color::Blue)),
	);

	let total_tx_text = format!("{:<13}: {:.0} / {:.0} MB",
		"Total Tx",
		monitor.metrics.total_mb_written,
		monitor.metrics.total_mb_transmitted,
	);

	text_items.push(
		ListItem::new(vec![Line::from(total_tx_text.clone())])
			.style(Style::default().fg(Color::Blue)),
	);

	push_storage_subheading(&mut text_items, &"Load".to_string());

	let node_text = format!("{:<13}: CPU {:8.2} (MAX {:2.2}) MEM {:.0}MB",
		"Node",
		monitor.metrics.cpu_usage_percent,
		monitor.metrics.cpu_usage_percent_max,
		monitor.metrics.memory_used_mb,
	);
	text_items.push(
		ListItem::new(vec![Line::from(node_text.clone())])
			.style(Style::default().fg(Color::Blue)),
	);

	let system_text = format!("{:<13}: CPU {:8.2} MEM {:.0} / {:.0} MB {:.1}%",
		"System",
		monitor.metrics.system_cpu,
		monitor.metrics.system_memory_used_mb,
		monitor.metrics.system_memory,
		monitor.metrics.system_memory_usage_percent,
	);
	text_items.push(
		ListItem::new(vec![Line::from(system_text.clone())])
			.style(Style::default().fg(Color::Blue)),
	);

	// Render text
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
		ListItem::new(vec![Line::from(subheading.clone())])
			.style(Style::default().fg(Color::Yellow)),
	);
}

fn push_storage_metric(items: &mut Vec<ListItem>, metric: &String, value: &String) {
	let s = format!("{:<13}:{:>9}", metric, value);
	items.push(
		ListItem::new(vec![Line::from(s.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
}

fn draw_timeline<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	dash_state: &mut DashState,
	monitor: &mut LogMonitor,
) {
	if let Some(active_timescale_name) = dash_state.get_active_timescale_name() {
		let window_widget = Block::default()
			.borders(Borders::ALL)
			.title(format!("Timeline - {}", active_timescale_name).to_string());
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

		const NUM_TIMELINES_VISIBLE: usize = 3;

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

		use crate::custom::timelines::MinMeanMax;

		let mut index = dash_state.top_timeline_index();
		for i in 1 ..= NUM_TIMELINES_VISIBLE {
			let mmm_ui_mode = dash_state.mmm_ui_mode();
			if index >= monitor.metrics.app_timelines.get_num_timelines() {
				index = 0;
			}

			if let Some(timeline) = monitor.metrics.app_timelines.get_timeline_by_index(index) {
				let mmm_text = if timeline.is_mmm {
					match mmm_ui_mode {
						MinMeanMax::Min => {" Min "}
						MinMeanMax::Mean => {" Mean"}
						MinMeanMax::Max => {" Max "}
					}
				} else { "" };

				if let Some(bucket_set) = timeline.get_bucket_set(active_timescale_name) {
					if let Some(buckets) = timeline.get_buckets(active_timescale_name, Some(mmm_ui_mode)) {
						// dash_state._debug_window(format!("bucket[0-2 to max]: {},{},{},{} to {}, for {}", buckets[0], buckets[1], buckets[2], buckets[3], buckets[buckets.len()-1], display_name).as_str());
						let duration_text = bucket_set.get_duration_text();

						let mut max_bucket_value = get_max_buckets_value(buckets);
						let mut min_bucket_value = get_min_buckets_value(buckets);
						let label_stats = if timeline.is_cumulative {
							format!("{}{} in last {}", bucket_set.values_total, timeline.units_text, duration_text)
						} else {
							dash_state._debug_window(format!("min: {} max: {}", min_bucket_value, max_bucket_value).as_str());
							if max_bucket_value == 0 {max_bucket_value = timeline.last_non_zero_value;}
							if min_bucket_value == u64::MAX || min_bucket_value == 0 {min_bucket_value = max_bucket_value;}
							format!("range {}-{}{} in last {}", min_bucket_value,  max_bucket_value, timeline.units_text, duration_text)
						};
						let label_scale = if max_bucket_value > 0 {
							format!( " (vertical scale: 0-{}{})", max_bucket_value, timeline.units_text)
						} else {
							String::from("")
						};
						let timeline_label = format!("{}{}: {}{}", timeline.name, mmm_text, label_stats, label_scale);
						draw_sparkline(f, chunks[i-1], &buckets, &timeline_label, timeline.colour);
					};
				};
			}
			index += 1;
		}
	}
}

fn draw_sparkline<B: Backend>(
	f: &mut Frame<B>,
	area: Rect,
	buckets: &Vec<u64>,
	title: &str,
	fg_colour: ratatui::style::Color,
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
			ListItem::new(vec![Line::from(s.clone())])
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

pub fn draw_summary_window<B: Backend>(f: &mut Frame<B>, area: Rect, dash_state: &mut DashState, monitors: &mut HashMap<String, LogMonitor>) {
	// let highlight_style = match dash_state.debug_window_has_focus {
	// 	true => Style::default()
	// 		.bg(Color::LightGreen)
	// 		.add_modifier(Modifier::BOLD),
	// 	false => Style::default().add_modifier(Modifier::BOLD),
	// };

	// let items: Vec<ListItem> = dash_state
	// 	.summary_window_list
	// 	.items
	// 	.iter()
	// 	.map(|s| {
	// 		ListItem::new(vec![Line::from(s.clone())])
	// 			.style(Style::default().fg(Color::Black).bg(Color::White))
	// 	})
	// 	.collect();

	// let debug_window_widget = List::new(items)
	// 	.block(
	// 		Block::default()
	// 			.borders(Borders::ALL)
	// 			.title(format!("{} v{} - {}", Opt::clap().get_name(), structopt::clap::crate_version!(), String::from(SUMMARY_WINDOW_NAME))),
	// 		)
	// 	.highlight_style(highlight_style);

	// f.render_stateful_widget(
	// 	debug_window_widget,
	// 	area,
	// 	&mut dash_state.summary_window_list.state,
	// );
}

pub fn draw_help_window<B: Backend>(f: &mut Frame<B>, area: Rect, dash_state: &mut DashState) {
	let mut items = Vec::<ListItem>::new();
	let mut help_title_text = String::from("{} v{} - Help");
	push_subheading(&mut items, &help_title_text);

	push_metric(
		&mut items,
		&"This is...".to_string(),
		&String::from("some help!"),
	);

	let help_widget = List::new(items).block(
		Block::default()
			.borders(Borders::ALL)
			.title(format!("{} v{} - {}", Opt::clap().get_name(), structopt::clap::crate_version!(), String::from(HELP_WINDOW_NAME))),
		);
	f.render_stateful_widget(help_widget, area, &mut dash_state.help_status.state);
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
			ListItem::new(vec![Line::from(s.clone())])
				.style(Style::default().fg(Color::Black).bg(Color::White))
		})
		.collect();

	let debug_window_widget = List::new(items)
		.block(
			Block::default()
				.borders(Borders::ALL)
				.title(format!("{} v{} - {}", Opt::clap().get_name(), structopt::clap::crate_version!(), String::from(DEBUG_WINDOW_NAME))),
			)
		.highlight_style(highlight_style);

	f.render_stateful_widget(
		debug_window_widget,
		area,
		&mut dash_state.debug_window_list.state,
	);
}
