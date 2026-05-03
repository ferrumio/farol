# Security Policy

## Supported versions

farol is pre-alpha. Only the latest release on `main` is supported. Older tags receive no security fixes.

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security problems.

Instead, use GitHub's [private vulnerability reporting](https://github.com/ferrumio/farol/security/advisories/new) on this repository. That routes the report directly to the maintainers.

Include, when possible:

- A clear description of the issue and its impact.
- Steps to reproduce.
- The version or commit affected.
- Any suggested fix or mitigation.

We will acknowledge the report within 72 hours and aim to publish a fix or mitigation within 30 days for confirmed issues. Critical issues (remote code execution, credential leaks) are prioritized.

## Scope

In scope:

- The `farol` Rust crates, Python bindings, and default theme.
- Build-time plugin loading and execution.
- Generated site output.

Out of scope:

- Third-party plugins distributed by other authors (report to them directly).
- Vulnerabilities in dependencies that are already tracked upstream.
