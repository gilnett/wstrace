// wstrace - TUI Application State & Event Filtering Engine

use std::collections::VecDeque;

use crate::event::{EventCategory, TraceEvent};

pub struct App {
    pub target_pid: u32,
    pub target_name: String,
    pub events: VecDeque<TraceEvent>,
    pub max_buffer_size: usize,
    pub dropped_events: usize,
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
        buffer_size: usize,
    ) -> Self {
        let max_buffer_size = buffer_size.max(1);
        Self {
            target_pid,
            target_name,
            events: VecDeque::with_capacity(max_buffer_size),
            max_buffer_size,
            dropped_events: 0,
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
            if let Some(app_name) = event
                .details
                .strip_prefix("Handoff to packaged application: ")
            {
                self.target_name = app_name.to_string();
            }
            self.status_message = Some(format!(
                "[HANDOFF] Tracing target application (PID: {})",
                event.pid
            ));
        }

        if event.operation == "ProcessExit" && self.target_pid != 0 && event.pid == self.target_pid
        {
            self.is_terminated = true;
            self.status_message = Some(format!("[PROCESS EXITED] {}", event.details));
        }

        if !self.is_paused {
            if self.events.len() >= self.max_buffer_size {
                self.events.pop_front();
                self.dropped_events += 1;
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            self.events.push_back(event);
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
        self.dropped_events = 0;
        self.selected_index = 0;
        self.status_message = Some("[CLEARED] Event buffer emptied (Ctrl+X)".to_string());
    }

    pub fn selected_event(&self) -> Option<TraceEvent> {
        let filtered = self.filtered_events();
        filtered
            .get(self.selected_index)
            .map(|(_, ev)| (*ev).clone())
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
                self.status_message =
                    Some("[CLIPBOARD ERROR] Failed to access clipboard".to_string());
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
            self.status_message = Some(format!(
                "[COPIED ALL] {} events copied to clipboard",
                filtered.len()
            ));
        } else {
            self.status_message = Some("[CLIPBOARD ERROR] Failed to access clipboard".to_string());
        }
        success
    }
}

#[cfg(test)]
mod tests;
