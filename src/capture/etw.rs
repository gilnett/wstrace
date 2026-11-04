// wstrace - Event Tracing for Windows (ETW) Consumer
// Real-time kernel session capture for elevated tracing (File, Network, Registry)

use std::ffi::c_void;
use std::mem::size_of;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use std::sync::mpsc::Sender;
use windows_sys::core::GUID;
use windows_sys::Win32::Foundation::{ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, NO_ERROR};
use windows_sys::Win32::System::Diagnostics::Etw::{
    CloseTrace, ControlTraceW, EnableTraceEx2, OpenTraceW, ProcessTrace, StartTraceW,
    CONTROLTRACE_HANDLE, EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD,
    EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_LOGFILEW, EVENT_TRACE_LOGFILEW_0, EVENT_TRACE_LOGFILEW_1,
    EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, PROCESS_TRACE_MODE_EVENT_RECORD,
    PROCESS_TRACE_MODE_REAL_TIME, TRACE_LEVEL_INFORMATION, WNODE_FLAG_TRACED_GUID,
};

use crate::capture::elevation::detect_elevation;
use crate::event::{current_timestamp, EventCategory, TraceEvent};

// Known Windows Kernel ETW Provider GUIDs
pub const KERNEL_FILE_GUID: GUID = GUID {
    data1: 0xedd08927,
    data2: 0x9cc4,
    data3: 0x4e65,
    data4: [0xb9, 0x70, 0xc2, 0x56, 0x0f, 0xb5, 0xc2, 0x89],
};

pub const KERNEL_NET_GUID: GUID = GUID {
    data1: 0x7dd42a49,
    data2: 0x5329,
    data3: 0x4832,
    data4: [0x8d, 0xfd, 0x43, 0xd9, 0x79, 0x15, 0x3a, 0x88],
};

pub const KERNEL_REG_GUID: GUID = GUID {
    data1: 0x70eb4f03,
    data2: 0xc1de,
    data3: 0x4f73,
    data4: [0xa0, 0x51, 0x33, 0xd1, 0x3d, 0x54, 0x13, 0xbd],
};

pub const KERNEL_PROC_GUID: GUID = GUID {
    data1: 0x22fb2cd6,
    data2: 0x0e7b,
    data3: 0x422b,
    data4: [0xa0, 0xc7, 0x2f, 0xad, 0x1f, 0xd0, 0xe7, 0x16],
};

struct CallbackState {
    filter_pid: u32,
    event_tx: Sender<TraceEvent>,
    event_counter: AtomicU64,
}

pub struct EtwConsumer {
    pid: u32,
    running: Arc<AtomicBool>,
    is_elevated: bool,
    session_handle: Arc<AtomicU64>,
    session_name: String,
}

impl EtwConsumer {
    pub fn new(pid: u32) -> Self {
        let is_elevated = detect_elevation().is_elevated();
        Self::with_elevation(pid, is_elevated)
    }

    pub fn with_elevation(pid: u32, is_elevated: bool) -> Self {
        let session_name = format!("wstrace-kernel-session-{}\0", std::process::id());
        Self {
            pid,
            running: Arc::new(AtomicBool::new(true)),
            is_elevated,
            session_handle: Arc::new(AtomicU64::new(0)),
            session_name,
        }
    }

    pub fn is_elevated(&self) -> bool {
        self.is_elevated
    }

    pub fn session_name(&self) -> &str {
        &self.session_name
    }

    pub fn start(&self, event_tx: Sender<TraceEvent>) {
        if !self.is_elevated {
            // Unprivileged execution: kernel ETW sessions require SeProfileSingleProcessPrivilege.
            // Operation gracefully falls back to unprivileged probes (Win32Probe / NetworkProbe).
            return;
        }

        let pid = self.pid;
        let running = Arc::clone(&self.running);
        let session_holder = Arc::clone(&self.session_handle);
        let session_name = self.session_name.clone();

        thread::spawn(move || {
            let session_name_utf16: Vec<u16> = session_name.encode_utf16().collect();
            let mut props_buffer = allocate_session_properties(&session_name);
            let props = props_buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;

            // Stop any dangling previous session with the same name
            unsafe {
                ControlTraceW(
                    CONTROLTRACE_HANDLE { Value: 0 },
                    session_name_utf16.as_ptr(),
                    props,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }

            // Reset properties before StartTraceW
            let mut props_buffer = allocate_session_properties(&session_name);
            let props = props_buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;

            let mut session_handle = CONTROLTRACE_HANDLE { Value: 0 };
            let start_status =
                unsafe { StartTraceW(&mut session_handle, session_name_utf16.as_ptr(), props) };

            if start_status != NO_ERROR && start_status != ERROR_ALREADY_EXISTS {
                if start_status == ERROR_ACCESS_DENIED {
                    return;
                }
                return;
            }

            session_holder.store(session_handle.Value, Ordering::SeqCst);

            // Enable kernel providers
            enable_provider(session_handle, &KERNEL_FILE_GUID);
            enable_provider(session_handle, &KERNEL_NET_GUID);
            enable_provider(session_handle, &KERNEL_REG_GUID);
            enable_provider(session_handle, &KERNEL_PROC_GUID);

            let callback_state = Box::new(CallbackState {
                filter_pid: pid,
                event_tx,
                event_counter: AtomicU64::new(100_000_000),
            });
            let context_ptr = Box::into_raw(callback_state);

            let mut session_name_copy = session_name_utf16.clone();
            let mut logfile: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
            logfile.LoggerName = session_name_copy.as_mut_ptr();
            logfile.Anonymous1 = EVENT_TRACE_LOGFILEW_0 {
                ProcessTraceMode: PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD,
            };
            logfile.Anonymous2 = EVENT_TRACE_LOGFILEW_1 {
                EventRecordCallback: Some(etw_record_callback),
            };
            logfile.Context = context_ptr as *mut c_void;

            let trace_handle = unsafe { OpenTraceW(&mut logfile) };

            if trace_handle.Value != 0 && trace_handle.Value != u64::MAX {
                let handles = [trace_handle];
                unsafe {
                    ProcessTrace(
                        handles.as_ptr(),
                        1,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    CloseTrace(trace_handle);
                }
            }

            // Cleanup callback state box
            unsafe {
                let _ = Box::from_raw(context_ptr);
            }

            // Cleanup session on exit
            while running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(100));
            }

            let mut stop_props_buffer = allocate_session_properties(&session_name);
            let stop_props = stop_props_buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
            unsafe {
                ControlTraceW(
                    session_handle,
                    session_name_utf16.as_ptr(),
                    stop_props,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
        let session_val = self.session_handle.load(Ordering::SeqCst);
        if session_val != 0 {
            let session_name_utf16: Vec<u16> = self.session_name.encode_utf16().collect();
            let mut stop_props_buffer = allocate_session_properties(&self.session_name);
            let stop_props = stop_props_buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;
            unsafe {
                ControlTraceW(
                    CONTROLTRACE_HANDLE { Value: session_val },
                    session_name_utf16.as_ptr(),
                    stop_props,
                    EVENT_TRACE_CONTROL_STOP,
                );
            }
        }
    }
}

fn allocate_session_properties(session_name: &str) -> Vec<u8> {
    let props_size = size_of::<EVENT_TRACE_PROPERTIES>();
    let name_bytes = (session_name.len() + 1) * 2;
    let total_size = props_size + name_bytes;

    let mut buffer = vec![0u8; total_size];
    let props = buffer.as_mut_ptr() as *mut EVENT_TRACE_PROPERTIES;

    unsafe {
        (*props).Wnode.BufferSize = total_size as u32;
        (*props).Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        (*props).Wnode.ClientContext = 1; // 1 = QPC clock resolution
        (*props).LogFileMode = EVENT_TRACE_REAL_TIME_MODE;
        (*props).LoggerNameOffset = props_size as u32;
    }

    buffer
}

fn enable_provider(session_handle: CONTROLTRACE_HANDLE, provider_guid: &GUID) {
    unsafe {
        EnableTraceEx2(
            session_handle,
            provider_guid,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_INFORMATION as u8,
            0,
            0,
            0,
            std::ptr::null(),
        );
    }
}

unsafe extern "system" fn etw_record_callback(record: *mut EVENT_RECORD) {
    if record.is_null() {
        return;
    }

    let header = (*record).EventHeader;
    let event_pid = header.ProcessId;

    let state_ptr = (*record).UserContext as *mut CallbackState;
    if state_ptr.is_null() {
        return;
    }

    let state = &*state_ptr;

    if state.filter_pid != 0 && event_pid != state.filter_pid {
        return;
    }

    let provider_guid = header.ProviderId;
    let (category, operation_prefix) = if guid_eq(&provider_guid, &KERNEL_FILE_GUID) {
        (EventCategory::FileSystem, "NtFile")
    } else if guid_eq(&provider_guid, &KERNEL_NET_GUID) {
        (EventCategory::Network, "NetKernel")
    } else if guid_eq(&provider_guid, &KERNEL_REG_GUID) {
        (EventCategory::Registry, "RegKernel")
    } else if guid_eq(&provider_guid, &KERNEL_PROC_GUID) {
        (EventCategory::Process, "ProcKernel")
    } else {
        return;
    };

    let opcode = header.EventDescriptor.Opcode;
    let operation = format!("{}Op{}", operation_prefix, opcode);
    let event_id = state.event_counter.fetch_add(1, Ordering::Relaxed);
    let now = current_timestamp();

    let event = TraceEvent {
        id: event_id,
        timestamp: now,
        pid: event_pid,
        tid: header.ThreadId,
        category,
        operation,
        target: format!("KernelEvent (Provider: {:08x})", provider_guid.data1),
        result: "STATUS_SUCCESS".to_string(),
        duration_us: 0,
        is_failure: false,
        details: format!("Opcode: {}, Kernel ETW Trace", opcode),
    };

    let _ = state.event_tx.send(event);
}

fn guid_eq(a: &GUID, b: &GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_etw_consumer_creation_and_elevation_state() {
        let consumer = EtwConsumer::with_elevation(1234, false);
        assert!(!consumer.is_elevated());
        assert_eq!(consumer.pid, 1234);

        let elevated_consumer = EtwConsumer::with_elevation(1234, true);
        assert!(elevated_consumer.is_elevated());
    }

    #[test]
    fn test_etw_consumer_lifecycle() {
        let consumer = EtwConsumer::new(0);
        let (tx, _rx) = std::sync::mpsc::channel();
        consumer.start(tx);
        consumer.stop();
        assert!(!consumer.running.load(Ordering::Relaxed));
    }

    #[test]
    fn test_guid_equality_predicate() {
        assert!(guid_eq(&KERNEL_FILE_GUID, &KERNEL_FILE_GUID));
        assert!(!guid_eq(&KERNEL_FILE_GUID, &KERNEL_NET_GUID));
    }

    #[test]
    fn test_allocate_session_properties_buffer() {
        let buffer = allocate_session_properties("test-session\0");
        assert!(buffer.len() > size_of::<EVENT_TRACE_PROPERTIES>());
        let props = buffer.as_ptr() as *const EVENT_TRACE_PROPERTIES;
        unsafe {
            assert_eq!((*props).LogFileMode, EVENT_TRACE_REAL_TIME_MODE);
            assert_eq!((*props).Wnode.ClientContext, 1);
        }
    }

    #[test]
    fn test_etw_session_name_includes_pid() {
        let consumer = EtwConsumer::new(0);
        assert!(consumer
            .session_name()
            .starts_with("wstrace-kernel-session-"));
        assert!(consumer.session_name().ends_with('\0'));
    }
}
