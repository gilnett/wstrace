# Contributing to wstrace

Thank you for your interest in contributing to `wstrace`. To ensure high standards of software quality, security, and institutional-grade legal compliance, please review the following guidelines before submitting any contribution.

---

## 1. Code of Conduct & Development Principles

`wstrace` is designed with strict production requirements:
- **Language:** Idiomatic Rust (2021 Edition).
- **Zero Drivers / Zero Kernel Bloat:** All instrumentation must strictly rely on non-invasive Win32 APIs, ToolHelp, and user-mode Event Tracing for Windows (ETW).
- **Performance:** Sub-millisecond latency, non-blocking asynchronous event bus (`crossbeam-channel`), and 60 FPS responsive terminal rendering.
- **Safety First:** Minimize `unsafe` blocks and document exact safety invariants.

---

## 2. Contributor License Agreement (CLA) & Intellectual Property Assignment

To protect the project, its users, and its long-term viability (including potential enterprise dual-licensing, institutional adoption, or corporate acquisition/M&A), all contributions are governed by this Contributor Agreement.

### 2.1 Copyright Assignment & Grant of Rights
By submitting a Pull Request, patch, issue code, or any other content to this repository, you ("Contributor") explicitly agree to the following terms:

1. **Original Work:** You certify that the contribution is your original creation and that you have the full legal right to grant the rights described herein.
2. **Grant of Rights:** You grant the project maintainer (**Gilles**) a perpetual, worldwide, irrevocable, royalty-free, transferable, and sublicensable right and license to use, reproduce, adapt, modify, perform, display, publish, sublicense, relicense (including under commercial, dual, or proprietary licenses), distribute, and commercialize your contributions in any medium.
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

## 5. Submitting Pull Requests

1. Fork the repository and create your branch from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
2. Ensure all unit and performance benchmarks pass:
   ```bash
   cargo test
   ```
3. Ensure formatting and clippy checks are clean:
   ```bash
   cargo clippy --all-targets --all-features
   cargo fmt --check
   ```
4. Submit the Pull Request with a clear description of the rationale and changes.
