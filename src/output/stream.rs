// wstrace - Live Stdout Stream Formatter & Engine

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::Receiver;

use crate::cli::Cli;
use crate::event::TraceEvent;

pub fn run_stream(event_rx: Receiver<TraceEvent>, args: &Cli, target_pid: u32) -> Vec<TraceEvent> {
    println!("--- LIVE STREAM TRACING (strace mode) ---");
    let running = Arc::new(AtomicBool::new(true));

    let timeout = args
        .duration
        .map(|d| std::time::Instant::now() + Duration::from_secs(d));
    let mut active_pid = target_pid;
    let mut collected_events = Vec::new();

    while running.load(Ordering::Relaxed) {
        if let Some(deadline) = timeout {
            if std::time::Instant::now() >= deadline {
                break;
            }
        }

        while let Ok(ev) = event_rx.try_recv() {
            if ev.operation == "ProcessHandOff" {
                active_pid = ev.pid;
            }

            let is_exit = target_pid != 0 && ev.operation == "ProcessExit" && ev.pid == active_pid;

            let mut passes = true;
            if args.only_failures && !ev.is_failure {
                passes = false;
            }

            if passes {
                if let Some(inc) = &args.include_pattern {
                    if !ev.target.to_lowercase().contains(&inc.to_lowercase())
                        && !ev.operation.to_lowercase().contains(&inc.to_lowercase())
                    {
                        passes = false;
                    }
                }
            }

            if passes {
                if let Some(exc) = &args.exclude_pattern {
                    if ev.target.to_lowercase().contains(&exc.to_lowercase())
                        || ev.operation.to_lowercase().contains(&exc.to_lowercase())
                    {
                        passes = false;
                    }
                }
            }

            if passes {
                let res_color = if ev.is_failure {
                    "\x1b[91m"
                } else {
                    "\x1b[92m"
                };

                let write_res = writeln!(
                    std::io::stdout(),
                    "[{}] [{}] {} -> {}{}\x1b[0m ({})",
                    ev.timestamp,
                    ev.category.as_str(),
                    ev.operation,
                    res_color,
                    ev.result,
                    ev.target
                );
                if write_res.is_err() {
                    running.store(false, Ordering::Relaxed);
                    break;
                }
                collected_events.push(ev);
            }

            if is_exit {
                running.store(false, Ordering::Relaxed);
                break;
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    collected_events
}
