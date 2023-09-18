use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::custom::opt::Opt;
use crate::custom::timelines::{Timeline, Buckets};
use tui::style::Color;

lazy_static::lazy_static! {
	pub static ref TIMESCALES: std::vec::Vec<(&'static str, Duration)> = vec!(
		("1 second columns", Duration::seconds(1)),
		("1 minute columns", Duration::minutes(1)),
		("1 hour columns", Duration::hours(1)),
		("1 day columns", Duration::days(1)),
		("1 week columns", Duration::days(7)),
		("1 year columns", Duration::days(365)),
	);
}

/// keys (used to access timelines)
pub const GETS_TIMELINE_KEY: &str = "gets";
pub const PUTS_TIMELINE_KEY: &str = "puts";
pub const STORAGE_FEE_TIMELINE_KEY: &str = "storage";
pub const EARNINGS_TIMELINE_KEY: &str = "earnings";
pub const ERRORS_TIMELINE_KEY: &str = "errors";

/// Defines the Timelines available for display
pub const APP_TIMELINES: [(&str, &str, bool, Color); 4] = [
//  (key, UI name, is_mmm)
    (GETS_TIMELINE_KEY, "GETS", false, Color::Green),
    (PUTS_TIMELINE_KEY, "PUTS", false, Color::Yellow),
    (STORAGE_FEE_TIMELINE_KEY, "Storage Fee", true, Color::LightBlue),
    // (EARNINGS_TIMELINE_KEY, "Earnings", false, Color::LightCyan),
    (ERRORS_TIMELINE_KEY, "ERRORS", false, Color::Red),
];

/// Holds the Timeline structs for a node, as used by this app
// #[derive(Default)]
pub struct AppTimelines {
    timelines:  HashMap<&'static str, Timeline>,
}

impl AppTimelines {

    pub fn new(opt: &Opt) -> AppTimelines {
        let mut app_timelines = AppTimelines {
            timelines: HashMap::<&'static str, Timeline>::new(),
		};

        for (key, name, is_mmm, colour) in APP_TIMELINES {
            app_timelines.timelines.insert(key, Timeline::new(name.to_string(), is_mmm, colour));
        }

        for (_, timeline) in app_timelines.timelines.iter_mut() {
			for i in 0..TIMESCALES.len() {
				if let Some(spec) = TIMESCALES.get(i) {
					timeline.add_bucket_set(spec.0, spec.1, opt.timeline_steps);
				}
			}
        }

        return app_timelines;
	}

    pub fn update_timelines(&mut self, now: &DateTime<Utc>) {
        for (_, timeline) in self.timelines.iter_mut() {
			timeline.update_current_time(&now);
		}
	}

    pub fn get_timeline_by_key(&mut self, key: &str) -> Option<&mut Timeline> {
        return self.timelines.get_mut(key);
    }

    pub fn get_timeline_by_index(&self, index: usize) -> Option<&Timeline> {
        let (key, _, _, _) = APP_TIMELINES[index];
        return self.timelines.get(key);
    }

    // Gets the set of buckets for the index'th Timeline, selecting with Min, Mean, Max if appropriate
    pub fn get_timeline_buckets(&mut self, index: usize, timescale_name: &str) -> Option<&mut Buckets> {
        let (key, _, _, _) = APP_TIMELINES[index];
        if let Some(timeline) = self.timelines.get_mut(key) {
            return timeline.get_bucket_set(timescale_name);
        }
        return None;
    }

    pub fn get_num_timelines(self: &AppTimelines)   -> usize { return APP_TIMELINES.len(); }
}