use super::*;
use crate::cli::{Commands, OutputMode};
use crate::event::EventCategory;
use crossbeam_channel::unbounded;
use std::path::PathBuf;

struct TempFileGuard(PathBuf);
impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.0.exists() {
            // Under Windows, file handles may take a few milliseconds to be fully released
            // by external services (e.g. indexer or antivirus); retry briefly before giving up.
            for _ in 0..10 {
                if std::fs::remove_file(&self.0).is_ok() || !self.0.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }
    }
}

#[test]
fn test_stream_when_valid_events_then_writes_valid_ndjson_records() {
    // Arrange
    let temp_path = std::env::temp_dir().join(format!(
        "wstrace_test_stream_{}.ndjson",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = TempFileGuard(temp_path.clone());
    let temp_file_str = temp_path.to_string_lossy().to_string();

    let (tx, rx) = unbounded::<TraceEvent>();

    let args = Cli {
        command: Commands::All,
        mode: OutputMode::Stream,
        only_failures: false,
        track_children: false,
        include_pattern: None,
        exclude_pattern: None,
        category: "all".to_string(),
        duration: Some(1),
        export_json: None,
        output: Some(temp_file_str),
        buffer_size: 50_000,
        to_clipboard: false,
    };

    let ev1 = TraceEvent::new(
        1,
        1234,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\test1.txt",
        "STATUS_SUCCESS",
        false,
    );
    let ev2 = TraceEvent::new(
        2,
        1234,
        1,
        EventCategory::Registry,
        "RegOpenKey",
        "HKLM\\Software",
        "STATUS_SUCCESS",
        false,
    );

    tx.send(ev1).unwrap();
    tx.send(ev2).unwrap();

    // Act
    let collected = run_stream(rx, &args, 1234);

    // Assert
    assert_eq!(collected.len(), 2);
    assert!(temp_path.exists());
    let content = std::fs::read_to_string(&temp_path).expect("Failed to read NDJSON file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2);

    // Verify JSON validity and content of each line
    let parsed_1: serde_json::Value =
        serde_json::from_str(lines[0]).expect("Line 1 must be valid JSON");
    assert_eq!(parsed_1["operation"], "NtOpenFile");
    assert_eq!(parsed_1["target"], "C:\\test1.txt");

    let parsed_2: serde_json::Value =
        serde_json::from_str(lines[1]).expect("Line 2 must be valid JSON");
    assert_eq!(parsed_2["operation"], "RegOpenKey");
    assert_eq!(parsed_2["target"], "HKLM\\Software");
}

#[test]
fn test_stream_when_filtered_by_failures_only_then_writes_only_failure_ndjson_records() {
    // Arrange
    let temp_path = std::env::temp_dir().join(format!(
        "wstrace_test_failures_{}.ndjson",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = TempFileGuard(temp_path.clone());
    let temp_file_str = temp_path.to_string_lossy().to_string();

    let (tx, rx) = unbounded::<TraceEvent>();

    let args = Cli {
        command: Commands::All,
        mode: OutputMode::Stream,
        only_failures: true, // Only failures
        track_children: false,
        include_pattern: None,
        exclude_pattern: None,
        category: "all".to_string(),
        duration: Some(1),
        export_json: None,
        output: Some(temp_file_str),
        buffer_size: 50_000,
        to_clipboard: false,
    };

    // Success event (should be skipped)
    tx.send(TraceEvent::new(
        1,
        1234,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\ok.txt",
        "STATUS_SUCCESS",
        false,
    ))
    .unwrap();

    // Failure event (should be written)
    tx.send(TraceEvent::new(
        2,
        1234,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\denied.txt",
        "STATUS_ACCESS_DENIED",
        true,
    ))
    .unwrap();

    // Act
    let collected = run_stream(rx, &args, 1234);

    // Assert
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].target, "C:\\denied.txt");

    let content = std::fs::read_to_string(&temp_path).expect("Failed to read NDJSON file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("STATUS_ACCESS_DENIED"));
}

#[test]
fn test_stream_when_include_exclude_active_then_filters_ndjson_file_output() {
    // Arrange
    let temp_path = std::env::temp_dir().join(format!(
        "wstrace_test_filter_{}.ndjson",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = TempFileGuard(temp_path.clone());
    let temp_file_str = temp_path.to_string_lossy().to_string();

    let (tx, rx) = unbounded::<TraceEvent>();

    let args = Cli {
        command: Commands::All,
        mode: OutputMode::Stream,
        only_failures: false,
        track_children: false,
        include_pattern: Some("target".to_string()),
        exclude_pattern: Some("ignore".to_string()),
        category: "all".to_string(),
        duration: Some(1),
        export_json: None,
        output: Some(temp_file_str),
        buffer_size: 50_000,
        to_clipboard: false,
    };

    // Event 1: matches include, not excluded -> written
    tx.send(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\target\\file.txt",
        "STATUS_SUCCESS",
        false,
    ))
    .unwrap();

    // Event 2: does not match include -> skipped
    tx.send(TraceEvent::new(
        2,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\other\\file.txt",
        "STATUS_SUCCESS",
        false,
    ))
    .unwrap();

    // Event 3: matches include but also matches exclude -> skipped
    tx.send(TraceEvent::new(
        3,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\target\\ignore.txt",
        "STATUS_SUCCESS",
        false,
    ))
    .unwrap();

    // Act
    let collected = run_stream(rx, &args, 100);

    // Assert
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].target, "C:\\target\\file.txt");

    let content = std::fs::read_to_string(&temp_path).expect("Failed to read NDJSON file");
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 1);
}

#[test]
fn test_stream_when_no_events_received_then_produces_empty_ndjson_file() {
    // Arrange
    let temp_path = std::env::temp_dir().join(format!(
        "wstrace_test_empty_{}.ndjson",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _guard = TempFileGuard(temp_path.clone());
    let temp_file_str = temp_path.to_string_lossy().to_string();

    let (_tx, rx) = unbounded::<TraceEvent>();

    let args = Cli {
        command: Commands::All,
        mode: OutputMode::Stream,
        only_failures: false,
        track_children: false,
        include_pattern: None,
        exclude_pattern: None,
        category: "all".to_string(),
        duration: Some(1),
        export_json: None,
        output: Some(temp_file_str),
        buffer_size: 50_000,
        to_clipboard: false,
    };

    // Act
    let collected = run_stream(rx, &args, 0);

    // Assert
    assert_eq!(collected.len(), 0);
    assert!(temp_path.exists());
    let metadata = std::fs::metadata(&temp_path).expect("Metadata must be readable");
    assert_eq!(metadata.len(), 0);
}

#[test]
fn test_stream_when_output_file_unwritable_then_continues_without_panic() {
    // Arrange
    let unwritable_path = "Z:\\nonexistent_invalid_dir_99999\\output.ndjson".to_string();
    let (tx, rx) = unbounded::<TraceEvent>();

    let args = Cli {
        command: Commands::All,
        mode: OutputMode::Stream,
        only_failures: false,
        track_children: false,
        include_pattern: None,
        exclude_pattern: None,
        category: "all".to_string(),
        duration: Some(1),
        export_json: None,
        output: Some(unwritable_path),
        buffer_size: 10,
        to_clipboard: false,
    };

    tx.send(TraceEvent::new(
        1,
        500,
        1,
        EventCategory::Process,
        "ProcessCreate",
        "test.exe",
        "STATUS_SUCCESS",
        false,
    ))
    .unwrap();

    // Act
    let collected = run_stream(rx, &args, 500);

    // Assert (should continue gracefully, retaining in memory without panic)
    assert_eq!(collected.len(), 1);
    assert_eq!(collected[0].operation, "ProcessCreate");
}

#[test]
fn test_stream_bounded_buffer_retention() {
    // Arrange
    let (tx, rx) = unbounded::<TraceEvent>();

    let args = Cli {
        command: Commands::All,
        mode: OutputMode::Stream,
        only_failures: false,
        track_children: false,
        include_pattern: None,
        exclude_pattern: None,
        category: "all".to_string(),
        duration: Some(1),
        export_json: None,
        output: None,
        buffer_size: 2, // bounded to 2
        to_clipboard: false,
    };

    // Act
    for i in 1..=5 {
        tx.send(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::Process,
            "ProcessCreate",
            format!("target_{}", i),
            "STATUS_SUCCESS",
            false,
        ))
        .unwrap();
    }

    let collected = run_stream(rx, &args, 100);

    // Assert
    assert_eq!(collected.len(), 2);
    assert_eq!(collected[0].target, "target_4");
    assert_eq!(collected[1].target, "target_5");
}
