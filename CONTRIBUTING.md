# Contributing to wstrace

Thank you for your interest in contributing to `wstrace`. To ensure high standards of software quality, security, and institutional-grade legal compliance, please review the following guidelines before submitting any contribution.

---

## 1. Code of Conduct & Development Principles

`wstrace` is designed with strict production requirements:
- **Core Engine:** Idiomatic Rust (2021 Edition).
- **Scripting & Packaging:** PowerShell (strictly scoped to packaging and installation scripts).
- **Zero Drivers / Zero Kernel Bloat:** All instrumentation must strictly rely on non-invasive Win32 APIs, ToolHelp, and user-mode Event Tracing for Windows (ETW).
- **Performance:** Sub-millisecond latency, non-blocking asynchronous event bus (`crossbeam-channel`), and real-time responsive terminal rendering.
- **Safety First:** Minimize `unsafe` blocks and document exact safety invariants.

---

## 2. Contributor License Agreement (CLA) & Intellectual Property Assignment

To protect the project, its users, and its long-term viability (including potential enterprise dual-licensing, institutional adoption, or corporate acquisition/M&A), all contributions are governed by this Contributor Agreement.

### 2.1 Copyright Assignment & Grant of Rights
By submitting a Pull Request, patch, issue code, or any other content to this repository, you ("Contributor") explicitly agree to the following terms:

1. **Original Work:** You certify that the contribution is your original creation and that you have the full legal right to grant the rights described herein.
2. **Grant of Rights:** You grant the project maintainer (**gilnett**) a perpetual, worldwide, irrevocable, royalty-free, transferable, and sublicensable right and license to use, reproduce, adapt, modify, perform, display, publish, sublicense, relicense (including under commercial, dual, or proprietary licenses), distribute, and commercialize your contributions in any medium.
3. **Single Ownership Integrity:** You acknowledge that this agreement ensures single-party governance over the project's intellectual property, preventing fragmented copyright ownership that would otherwise impede corporate restructuring, commercial dual-licensing, or technology transfers.

---

## 3. Privacy & GDPR (RGPD) Compliance Guidelines

Contributions must strictly preserve our **Privacy by Design** architecture:
- **Zero Telemetry / Air-Gapped by Design:** `wstrace` does not collect, transmit, or phone home any data over the network. No outbound network requests are permitted.
- **Local Data Isolation:** All telemetry and trace logs remain strictly on the user's local machine.
- **Sensitive Data Handling (PII):** Do not log raw personal identifiable information (PII) beyond what is strictly necessary for system diagnostics.

---

## 4. Security & OWASP Standards

Every pull request is subject to automated security analysis:
- **Zero Hardcoded Secrets (OWASP Rule #1):** Never commit API keys, tokens, credentials, or private certificates.
- **Dependency Hygiene:** Third-party crates must be audited against known vulnerability databases (`cargo audit`).
- **Memory Safety:** Leverage Rust's borrow checker to eliminate memory corruption vulnerabilities (buffer overflows, use-after-free).

---

## 5. Submitting Pull Requests & Verification Requirements

### 5.1 Pre-Submission Local Audit
Before submitting a PR, your branch must satisfy all local verification criteria:
1. **Pass Comprehensive Project Audit**:
   Execute the full automated quality, security, and packaging test suite:
   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/audit.ps1
   ```
   All 13 checks must pass (Rust formatting, Clippy zero-warning policy, unit tests, Scoop/Winget manifest synchronization, and PowerShell script analysis).
2. **Git Commit Standards**:
   - Keep all Git commit titles under 72 characters (hard limit 80 characters).
   - Use standard conventional prefixes (`feat:`, `fix:`, `ci:`, `docs:`, `refactor:`).
   - Maintain a solo developer tone ("I", never "we") in commit bodies and messages.

### 5.2 Automated AI Code Review Workflow (Google Gemini)
- **Initial Automated Review**: An AI review (#1/3) is triggered automatically upon opening the Pull Request.
- **On-Demand Re-Reviews**: After pushing updates or addressing comments, request a new review by posting a PR comment containing:
  ```text
  @gemini review
  ```
- **Review Quota**: Each Pull Request is allocated a maximum of **3 automated AI reviews** (#1/3, #2/3, #3/3) to conserve API resources.
- **Review Resolution**: Contributors are expected to address or discuss any critical observations (e.g. memory leaks, Win32 edge cases, packaging checksum mismatches) identified by the AI review before merging.
