# wstrace Architecture & Internals

This document details the internal architecture, design principles, and component layout of `wstrace`.

---

## 1. High-Level Architecture

`wstrace` is organized into decoupled subsystems:

```text
                  +--------------------------------+
                  |           wstrace CLI          |
                  |          (src/cli.rs)          |
                  +---------------+----------------+
                                  |
                +-----------------+-----------------+
                |                                   |
                v                                   v
    +-----------------------+           +-----------------------+
    |   Target Execution    |           |   Target Attachment   |
    | (process_launcher.rs) |           |   (ToolHelp Snapshot) |
    +-----------+-----------+           +-----------+-----------+
                |                                   |
                +-----------------+-----------------+
                                  |
                                  v
                  +--------------------------------+
                  |         Capture Engine         |
                  |     (src/capture/win32_probe)  |
                  |     (src/capture/etw.rs)       |
                  +---------------+----------------+
                                  |
                                  v (Crossbeam Channel Bus)
                  +--------------------------------+
                  |          Event Model           |
                  |         (src/event.rs)         |
                  +---------------+----------------+
                                  |
                +-----------------+-----------------+
                |                                   |
                v                                   v
    +-----------------------+           +-----------------------+
    |    Interactive TUI    |           |    Output Pipeline    |
    |      (src/tui/)       |           |     (src/output/)     |
    |  - Table layout       |           |  - Stream Formatter   |
    |  - Keybindings engine |           |  - Summary Reporter   |
    |  - Windows Clipboard  |           |  - JSON Serialization |
    +-----------------------+           +-----------------------+
```

---

## 2. Component Layout

### `src/capture/` (Capture Engine)
- **`win32_probe.rs`**: Probes target processes and the Windows operating system via `ToolHelp32` snapshots without requiring kernel drivers. Detects module loads (`LoadLibraryW`), thread lifecycle (`CreateThread`), process spawning, and exits.
- **`etw.rs`**: Lightweight Event Tracing for Windows (ETW) consumer interface.
- **`process_launcher.rs`**: Safely launches child processes, manages command-line arguments, and suppresses console windows in TUI mode.

### `src/event.rs` (Data Model)
- Defines `TraceEvent`, `EventCategory` (`FileSystem`, `Registry`, `Network`, `Process`, `Memory`), and JSON serialization logic.

### `src/output/` (Output Pipeline)
- **`stream.rs`**: Live streaming engine printing formatted, ANSI-colored events directly to `stdout`.
- **`summary.rs`**: Aggregates final session metrics (total events, failures, unique targets) upon exit.
- **`json.rs`**: Structured JSON trace serialization.

### `src/tui/` (Terminal User Interface)
- **`app.rs`**: Application state machine, category filter cycling, search query filtering, and event buffer management.
- **`ui.rs`**: Ratatui rendering engine for telemetry headers, event stream tables, detail inspection panels, and footers.
- **`clipboard.rs`**: Windows API clipboard integration for copying individual or all filtered events.

---

## 3. Design Principles

1. **Zero Kernel Drivers (Ring-3 Safety)**: Completely userland-based, avoiding Blue Screen of Death (BSOD) risks or rootkit attack surfaces.
2. **Air-Gapped & Sovereign**: Zero network calls, zero telemetry reporting. Trace data stays strictly within workstation RAM.
3. **Decoupled Event Bus**: Multi-threaded capture threads communicate with rendering or output engines via bounded/unbounded crossbeam channels to avoid blocking the monitored application.
