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
    #[allow(clippy::too_many_arguments)]
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
mod tests;
