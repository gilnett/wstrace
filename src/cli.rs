// wstrace - Command Line Interface (clap)

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "wstrace",
    author = "gilnett",
    version,
    about = "High-performance zero-driver Windows system and API trace monitor",
    long_about = "wstrace traces Win32 API calls, file system I/O, registry, network sockets, and loaded modules with a real-time TUI or stdout streaming mode.",
    after_help = "EXAMPLES:\n\
  # Trace an application with interactive TUI (default)\n\
  wstrace run notepad.exe\n\
\n\
  # Trace a command with arguments\n\
  wstrace run cmd.exe -- /c dir\n\
\n\
  # Stream file system events directly to terminal stdout\n\
  wstrace -m stream -c file run myapp.exe\n\
\n\
  # Stream events directly to file in NDJSON format\n\
  wstrace -m stream -o trace.ndjson run myapp.exe\n\
\n\
  # Trace an application and record failures only\n\
  wstrace -f run myapp.exe\n\
\n\
  # Trace all system-wide processes and events\n\
  wstrace all\n\
\n\
  # Attach to an existing process by PID or executable name\n\
  wstrace attach -p 4820\n\
  wstrace attach -n msedge.exe\n\
\n\
TUI KEYBOARD SHORTCUTS:\n\
  [Ctrl+C], [Ctrl+A]         Copy highlighted event / copy all events to clipboard\n\
  [Ctrl+E], [Space]          Pause / resume live tracing\n\
  [Ctrl+X]                   Clear event display buffer\n\
  [Tab]                      Cycle category filter (ALL -> FILE -> REG -> NET -> PROC)\n\
  [f]                        Toggle failures only filter\n\
  [Up], [Down] (or j/k)      Navigate event list\n\
  [q], [Esc]                 Quit wstrace session"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Output mode: 'tui' (interactive interface) or 'stream' (stdout trace stream)
    #[arg(short, long, default_value = "tui")]
    pub mode: OutputMode,

    /// Filter exclusively for failed operations (e.g. access denied, not found)
    #[arg(short = 'f', long = "only-failures")]
    pub only_failures: bool,

    /// Track child processes spawned by target
    #[arg(short = 'C', long = "children")]
    pub track_children: bool,

    /// Include only targets/operations containing this substring
    #[arg(long = "include")]
    pub include_pattern: Option<String>,

    /// Exclude targets/operations containing this substring
    #[arg(long = "exclude")]
    pub exclude_pattern: Option<String>,

    /// Monitored event categories (file, reg, net, proc, mem, or all)
    #[arg(short, long, default_value = "all")]
    pub category: String,

    /// Maximum tracing duration in seconds
    #[arg(short, long)]
    pub duration: Option<u64>,

    /// Export session events to JSON file upon exit
    #[arg(long)]
    pub export_json: Option<String>,

    /// Stream events directly to file in line-delimited NDJSON format
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,

    /// Maximum in-memory event buffer capacity for TUI ring buffer
    #[arg(long = "buffer-size", default_value_t = 50_000)]
    pub buffer_size: usize,

    /// Copy all captured events to Windows clipboard upon exit
    #[arg(long = "copy", alias = "to-clipboard")]
    pub to_clipboard: bool,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Launch an executable and trace its execution
    #[command(
        about = "Launch an executable and trace its execution",
        long_about = "Launches the target executable under wstrace monitoring. Traces file system, registry, network sockets, process lifecycle, and Win32 operations.",
        after_help = "EXAMPLES:\n\
  # Basic launch in interactive TUI\n\
  wstrace run notepad.exe\n\
\n\
  # Pass arguments to the target binary using '--'\n\
  wstrace run cmd.exe -- /c dir\n\
\n\
  # Recursively monitor child processes\n\
  wstrace -C run build_script.exe\n\
\n\
  # Monitor only failures (e.g. ACCESS_DENIED, FILE_NOT_FOUND)\n\
  wstrace -f run test_suite.exe\n\
\n\
  # Stream output to terminal for CI/CD or logging\n\
  wstrace -m stream -c file run updater.exe\n\
\n\
  # Save trace results to JSON\n\
  wstrace --export-json result.json run app.exe"
    )]
    Run {
        /// Program binary path
        command: String,

        /// Additional arguments for the program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Attach to an existing running process
    #[command(
        about = "Attach to an existing running process",
        long_about = "Attaches wstrace monitoring to a running target process identified by Process Identifier (PID) or process executable name.",
        after_help = "EXAMPLES:\n\
  # Attach by PID\n\
  wstrace attach -p 4820\n\
\n\
  # Attach by executable name\n\
  wstrace attach -n msedge.exe\n\
\n\
  # Attach in stream mode for live stdout logging\n\
  wstrace -m stream attach -n python.exe\n\
\n\
  # Attach and record failures only\n\
  wstrace -f attach -p 1234"
    )]
    Attach {
        /// Target Process Identifier (PID)
        #[arg(short, long)]
        pid: Option<u32>,

        /// Target Process Executable Name (e.g. explorer.exe)
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Trace all system-wide processes and events
    #[command(
        about = "Trace all system-wide processes and events",
        long_about = "Monitors the entire Windows system in real time, capturing all process creations, exits, module loads, and activities across all running applications.",
        after_help = "EXAMPLES:\n\
  # Interactive system-wide trace\n\
  wstrace all\n\
\n\
  # Stream all system activity directly to terminal stdout\n\
  wstrace -m stream all\n\
\n\
  # System-wide trace filtering for failures only\n\
  wstrace -f all\n\
\n\
  # Filter system-wide events matching a specific name or path\n\
  wstrace --include \"chrome\" all\n\
\n\
  # Export 30 seconds of system-wide activity to JSON\n\
  wstrace -d 30 --export-json system_trace.json all"
    )]
    All,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputMode {
    Tui,
    Stream,
}

#[cfg(test)]
mod tests;
