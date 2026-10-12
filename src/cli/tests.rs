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
    let args = vec![
        "wstrace", "-m", "stream", "-f", "-C", "attach", "-p", "1234",
    ];
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

#[test]
fn test_cli_parse_all_command() {
    let args = vec!["wstrace", "-m", "stream", "-f", "all"];
    let parsed = Cli::try_parse_from(args).expect("CLI parsing must succeed");

    assert_eq!(parsed.mode, OutputMode::Stream);
    assert!(parsed.only_failures);
    assert_eq!(parsed.command, Commands::All);
    assert_eq!(parsed.buffer_size, 50_000);
    assert_eq!(parsed.output, None);
}

#[test]
fn test_cli_parse_buffer_size_and_output_flags() {
    // Arrange
    let args = vec![
        "wstrace",
        "-m",
        "stream",
        "-o",
        "trace.ndjson",
        "--buffer-size",
        "20000",
        "all",
    ];

    // Act
    let parsed = Cli::try_parse_from(args).expect("CLI parsing must succeed");

    // Assert
    assert_eq!(parsed.mode, OutputMode::Stream);
    assert_eq!(parsed.output, Some("trace.ndjson".to_string()));
    assert_eq!(parsed.buffer_size, 20_000);
}

#[test]
fn test_cli_parse_when_buffer_size_omitted_then_defaults_to_50_000() {
    // Arrange
    let args = vec!["wstrace", "run", "notepad.exe"];

    // Act
    let parsed = Cli::try_parse_from(args).expect("Default parsing must succeed");

    // Assert
    assert_eq!(parsed.buffer_size, 50_000);
    assert_eq!(parsed.output, None);
}

#[test]
fn test_cli_parse_when_output_long_flag_provided_then_sets_output_path() {
    // Arrange
    let args = vec![
        "wstrace",
        "--output",
        "custom_output.ndjson",
        "run",
        "test.exe",
    ];

    // Act
    let parsed = Cli::try_parse_from(args).expect("Parsing with --output must succeed");

    // Assert
    assert_eq!(parsed.output, Some("custom_output.ndjson".to_string()));
}

#[test]
fn test_cli_parse_when_buffer_size_zero_then_parses_zero() {
    // Arrange
    let args = vec!["wstrace", "--buffer-size", "0", "all"];

    // Act
    let parsed = Cli::try_parse_from(args).expect("Zero buffer size parsing must succeed");

    // Assert
    assert_eq!(parsed.buffer_size, 0);
}

#[test]
fn test_cli_parse_when_buffer_size_invalid_string_then_returns_err() {
    // Arrange
    let args = vec!["wstrace", "--buffer-size", "not_a_number", "all"];

    // Act
    let result = Cli::try_parse_from(args);

    // Assert
    assert!(result.is_err());
}

#[test]
fn test_cli_parse_when_buffer_size_missing_value_then_returns_err() {
    // Arrange
    let args = vec!["wstrace", "all", "--buffer-size"];

    // Act
    let result = Cli::try_parse_from(args);

    // Assert
    assert!(result.is_err());
}

#[test]
fn test_cli_parse_when_output_missing_value_then_returns_err() {
    // Arrange
    let args = vec!["wstrace", "-o"];

    // Act
    let result = Cli::try_parse_from(args);

    // Assert
    assert!(result.is_err());
}
