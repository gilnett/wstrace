// wstrace - CLI Integration Test Suite

use clap::Parser;
use wstrace::cli::{Cli, Commands, OutputMode};
use wstrace::event::{EventCategory, TraceEvent};
use wstrace::output::json::export_json;

#[test]
fn test_cli_integration_run_subcommand() {
    let args = vec![
        "wstrace", "-m", "stream", "-C", "run", "cmd.exe", "--", "/c", "echo", "test",
    ];
    let parsed = Cli::try_parse_from(args).expect("CLI parsing failed");

    assert_eq!(parsed.mode, OutputMode::Stream);
    assert!(parsed.track_children);
    match parsed.command {
        Commands::Run { command, args } => {
            assert_eq!(command, "cmd.exe");
            assert_eq!(args, vec!["/c", "echo", "test"]);
        }
        _ => panic!("Expected Commands::Run"),
    }
}

#[test]
fn test_cli_integration_attach_subcommand() {
    let args = vec!["wstrace", "-f", "attach", "-n", "notepad.exe"];
    let parsed = Cli::try_parse_from(args).expect("CLI parsing failed");

    assert!(parsed.only_failures);
    match parsed.command {
        Commands::Attach { pid, name } => {
            assert_eq!(pid, None);
            assert_eq!(name, Some("notepad.exe".to_string()));
        }
        _ => panic!("Expected Commands::Attach"),
    }
}

#[test]
fn test_cli_integration_all_subcommand() {
    let args = vec!["wstrace", "-m", "stream", "-d", "10", "all"];
    let parsed = Cli::try_parse_from(args).expect("CLI parsing failed");

    assert_eq!(parsed.mode, OutputMode::Stream);
    assert_eq!(parsed.duration, Some(10));
    assert_eq!(parsed.command, Commands::All);
}

#[test]
fn test_output_json_integration() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("wstrace_test_export.json");

    let event = TraceEvent::new(
        1,
        1000,
        1,
        EventCategory::Process,
        "ProcessSpawn",
        "cmd.exe",
        "STATUS_SUCCESS",
        false,
    );

    let res = export_json(&temp_file, &[event]);
    assert!(res.is_ok());
    assert!(temp_file.exists());

    let _ = std::fs::remove_file(temp_file);
}

#[test]
fn test_cli_integration_buffer_size_and_output() {
    // Arrange
    let args = vec![
        "wstrace",
        "--buffer-size",
        "10000",
        "-o",
        "stream.ndjson",
        "run",
        "myapp.exe",
    ];

    // Act
    let parsed = Cli::try_parse_from(args).expect("CLI parsing failed");

    // Assert
    assert_eq!(parsed.buffer_size, 10_000);
    assert_eq!(parsed.output, Some("stream.ndjson".to_string()));
    match parsed.command {
        Commands::Run { command, args } => {
            assert_eq!(command, "myapp.exe");
            assert!(args.is_empty());
        }
        _ => panic!("Expected Commands::Run"),
    }
}

#[test]
fn test_cli_integration_buffer_size_with_attach() {
    // Arrange
    let args = vec![
        "wstrace",
        "--buffer-size",
        "25000",
        "-o",
        "attach.ndjson",
        "attach",
        "-p",
        "9876",
    ];

    // Act
    let parsed = Cli::try_parse_from(args).expect("CLI parsing failed");

    // Assert
    assert_eq!(parsed.buffer_size, 25_000);
    assert_eq!(parsed.output, Some("attach.ndjson".to_string()));
    match parsed.command {
        Commands::Attach { pid, name } => {
            assert_eq!(pid, Some(9876));
            assert_eq!(name, None);
        }
        _ => panic!("Expected Commands::Attach"),
    }
}

#[test]
fn test_cli_integration_buffer_size_with_all() {
    // Arrange
    let args = vec!["wstrace", "-m", "stream", "--buffer-size", "5000", "all"];

    // Act
    let parsed = Cli::try_parse_from(args).expect("CLI parsing failed");

    // Assert
    assert_eq!(parsed.buffer_size, 5_000);
    assert_eq!(parsed.mode, OutputMode::Stream);
    assert_eq!(parsed.command, Commands::All);
}

#[test]
fn test_cli_integration_invalid_buffer_size_fails() {
    // Arrange
    let args = vec!["wstrace", "--buffer-size", "invalid_number", "all"];

    // Act
    let result = Cli::try_parse_from(args);

    // Assert
    assert!(result.is_err());
}
