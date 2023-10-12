///! Terminal based interface and dashboard
///!
use super::app::{DashState, HELP_WINDOW_NAME};
use crate::custom::ui::{ push_subheading, push_text, push_blank, push_multiline_text};
use crate::custom::opt::{get_app_name, get_app_version};

use ratatui::{
	backend::Backend,
	layout::Rect,
	widgets::{Block, Borders, List, ListItem},
	Frame,
};


pub fn draw_help_dash<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
) {
	draw_help_window(f, f.size(), dash_state);
}

pub fn draw_help_window<B: Backend>(f: &mut Frame<B>, area: Rect, dash_state: &mut DashState) {
	let mut items = Vec::<ListItem>::new();

	push_blank(&mut items);
	push_text(&mut items, &String::from("    For vdash command usage:"), None);
	push_text(&mut items, &String::from("        vdash --help"), None);

	push_blank(&mut items);
	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    Keyboard Commands"));
	push_multiline_text(&mut items, "
    'n' or 'N'     :   Switch to Node Details which displays metrics for one node and lets you cycle through all monitored nodes.\n
    's' or 'S'     :   Switch to Summary Screen which provides a summary of every monitored node.

    'h', 'H' or '?':   Shows this help. Press 'n' or 's' to exit help.");

	push_blank(&mut items);
	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    Node Details: selecting a node"));
	push_blank(&mut items);

	push_text(&mut items, &String::from("    Use right arrow and left arrow to cycle forward and backwards through multiple monitored nodes."), None);
	push_blank(&mut items);

	push_blank(&mut items);
	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    Node Details: timelines"));

	push_multiline_text(&mut items,"
    'o', 'O' or '-':   Zoom timeline out.
    'i', 'I' or '+':   Zoom timeline in.

    'm' or 'M'     :   Cycle through minimum, mean and maximum values for a non-cumulative timeline such as Storage Cost.

    't':           :   Scroll timelines up. Only three of the available timelines are visible at one time.
    'T':           :   Scroll timelines down.
	");

	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    To exit Help press 'n' or 's'"));

	if dash_state.debug_window {
		push_blank(&mut items);
		push_blank(&mut items);
		push_text(&mut items, &String::from("    'g' for debug window"), None);
	}

	let help_title_text =format!("{} v{} - {}", get_app_name(), get_app_version(), String::from(HELP_WINDOW_NAME));
	let help_widget = List::new(items).block(
		Block::default()
			.borders(Borders::ALL)
			.title(help_title_text),
		);
	f.render_stateful_widget(help_widget, area, &mut dash_state.help_status.state);
}
