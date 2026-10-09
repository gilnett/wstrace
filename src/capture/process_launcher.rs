#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::{Child, Command};
use anyhow::{Context, Result};


pub struct LaunchedProcess {
    pub pid: u32,
    #[allow(dead_code)]
    pub child: Child,
}

pub fn spawn_target_process(
    program: &str,
    args: &[String],
    suppress_output: bool,
) -> Result<LaunchedProcess> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    // In TUI mode, spawn with CREATE_NEW_CONSOLE:
    // 1. Console apps (cmd.exe, powershell) get their own interactive window and don't exit on stdin EOF
    // 2. Child console output goes to its own window and never corrupts Ratatui's alternate screen buffer
    // 3. GUI apps (notepad.exe) ignore this flag and open normally
    if suppress_output {
        #[cfg(windows)]
        cmd.creation_flags(0x00000010); // CREATE_NEW_CONSOLE
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn program '{}'", program))?;

    let pid = child.id();
    Ok(LaunchedProcess { pid, child })

}

