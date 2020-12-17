///! Terminal based interface and dashboard
///!
use super::app::{DashState, DashViewMain, LogMonitor};
use std::collections::HashMap;

use tui::{
	backend::Backend,
	Frame
};

pub fn draw_dashboard<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	match dash_state.main_view {
		DashViewMain::DashSummary => {}
		DashViewMain::DashNode => {}
		DashViewMain::DashDebug => draw_debug_dashboard(f, dash_state, monitors),
	}
}

use super::ui::draw_logfile;

fn draw_debug_dashboard<B: Backend>(
	f: &mut Frame<B>,
	_dash_state: &DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	for (logfile, monitor) in monitors.iter_mut() {
		if monitor.is_debug_dashboard_log {
			draw_logfile(f, f.size(), logfile, monitor);
		}
	}
}
