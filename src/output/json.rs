// wstrace - JSON Export Writer

use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;

use crate::event::TraceEvent;

pub fn export_json<P: AsRef<Path>>(path: P, events: &[TraceEvent]) -> Result<()> {
    let file = File::create(path.as_ref()).with_context(|| {
        format!(
            "Failed to create JSON output file '{}'",
            path.as_ref().display()
        )
    })?;
    serde_json::to_writer_pretty(file, events)
        .with_context(|| "Failed to serialize trace events to JSON")?;
    println!(
        "[OK] JSON trace report exported: {}",
        path.as_ref().display()
    );
    Ok(())
}
