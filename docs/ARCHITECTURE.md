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

## 3. Design Principles & Technical Boundaries

1. **Zero Kernel Drivers (Ring-3 Safety)**: Completely userland-based, avoiding Blue Screen of Death (BSOD) risks or rootkit attack surfaces.
2. **Air-Gapped & Sovereign**: Zero network calls, zero telemetry reporting. Trace data stays strictly within workstation RAM or local output files.
3. **Decoupled Event Bus & Dual Retention**: Multi-threaded capture threads communicate with rendering and streaming engines via Crossbeam channels. Supports lossy real-time monitoring (default, dropping overruns to maintain responsiveness) and planned lossless mode (`--zero-drop`).
4. **Minimal Dependencies & Lean Footprint**: Pure standard library Rust and lean raw bindings (`windows-sys`). Standalone release executable targeted under 5 MB, with strict RAM caps (~15 MB for 50,000 buffered events).

---

## 4. Testing & Quality Assurance Architecture

Every component in `wstrace` follows strict verification standards:

1. **100% CLI Synchronization**:
   - Every CLI argument, flag, and subcommand defined in `src/cli.rs` must have corresponding parsing and edge-case validation tests in `tests/cli_tests.rs` and `src/cli.rs`.
2. **Isolated & Unprivileged Unit Tests**:
   - All unit tests run in standard user mode without requiring Administrator elevation, external network connectivity, or active kernel ETW sessions.
   - Deterministic test fixtures and the Arrange-Act-Assert (AAA) pattern isolate business logic (e.g., ring buffer eviction, filter predicates, JSON serialization).
3. **Headless TUI Testing**:
   - The TUI rendering pipeline is tested using Ratatui's in-memory `TestBackend`, validating layout formatting, overrun badges, and status bar elements without spawning a physical console window.
4. **Performance & Reliability Benchmarking**:
   - Micro-benchmarks profile hot-path event ingestion, ring buffer operations, and JSON serialization (`benches/perf_bench.rs`).
   - Macro-benchmarks measure target process overhead and host resource utilization.
   - Endurance and reliability testing verifies zero memory or handle leaks and capture integrity under heavy burst loads.

---
