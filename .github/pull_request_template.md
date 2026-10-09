## Description of Changes

Please provide a clear and concise summary of the changes made in this Pull Request.

---

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Performance optimization (latency or memory footprint reduction)
- [ ] Documentation update
- [ ] Refactoring / Code cleanup

---

## Testing & Verification

Describe the tests executed to verify your changes:
- Operating system tested on:
- Tested command line:
- [ ] `cargo test` passes with 0 failures
- [ ] `cargo check --release` passes with 0 warnings
- [ ] No regressions observed in interactive TUI or headless stream mode

---

## AI Assistance & Tooling Disclosure

Please disclose any AI assistance used during the conception, writing, or refactoring of this PR:
- [ ] **No AI used**: 100% human-authored code.
- [ ] **Public / Cloud AI used** (e.g., Claude, Gemini, ChatGPT, GitHub Copilot):
  - *Model / Service details*: 
- [ ] **Local / Self-hosted AI used** (running locally via Ollama, LM Studio, llama.cpp, etc.):
  - *Exact model name & parameters / version* (e.g., DeepSeek-Coder-V2, Qwen-2.5-Coder-32B, Llama-3.3-70B): 

> **Important**: If AI tooling was utilized, you certify that you have thoroughly audited, tested, and understood every line generated, verifying there are no hallucinated APIs, security vulnerabilities, or copyright infringements.

---

## Contributor Agreement, Enterprise Compliance & Security Checklist

By submitting this pull request, I confirm that:
- [ ] **Contributor License Agreement (CLA)**: I have read and agree to the CLA in [CONTRIBUTING.md](CONTRIBUTING.md), certifying original authorship and granting full project governance rights.
- [ ] **Zero Cloud Leaks (Air-Gapped & Sovereign)**: No network telemetry, third-party tracking, or remote network calls/sockets have been introduced (SOC 2, ISO 27001, GDPR & CCPA).
- [ ] **Zero Kernel Drivers (Zero Ring-0 Footprint)**: No third-party `.sys` kernel drivers, ring-0 hooks, or unstable driver routines are introduced; all logic strictly adheres to userland Win32 and native OS ETW.
- [ ] **Local Access Control & Least Privilege**: Changes respect Windows security boundaries, user session isolation, and Mandatory Integrity Control (MIC) without unverified privilege escalation.
- [ ] **OWASP Rule #1**: Zero hardcoded credentials, secrets, private keys, or API tokens are present in this code.
- [ ] **Licensing & Dependencies**: Any new dependencies are strictly permissively licensed (MIT or Apache-2.0) and documented for SBOM compliance.
