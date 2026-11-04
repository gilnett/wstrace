# Agent & Contributor Guidelines for wstrace

This document defines the technical rules, architectural principles, and engineering standards for autonomous AI coding agents and human contributors working on `wstrace`.

---

## 1. Project Overview & Architecture

`wstrace` is a high-performance, zero-driver Windows system and API trace monitor written in 100% native Rust.

### Core Tenets
- **Zero Kernel Drivers (Ring-3 Safety)**: Completely userland-based. Never introduce third-party kernel drivers (`.sys`). Instrument via non-invasive Win32 debugging APIs, user-mode Event Tracing for Windows (ETW), and IP Helper APIs.
- **Air-Gapped & Sovereign**: Zero network calls, zero external telemetry. Trace telemetry is strictly local to the workstation.
- **Minimal Dependencies**: Strictly lean footprint. Prefer the standard library and raw bindings (`windows-sys`). Avoid runtime bloat or heavy frameworks.

---

## 2. Rust Code & Engineering Standards

### Error Handling & Safety
- **Zero Production `.unwrap()`**: Never use raw `.unwrap()` in production execution paths. Propagate errors via `Result` and the `?` operator, or document strict invariants using `.expect("invariant: <reason>")`. Raw `.unwrap()` is permitted exclusively in unit tests (`#[test]`) and `tests/`.
- **Win32 Status Integrity**: Explicitly distinguish NTSTATUS from Win32 error codes. Ensure proper handling of unmapped failure codes and check severity bits where applicable.
- **Deterministic Resource Cleanup**: Every opened Win32 handle (`HANDLE`, `HMODULE`, trace session handle) must be closed deterministically or managed via RAII guards to prevent handle leaks.

### Module & Import Hygiene
- **No Glob Imports**: Never use glob imports (`use crate::*` or `use module::*`) in production code. Always use explicit, qualified symbol imports.
- **Test Modules**: `use super::*;` is allowed solely inside `#[cfg(test)] mod tests` blocks and integration test files.

### Concurrency & Data Flow
- **Minimize Shared Ownership**: Prioritize unique ownership, borrowing, and channels over `Arc` or `Rc` on hot paths.
- **Decoupled Architecture**: Multi-threaded capture threads communicate with rendering and streaming engines via channels (`std::sync::mpsc`).
- **Data Loss Accounting**: Differentiate lossy real-time monitoring (default, dropping overruns to maintain UI responsiveness) from planned lossless/zero-drop modes. Always track and expose dropped event counters during backpressure.

### Type System & Design
- **Concrete Types First**: Avoid introducing traits solely for code organization. Retain concrete structs unless polymorphism or mock boundaries are genuinely necessary.
- **Preserve Documentation**: Maintain existing comments, docstrings, and tests unrelated to active changes.

---

## 3. CLI & Documentation Parity

- **Source of Truth**: Always verify CLI arguments, options, and flags against `src/cli.rs`. Never assume or invent flags not present in active source code.
- **Synchronized Testing**: Every CLI change, new flag, or subcommand must include corresponding unit tests in `src/cli.rs` and integration tests in `tests/cli_tests.rs`.
- **Documentation Parity**: Keep `docs/CLI_REFERENCE.md` strictly synchronized with `src/cli.rs`. All flags and subcommands must be documented with verified usage examples.
- **Feature Status**: Clearly differentiate implemented features from roadmap items.

---

## 4. Git & Pull Request Discipline

- **Pull Request Scoping**:
  - **Features**: Strictly at most **one feature per PR** (`1 feature max per PR`). Never combine multiple new capabilities into a single branch.
  - **Bugfixes**: Multiple related bugfixes and hardening patches can be grouped into a single PR (`fix/*`).
  - **Documentation**: Documentation updates, roadmap synchronizations, and guides can be consolidated into a single PR per version release (`docs/*`).
  - **No Unrelated Mixing**: Never bundle unrelated refactorings or cosmetic UI changes into a feature or bugfix PR.
- **Commit Title Hygiene**: Keep commit titles under 72 characters (never exceed 80 characters). Use conventional commit prefixes (`feat:`, `fix:`, `docs:`, `chore:`, `perf:`, `refactor:`).
- **Quality Gate**: Run `pwsh scripts/audit.ps1` before committing or tagging releases. All 17 checks must pass with zero warnings and zero failures.

