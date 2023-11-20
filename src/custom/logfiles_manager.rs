use linemux::MuxedLines;
use std::collections::HashMap;
use glob::glob;

use crate::custom::app::{LogMonitor, DashState};

pub struct LogfilesManager {
    pub logfiles_added: Vec<String>,
    pub globpaths: Vec<String>,

    pub logfiles_monitored: Vec<String>,    // Paths to all logfiles being monitored
    pub logfiles_failed: Vec<String>,       // Paths to any files which failed to begin monitoring

    pub linemux_files: MuxedLines,
}

// TODO maybe support re-scanning globpaths
// TODO maybe add UI for display of lists (paths/globpaths/failed paths)
// TODO maybe add UI for adding paths/globpaths interactively
impl LogfilesManager {
    pub fn new(globpaths: Vec<String>) -> LogfilesManager {
        match MuxedLines::new() {
            Ok(linemux) => return LogfilesManager {
                logfiles_added: Vec::new(),
                globpaths: globpaths,

                logfiles_monitored: Vec::new(),
                logfiles_failed: Vec::new(),

                linemux_files: linemux,
            },

            Err(e) => panic!("Initialisation failed at MuxedLines::new(): {}", e)
        }
    }

    pub async fn monitor_multi_paths(&mut self, filepaths: Vec<String>, monitors: &mut HashMap<String, LogMonitor>, dash_state: &mut DashState, disable_status: bool) {
        if !disable_status { dash_state.vdash_status.message(&format!("Loading {} files...", filepaths.len()), None); }
        for f in &filepaths {
			self.monitor_path(&f.to_string(), monitors, dash_state, disable_status).await;
		}
    }

    pub async fn scan_multi_globpaths(&mut self, globpaths: Vec<String>, monitors: &mut HashMap<String, LogMonitor>, dash_state: &mut DashState, disable_status: bool) {
        if !disable_status { dash_state.vdash_status.message(&format!("Scanning {} globpaths...", globpaths.len()), None); }
        for f in &globpaths {
            self.scan_globpath(f.to_string(), monitors, dash_state, disable_status).await;
        }
    }

    // Attempts to setup a LogMonitor for the logfile at fullpath
    pub async fn monitor_path(&mut self, fullpath: &String, monitors: &mut HashMap<String, LogMonitor>, dash_state: &mut DashState, disable_status: bool) {
        if self.logfiles_added.contains(&fullpath) {
            return;
        }

        if !disable_status { dash_state.vdash_status.message(&format!("file: {}", &fullpath), None); }

		let mut monitor = LogMonitor::new( fullpath.to_string());

        let checkpoint_result = super::logfile_checkpoints::restore_checkpoint(&mut monitor);

        let checkpoint_was_restored = match checkpoint_result {
            Ok(message) => {
                if message.len() > 0 {
                    if !disable_status { dash_state.vdash_status.message(&format!("{}", &message), None); }
                };
                true
            },
            Err(e) => {
                if !disable_status { dash_state.vdash_status.message(&format!("{}", &e.to_string()), None); }
                false   // TODO note: do I need to handle version errors in some way? (due to change in serialised struct)
            }
        };

        let result = if super::app::OPT.lock().unwrap().ignore_existing {
            self.linemux_files.add_file(fullpath).await
        } else {
            if checkpoint_was_restored {
                match monitor.load_logfile_from_time(dash_state, monitor.latest_checkpoint_time) {
                    Ok(_) => Ok(std::path::PathBuf::from(fullpath)),
                    Err(e) => Err(e),
                }
            } else {
                match monitor.load_logfile_from_time(dash_state, None) {
                    Ok(_) => self.linemux_files.add_file(fullpath).await,
                    Err(e) => Err(e),
                }

                // // This method is 25% slower or worse
                // self.linemux_files.add_file_from_start(fullpath).await
            }
        };

        match  result {
            Ok(_) => {
                monitors.insert(fullpath.to_string(), monitor);
                if !self.logfiles_added.contains(&fullpath) { self.logfiles_added.push(fullpath.to_string()); }
                if let Some(index) = self.logfiles_failed.iter().position(|s| s == fullpath.as_str()) {
					self.logfiles_failed.remove(index);
				}
            }
            Err(e) => {
                if !self.logfiles_failed.contains(&fullpath) { self.logfiles_failed.push(fullpath.to_string()); }
                eprintln!("...load failed: {}", e);
                eprintln!( "Note: it is ok for the file not to exist, but the file's parent directory must exist." );
            }
        }
    }

    /// Scans (or re-scans) the globpath and attempts to setup LogMonitors for any files found
    pub async fn scan_globpath(&mut self, globpath: String, monitors: &mut HashMap<String, LogMonitor>, dash_state: &mut DashState, disable_status: bool) {
        if !disable_status { dash_state.vdash_status.message(&format!("globpath: {}", globpath), None); }

        let paths_to_scan = globpath.clone();
        if !self.globpaths.contains(&globpath) { self.globpaths.push(globpath) }

        for entry in glob(paths_to_scan.as_str()).unwrap() {
            match entry {
                Ok(path) => {
                    if let Some(filepath) = path.to_str() {
                        self.monitor_path(&filepath.to_string(), monitors, dash_state, disable_status).await
                    }
                },
                Err(e) => eprintln!("...globpath failed: {}", e),
            }
        }
    }
}