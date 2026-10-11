// wstrace - Main Application Entry Point

mod capture;
mod cli;
mod event;
mod output;
mod tui;

use std::process;

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
        Commands::All => (0, "SYSTEM-WIDE".to_string()),
    };

    if args.mode == OutputMode::Stream {
        if target_pid == 0 {
            println!("[INFO] Monitoring all system processes (System-wide trace)");
        } else {
            println!("[INFO] Monitoring '{}' (PID: {})", target_name, target_pid);
        }
    }

    // High-performance unbounded event bus
    let (event_tx, event_rx) = unbounded::<TraceEvent>();

    // Initialize Win32 and ETW monitoring probes (track children by default for 'run')
    let track_children = args.track_children || matches!(args.command, Commands::Run { .. });
    let probe = Win32Probe::new(target_pid, target_name.clone(), track_children);
    probe.start_monitor(event_tx.clone());

    let etw = EtwConsumer::new(target_pid);
    etw.start(event_tx.clone());

    let collected_events = match args.mode {
        OutputMode::Tui => {
            let app = App::new(
                target_pid,
                target_name.clone(),
                args.only_failures,
                args.include_pattern.clone(),
                args.exclude_pattern.clone(),
            );
            tui::run_tui(app, event_rx).unwrap_or_else(|e| {
                eprintln!("[ERROR] TUI error: {:?}", e);
                Vec::new()
            })
        }
        OutputMode::Stream => output::run_stream(event_rx, &args, target_pid),
    };

    // Stop probes
    probe.stop();
    etw.stop();

    // Summary counters display
    output::print_summary(&target_name, target_pid, &collected_events);

    // Export JSON if requested
    if let Some(json_path) = &args.export_json {
        let _ = output::export_json(json_path, &collected_events);
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
