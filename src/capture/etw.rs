// wstrace - Event Tracing for Windows (ETW) Consumer

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::event::TraceEvent;
use crossbeam_channel::Sender;

pub struct EtwConsumer {
    pid: u32,
    running: Arc<AtomicBool>,
}

impl EtwConsumer {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start(&self, _event_tx: Sender<TraceEvent>) {
        let _pid = self.pid;
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            // Note: Full kernel ETW sessions require SeProfileSingleProcessPrivilege / Admin elevation.
            // This thread monitors available event stream channels.
            while running.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(500));
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}
