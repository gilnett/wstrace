// wstrace - TUI Application State & Event Filtering Engine

use crate::event::{EventCategory, TraceEvent};

pub struct App {
    pub target_pid: u32,
    pub target_name: String,
    pub events: Vec<TraceEvent>,
    pub selected_index: usize,
    pub is_paused: bool,
    pub search_query: String,
    #[allow(dead_code)]
    pub is_searching: bool,
    pub filter_failures_only: bool,
    pub filter_category: Option<EventCategory>,
    pub include_pattern: Option<String>,
    pub exclude_pattern: Option<String>,
    pub should_quit: bool,
    pub status_message: Option<String>,
    pub is_terminated: bool,
}

impl App {
    pub fn new(
        target_pid: u32,
        target_name: String,
        only_failures: bool,
        include_pattern: Option<String>,
        exclude_pattern: Option<String>,
    ) -> Self {
        Self {
            target_pid,
            target_name,
            events: Vec::with_capacity(50_000),
            selected_index: 0,
            is_paused: false,
            search_query: String::new(),
            is_searching: false,
            filter_failures_only: only_failures,
            filter_category: None,
            include_pattern,
            exclude_pattern,
            should_quit: false,
            status_message: None,
            is_terminated: false,
        }
    }

    pub fn push_event(&mut self, event: TraceEvent) {
        if event.operation == "ProcessHandOff" {
            self.target_pid = event.pid;
            self.is_terminated = false;
            if let Some(app_name) = event.details.strip_prefix("Handoff to packaged application: ") {
                self.target_name = app_name.to_string();
            }
            self.status_message = Some(format!("[HANDOFF] Tracing target application (PID: {})", event.pid));
        }

        if event.operation == "ProcessExit" && event.pid == self.target_pid {
            self.is_terminated = true;
            self.status_message = Some(format!("[PROCESS EXITED] {}", event.details));
        }

        if !self.is_paused {
            self.events.push(event);
            if self.events.len() > 1 && self.selected_index == self.events.len() - 2 {
                self.selected_index = self.events.len() - 1;
            }
        }
    }


    pub fn filtered_events(&self) -> Vec<(usize, &TraceEvent)> {
        self.events
            .iter()
            .enumerate()
            .filter(|(_, ev)| {
                if self.filter_failures_only && !ev.is_failure {
                    return false;
                }
                if let Some(cat) = self.filter_category {
                    if ev.category != cat {
                        return false;
                    }
                }
                if let Some(inc) = &self.include_pattern {
                    if !ev.target.to_lowercase().contains(&inc.to_lowercase())
                        && !ev.operation.to_lowercase().contains(&inc.to_lowercase())
                    {
                        return false;
                    }
                }
                if let Some(exc) = &self.exclude_pattern {
                    if ev.target.to_lowercase().contains(&exc.to_lowercase())
                        || ev.operation.to_lowercase().contains(&exc.to_lowercase())
                    {
                        return false;
                    }
                }
                if !self.search_query.is_empty() {
                    let q = self.search_query.to_lowercase();
                    if !ev.operation.to_lowercase().contains(&q)
                        && !ev.target.to_lowercase().contains(&q)
                        && !ev.result.to_lowercase().contains(&q)
                    {
                        return false;
                    }
                }
                true
            })
            .collect()
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_events().len();
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
    }

    pub fn select_previous(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.is_paused = !self.is_paused;
        self.status_message = Some(if self.is_paused {
            "Tracing PAUSED - Press [Space] to resume".to_string()
        } else {
            "Tracing LIVE".to_string()
        });
    }

    pub fn toggle_failures_only(&mut self) {
        self.filter_failures_only = !self.filter_failures_only;
        self.selected_index = 0;
    }

    pub fn cycle_category(&mut self) {
        self.filter_category = match self.filter_category {
            None => Some(EventCategory::FileSystem),
            Some(EventCategory::FileSystem) => Some(EventCategory::Registry),
            Some(EventCategory::Registry) => Some(EventCategory::Network),
            Some(EventCategory::Network) => Some(EventCategory::Process),
            Some(EventCategory::Process) => None,
            _ => None,
        };
        self.selected_index = 0;
    }

    pub fn clear_events(&mut self) {
        self.events.clear();
        self.selected_index = 0;
        self.status_message = Some("[CLEARED] Event buffer emptied (Ctrl+X)".to_string());
    }

    pub fn selected_event(&self) -> Option<TraceEvent> {
        let filtered = self.filtered_events();
        filtered.get(self.selected_index).map(|(_, ev)| (*ev).clone())
    }

    pub fn copy_selected_to_clipboard(&mut self) -> bool {
        if let Some(ev) = self.selected_event() {
            let formatted = format!(
                "[{}] [{}] {} -> {} ({}) | Details: {}",
                ev.timestamp,
                ev.category.as_str(),
                ev.operation,
                ev.result,
                ev.target,
                ev.details
            );
            let success = crate::tui::clipboard::set_clipboard_text(&formatted);
            if success {
                self.status_message = Some(format!("[COPIED TO CLIPBOARD] {}", ev.target));
            } else {
                self.status_message = Some("[CLIPBOARD ERROR] Failed to access clipboard".to_string());
            }
            success
        } else {
            self.status_message = Some("[EMPTY] No event selected to copy".to_string());
            false
        }
    }

    pub fn copy_all_to_clipboard(&mut self) -> bool {
        let filtered = self.filtered_events();
        if filtered.is_empty() {
            self.status_message = Some("[EMPTY] No events to copy".to_string());
            return false;
        }

        let mut buffer = String::with_capacity(filtered.len() * 128);
        for (_, ev) in &filtered {
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

        let success = crate::tui::clipboard::set_clipboard_text(&buffer);
        if success {
            self.status_message = Some(format!("[COPIED ALL] {} events copied to clipboard", filtered.len()));
        } else {
            self.status_message = Some("[CLIPBOARD ERROR] Failed to access clipboard".to_string());
        }
        success
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failure_filtering_invariant() {
        let mut app = App::new(100, "process.exe".to_string(), false, None, None);
        app.push_event(TraceEvent::new(1, 100, 1, EventCategory::FileSystem, "NtCreateFile", "\\path\\success", "STATUS_SUCCESS", false));
        app.push_event(TraceEvent::new(2, 100, 1, EventCategory::FileSystem, "NtCreateFile", "\\path\\error", "STATUS_OBJECT_NAME_NOT_FOUND", true));

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
        );

        app.push_event(TraceEvent::new(1, 100, 1, EventCategory::FileSystem, "NtOpenFile", "\\AppData\\Roaming\\config.json", "STATUS_SUCCESS", false));
        app.push_event(TraceEvent::new(2, 100, 1, EventCategory::FileSystem, "NtOpenFile", "\\AppData\\Roaming\\Cache\\data.bin", "STATUS_SUCCESS", false));
        app.push_event(TraceEvent::new(3, 100, 1, EventCategory::FileSystem, "NtOpenFile", "\\Windows\\System32\\ntdll.dll", "STATUS_SUCCESS", false));

        let filtered = app.filtered_events();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1.target, "\\AppData\\Roaming\\config.json");
    }

    #[test]
    fn test_category_state_machine_transition() {
        let mut app = App::new(100, "process.exe".to_string(), false, None, None);
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
        let mut app = App::new(100, "process.exe".to_string(), false, None, None);
        app.push_event(TraceEvent::new(1, 100, 1, EventCategory::Process, "CreateThread", "TID: 1001", "STATUS_SUCCESS", false));
        assert_eq!(app.events.len(), 1);

        app.toggle_pause();
        assert!(app.is_paused);

        // While paused, mutations to live stream must be discarded
        app.push_event(TraceEvent::new(2, 100, 2, EventCategory::Process, "CreateThread", "TID: 1002", "STATUS_SUCCESS", false));
        assert_eq!(app.events.len(), 1);

        app.toggle_pause();
        assert!(!app.is_paused);

        // After resuming, new events are pushed normally
        app.push_event(TraceEvent::new(3, 100, 3, EventCategory::Process, "CreateThread", "TID: 1003", "STATUS_SUCCESS", false));
        assert_eq!(app.events.len(), 2);
    }

    #[test]
    fn test_clear_events_invariant() {
        let mut app = App::new(100, "process.exe".to_string(), false, None, None);
        app.push_event(TraceEvent::new(1, 100, 1, EventCategory::Process, "CreateThread", "TID: 1001", "STATUS_SUCCESS", false));
        app.push_event(TraceEvent::new(2, 100, 2, EventCategory::FileSystem, "NtCreateFile", "C:\\test.txt", "STATUS_SUCCESS", false));
        assert_eq!(app.events.len(), 2);

        app.clear_events();
        assert_eq!(app.events.len(), 0);
        assert_eq!(app.selected_index, 0);
        assert!(app.selected_event().is_none());
        assert!(app.status_message.as_ref().unwrap().contains("CLEARED"));
    }

    #[test]
    fn test_selected_event_and_copy_behavior() {
        let mut app = App::new(100, "process.exe".to_string(), false, None, None);
        // Empty state
        assert!(!app.copy_selected_to_clipboard());
        assert!(app.status_message.as_ref().unwrap().contains("EMPTY"));

        // With event present
        app.push_event(TraceEvent::new(1, 100, 1, EventCategory::FileSystem, "NtOpenFile", "C:\\Windows\\notepad.exe", "STATUS_SUCCESS", false));
        let selected = app.selected_event();
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().target, "C:\\Windows\\notepad.exe");

        // Copy execution
        let copied = app.copy_selected_to_clipboard();
        assert!(copied);
        assert!(app.status_message.as_ref().unwrap().contains("COPIED TO CLIPBOARD"));
    }

    #[test]
    fn test_copy_all_to_clipboard_behavior() {
        let mut app = App::new(100, "process.exe".to_string(), false, None, None);
        // Empty state
        assert!(!app.copy_all_to_clipboard());
        assert!(app.status_message.as_ref().unwrap().contains("EMPTY"));

        // With multiple events
        app.push_event(TraceEvent::new(1, 100, 1, EventCategory::FileSystem, "NtOpenFile", "C:\\file1.txt", "STATUS_SUCCESS", false));
        app.push_event(TraceEvent::new(2, 100, 1, EventCategory::FileSystem, "NtOpenFile", "C:\\file2.txt", "STATUS_SUCCESS", false));
        
        let copied = app.copy_all_to_clipboard();
        assert!(copied);
        assert!(app.status_message.as_ref().unwrap().contains("COPIED ALL"));
        assert!(app.status_message.as_ref().unwrap().contains("2 events"));
    }

    #[test]
    fn test_process_exit_event_handling() {
        let mut app = App::new(20504, "notepad.exe".to_string(), false, None, None);
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
        assert!(app.status_message.as_ref().unwrap().contains("[PROCESS EXITED]"));
        assert!(app.status_message.as_ref().unwrap().contains("Exit Code: 0"));
    }

    #[test]
    fn test_launcher_handoff_and_exit_handling() {
        let mut app = App::new(1000, "notepad.exe".to_string(), false, None, None);
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
}

