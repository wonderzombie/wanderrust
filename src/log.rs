//! Sets up two log layers and log rotation.
//!
//! The most verbose layer is [`trace_file_layer`] which logs at the TRACE level. That goes to a
//! file which is rotated among three (0, 1, 2) outputs and stored in `logs/`. This kind of log is
//! suitable for more detailed forensics.
//!
//! [`stdout_layer`] is INFO, suitable for on-the-spot debugging and testing, possibly for
//! a bug report. The filters may be overridden by `WANDER_LOG` when set in the shell env.
use bevy::log::tracing_subscriber::{self, EnvFilter, Layer};
use bevy::log::{BoxedFmtLayer, BoxedLayer};
use bevy::prelude::*;
use std::{fs, path::Path, sync::Mutex};

const LOG_DIR: &str = "logs";
const KEEP: usize = 3;

/// Shift trace.log.1 -> .2, trace.log.0 -> .1; the oldest gets overwritten.
pub fn rotate_trace_logs(dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    for i in (0..KEEP - 1).rev() {
        let from = dir.join(format!("trace.log.{i}"));
        if from.exists() {
            fs::rename(&from, dir.join(format!("trace.log.{}", i + 1)))?;
        }
    }
    Ok(())
}

/// The bug-report log: everything the global filter lets through, no colors.
pub fn trace_file_layer(_app: &mut App) -> Option<BoxedLayer> {
    let dir = Path::new(LOG_DIR);
    rotate_trace_logs(dir).ok()?;
    let file = fs::File::create(dir.join("trace.log.0")).ok()?;
    Some(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(Mutex::new(file))
            .boxed(),
    )
}

/// The working log: stdout, info and up (overridable via WANDER_LOG).
pub fn stdout_layer(_app: &mut App) -> Option<BoxedFmtLayer> {
    let filter = std::env::var("WANDER_LOG").unwrap_or_else(|_| "info".into());
    Some(Box::new(
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stdout)
            .with_filter(EnvFilter::new(filter)),
    ))
}
