// wstrace - Final Trace Summary Reporter

use crate::event::TraceEvent;
use std::collections::HashSet;

pub fn print_summary(target_name: &str, target_pid: u32, collected_events: &[TraceEvent]) {
    if collected_events.is_empty() {
        return;
    }

    let total = collected_events.len();
    let failures = collected_events.iter().filter(|e| e.is_failure).count();
    let unique_targets: HashSet<&str> =
        collected_events.iter().map(|e| e.target.as_str()).collect();

    println!("\n========================================================");
    println!("                FINAL TRACE SUMMARY REPORT              ");
    println!("========================================================");
    println!(
        "  Target Process        : {} (PID: {})",
        target_name, target_pid
    );
    println!("  Captured Events       : {}", total);
    println!("  Failures / Errors     : {}", failures);
    println!("  Unique Target Modules : {}", unique_targets.len());
    println!("========================================================");
}
