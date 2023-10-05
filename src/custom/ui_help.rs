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
	_monitors: &mut HashMap<String, LogMonitor>,
) {
	match dash_state.main_view {
		DashViewMain::DashSummary => {}
		DashViewMain::DashNode => {}
		DashViewMain::DashHelp => draw_help_dashboard(f, dash_state),
		DashViewMain::DashDebug => {}
	}
}

fn draw_help_dashboard<B: Backend>(
	f: &mut Frame<B>,
	dash_state: &mut DashState,
) {
	super::ui::draw_help_window(f, f.size(), dash_state);
}
