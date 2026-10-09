// wstrace - Real-Time Win32 Inspection Probe (Modules, Threads, Memory, Children)

use std::collections::HashSet;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_channel::Sender;
use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, Thread32First, Thread32Next,
    MODULEENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPTHREAD, THREADENTRY32,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

const STILL_ACTIVE: u32 = 259;

use crate::event::{EventCategory, TraceEvent};

pub struct Win32Probe {
    pid: u32,
    target_name: String,
    track_children: bool,
    running: Arc<AtomicBool>,
}

impl Win32Probe {
    pub fn new(pid: u32, target_name: String, track_children: bool) -> Self {
        Self {
            pid,
            target_name,
            track_children,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start_monitor(&self, event_tx: Sender<TraceEvent>) {
        let mut root_pid = self.pid;
        let target_name = self.target_name.clone();
        let track_children = self.track_children;
        let running = Arc::clone(&self.running);
        let start_time = std::time::Instant::now();

        let clean_target = target_name
            .trim_end_matches(".exe")
            .trim_end_matches(".EXE")
            .to_lowercase();

        let is_alias_candidate = clean_target.contains("notepad")
            || clean_target.contains("calc")
            || clean_target.contains("paint")
            || clean_target.contains("terminal")
            || clean_target.contains("wt");

        let mut is_launcher = is_alias_candidate;
        let mut launcher_pids: HashSet<u32> = HashSet::new();
        if is_launcher {
            launcher_pids.insert(root_pid);
        }

        thread::spawn(move || {
            let mut monitored_pids: HashSet<u32> = HashSet::new();
            monitored_pids.insert(root_pid);

            let mut known_modules: HashSet<(u32, String)> = HashSet::new();
            let mut known_threads: HashSet<u32> = HashSet::new();
            let mut event_id = 1000u64;

            let mut h_proc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, root_pid) };

            // Initial attachment event
            let mut start_evt = TraceEvent::new(
                event_id,
                root_pid,
                0,
                EventCategory::Process,
                if is_launcher { "LauncherAttach" } else { "ProcessAttach" },
                if is_launcher { format!("PID: {} [Launcher]", root_pid) } else { format!("PID: {}", root_pid) },
                "STATUS_SUCCESS",
                false,
            );
            start_evt.details = if is_launcher {
                format!("Launcher Stub: {}", target_name)
            } else {
                format!("Target Process: {}", target_name)
            };
            event_id += 1;
            let _ = event_tx.send(start_evt);

            while running.load(Ordering::Relaxed) {
                // 0. Detect target process exit code
                let mut exit_code: u32 = 0;
                let root_dead = unsafe {
                    if !h_proc.is_null() {
                        let mut code = 0u32;
                        if GetExitCodeProcess(h_proc, &mut code) != 0 && code != STILL_ACTIVE {
                            exit_code = code;
                            true
                        } else {
                            false
                        }
                    } else {
                        let test_h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, root_pid);
                        if !test_h.is_null() {
                            let mut code = 0u32;
                            let dead = GetExitCodeProcess(test_h, &mut code) != 0 && code != STILL_ACTIVE;
                            if dead {
                                exit_code = code;
                            }
                            CloseHandle(test_h);
                            dead
                        } else {
                            true
                        }
                    }
                };

                let mut all_dead = root_dead;

                // Handle Windows 11 AppExecutionAlias stub handoff (e.g. notepad.exe launcher handoff)
                if root_dead && is_launcher && start_time.elapsed() < Duration::from_millis(5000) {
                    let mut handoff_result = None;
                    for _ in 0..15 {
                        if !running.load(Ordering::Relaxed) {
                            break;
                        }
                        if let Some(res) = find_handoff_process(&clean_target, root_pid, &monitored_pids) {
                            handoff_result = Some(res);
                            break;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }

                    if let Some((new_pid, app_name)) = handoff_result {
                        // 1. Emit LauncherExit
                        let mut lex_evt = TraceEvent::new(
                            event_id,
                            root_pid,
                            0,
                            EventCategory::Process,
                            "LauncherExit",
                            format!("PID: {} [Launcher]", root_pid),
                            "STATUS_SUCCESS",
                            false,
                        );
                        lex_evt.details = format!(
                            "Launcher stub exited (Exit Code: {}). Transitioning to target app: {}",
                            exit_code, app_name
                        );
                        event_id += 1;
                        let _ = event_tx.send(lex_evt);

                        // 2. Emit ProcessHandOff
                        let mut handoff_evt = TraceEvent::new(
                            event_id,
                            new_pid,
                            0,
                            EventCategory::Process,
                            "ProcessHandOff",
                            format!("PID: {} [Launcher] -> PID: {} [Target App]", root_pid, new_pid),
                            "STATUS_SUCCESS",
                            false,
                        );
                        handoff_evt.details = format!("Handoff to packaged application: {}", app_name);
                        event_id += 1;
                        let _ = event_tx.send(handoff_evt);

                        // 3. Emit ProcessAttach for target app
                        let mut app_att_evt = TraceEvent::new(
                            event_id,
                            new_pid,
                            0,
                            EventCategory::Process,
                            "ProcessAttach",
                            format!("PID: {} [Target App]", new_pid),
                            "STATUS_SUCCESS",
                            false,
                        );
                        app_att_evt.details = format!("Attached to target application: {}", app_name);
                        event_id += 1;
                        let _ = event_tx.send(app_att_evt);

                        if !h_proc.is_null() {
                            unsafe { CloseHandle(h_proc) };
                        }
                        launcher_pids.insert(root_pid);
                        root_pid = new_pid;
                        monitored_pids.insert(new_pid);
                        h_proc = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, new_pid) };
                        is_launcher = false;
                        all_dead = false;
                    }
                }

                if root_dead && track_children {
                    // Check if any tracked child process is still running
                    for &cpid in &monitored_pids {
                        if cpid != root_pid {
                            let ch = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, cpid) };
                            if !ch.is_null() {
                                let mut code = 0u32;
                                let is_child_dead = unsafe { GetExitCodeProcess(ch, &mut code) != 0 && code != STILL_ACTIVE };
                                unsafe { CloseHandle(ch) };
                                if !is_child_dead {
                                    all_dead = false;
                                    break;
                                }
                            }
                        }
                    }
                }

                if all_dead {
                    let is_root_launcher = launcher_pids.contains(&root_pid);
                    let tag = if is_root_launcher { " [Launcher]" } else { " [Target App]" };
                    let mut evt = TraceEvent::new(
                        event_id,
                        root_pid,
                        0,
                        EventCategory::Process,
                        "ProcessExit",
                        format!("PID: {}{}", root_pid, tag),
                        if exit_code == 0 { "STATUS_SUCCESS" } else { "STATUS_UNSUCCESSFUL" },
                        exit_code != 0,
                    );
                    evt.details = format!("Exit Code: {} (0x{:X})", exit_code, exit_code);
                    let _ = event_tx.send(evt);
                    break;
                }

                // 1. Detect child processes if child tracking is enabled
                if track_children {
                    unsafe {
                        use windows_sys::Win32::System::Diagnostics::ToolHelp::{
                            Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
                        };
                        let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
                        if h_snap != INVALID_HANDLE_VALUE {
                            let mut pe: PROCESSENTRY32W = std::mem::zeroed();
                            pe.dwSize = size_of::<PROCESSENTRY32W>() as u32;

                            if Process32FirstW(h_snap, &mut pe) != 0 {
                                loop {
                                    if monitored_pids.contains(&pe.th32ParentProcessID)
                                        && !monitored_pids.contains(&pe.th32ProcessID)
                                    {
                                        let child_pid = pe.th32ProcessID;
                                        monitored_pids.insert(child_pid);

                                        let child_name = String::from_utf16_lossy(
                                            &pe.szExeFile[..pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len())],
                                        );

                                        let mut evt = TraceEvent::new(
                                            event_id,
                                            child_pid,
                                            0,
                                            EventCategory::Process,
                                            "ChildProcessSpawned",
                                            child_name,
                                            "STATUS_SUCCESS",
                                            false,
                                        );
                                        evt.details = format!("Parent PID: {}", pe.th32ParentProcessID);
                                        event_id += 1;
                                        let _ = event_tx.send(evt);
                                    }

                                    if Process32NextW(h_snap, &mut pe) == 0 {
                                        break;
                                    }
                                }
                            }
                            CloseHandle(h_snap);
                        }
                    }
                }

                // 2. Detect loaded modules (DLLs) for each monitored PID
                let pids_to_check: Vec<u32> = monitored_pids.iter().copied().collect();
                for &target_pid in &pids_to_check {
                    let is_curr_launcher = launcher_pids.contains(&target_pid);
                    let tag = if is_curr_launcher { "[Launcher]" } else { "[Target App]" };

                    unsafe {
                        let h_snap = CreateToolhelp32Snapshot(
                            TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
                            target_pid,
                        );
                        if h_snap != INVALID_HANDLE_VALUE {
                            let mut me: MODULEENTRY32W = std::mem::zeroed();
                            me.dwSize = size_of::<MODULEENTRY32W>() as u32;

                            if Module32FirstW(h_snap, &mut me) != 0 {
                                loop {
                                    let mod_name = String::from_utf16_lossy(
                                        &me.szModule[..me.szModule.iter().position(|&c| c == 0).unwrap_or(me.szModule.len())],
                                    );
                                    let mod_path = String::from_utf16_lossy(
                                        &me.szExePath[..me.szExePath.iter().position(|&c| c == 0).unwrap_or(me.szExePath.len())],
                                    );

                                    let key = (target_pid, mod_name.clone());
                                    if !known_modules.contains(&key) {
                                        known_modules.insert(key);

                                        let mut evt = TraceEvent::new(
                                            event_id,
                                            target_pid,
                                            0,
                                            EventCategory::Process,
                                            "LoadLibraryW",
                                            mod_path,
                                            "STATUS_SUCCESS",
                                            false,
                                        );
                                        evt.details = format!("{} Base: 0x{:X}, Size: {} KB", tag, me.modBaseAddr as usize, me.modBaseSize / 1024);
                                        event_id += 1;
                                        let _ = event_tx.send(evt);
                                    }

                                    if Module32NextW(h_snap, &mut me) == 0 {
                                        break;
                                    }
                                }
                            }
                            CloseHandle(h_snap);
                        }
                    }
                }

                // 3. Detect created threads
                unsafe {
                    let h_snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
                    if h_snap != INVALID_HANDLE_VALUE {
                        let mut te: THREADENTRY32 = std::mem::zeroed();
                        te.dwSize = size_of::<THREADENTRY32>() as u32;

                        if Thread32First(h_snap, &mut te) != 0 {
                            loop {
                                if monitored_pids.contains(&te.th32OwnerProcessID) && !known_threads.contains(&te.th32ThreadID) {
                                    known_threads.insert(te.th32ThreadID);

                                    let is_curr_launcher = launcher_pids.contains(&te.th32OwnerProcessID);
                                    let tag = if is_curr_launcher { "[Launcher]" } else { "[Target App]" };

                                    let mut evt = TraceEvent::new(
                                        event_id,
                                        te.th32OwnerProcessID,
                                        te.th32ThreadID,
                                        EventCategory::Process,
                                        "CreateThread",
                                        format!("TID: {} {}", te.th32ThreadID, tag),
                                        "STATUS_SUCCESS",
                                        false,
                                    );
                                    evt.details = format!("{} Priority: {}", tag, te.tpBasePri);
                                    event_id += 1;
                                    let _ = event_tx.send(evt);
                                }

                                if Thread32Next(h_snap, &mut te) == 0 {
                                    break;
                                }
                            }
                        }
                        CloseHandle(h_snap);
                    }
                }

                thread::sleep(Duration::from_millis(150));
            }

            if !h_proc.is_null() {
                unsafe { CloseHandle(h_proc) };
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn find_handoff_process(
    clean_target: &str,
    launcher_pid: u32,
    monitored_pids: &HashSet<u32>,
) -> Option<(u32, String)> {
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
        pe.dwSize = size_of::<PROCESSENTRY32W>() as u32;

        let mut matches = Vec::new();
        if Process32FirstW(h_snap, &mut pe) != 0 {
            loop {
                let pe_id = pe.th32ProcessID;
                if pe_id != launcher_pid && !monitored_pids.contains(&pe_id) {
                    let exe_name = String::from_utf16_lossy(
                        &pe.szExeFile[..pe.szExeFile.iter().position(|&c| c == 0).unwrap_or(pe.szExeFile.len())],
                    );
                    let lower = exe_name.to_lowercase();
                    if lower.contains(clean_target) {
                        matches.push((pe_id, exe_name));
                    }
                }
                if Process32NextW(h_snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(h_snap);

        // Pick highest PID (latest spawned in Windows)
        matches.into_iter().max_by_key(|(pid, _)| *pid)
    }
}

