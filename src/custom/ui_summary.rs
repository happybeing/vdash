///! Terminal based interface and dashboard
///!
use std::collections::HashMap;

use super::app::{DashState, LogMonitor, MmmStat, SUMMARY_WINDOW_NAME};

use super::opt::{get_app_name, get_app_version};
use super::ui::{monetary_string, push_blank, push_metric, push_price, push_subheading};
use super::web_requests::{BTC_TICKER, SAFE_TOKEN_TICKER};

use ratatui::{
	layout::{Constraint, Direction, Layout, Rect},
	widgets::{Block, Borders, List, ListItem},
	Frame,
};
struct SummaryStats {
	node_count: u32,
	active_node_count: u32,

	storage_cost: MmmStat,
	records: MmmStat,
	earnings: MmmStat,
	puts: MmmStat,
	gets: MmmStat,
	errors: MmmStat,
	connections: MmmStat,
	ram: MmmStat,
}

impl SummaryStats {
	pub fn new(
		dash_state: &mut DashState,
		monitors: &mut HashMap<String, LogMonitor>,
	) -> SummaryStats {
		let mut summary_stats = SummaryStats {
			node_count: 0,
			active_node_count: 0,

			storage_cost: MmmStat::new(),
			records: MmmStat::new(),
			earnings: MmmStat::new(),
			puts: MmmStat::new(),
			gets: MmmStat::new(),
			errors: MmmStat::new(),
			connections: MmmStat::new(),
			ram: MmmStat::new(),
		};

		summary_stats.calculate_summary_stats(&dash_state, &monitors);
		summary_stats
	}

	fn calculate_summary_stats(
		&mut self,
		_dash_state: &DashState,
		monitors: &HashMap<String, LogMonitor>,
	) {
		for entry in monitors.into_iter() {
			let (_logfile, monitor) = entry;
			if monitor.is_node() {
				self.node_count += 1;
				self.active_node_count += if monitor.metrics.is_node_active() {
					1
				} else {
					0
				};

				self
					.storage_cost
					.add_sample(monitor.metrics.storage_cost.most_recent);
				self.records.add_sample(monitor.metrics.records_stored);
				self.earnings.add_sample(monitor.metrics.attos_earned.total);
				self.puts.add_sample(monitor.metrics.activity_puts.total);
				self.gets.add_sample(monitor.metrics.activity_gets.total);
				self
					.errors
					.add_sample(monitor.metrics.activity_errors.total);
				self
					.connections
					.add_sample(monitor.metrics.peers_connected.most_recent);
				self
					.ram
					.add_sample(u64::from(monitor.metrics.memory_used_mb.most_recent));
			}
		}
	}
}

pub fn draw_summary_dash(
	f: &mut Frame,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let constraints = [
		Constraint::Length(13), // Summary statistics for all nodes
		Constraint::Min(0),     // Header above line of details for each node
	];

	let chunks = Layout::default()
		.direction(Direction::Vertical)
		.constraints(constraints.as_ref())
		.margin(1)
		.split(f.size());

	let summary_list_widget = Block::default().borders(Borders::ALL).title(format!(
		"{}  ({} v{}:  {})",
		String::from(SUMMARY_WINDOW_NAME),
		get_app_name(),
		get_app_version(),
		&dash_state.vdash_status.get_status()
	));

	f.render_widget(summary_list_widget, f.size());

	draw_summary_stats_window(f, chunks[0], dash_state, monitors);
	crate::custom::ui_summary_table::draw_summary_table_window(f, chunks[1], dash_state, monitors);
}

fn draw_summary_stats_window(
	f: &mut Frame,
	area: Rect,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let constraints = [
		Constraint::Length(81), // Summary Statistics Panel (left)
		Constraint::Length(15), // Live Prices Panel (right)
	];

	let chunks = Layout::default()
		.direction(Direction::Horizontal)
		.constraints(constraints.as_ref())
		.split(area);

	draw_summary_stats(f, chunks[0], dash_state, monitors);
	draw_live_prices(f, chunks[1], dash_state, monitors);
}

fn draw_summary_stats(
	f: &mut Frame,
	area: Rect,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let mut items = Vec::<ListItem>::new();

	let ss = SummaryStats::new(dash_state, monitors);

	let active_nodes_text = format!("{}/{}", ss.active_node_count, ss.node_count);
	push_metric(&mut items, &"Active Nodes".to_string(), &active_nodes_text);

	let units_text = if dash_state.ui_uses_currency {
		""
	} else {
		crate::custom::app_timelines::EARNINGS_UNITS_TEXT
	};

	push_subheading(
		&mut items,
		&String::from(
			"                       Total                min          mean           max         ",
		),
	);
	let earnings_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12}",
		monetary_string(dash_state, ss.earnings.total),
		units_text,
		monetary_string(dash_state, ss.earnings.min),
		monetary_string(dash_state, ss.earnings.mean),
		monetary_string(dash_state, ss.earnings.max)
	);
	let records_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12}",
		ss.records.total, "", ss.records.min, ss.records.mean, ss.records.max
	);
	let puts_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12}",
		ss.puts.total, "", ss.puts.min, ss.puts.mean, ss.puts.max
	);
	let gets_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12}",
		ss.gets.total, "", ss.gets.min, ss.gets.mean, ss.gets.max
	);
	let errors_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12}",
		ss.errors.total, "", ss.errors.min, ss.errors.mean, ss.errors.max
	);

	push_metric(&mut items, &"Earnings".to_string(), &earnings_text);
	push_metric(&mut items, &"Records".to_string(), &records_text);
	push_metric(&mut items, &"PUTS".to_string(), &puts_text);
	push_metric(&mut items, &"GETS".to_string(), &gets_text);
	push_metric(&mut items, &"ERRORS".to_string(), &errors_text);

	push_blank(&mut items);
	push_subheading(
		&mut items,
		&String::from(
			"                                            min          mean           max         ",
		),
	);
	let storage_cost_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12} {}",
		"-",
		"",
		monetary_string(dash_state, ss.storage_cost.min),
		monetary_string(dash_state, ss.storage_cost.mean),
		monetary_string(dash_state, ss.storage_cost.max),
		units_text
	);
	let connections_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12}",
		"-", "", ss.connections.min, ss.connections.mean, ss.connections.max
	);
	let ram_text = format!(
		"{:>14} {:<6}{:>12}  {:>12}  {:>12} {}",
		"-", "", ss.ram.min, ss.ram.mean, ss.ram.max, "MB"
	);

	push_metric(&mut items, &"Storage Cost".to_string(), &storage_cost_text);
	push_metric(&mut items, &"Connections".to_string(), &connections_text);
	push_metric(&mut items, &"RAM".to_string(), &ram_text);

	let monitor_widget = List::new(items).block(Block::default());
	f.render_widget(monitor_widget, area);
}

fn draw_live_prices(
	f: &mut Frame,
	area: Rect,
	_dash_state: &mut DashState,
	_monitors: &mut HashMap<String, LogMonitor>,
) {
	let mut items = Vec::<ListItem>::new();

	let mut constraints = [
		Constraint::Length(3), // Live prices height (rows) for just one price
		Constraint::Min(0),    // Rest for remainder
	];

	let prices = super::app::WEB_PRICES.lock().unwrap();
	if let Some(snt_rate) = prices.snt_rate {
		let value_text = format!("{}{:.2}", prices.currency_symbol, snt_rate);
		push_price(&mut items, &SAFE_TOKEN_TICKER.to_string(), &value_text);

		if let Some(btc_rate) = prices.btc_rate {
			let btc_per_snt = snt_rate / btc_rate;
			let btc_snt_text = format!("B{:.6}", btc_per_snt);
			push_price(&mut items, &SAFE_TOKEN_TICKER.to_string(), &btc_snt_text);

			let btc_currency_text = format!("{}{:.0}", prices.currency_symbol, btc_rate);
			push_price(&mut items, &BTC_TICKER.to_string(), &btc_currency_text);

			constraints = [
				Constraint::Length(5), // Live prices height (rows) for three prices
				Constraint::Min(0),    // Rest for remainder
			];
		}

		let mut age_string = String::from("not available");
		if let Some(last_update) = prices.last_update_time {
			age_string = super::timelines::get_duration_text(chrono::Utc::now() - last_update);
		}

		let live_prices_title = format!("Prices ({})", age_string);
		let items_widget = List::new(items).block(
			Block::default()
				.title(live_prices_title)
				.borders(Borders::ALL),
		);

		let chunks = Layout::default()
			.direction(Direction::Vertical)
			.constraints(constraints.as_ref())
			.margin(1)
			.split(area);

		f.render_widget(items_widget, chunks[0]);
	}
}
