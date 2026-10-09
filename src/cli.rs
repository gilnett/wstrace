// wstrace - Command Line Interface (clap)

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "wstrace",
    author = "gilnett",
    version = "0.1.0",
    about = "Modern strace & real-time Process Monitor for Windows",
    long_about = "wstrace traces Win32 API calls, file system I/O, registry, network sockets, and loaded modules with a real-time TUI or stdout streaming mode."
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

    /// Copy all captured events to Windows clipboard upon exit
    #[arg(long = "copy", alias = "to-clipboard")]
    pub to_clipboard: bool,
}

#[derive(Subcommand, Debug, PartialEq)]
pub enum Commands {
    /// Launch an executable and trace its execution
    Run {
        /// Program binary path
        command: String,

        /// Additional arguments for the program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Attach to an existing running process
    Attach {
        /// Target Process Identifier (PID)
        #[arg(short, long)]
        pid: Option<u32>,

        /// Target Process Executable Name (e.g. explorer.exe)
        #[arg(short, long)]
        name: Option<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputMode {
    Tui,
    Stream,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_run_command() {
        let args = vec!["wstrace", "run", "cmd.exe", "--", "/c", "dir"];
        let parsed = Cli::try_parse_from(args).expect("CLI parsing must succeed");

        match parsed.command {
            Commands::Run { command, args } => {
                assert_eq!(command, "cmd.exe");
                assert_eq!(args, vec!["/c", "dir"]);
            }
            _ => panic!("Expected Run subcommand"),
        }
        assert_eq!(parsed.mode, OutputMode::Tui);
        assert!(!parsed.track_children);

        // Verify arguments can also be passed directly without '--'
        let args_direct = vec!["wstrace", "run", "cmd.exe", "/c", "dir"];
        let parsed_direct = Cli::try_parse_from(args_direct).expect("Direct args parsing must succeed");
        match parsed_direct.command {
            Commands::Run { command, args } => {
                assert_eq!(command, "cmd.exe");
                assert_eq!(args, vec!["/c", "dir"]);
            }
            _ => panic!("Expected Run subcommand"),
        }
    }

    #[test]
    fn test_cli_parse_attach_pid() {
        let args = vec!["wstrace", "-m", "stream", "-f", "-C", "attach", "-p", "1234"];
        let parsed = Cli::try_parse_from(args).expect("CLI parsing must succeed");

        assert_eq!(parsed.mode, OutputMode::Stream);
        assert!(parsed.only_failures);
        assert!(parsed.track_children);

        match parsed.command {
            Commands::Attach { pid, name } => {
                assert_eq!(pid, Some(1234));
                assert_eq!(name, None);
            }
            _ => panic!("Expected Attach subcommand"),
        }
    }

    #[test]
    fn test_cli_parse_include_exclude_flags() {
        let args = vec![
            "wstrace",
            "--include",
            "AppData",
            "--exclude",
            "System32",
            "attach",
            "-n",
            "test_app.exe",
        ];
        let parsed = Cli::try_parse_from(args).expect("CLI parsing must succeed");

        assert_eq!(parsed.include_pattern, Some("AppData".to_string()));
        assert_eq!(parsed.exclude_pattern, Some("System32".to_string()));

        match parsed.command {
            Commands::Attach { pid, name } => {
                assert_eq!(pid, None);
                assert_eq!(name, Some("test_app.exe".to_string()));
            }
            _ => panic!("Expected Attach subcommand"),
        }
    }
}
