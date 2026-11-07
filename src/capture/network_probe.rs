// wstrace - Real-Time Unprivileged Network Probe (TCP / UDP IPv4 + IPv6 via IP Helper)

use std::collections::HashSet;
use std::ffi::c_void;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use std::sync::mpsc::Sender;
use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
use windows_sys::Win32::NetworkManagement::IpHelper::{
    GetExtendedTcpTable, GetExtendedUdpTable, MIB_TCP6ROW_OWNER_PID, MIB_TCP6TABLE_OWNER_PID,
    MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, MIB_UDP6ROW_OWNER_PID, MIB_UDP6TABLE_OWNER_PID,
    MIB_UDPROW_OWNER_PID, MIB_UDPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_ALL, UDP_TABLE_OWNER_PID,
};
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6};

use crate::event::{current_timestamp, EventCategory, TraceEvent};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SocketRecord {
    pub protocol: &'static str,
    pub local_addr: String,
    pub local_port: u16,
    pub remote_addr: String,
    pub remote_port: u16,
    pub state: String,
    pub pid: u32,
}

pub struct NetworkProbe {
    pid: u32,
    target_name: String,
    running: Arc<AtomicBool>,
}

impl NetworkProbe {
    pub fn new(pid: u32, target_name: String) -> Self {
        Self {
            pid,
            target_name,
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn start(&self, event_tx: Sender<TraceEvent>) {
        let monitored_pid = self.pid;
        let target_name = self.target_name.clone();
        let running = Arc::clone(&self.running);

        thread::spawn(move || {
            // Pre-allocate collections and raw buffer to eliminate allocator pressure in polling loop
            let mut known_sockets: HashSet<SocketRecord> = HashSet::with_capacity(128);
            let mut current_sockets: HashSet<SocketRecord> = HashSet::with_capacity(128);
            let mut raw_buffer: Vec<u64> = Vec::with_capacity(2048);
            let mut event_id_counter: u64 = 50_000_000;

            while running.load(Ordering::Relaxed) {
                query_active_sockets_into(monitored_pid, &mut current_sockets, &mut raw_buffer);

                for sock in &current_sockets {
                    if !known_sockets.contains(sock) {
                        event_id_counter += 1;
                        let now = current_timestamp();

                        let target_display = if sock.remote_port > 0 {
                            format!(
                                "{}:{} -> {}:{}",
                                sock.local_addr,
                                sock.local_port,
                                sock.remote_addr,
                                sock.remote_port
                            )
                        } else {
                            format!("{}:{} ({})", sock.local_addr, sock.local_port, sock.state)
                        };

                        let operation = match (sock.protocol, sock.state.as_str()) {
                            ("TCP", "LISTEN") => "TcpListen".to_string(),
                            ("TCP", "ESTABLISHED") => "TcpConnect".to_string(),
                            ("TCP", "CLOSE_WAIT") | ("TCP", "TIME_WAIT") | ("TCP", "CLOSED") => {
                                "TcpDisconnect".to_string()
                            }
                            ("UDP", _) => "UdpBind".to_string(),
                            _ => format!("{}Active", sock.protocol),
                        };

                        let details = format!(
                            "Proto: {}, State: {}, Process: {} (PID: {})",
                            sock.protocol, sock.state, target_name, sock.pid
                        );

                        let event = TraceEvent {
                            id: event_id_counter,
                            timestamp: now,
                            pid: sock.pid,
                            tid: 0,
                            category: EventCategory::Network,
                            operation,
                            target: target_display,
                            result: "STATUS_SUCCESS".to_string(),
                            duration_us: 0,
                            is_failure: false,
                            details,
                        };

                        if event_tx.send(event).is_err() {
                            return;
                        }
                    }
                }

                // Swap sets to preserve capacity and avoid heap allocations across iterations
                std::mem::swap(&mut known_sockets, &mut current_sockets);

                thread::sleep(Duration::from_millis(250));
            }
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

/// Populates an existing HashSet with active sockets using a reusable raw buffer.
pub fn query_active_sockets_into(
    filter_pid: u32,
    records: &mut HashSet<SocketRecord>,
    raw_buffer: &mut Vec<u64>,
) {
    records.clear();

    // Query IPv4 and IPv6 TCP connections
    query_tcp_ipv4(filter_pid, records, raw_buffer);
    query_tcp_ipv6(filter_pid, records, raw_buffer);

    // Query IPv4 and IPv6 UDP endpoints
    query_udp_ipv4(filter_pid, records, raw_buffer);
    query_udp_ipv6(filter_pid, records, raw_buffer);
}

pub fn query_active_sockets(filter_pid: u32) -> HashSet<SocketRecord> {
    let mut records = HashSet::with_capacity(128);
    let mut buffer = Vec::with_capacity(2048);
    query_active_sockets_into(filter_pid, &mut records, &mut buffer);
    records
}

/// Fetches extended TCP table with 8-byte alignment (Vec<u64>) and TOCTOU retry loop into a reusable buffer.
fn fetch_extended_tcp_table(family: u32, buffer: &mut Vec<u64>) -> bool {
    let mut buffer_size: u32 = (buffer.len() * 8) as u32;

    for _ in 0..4 {
        let ret = unsafe {
            GetExtendedTcpTable(
                buffer.as_mut_ptr() as *mut c_void,
                &mut buffer_size,
                0,
                family,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };

        if ret == NO_ERROR {
            return true;
        }

        if ret == ERROR_INSUFFICIENT_BUFFER {
            // Over-allocate with 4KB (512 u64s) headroom to eliminate TOCTOU races under connection churn
            let words = (buffer_size as usize).div_ceil(8) + 512;
            buffer.resize(words, 0);
            buffer_size = (buffer.len() * 8) as u32;
            continue;
        }

        break;
    }

    false
}

/// Fetches extended UDP table with 8-byte alignment (Vec<u64>) and TOCTOU retry loop into a reusable buffer.
fn fetch_extended_udp_table(family: u32, buffer: &mut Vec<u64>) -> bool {
    let mut buffer_size: u32 = (buffer.len() * 8) as u32;

    for _ in 0..4 {
        let ret = unsafe {
            GetExtendedUdpTable(
                buffer.as_mut_ptr() as *mut c_void,
                &mut buffer_size,
                0,
                family,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };

        if ret == NO_ERROR {
            return true;
        }

        if ret == ERROR_INSUFFICIENT_BUFFER {
            // Over-allocate with 4KB (512 u64s) headroom
            let words = (buffer_size as usize).div_ceil(8) + 512;
            buffer.resize(words, 0);
            buffer_size = (buffer.len() * 8) as u32;
            continue;
        }

        break;
    }

    false
}

fn query_tcp_ipv4(filter_pid: u32, records: &mut HashSet<SocketRecord>, buffer: &mut Vec<u64>) {
    if fetch_extended_tcp_table(AF_INET as u32, buffer) {
        unsafe {
            let table = buffer.as_ptr() as *const MIB_TCPTABLE_OWNER_PID;
            let num_entries = (*table).dwNumEntries as usize;
            let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_TCPROW_OWNER_PID;

            for i in 0..num_entries {
                let row = std::ptr::read_unaligned(rows_ptr.add(i));

                if filter_pid != 0 && row.dwOwningPid != filter_pid {
                    continue;
                }

                let local_ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string();
                let local_port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);
                let remote_ip = Ipv4Addr::from(u32::from_be(row.dwRemoteAddr)).to_string();
                let remote_port = u16::from_be((row.dwRemotePort & 0xFFFF) as u16);
                let state = tcp_state_to_str(row.dwState).to_string();

                records.insert(SocketRecord {
                    protocol: "TCP",
                    local_addr: local_ip,
                    local_port,
                    remote_addr: remote_ip,
                    remote_port,
                    state,
                    pid: row.dwOwningPid,
                });
            }
        }
    }
}

fn query_tcp_ipv6(filter_pid: u32, records: &mut HashSet<SocketRecord>, buffer: &mut Vec<u64>) {
    if fetch_extended_tcp_table(AF_INET6 as u32, buffer) {
        unsafe {
            let table = buffer.as_ptr() as *const MIB_TCP6TABLE_OWNER_PID;
            let num_entries = (*table).dwNumEntries as usize;
            let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_TCP6ROW_OWNER_PID;

            for i in 0..num_entries {
                let row = std::ptr::read_unaligned(rows_ptr.add(i));

                if filter_pid != 0 && row.dwOwningPid != filter_pid {
                    continue;
                }

                let local_ip = Ipv6Addr::from(row.ucLocalAddr).to_string();
                let local_port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);
                let remote_ip = Ipv6Addr::from(row.ucRemoteAddr).to_string();
                let remote_port = u16::from_be((row.dwRemotePort & 0xFFFF) as u16);
                let state = tcp_state_to_str(row.dwState).to_string();

                records.insert(SocketRecord {
                    protocol: "TCP",
                    local_addr: local_ip,
                    local_port,
                    remote_addr: remote_ip,
                    remote_port,
                    state,
                    pid: row.dwOwningPid,
                });
            }
        }
    }
}

fn query_udp_ipv4(filter_pid: u32, records: &mut HashSet<SocketRecord>, buffer: &mut Vec<u64>) {
    if fetch_extended_udp_table(AF_INET as u32, buffer) {
        unsafe {
            let table = buffer.as_ptr() as *const MIB_UDPTABLE_OWNER_PID;
            let num_entries = (*table).dwNumEntries as usize;
            let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_UDPROW_OWNER_PID;

            for i in 0..num_entries {
                let row = std::ptr::read_unaligned(rows_ptr.add(i));

                if filter_pid != 0 && row.dwOwningPid != filter_pid {
                    continue;
                }

                let local_ip = Ipv4Addr::from(u32::from_be(row.dwLocalAddr)).to_string();
                let local_port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);

                records.insert(SocketRecord {
                    protocol: "UDP",
                    local_addr: local_ip,
                    local_port,
                    remote_addr: "*".to_string(),
                    remote_port: 0,
                    state: "BOUND".to_string(),
                    pid: row.dwOwningPid,
                });
            }
        }
    }
}

fn query_udp_ipv6(filter_pid: u32, records: &mut HashSet<SocketRecord>, buffer: &mut Vec<u64>) {
    if fetch_extended_udp_table(AF_INET6 as u32, buffer) {
        unsafe {
            let table = buffer.as_ptr() as *const MIB_UDP6TABLE_OWNER_PID;
            let num_entries = (*table).dwNumEntries as usize;
            let rows_ptr = std::ptr::addr_of!((*table).table) as *const MIB_UDP6ROW_OWNER_PID;

            for i in 0..num_entries {
                let row = std::ptr::read_unaligned(rows_ptr.add(i));

                if filter_pid != 0 && row.dwOwningPid != filter_pid {
                    continue;
                }

                let local_ip = Ipv6Addr::from(row.ucLocalAddr).to_string();
                let local_port = u16::from_be((row.dwLocalPort & 0xFFFF) as u16);

                records.insert(SocketRecord {
                    protocol: "UDP",
                    local_addr: local_ip,
                    local_port,
                    remote_addr: "*".to_string(),
                    remote_port: 0,
                    state: "BOUND".to_string(),
                    pid: row.dwOwningPid,
                });
            }
        }
    }
}

fn tcp_state_to_str(state: u32) -> &'static str {
    match state {
        1 => "CLOSED",
        2 => "LISTEN",
        3 => "SYN_SENT",
        4 => "SYN_RCVD",
        5 => "ESTABLISHED",
        6 => "FIN_WAIT1",
        7 => "FIN_WAIT2",
        8 => "CLOSE_WAIT",
        9 => "CLOSING",
        10 => "LAST_ACK",
        11 => "TIME_WAIT",
        12 => "DELETE_TCB",
        _ => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tcp_state_to_str_mapping() {
        assert_eq!(tcp_state_to_str(1), "CLOSED");
        assert_eq!(tcp_state_to_str(2), "LISTEN");
        assert_eq!(tcp_state_to_str(5), "ESTABLISHED");
        assert_eq!(tcp_state_to_str(8), "CLOSE_WAIT");
        assert_eq!(tcp_state_to_str(11), "TIME_WAIT");
        assert_eq!(tcp_state_to_str(99), "UNKNOWN");
    }

    #[test]
    fn test_query_active_sockets_runs_without_panic() {
        let current_pid = std::process::id();
        let sockets = query_active_sockets(current_pid);
        // Current test process may or may not have open sockets, but call must not panic
        let _ = sockets.len();
    }

    #[test]
    fn test_query_active_sockets_into_reusable_buffers() {
        let current_pid = std::process::id();
        let mut sockets = HashSet::with_capacity(64);
        let mut buffer = Vec::with_capacity(1024);
        query_active_sockets_into(current_pid, &mut sockets, &mut buffer);
        let _ = sockets.len();
    }

    #[test]
    fn test_network_probe_lifecycle() {
        let probe = NetworkProbe::new(0, "SYSTEM-WIDE".to_string());
        let (tx, _rx) = std::sync::mpsc::channel();
        probe.start(tx);
        probe.stop();
        assert!(!probe.running.load(Ordering::Relaxed));
    }
}
