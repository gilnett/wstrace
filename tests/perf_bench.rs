// wstrace - Throughput and Latency Performance Benchmarks

use std::time::Instant;
use wstrace::event::{EventCategory, TraceEvent};
use wstrace::tui::app::App;

#[test]
fn test_event_throughput_and_latency() {
    let count = 50_000;
    let t_start = Instant::now();

    // 1. Event creation benchmark
    let mut events = Vec::with_capacity(count);
    for i in 0..count as u64 {
        events.push(TraceEvent::new(
            i,
            1234,
            1,
            if i % 2 == 0 { EventCategory::FileSystem } else { EventCategory::Registry },
            "NtCreateFile",
            "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll",
            if i % 10 == 0 { "STATUS_ACCESS_DENIED" } else { "STATUS_SUCCESS" },
            i % 10 == 0,
        ));
    }
    let duration_creation = t_start.elapsed();
    let throughput_creation = (count as f64) / duration_creation.as_secs_f64();

    println!(
        "\n[PERF] Created {} events: {:.2?} ({:.0} ev/sec)",
        count, duration_creation, throughput_creation
    );

    // 2. Real-time TUI filter benchmark
    let mut app = App::new(1234, "process.exe".to_string(), false, None, None);
    for ev in events {
        app.push_event(ev);
    }

    let t_filter = Instant::now();
    app.toggle_failures_only();
    let filtered = app.filtered_events();
    let duration_filter = t_filter.elapsed();

    println!(
        "[PERF] Filtered {} events (failures only): {:.2?} (Found: {})",
        count, duration_filter, filtered.len()
    );

    assert_eq!(filtered.len(), count / 10);
    // Ensure filtering finishes well under 50ms for 50k events
    assert!(duration_filter.as_millis() < 50, "Filtering duration exceeded 50ms SLA");
}

#[test]
fn test_json_serialization_speed() {
    let count = 10_000;
    let mut events = Vec::with_capacity(count);
    for i in 0..count as u64 {
        events.push(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::Network,
            "connect",
            "192.168.1.1:443",
            "STATUS_SUCCESS",
            false,
        ));
    }

    let t_start = Instant::now();
    let json = serde_json::to_string(&events).expect("JSON serialization failed");
    let duration = t_start.elapsed();

    println!(
        "[PERF] Serialized {} events to JSON: {:.2?} (Payload size: {:.2} MB)",
        count,
        duration,
        (json.len() as f64) / (1024.0 * 1024.0)
    );

    assert!(duration.as_millis() < 250);
}
