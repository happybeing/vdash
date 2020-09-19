///! Terminal based interface and dashboard
///!
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
	match dash_state.main_view {
		DashViewMain::DashSummary => {}
		DashViewMain::DashVault => {}
		DashViewMain::DashDebug => draw_debug_dashboard(f, dash_state, monitors),
	}
}

use super::ui::draw_logfile;

fn draw_debug_dashboard<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	let mut index = 0;
	for (logfile, monitor) in monitors.iter_mut() {
		if monitor.is_debug_dashboard_log {
			draw_logfile(f, f.size(), logfile, monitor);
		}
		index += 1;
	}
}
