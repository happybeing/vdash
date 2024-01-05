use crossterm::event::KeyCode;

use crate::custom::app::{App, DashViewMain, set_main_view};

/// Handle a keyboard event and return false to cause exit of app (vdash)
pub async fn handle_keyboard_event(mut app: &mut App, event: &crossterm::event::KeyEvent, opt_debug_window: bool) -> bool {

    match event.code {
        // For debugging, ~ sends a line to the debug_window
        KeyCode::Char('~') => app.dash_state._debug_window(format!("Event::Input({:#?})", event).as_str()),

        KeyCode::Char('q')|
        KeyCode::Char('Q') => {
            return false;
        },
        KeyCode::Enter => {
            if app.dash_state.main_view == DashViewMain::DashHelp {
                set_main_view(app.dash_state.previous_main_view, &mut app);
            } else {
                if app.logfiles_manager.logfiles_added.len() > 0 {
                    if app.dash_state.main_view == DashViewMain::DashNode {
                        app.preserve_node_selection();
                        set_main_view(DashViewMain::DashSummary, &mut app);
                    } else if app.dash_state.main_view == DashViewMain::DashSummary {
                        app.preserve_node_selection();
                        set_main_view(DashViewMain::DashNode, &mut app);
                    }
                }
            }
        }

        KeyCode::Char(' ') => {
            if app.dash_state.main_view == DashViewMain::DashSummary {
                app.dash_state.logfile_names_sorted_ascending = !app.dash_state.logfile_names_sorted_ascending;
                app.update_summary_window();
            }
        }

        KeyCode::Char('s')|
        KeyCode::Char('S') => {
            app.preserve_node_selection();
            set_main_view(DashViewMain::DashSummary, &mut app);
        },

        KeyCode::Char('h')|
        KeyCode::Char('H')|
        KeyCode::Char('?') => set_main_view(DashViewMain::DashHelp, &mut app),
        KeyCode::Char('n')|
        KeyCode::Char('N') => {
            if app.logfiles_manager.logfiles_added.len() > 0 {
                app.preserve_node_selection();
                set_main_view(DashViewMain::DashNode, &mut app);
            }
        },

        KeyCode::Char('+')|
        KeyCode::Char('i')|
        KeyCode::Char('I') => app.scale_timeline_up(),
        KeyCode::Char('-')|
        KeyCode::Char('o')|
        KeyCode::Char('O') => app.scale_timeline_down(),

        KeyCode::Char('l')|
        KeyCode::Char('L') => app.toggle_logfile_area(),

        KeyCode::Char('m')|
        KeyCode::Char('M') => app.bump_mmm_ui_mode(),

        KeyCode::Char('r')|
        KeyCode::Char('R') => app.scan_glob_paths(false, false).await,

        KeyCode::Char('t') => app.top_timeline_next(),
        KeyCode::Char('T') => app.top_timeline_previous(),

        KeyCode::Down => app.handle_arrow_down(),
        KeyCode::Up => app.handle_arrow_up(),
        KeyCode::Right|
        KeyCode::Tab => app.change_focus_next(),
        KeyCode::Left => app.change_focus_previous(),

        KeyCode::Char('g') => {
            if opt_debug_window { set_main_view(DashViewMain::DashDebug, &mut app); }
        },
        _ => {}
    };

    return true;
}
