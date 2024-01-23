///! Terminal based interface and dashboard
///!
use super::app::{DashState, HELP_WINDOW_NAME};
use crate::custom::ui::{ push_subheading, push_text, push_blank, push_multiline_text};
use crate::custom::opt::{get_app_name, get_app_version};

use ratatui::{
	layout::Rect,
	widgets::{Block, Borders, List, ListItem},
	Frame,
};


pub fn draw_help_dash(
	f: &mut Frame,
	dash_state: &mut DashState,
) {
	draw_help_window(f, f.size(), dash_state);
}

pub fn draw_help_window(f: &mut Frame, area: Rect, dash_state: &mut DashState) {
	let mut items = Vec::<ListItem>::new();

	push_blank(&mut items);
	push_text(&mut items, &String::from("    For vdash command usage:"), None);
	push_text(&mut items, &String::from("        vdash --help"), None);

	push_blank(&mut items);
	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    Keyboard Commands"));
	push_multiline_text(&mut items, "
    'n' or 'enter' :   Switch to Node Status where you can cycle through status of each node.\n
    's' or 'enter' :   Switch to Summary of all monitored nodes.\n
    'r'            :   Re-scan any 'glob' paths to add new nodes.\n
    '$'            :   Toggle between nanos and a currency (if rate specified on the command line).

    'h' or '?'     :   Shows this help. Press 'n' or 's' to exit help.");

	push_blank(&mut items);
	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    Node Status: selecting a node"));
	push_blank(&mut items);

	push_text(&mut items, &String::from("    Use right arrow and left arrow to cycle forward and backwards through multiple monitored nodes."), None);
	push_blank(&mut items);

	push_blank(&mut items);
	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    Node Status: timelines"));

	push_multiline_text(&mut items,"
    'o' or '-'     :   Zoom timeline out.
    'i' or '+'     :   Zoom timeline in.

    'm'            :   Cycle through min, mean, max values for non-cumulative timelines (e.g. Storage Cost).

    't':           :   Scroll timelines up if some are hidden due to lack of vertical space.
    'T':           :   Scroll timelines down.

    'l'            :   Toggle between show logfile plus 3 timelines and hide logfile to show more timelines.
	");

	push_blank(&mut items);
	push_subheading(&mut items, &String::from("    To exit Help press 'enter'"));

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
