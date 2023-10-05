///! Terminal based interface and dashboard
///!
use super::app::{DashState, DashViewMain, LogMonitor};
use std::collections::HashMap;

use ratatui::{
	backend::Backend,
	Frame
};

pub fn draw_dashboard<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	match dash_state.main_view {
		DashViewMain::DashSummary => draw_summary_dash(f, dash_state, monitors),
		DashViewMain::DashNode => {}
		DashViewMain::DashHelp => {}
		DashViewMain::DashDebug => {}
	}
}

use super::ui::draw_summary_window;

fn draw_summary_dash<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
	monitors: &mut HashMap<String, LogMonitor>,
) {
	draw_summary_window(f, f.size(), dash_state, monitors);
}
