
use std::fs::{self};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use serde_json;
use chrono::{DateTime, Utc};

use super::app::{LogMonitor, NodeMetrics};

const CHECKPOINT_EXT: &str = "vdash";
const CHECKPOINT_TMP_EXT: &str = "vdash-tmp";

pub fn save_checkpoint(monitor: &mut LogMonitor) -> Result<String, Error> {
    let mut checkpoint_tmp_path = PathBuf::from(&monitor.logfile);
    if !checkpoint_tmp_path.set_extension(CHECKPOINT_TMP_EXT) {
        return Err(Error::new(ErrorKind::Other, "checkpoint set_extension() failed"));
    }

    let last_entry_time = if let Some(metadata) = &monitor.metrics.entry_metadata {
        Some(metadata.message_time)
    } else {
        None
    };

    let mut checkpoint = LogfileCheckpoint::new();
    monitor.to_checkpoint(&mut checkpoint);

    let checkpoint_string = serde_json::to_string(&checkpoint).unwrap();
    match fs::write(checkpoint_tmp_path.clone(), checkpoint_string) {
        Ok(_) => {
            let mut checkpoint_path = PathBuf::from(&monitor.logfile);
            if checkpoint_path.set_extension(CHECKPOINT_EXT) && fs::rename(checkpoint_tmp_path, checkpoint_path.clone()).is_ok() {
                    monitor.latest_checkpoint_time = last_entry_time;
                return Ok("Checkpoint updated".to_string());
            } else {
                return Err(Error::new(ErrorKind::Other, format!("FAILED to rename checkpoint to '{:?}'", checkpoint_path.as_os_str()).as_str()));
            }
        },
        Err(e) => return Err(e),
    };
}

/// Look for and attempt to update metrics from a checkpoint
/// Returns Ok() if the checkpoint was found and restored
pub fn restore_checkpoint(monitor: &mut LogMonitor) -> Result<String, Error> {
    let mut checkpoint_path = PathBuf::from(&monitor.logfile);
    if !checkpoint_path.set_extension(CHECKPOINT_EXT) {
        return Err(Error::new(ErrorKind::Other, "checkpoint set_extension() failed"));
    }

    let mut checkpoint = LogfileCheckpoint::new();
    monitor.to_checkpoint(&mut checkpoint);

    match fs::read_to_string(&checkpoint_path) {
        Ok(checkpoint_string) => {
            match serde_json::from_str(checkpoint_string.as_str()) {
                Ok(checkpoint) => monitor.from_checkpoint(&checkpoint),

                // TODO could be versioning issue (e.g. any change in serialized structs)
                // TODO maybe report so user can delete invalid checkpoint file
                Err(e) => return Err(Error::new(ErrorKind::Other, e.to_string())),
            };
        },
        Err(e) => return Err(e),   // No checkpoint file found
    }

    Ok(format!("checkpoint restored from: {:?}", checkpoint_path.as_os_str()))
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogfileCheckpoint {
    pub latest_entry_time: Option<DateTime<Utc>>,
    pub monitor_index: usize,
    pub monitor_metrics: NodeMetrics,
}

impl LogfileCheckpoint {
    pub fn new() -> LogfileCheckpoint {
        LogfileCheckpoint {
            latest_entry_time: None,
            monitor_index: 0,
            monitor_metrics: NodeMetrics::new(),
        }
    }
}
