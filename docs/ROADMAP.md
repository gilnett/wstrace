# Roadmap

This document outlines the version milestones and planned capabilities for `wstrace`.

---

## Milestone Overview

| Version | Status | Primary Focus & Key Additions |
| :--- | :--- | :--- |
| **v0.2.0** | **Released** | Interactive Ratatui TUI, Win32 Toolhelp probe, stream mode, category filtering, JSON session export |
| **v0.2.1** | **Implemented** | Bounded TUI ring buffer (`VecDeque`), `--buffer-size`, streaming to disk (`-o`), drop overrun metrics |
| **v0.2.2** | **Implemented** | Audit suite hardening, secrets & personal paths scanner, source hygiene, CLI doc sync, leak bench |
| **v0.3.0** | **Planned** | Native user-space ETW kernel ingestion (`File`, `Reg`, `Net`), QPC hardware timestamps, ProcMon parity |
| **v0.4.0** | **Planned** | Advanced filter engine, Win32 error code filters, call summary statistics (`--summary-stats`), TUI columns |
| **v0.5.0** | **Planned** | Lossless capture mode (`--zero-drop`), causal Activity ID correlation, Perfetto export format |
| **v0.6.0** | **Planned** | Real-time resource dashboard (CPU, Working Set RSS, Handles, GDI/USER objects), process tree view |
| **v0.7.0** | **Planned** | Userland call stack unwinding on debug events, PDB symbol resolution via `DbgHelp` |
| **v0.8.0** | **Planned** | Compact native binary trace format (`.wst`), offline trace replay (`save` / `load`), memory-mapped buffer |
| **v0.9.0** | **Planned** | Incremental search engine, privacy redaction (`--redact`), statistical profiling, HTML/Markdown reports |
| **v1.0.0** | **Planned** | Headless Windows Service daemon mode, CI policy assertions (`--assert-no-touch`), SARIF 2.1 support |

---

## Version Details

### v0.2.0 — Interactive TUI & Core Engine (Released)
- Interactive Ratatui terminal UI with category filtering (`ALL`, `FILE`, `REG`, `NET`, `PROC`).
- Non-invasive Win32 Toolhelp probe tracking process tree, threads, and dynamic module loads.
- Console streaming mode (`-m stream`) and end-of-session JSON export (`--export-json`).
- Windows clipboard export integration (`Ctrl+C`, `Ctrl+A`).

### v0.2.1 — Memory Buffering & Disk Streaming (Implemented)
- Circular ring buffer (`VecDeque`) evicting oldest events once capacity is reached.
- Configurable buffer depth via `--buffer-size <N>` (default: `50000`, ~15 MB RAM footprint).
- Direct-to-file line-delimited NDJSON streaming via `-o` / `--output <PATH>`.
- Visual drop metrics displayed in the TUI status header during buffer overruns.

### v0.2.2 — Audit Suite Hardening & Quality Gate (Implemented)
- Automated CLI documentation synchronization verification in `audit.ps1`, ensuring all arguments in `src/cli.rs` are documented in `docs/CLI_REFERENCE.md`.
- Static security & secret scanner: detect hardcoded API keys, auth tokens, private keys, and personal workstation paths (`C:\Users\<user>\...`).
- Source code hygiene scanner: enforce zero emojis in code/public documentation and English-only comments across the codebase.
- Integrity check for Divan micro-benchmarks (`cargo bench --no-run`) within the audit pipeline.
- Automated dynamic leak audit using `giga_audit bench` to verify flat memory slope (0 MB/min) and handle stability on `wstrace.exe`.
- Dependency tree validation (`cargo tree`) enforcing lean footprints, zero bloated runtime crates, and pure `windows-sys` bindings.

### v0.3.0 — Native ETW Ingestion (Planned)
- Native user-space ETW kernel sessions without third-party drivers (`OpenTraceW`, `ProcessTrace`).
- Core providers: `Microsoft-Windows-Kernel-File`, `Registry`, and `Network`.
- Hardware QPC timestamps for nanosecond-precision cross-provider ordering.
- Graceful privilege fallback to userland probes when running unprivileged.

### v0.4.0 — Advanced Filter Engine & Operation Statistics (Planned)
- Non-destructive filtering engine with boolean combinators (`AND`, `OR`, `NOT`) and regex predicates.
- Specific Win32 / NTSTATUS error code filtering (e.g. `--error-code 5` for `ERROR_ACCESS_DENIED`, `--error-code 2` for `ERROR_FILE_NOT_FOUND`).
- Call summary statistics inspired by `strace -c` (`--summary-stats`): tabular breakdown of API calls, invocation counts, failure ratios, and aggregate elapsed time.
- Interactive filter builder modal in the TUI.
- Configurable column visibility, ordering, and width presets.

### v0.5.0 — Lossless Tracing & Causal Correlation (Planned)
- Strict lossless / zero-drop mode (`--zero-drop`) applying blocking backpressure on channels.
- Cross-event causal correlation using ETW Activity IDs and thread dispatch context.
- Export to Google Perfetto / Chrome Trace viewer (`.json` / `.pftrace`).

### v0.6.0 — Resource Dashboard & Process Hierarchy (Planned)
- Real-time telemetry dashboard (CPU %, Working Set RSS, Commit VMS, Handles, GDI/USER objects).
- Interactive process tree browser with ancestor lineage and child tracking.

### v0.7.0 — Stack Walking & Symbol Resolution (Planned)
- Call stack unwinding on debug events without kernel drivers.
- Symbol resolution via Microsoft `DbgHelp` API with local PDB cache support.

### v0.8.0 — Native Binary Format & Offline Replay (Planned)
- Compact, indexed `.wst` binary format for long captures.
- Session persistence (`save` / `load`) and offline trace replay in the TUI.
- Memory-mapped buffer (`memmap2`) supporting large captures with minimal RAM overhead.

### v0.9.0 — Search, Privacy Redaction & Analytics Reporting (Planned)
- Incremental, cancellable search engine across in-memory buffers.
- Data redaction and anonymization mode (`--redact`): sanitizes user profile paths (`C:\Users\<user>\...` -> `C:\Users\<redacted>\...`) and sensitive registry values for sovereign trace sharing.
- Statistical operation profiling (top failing paths, slowest I/O calls, error rate breakdown).
- Standalone HTML and Markdown audit report generation.

### v1.0.0 — Production Daemon & CI Assertions (Planned)
- Headless Windows Service daemon for background server monitoring.
- Automated CI test assertions and security guards:
  - `--assert-no-errors`: fail execution if target process encounters runtime failures.
  - `--assert-no-touch <PATH>`: enforce sandbox access boundaries, raising exit codes if target touches unauthorized paths (e.g., `C:\Windows\System32`).
- SARIF 2.1 static/dynamic report output for DevSecOps pipelines.
