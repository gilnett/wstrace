use super::*;

#[test]
fn test_failure_filtering_invariant() {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::FileSystem,
        "NtCreateFile",
        "\\path\\success",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        2,
        100,
        1,
        EventCategory::FileSystem,
        "NtCreateFile",
        "\\path\\error",
        "STATUS_OBJECT_NAME_NOT_FOUND",
        true,
    ));

    assert_eq!(app.filtered_events().len(), 2);

    app.toggle_failures_only();
    let failures = app.filtered_events();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].1.is_failure);
    assert_eq!(failures[0].1.result, "STATUS_OBJECT_NAME_NOT_FOUND");
}

#[test]
fn test_include_exclude_combination_invariant() {
    let mut app = App::new(
        100,
        "process.exe".to_string(),
        false,
        Some("Roaming".to_string()),
        Some("Cache".to_string()),
        50_000,
    );

    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "\\AppData\\Roaming\\config.json",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        2,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "\\AppData\\Roaming\\Cache\\data.bin",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        3,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "\\Windows\\System32\\ntdll.dll",
        "STATUS_SUCCESS",
        false,
    ));

    let filtered = app.filtered_events();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].1.target, "\\AppData\\Roaming\\config.json");
}

#[test]
fn test_category_state_machine_transition() {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    assert_eq!(app.filter_category, None);

    app.cycle_category();
    assert_eq!(app.filter_category, Some(EventCategory::FileSystem));

    app.cycle_category();
    assert_eq!(app.filter_category, Some(EventCategory::Registry));

    app.cycle_category();
    assert_eq!(app.filter_category, Some(EventCategory::Network));

    app.cycle_category();
    assert_eq!(app.filter_category, Some(EventCategory::Process));

    app.cycle_category();
    assert_eq!(app.filter_category, None);
}

#[test]
fn test_pause_buffer_isolation() {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::Process,
        "CreateThread",
        "TID: 1001",
        "STATUS_SUCCESS",
        false,
    ));
    assert_eq!(app.events.len(), 1);

    app.toggle_pause();
    assert!(app.is_paused);

    // While paused, mutations to live stream must be discarded
    app.push_event(TraceEvent::new(
        2,
        100,
        2,
        EventCategory::Process,
        "CreateThread",
        "TID: 1002",
        "STATUS_SUCCESS",
        false,
    ));
    assert_eq!(app.events.len(), 1);

    app.toggle_pause();
    assert!(!app.is_paused);

    // After resuming, new events are pushed normally
    app.push_event(TraceEvent::new(
        3,
        100,
        3,
        EventCategory::Process,
        "CreateThread",
        "TID: 1003",
        "STATUS_SUCCESS",
        false,
    ));
    assert_eq!(app.events.len(), 2);
}

#[test]
fn test_clear_events_invariant() {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::Process,
        "CreateThread",
        "TID: 1001",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        2,
        100,
        2,
        EventCategory::FileSystem,
        "NtCreateFile",
        "C:\\test.txt",
        "STATUS_SUCCESS",
        false,
    ));
    assert_eq!(app.events.len(), 2);

    app.clear_events();
    assert_eq!(app.events.len(), 0);
    assert_eq!(app.dropped_events, 0);
    assert_eq!(app.selected_index, 0);
    assert!(app.selected_event().is_none());
    assert!(app.status_message.as_ref().unwrap().contains("CLEARED"));
}

#[test]
fn test_selected_event_and_copy_behavior() {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    // Empty state
    assert!(!app.copy_selected_to_clipboard());
    assert!(app.status_message.as_ref().unwrap().contains("EMPTY"));

    // With event present
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\Windows\\notepad.exe",
        "STATUS_SUCCESS",
        false,
    ));
    let selected = app.selected_event();
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().target, "C:\\Windows\\notepad.exe");

    // Copy execution
    let copied = app.copy_selected_to_clipboard();
    if copied {
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("COPIED TO CLIPBOARD"));
    } else {
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("CLIPBOARD ERROR"));
    }
}

#[test]
fn test_copy_all_to_clipboard_behavior() {
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 50_000);
    // Empty state
    assert!(!app.copy_all_to_clipboard());
    assert!(app.status_message.as_ref().unwrap().contains("EMPTY"));

    // With multiple events
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\file1.txt",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        2,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\file2.txt",
        "STATUS_SUCCESS",
        false,
    ));

    let copied = app.copy_all_to_clipboard();
    if copied {
        assert!(app.status_message.as_ref().unwrap().contains("COPIED ALL"));
        assert!(app.status_message.as_ref().unwrap().contains("2 events"));
    } else {
        assert!(app
            .status_message
            .as_ref()
            .unwrap()
            .contains("CLIPBOARD ERROR"));
    }
}

#[test]
fn test_process_exit_event_handling() {
    let mut app = App::new(20504, "notepad.exe".to_string(), false, None, None, 50_000);
    assert!(!app.is_terminated);

    let mut exit_evt = TraceEvent::new(
        9999,
        20504,
        0,
        EventCategory::Process,
        "ProcessExit",
        "PID: 20504".to_string(),
        "STATUS_SUCCESS",
        false,
    );
    exit_evt.details = "Exit Code: 0 (0x0)".to_string();

    app.push_event(exit_evt);
    assert!(app.is_terminated);
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .contains("[PROCESS EXITED]"));
    assert!(app
        .status_message
        .as_ref()
        .unwrap()
        .contains("Exit Code: 0"));
}

#[test]
fn test_launcher_handoff_and_exit_handling() {
    let mut app = App::new(1000, "notepad.exe".to_string(), false, None, None, 50_000);
    assert!(!app.is_terminated);

    // 1. LauncherExit for PID 1000 must NOT terminate the app session
    let mut lex_evt = TraceEvent::new(
        1001,
        1000,
        0,
        EventCategory::Process,
        "LauncherExit",
        "PID: 1000 [Launcher]".to_string(),
        "STATUS_SUCCESS",
        false,
    );
    lex_evt.details = "Launcher stub exited (code: 0)".to_string();
    app.push_event(lex_evt);
    assert!(!app.is_terminated);

    // 2. ProcessHandOff updates target_pid to 2000
    let mut handoff_evt = TraceEvent::new(
        1002,
        2000,
        0,
        EventCategory::Process,
        "ProcessHandOff",
        "PID: 1000 -> PID: 2000".to_string(),
        "STATUS_SUCCESS",
        false,
    );
    handoff_evt.details = "Handoff to packaged application: Notepad.exe".to_string();
    app.push_event(handoff_evt);
    assert_eq!(app.target_pid, 2000);
    assert_eq!(app.target_name, "Notepad.exe");
    assert!(!app.is_terminated);

    // 3. ProcessExit for target app (PID 2000) terminates the session
    let mut exit_evt = TraceEvent::new(
        1003,
        2000,
        0,
        EventCategory::Process,
        "ProcessExit",
        "PID: 2000 [Target App]".to_string(),
        "STATUS_SUCCESS",
        false,
    );
    exit_evt.details = "Exit Code: 0 (0x0)".to_string();
    app.push_event(exit_evt);
    assert!(app.is_terminated);
}

#[test]
fn test_bounded_ring_buffer_eviction_and_drop_counter() {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 3);
    assert_eq!(app.max_buffer_size, 3);
    assert_eq!(app.dropped_events, 0);

    // Act
    for i in 1..=5 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::FileSystem,
            "NtOpenFile",
            format!("C:\\file{}.txt", i),
            "STATUS_SUCCESS",
            false,
        ));
    }

    // Assert
    assert_eq!(app.events.len(), 3);
    assert_eq!(app.dropped_events, 2);
    assert_eq!(app.events[0].target, "C:\\file3.txt");
    assert_eq!(app.events[1].target, "C:\\file4.txt");
    assert_eq!(app.events[2].target, "C:\\file5.txt");
}

#[test]
fn test_bounded_ring_buffer_when_capacity_is_zero_then_clamps_to_one() {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 0);

    // Act & Assert (initial capacity check)
    assert_eq!(app.max_buffer_size, 1);
    assert_eq!(app.dropped_events, 0);

    // Act (pushing multiple events to capacity 1)
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\first.txt",
        "STATUS_SUCCESS",
        false,
    ));
    assert_eq!(app.events.len(), 1);
    assert_eq!(app.dropped_events, 0);

    app.push_event(TraceEvent::new(
        2,
        100,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\second.txt",
        "STATUS_SUCCESS",
        false,
    ));

    // Assert
    assert_eq!(app.events.len(), 1);
    assert_eq!(app.dropped_events, 1);
    assert_eq!(app.events[0].target, "C:\\second.txt");
}

#[test]
fn test_bounded_ring_buffer_when_burst_workload_then_evicts_oldest_records_fifo() {
    // Arrange
    const CAPACITY: usize = 10;
    const BURST_TOTAL: usize = 100;
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, CAPACITY);

    // Act
    for i in 1..=BURST_TOTAL {
        app.push_event(TraceEvent::new(
            i as u64,
            100,
            1,
            EventCategory::FileSystem,
            "NtReadFile",
            format!("C:\\burst_{}.dat", i),
            "STATUS_SUCCESS",
            false,
        ));
    }

    // Assert
    assert_eq!(app.events.len(), CAPACITY);
    assert_eq!(app.dropped_events, BURST_TOTAL - CAPACITY);
    for (idx, ev) in app.events.iter().enumerate() {
        let expected_seq = BURST_TOTAL - CAPACITY + 1 + idx;
        assert_eq!(ev.target, format!("C:\\burst_{}.dat", expected_seq));
    }
}

#[test]
fn test_bounded_ring_buffer_when_selected_at_head_and_eviction_occurs_then_clamps_selected_index() {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 3);
    app.push_event(TraceEvent::new(
        1,
        100,
        1,
        EventCategory::Process,
        "CreateProcess",
        "p1",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        2,
        100,
        1,
        EventCategory::Process,
        "CreateProcess",
        "p2",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        3,
        100,
        1,
        EventCategory::Process,
        "CreateProcess",
        "p3",
        "STATUS_SUCCESS",
        false,
    ));
    app.selected_index = 0;

    // Act (trigger eviction while selected_index is at head 0)
    app.push_event(TraceEvent::new(
        4,
        100,
        1,
        EventCategory::Process,
        "CreateProcess",
        "p4",
        "STATUS_SUCCESS",
        false,
    ));

    // Assert (selected_index stays at 0 without underflow, pointing to new head p2)
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.dropped_events, 1);
    assert_eq!(app.events[0].target, "p2");
}

#[test]
fn test_bounded_ring_buffer_when_selected_at_tail_and_eviction_occurs_then_auto_scrolls_to_new_tail(
) {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 3);
    for i in 1..=3 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::Process,
            "CreateProcess",
            format!("p{}", i),
            "STATUS_SUCCESS",
            false,
        ));
    }
    assert_eq!(app.selected_index, 2); // already at tail

    // Act (pushing p4 evicts p1 and appends p4 at tail)
    app.push_event(TraceEvent::new(
        4,
        100,
        1,
        EventCategory::Process,
        "CreateProcess",
        "p4",
        "STATUS_SUCCESS",
        false,
    ));

    // Assert (selected_index auto-scrolls to the new tail at index 2)
    assert_eq!(app.selected_index, 2);
    assert_eq!(app.events[app.selected_index].target, "p4");
}

#[test]
fn test_bounded_ring_buffer_when_selected_in_middle_and_eviction_occurs_then_decrements_index() {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 3);
    for i in 1..=3 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::Registry,
            "RegQueryValue",
            format!("Key_{}", i),
            "STATUS_SUCCESS",
            false,
        ));
    }
    app.selected_index = 1; // pointing to Key_2

    // Act (push Key_4 which evicts Key_1)
    app.push_event(TraceEvent::new(
        4,
        100,
        1,
        EventCategory::Registry,
        "RegQueryValue",
        "Key_4",
        "STATUS_SUCCESS",
        false,
    ));

    // Assert (selected_index decremented to 0, which still points to Key_2)
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.events[app.selected_index].target, "Key_2");
}

#[test]
fn test_bounded_ring_buffer_when_cleared_then_resets_dropped_events_counter() {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 2);
    for i in 1..=5 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::Network,
            "SocketConnect",
            format!("127.0.0.1:{}", i),
            "STATUS_SUCCESS",
            false,
        ));
    }
    assert_eq!(app.dropped_events, 3);

    // Act
    app.clear_events();

    // Assert
    assert_eq!(app.events.len(), 0);
    assert_eq!(app.dropped_events, 0);
    assert_eq!(app.selected_index, 0);
}

#[test]
fn test_bounded_ring_buffer_when_paused_then_discards_incoming_events_without_drops() {
    // Arrange
    let mut app = App::new(100, "process.exe".to_string(), false, None, None, 3);
    for i in 1..=3 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::FileSystem,
            "NtReadFile",
            format!("file_{}.txt", i),
            "STATUS_SUCCESS",
            false,
        ));
    }
    assert_eq!(app.events.len(), 3);
    assert_eq!(app.dropped_events, 0);

    // Act
    app.toggle_pause();
    assert!(app.is_paused);
    for i in 4..=8 {
        app.push_event(TraceEvent::new(
            i,
            100,
            1,
            EventCategory::FileSystem,
            "NtReadFile",
            format!("file_{}.txt", i),
            "STATUS_SUCCESS",
            false,
        ));
    }

    // Assert
    assert_eq!(app.events.len(), 3);
    assert_eq!(app.dropped_events, 0);
    assert_eq!(app.events[0].target, "file_1.txt");
}
