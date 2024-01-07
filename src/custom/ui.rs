/// Terminal based interface and dashboard
///
/// Edit src/custom/ui.rs to create a customised fork of logtail-dash

use super::app::{App, DashViewMain, DashState};
use super::ui_summary::draw_summary_dash;
use super::ui_node::draw_node_dash;
use super::ui_help::draw_help_dash;
use super::ui_debug::draw_debug_dash;

/// Provides string representation of a nanos amount, in either nanos or currency depending on dash_state
pub fn monetary_string(dash_state: &DashState, nanos: u64) -> String {
	if dash_state.ui_uses_currency && dash_state.currency_per_token.is_some() {
		let value = (dash_state.currency_per_token.unwrap() * (nanos as f32)) / 1e9 as f32;
		return if value >= 0.01 {
			format!("{:<1}{:.2}", dash_state.currency_symbol, value)
		} else {
			format!("{:<1}{}", dash_state.currency_symbol, value)
		}
	} else {
		return format!("{}", nanos);
	}
}

#[path = "../widgets/mod.rs"]
pub mod widgets;
use self::widgets::sparkline::Sparkline2;

use ratatui::{
	layout::Rect,
	style::{Color, Style},
	text::Line,
	widgets::{Block, ListItem},
	Frame,
};

pub fn draw_dashboard(f: &mut Frame, app: &mut App) {
	match app.dash_state.main_view {
		DashViewMain::DashSummary => draw_summary_dash(f, &mut app.dash_state, &mut app.monitors),
		DashViewMain::DashNode => draw_node_dash(f, &mut app.dash_state, &mut app.monitors),
		DashViewMain::DashHelp => draw_help_dash(f, &mut app.dash_state),
		DashViewMain::DashDebug => draw_debug_dash(f, &mut app.dash_state, &mut app.monitors),
	}
}

pub fn push_subheading(items: &mut Vec<ListItem>, subheading: &String) {
	items.push(
		ListItem::new(vec![Line::from(subheading.clone())])
			.style(Style::default().fg(Color::Yellow)),
	);
}

pub fn push_text(items: &mut Vec<ListItem>, subheading: &String, optional_style: Option<Style>) {
	let style = match optional_style {
		Some(style) => style,
		None => Style::default().fg(Color::Green),
	};

	items.push(
		ListItem::new(vec![Line::from(subheading.clone())])
			.style(style),
	);
}

pub fn push_blank(items: &mut Vec<ListItem>) {
	push_text(items, &String::from(""), None);
}

pub fn push_multiline_text(mut items: &mut Vec<ListItem>, lines: &str) {
	for line in lines.lines().into_iter() {
		push_text(&mut items, &String::from(line), None);
	}
}

pub fn push_metric(items: &mut Vec<ListItem>, metric: &String, value: &String) {
	let s = format!("{:<12}: {:>12}", metric, value);
	items.push(
		ListItem::new(vec![Line::from(s.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
}

pub fn push_metric_with_units(items: &mut Vec<ListItem>, metric: &String, value: &String, units: &String) {
	let s = format!("{:<12}: {:>12} {}", metric, value, units);
	items.push(
		ListItem::new(vec![Line::from(s.clone())])
			.style(Style::default().fg(Color::Blue)),
	);
}

pub fn draw_sparkline(
	f: &mut Frame,
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

