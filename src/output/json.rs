// wstrace - JSON Export Writer

use std::fs::File;
use std::io::Result;
use std::path::Path;

use crate::event::TraceEvent;

pub fn export_json<P: AsRef<Path>>(path: P, events: &[TraceEvent]) -> Result<()> {
    let file = File::create(path.as_ref())?;
    serde_json::to_writer_pretty(file, events).map_err(std::io::Error::other)?;
    println!(
        "[OK] JSON trace report exported: {}",
        path.as_ref().display()
    );
    Ok(())
}
