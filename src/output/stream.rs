// wstrace - Live Stdout Stream Formatter & Engine

use std::collections::VecDeque;
use std::fs::File;
use std::io::{BufWriter, Write};
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

    let mut file_writer: Option<BufWriter<File>> =
        args.output
            .as_ref()
            .and_then(|path| match File::create(path) {
                Ok(f) => {
                    println!("[INFO] Streaming NDJSON events to: '{}'", path);
                    Some(BufWriter::new(f))
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to create output file '{}': {}", path, e);
                    None
                }
            });

    let timeout = args
        .duration
        .map(|d| std::time::Instant::now() + Duration::from_secs(d));
    let mut active_pid = target_pid;
    let max_buffer = args.buffer_size;
    let mut collected_events: VecDeque<TraceEvent> = VecDeque::with_capacity(max_buffer.min(1000));

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

                if let Some(ref mut writer) = file_writer {
                    if let Err(e) = serde_json::to_writer(&mut *writer, &ev) {
                        eprintln!("[ERROR] Failed to write NDJSON event: {}", e);
                    } else {
                        let _ = writer.write_all(b"\n");
                    }
                }

                if collected_events.len() >= max_buffer {
                    collected_events.pop_front();
                }
                collected_events.push_back(ev);
            }

            if is_exit {
                running.store(false, Ordering::Relaxed);
                break;
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    if let Some(mut writer) = file_writer.take() {
        let _ = writer.flush();
        drop(writer);
    }

    collected_events.into_iter().collect()
}

#[cfg(test)]
mod tests;
