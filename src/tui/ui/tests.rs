use super::*;
use crate::event::TraceEvent;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::Terminal;

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut text = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            text.push_str(buffer[(x, y)].symbol());
        }
        text.push('\n');
    }
    text
}

#[test]
fn test_ui_header_when_zero_dropped_events_then_drop_badge_is_hidden() {
    // Arrange
    let backend = TestBackend::new(140, 5);
    let mut terminal = Terminal::new(backend).expect("Terminal initialization must succeed");
    let app = App::new(1234, "test_app.exe".to_string(), false, None, None, 50_000);
    assert_eq!(app.dropped_events, 0);

    // Act
    terminal
        .draw(|f| render_header(f, f.area(), &app))
        .expect("Render must succeed");

    // Assert
    let rendered_text = buffer_to_string(terminal.backend().buffer());
    assert!(rendered_text.contains("test_app.exe"));
    assert!(rendered_text.contains("PID: 1234"));
    assert!(!rendered_text.contains("DROP:"));
}

#[test]
fn test_ui_header_when_dropped_events_present_then_renders_drop_badge() {
    // Arrange
    let backend = TestBackend::new(140, 5);
    let mut terminal = Terminal::new(backend).expect("Terminal initialization must succeed");
    let mut app = App::new(1234, "test_app.exe".to_string(), false, None, None, 50_000);
    app.dropped_events = 42;

    // Act
    terminal
        .draw(|f| render_header(f, f.area(), &app))
        .expect("Render must succeed");

    // Assert
    let rendered_text = buffer_to_string(terminal.backend().buffer());
    assert!(rendered_text.contains("[DROP: 42]"));
}

#[test]
fn test_ui_header_when_extreme_dropped_events_count_then_renders_formatted_counter() {
    // Arrange
    let backend = TestBackend::new(160, 5);
    let mut terminal = Terminal::new(backend).expect("Terminal initialization must succeed");
    let mut app = App::new(1234, "test_app.exe".to_string(), false, None, None, 50_000);
    app.dropped_events = 1_000_000;

    // Act
    terminal
        .draw(|f| render_header(f, f.area(), &app))
        .expect("Render must succeed");

    // Assert
    let rendered_text = buffer_to_string(terminal.backend().buffer());
    assert!(rendered_text.contains("[DROP: 1000000]"));
}

#[test]
fn test_ui_full_render_when_active_events_then_renders_all_panes_without_panic() {
    // Arrange
    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).expect("Terminal initialization must succeed");
    let mut app = App::new(4321, "myapp.exe".to_string(), false, None, None, 5);
    app.push_event(TraceEvent::new(
        1,
        4321,
        1,
        EventCategory::FileSystem,
        "NtOpenFile",
        "C:\\data.txt",
        "STATUS_SUCCESS",
        false,
    ));
    app.push_event(TraceEvent::new(
        2,
        4321,
        1,
        EventCategory::Registry,
        "RegQueryValue",
        "HKCU\\Test",
        "STATUS_OBJECT_NAME_NOT_FOUND",
        true,
    ));

    // Act
    let draw_res = terminal.draw(|f| render(f, &mut app));

    // Assert
    assert!(draw_res.is_ok());
    let rendered_text = buffer_to_string(terminal.backend().buffer());
    assert!(rendered_text.contains("myapp.exe"));
    assert!(rendered_text.contains("NtOpenFile"));
    assert!(rendered_text.contains("RegQueryValue"));
}
