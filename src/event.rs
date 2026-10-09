// wstrace - Event Data Structures

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventCategory {
    FileSystem,
    Registry,
    Network,
    Process,
    Memory,
    Error,
}

impl EventCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FileSystem => "FILE",
            Self::Registry => "REG",
            Self::Network => "NET",
            Self::Process => "PROC",
            Self::Memory => "MEM",
            Self::Error => "ERR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub id: u64,
    pub timestamp: String,
    pub pid: u32,
    pub tid: u32,
    pub category: EventCategory,
    pub operation: String,
    pub target: String,
    pub result: String,
    pub duration_us: u64,
    pub is_failure: bool,
    pub details: String,
}

impl TraceEvent {
    pub fn new(
        id: u64,
        pid: u32,
        tid: u32,
        category: EventCategory,
        operation: impl Into<String>,
        target: impl Into<String>,
        result: impl Into<String>,
        is_failure: bool,
    ) -> Self {
        let now = chrono::Local::now().format("%H:%M:%S%.3f").to_string();
        Self {
            id,
            timestamp: now,
            pid,
            tid,
            category,
            operation: operation.into(),
            target: target.into(),
            result: result.into(),
            duration_us: 0,
            is_failure,
            details: String::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_category_string_mapping() {
        assert_eq!(EventCategory::FileSystem.as_str(), "FILE");
        assert_eq!(EventCategory::Registry.as_str(), "REG");
        assert_eq!(EventCategory::Network.as_str(), "NET");
        assert_eq!(EventCategory::Process.as_str(), "PROC");
        assert_eq!(EventCategory::Memory.as_str(), "MEM");
        assert_eq!(EventCategory::Error.as_str(), "ERR");
    }

    #[test]
    fn test_event_lifecycle_and_serialization() {
        let event = TraceEvent::new(
            1,
            4096,
            12,
            EventCategory::FileSystem,
            "NtCreateFile",
            "\\Device\\HarddiskVolume3\\Windows\\System32\\ntdll.dll",
            "STATUS_SUCCESS",
            false,
        );

        assert_eq!(event.id, 1);
        assert_eq!(event.pid, 4096);
        assert_eq!(event.tid, 12);
        assert_eq!(event.category, EventCategory::FileSystem);
        assert!(!event.is_failure);

        let serialized = serde_json::to_string(&event).expect("Serialization must succeed");
        let deserialized: TraceEvent = serde_json::from_str(&serialized).expect("Deserialization must succeed");

        assert_eq!(deserialized.id, event.id);
        assert_eq!(deserialized.pid, event.pid);
        assert_eq!(deserialized.operation, event.operation);
        assert_eq!(deserialized.target, event.target);
        assert_eq!(deserialized.result, event.result);
        assert_eq!(deserialized.is_failure, event.is_failure);
    }
}
