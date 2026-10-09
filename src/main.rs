// wstrace - Main Application Entry Point

mod capture;
mod cli;
mod event;
mod tui;

use std::fs::File;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use crossbeam_channel::unbounded;

use capture::{spawn_target_process, EtwConsumer, Win32Probe};
use cli::{Cli, Commands, OutputMode};
use event::TraceEvent;
use tui::app::App;

fn main() -> Result<()> {
    let args = Cli::parse();

    // Resolve target PID and process name
    let (target_pid, target_name) = match &args.command {
        Commands::Run {
            command,
            args: cmd_args,
        } => {
            if args.mode == OutputMode::Stream {
                println!("[INFO] Launching target binary: '{}'", command);
            }
            let suppress_output = args.mode == OutputMode::Tui;
            let launched = spawn_target_process(command, cmd_args, suppress_output)
                .with_context(|| format!("Failed to spawn process '{}'", command))?;
            (launched.pid, command.clone())
        }
        Commands::Attach { pid, name } => {
            if let Some(p) = pid {
                (*p, format!("PID:{}", p))
            } else if let Some(n) = name {
                let found_pid = find_pid_by_name(n)
                    .with_context(|| format!("No active process named '{}' found.", n))?;
                (found_pid, n.clone())
            } else {
                eprintln!("[ERROR] Specify either --pid or --name for the attach command.");
                process::exit(1);
            }
        }
    };

    if args.mode == OutputMode::Stream {
        println!("[INFO] Monitoring '{}' (PID: {})", target_name, target_pid);
    }

    // High-performance unbounded event bus
    let (event_tx, event_rx) = unbounded::<TraceEvent>();

    // Initialize Win32 and ETW monitoring probes (track children by default for 'run')
    let track_children = args.track_children || matches!(args.command, Commands::Run { .. });
    let probe = Win32Probe::new(target_pid, target_name.clone(), track_children);
    probe.start_monitor(event_tx.clone());

    let etw = EtwConsumer::new(target_pid);
    etw.start(event_tx.clone());

    let mut collected_events = Vec::new();

    match args.mode {
        OutputMode::Tui => {
            let app = App::new(
                target_pid,
                target_name.clone(),
                args.only_failures,
                args.include_pattern.clone(),
                args.exclude_pattern.clone(),
            );
            match tui::run_tui(app, event_rx) {
                Ok(events) => collected_events = events,
                Err(e) => eprintln!("[ERROR] TUI error: {:?}", e),
            }
        }
        OutputMode::Stream => {
            println!("--- LIVE STREAM TRACING (strace mode) ---");
            let running = Arc::new(AtomicBool::new(true));
            let r_clone = Arc::clone(&running);

            ctrlc_handler(r_clone);

            let timeout = args
                .duration
                .map(|d| std::time::Instant::now() + Duration::from_secs(d));
            let mut active_pid = target_pid;

            while running.load(Ordering::Relaxed) {
                if let Some(deadline) = timeout {
                    if std::time::Instant::now() >= deadline {
                        break;
                    }
                }

                while let Ok(ev) = event_rx.try_recv() {
                    if args.only_failures && !ev.is_failure {
                        continue;
                    }

                    if let Some(inc) = &args.include_pattern {
                        if !ev.target.to_lowercase().contains(&inc.to_lowercase())
                            && !ev.operation.to_lowercase().contains(&inc.to_lowercase())
                        {
                            continue;
                        }
                    }

                    if let Some(exc) = &args.exclude_pattern {
                        if ev.target.to_lowercase().contains(&exc.to_lowercase())
                            || ev.operation.to_lowercase().contains(&exc.to_lowercase())
                        {
                            continue;
                        }
                    }

                    let res_color = if ev.is_failure {
                        "\x1b[91m"
                    } else {
                        "\x1b[92m"
                    };
                    use std::io::Write;
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

                    if ev.operation == "ProcessHandOff" {
                        active_pid = ev.pid;
                    }

                    let is_exit = ev.operation == "ProcessExit" && ev.pid == active_pid;
                    collected_events.push(ev);
                    if is_exit {
                        running.store(false, Ordering::Relaxed);
                        break;
                    }
                }

                thread::sleep(Duration::from_millis(50));
            }
        }
    }

    // Stop probes
    probe.stop();
    etw.stop();

    // Summary counters display
    if !collected_events.is_empty() {
        let total = collected_events.len();
        let failures = collected_events.iter().filter(|e| e.is_failure).count();
        let unique_targets: std::collections::HashSet<&str> =
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

    // Export JSON if requested
    if let Some(json_path) = args.export_json {
        if let Ok(mut f) = File::create(&json_path) {
            let _ = serde_json::to_writer_pretty(&mut f, &collected_events);
            println!("[OK] JSON trace report exported: {}", json_path);
        }
    }

    // Export to Windows clipboard if requested
    if args.to_clipboard && !collected_events.is_empty() {
        let mut buffer = String::with_capacity(collected_events.len() * 128);
        for ev in &collected_events {
            buffer.push_str(&format!(
                "[{}] [{}] {} -> {} ({}) | Details: {}\r\n",
                ev.timestamp,
                ev.category.as_str(),
                ev.operation,
                ev.result,
                ev.target,
                ev.details
            ));
        }
        if tui::clipboard::set_clipboard_text(&buffer) {
            println!(
                "[OK] All {} events copied to Windows clipboard.",
                collected_events.len()
            );
        }
    }

    Ok(())
}

fn find_pid_by_name(name: &str) -> Option<u32> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if h_snap == INVALID_HANDLE_VALUE {
            return None;
        }

        let mut pe: PROCESSENTRY32W = std::mem::zeroed();
        pe.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(h_snap, &mut pe) != 0 {
            loop {
                let proc_name = String::from_utf16_lossy(
                    &pe.szExeFile[..pe
                        .szExeFile
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(pe.szExeFile.len())],
                );
                if proc_name.to_lowercase() == name.to_lowercase() {
                    CloseHandle(h_snap);
                    return Some(pe.th32ProcessID);
                }

                if Process32NextW(h_snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(h_snap);
    }
    None
}

fn ctrlc_handler(_running: Arc<AtomicBool>) {
    thread::spawn(move || {
        // Signal handler placeholder for stream mode
    });
}
