// wstrace - Dynamic Performance Benchmarks via Divan
// Executed only via `cargo bench` by contributors and CI - never included in release binaries.

use divan::{black_box, Bencher};
use wstrace::event::{EventCategory, TraceEvent};
use wstrace::tui::app::App;

fn main() {
    divan::main();
}

const SIZES: &[usize] = &[1_000, 10_000, 50_000];

/// Benchmark event construction and field allocation throughput across scales.
#[divan::bench(args = SIZES)]
fn bench_event_creation(bencher: Bencher, count: usize) {
    bencher.bench_local(|| {
        let mut events = Vec::with_capacity(count);
        for i in 0..count as u64 {
            events.push(TraceEvent::new(
                i,
                1234,
                1,
                if i % 2 == 0 {
                    EventCategory::FileSystem
                } else {
                    EventCategory::Registry
                },
                "NtCreateFile",
                "\\Device\\HarddiskVolume3\\Windows\\System32\\kernel32.dll",
                if i % 10 == 0 {
                    "STATUS_ACCESS_DENIED"
                } else {
                    "STATUS_SUCCESS"
                },
                i % 10 == 0,
            ));
        }
        black_box(events)
    });
}

/// Benchmark bounded ring buffer ingestion and FIFO eviction throughput under burst loads.
#[divan::bench(args = SIZES)]
fn bench_ring_buffer_ingestion(bencher: Bencher, count: usize) {
    let events: Vec<TraceEvent> = (0..count as u64)
        .map(|i| {
            TraceEvent::new(
                i,
                100,
                1,
                EventCategory::FileSystem,
                "NtReadFile",
                format!("C:\\data_{}.bin", i),
                "STATUS_SUCCESS",
                false,
            )
        })
        .collect();

    bencher.bench_local(|| {
        let mut app = App::new(100, "process.exe".to_string(), false, None, None, 10_000);
        for ev in &events {
            app.push_event(ev.clone());
        }
        black_box(app.events.len())
    });
}

/// Benchmark real-time non-destructive filter evaluation across a pre-filled 50,000-event buffer.
#[divan::bench]
fn bench_tui_filtering(bencher: Bencher) {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    for i in 0..50_000u64 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            if i % 3 == 0 {
                EventCategory::FileSystem
            } else {
                EventCategory::Registry
            },
            "NtOpenFile",
            format!("C:\\Windows\\System32\\driver_{}.sys", i),
            if i % 10 == 0 {
                "STATUS_ACCESS_DENIED"
            } else {
                "STATUS_SUCCESS"
            },
            i % 10 == 0,
        ));
    }

    app.filter_failures_only = true;
    bencher.bench_local(|| {
        let filtered = app.filtered_events();
        black_box(filtered.len())
    });
}

/// Benchmark JSON serialization throughput for forensic session exports.
#[divan::bench(args = &[1_000, 10_000])]
fn bench_json_serialization(bencher: Bencher, count: usize) {
    let events: Vec<TraceEvent> = (0..count as u64)
        .map(|i| {
            TraceEvent::new(
                i,
                100,
                1,
                EventCategory::Network,
                "connect",
                "192.168.1.1:443",
                "STATUS_SUCCESS",
                false,
            )
        })
        .collect();

    bencher.bench_local(|| {
        let json = serde_json::to_string(&events).expect("JSON serialization failed");
        black_box(json.len())
    });
}
