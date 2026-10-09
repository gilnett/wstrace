# Security Policy

The `wstrace` project takes security and system integrity very seriously. Because `wstrace` performs low-level syscall tracing, Win32 debugging, and Windows Kernel ETW parsing, maintaining a robust security posture is our highest priority.

---

## Supported Versions

Only the latest release receives security patches and updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

---

## Reporting a Vulnerability

If you discover a security vulnerability or sensitive flaw within `wstrace`:

**Please DO NOT open a public GitHub Issue.** Public disclosure puts users and systems at risk before a fix is available.

### How to Report Privately:
1. **GitHub Private Vulnerability Reporting (Recommended)**:
   Navigate to the [Security Advisories tab](https://github.com/gilnett/wstrace/security/advisories) of this repository and click **"Report a vulnerability"**.
2. **Direct Contact**:
   If private advisories are unavailable, please send an encrypted or direct email to the project maintainer via GitHub profile contact (`gilnett`).

### What to Include in Your Report:
- A clear description of the vulnerability and its potential impact.
- Step-by-step instructions to reproduce the issue (proof of concept, CLI flags, target binary).
- Operating system version and architecture (e.g., Windows 11 23H2 x86_64 / ARM64).
- Any proposed mitigations or patches, if available.

---

## Our Security Commitments

- **Acknowledgment**: We aim to acknowledge receipt of all vulnerability reports within **48 hours**.
- **Assessment**: We will validate and classify the issue according to CVSS standards within **7 business days**.
- **Coordinated Disclosure**: We adhere to coordinated vulnerability disclosure. We will work with the reporter to verify the fix and draft a security advisory prior to publishing the release.
- **Credit**: Security researchers who responsibly report vulnerabilities will be acknowledged in release notes (unless anonymity is requested).

---

## Core Security Tenets of `wstrace`

- **Privacy by Design (Air-Gapped)**: `wstrace` never initiates telemetry, tracking, or outbound network calls. Traces remain exclusively on the user's local machine.
- **Memory Safety**: Built 100% in Rust with `#![deny(unsafe_op_in_unsafe_fn)]` and strict boundary validation when interacting with Win32 APIs and kernel buffers.
- **Least Privilege**: Non-kernel userland probing operates under standard user privileges without demanding administrator rights unless ETW kernel providers are explicitly enabled.
