// wstrace - Event Data Structures

use serde::{Deserialize, Serialize};
use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

pub fn current_timestamp() -> String {
    unsafe {
        let mut st = std::mem::zeroed::<SYSTEMTIME>();
        GetLocalTime(&mut st);
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            st.wHour, st.wMinute, st.wSecond, st.wMilliseconds
        )
    }
}

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
        let now = current_timestamp();
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

    pub fn with_duration(mut self, duration_us: u64) -> Self {
        self.duration_us = duration_us;
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = details.into();
        self
    }

    pub fn with_status_code(mut self, code: u32) -> Self {
        let is_nt = self.operation.starts_with("Nt") || self.operation.starts_with("Zw");
        let resolved = if is_nt {
            crate::capture::lookup_ntstatus(code)
                .or_else(|| crate::capture::lookup_win32_error(code))
        } else {
            crate::capture::lookup_error_code(code)
        };

        if let Some(info) = resolved {
            self.result = format!("{} (0x{:08X})", info.name, code);
            self.is_failure = info.severity.is_failure();
        } else {
            self.result = format!("0x{:08X}", code);
            self.is_failure = if is_nt {
                code >= 0x8000_0000
            } else {
                code != 0
            };
        }
        self
    }

    pub fn with_ntstatus(mut self, status: u32) -> Self {
        if let Some(info) = crate::capture::lookup_ntstatus(status) {
            self.result = format!("{} (0x{:08X})", info.name, status);
            self.is_failure = info.severity.is_failure();
        } else {
            self.result = format!("0x{:08X}", status);
            self.is_failure = status >= 0x8000_0000;
        }
        self
    }
}

#[cfg(test)]
mod tests;
