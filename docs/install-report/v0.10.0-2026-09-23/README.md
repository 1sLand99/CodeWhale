# Install test report: Codewhale v0.10.0 (2026-09-23)

This folder holds the evidence behind the rewritten [`docs/INSTALL.md`](../../INSTALL.md).
Every install path was run on a fresh Ubuntu 24.04 x86_64 VM, one new Linux
user per path, and the TUI was run inside Ghostty 1.3.1.

| File | Contents |
|---|---|
| [RECEIPTS.md](RECEIPTS.md) | For each path: commands, trimmed output, pass/fail, timings |
| [DOC_DEFECTS.md](DOC_DEFECTS.md) | Where the previous README/INSTALL.md was wrong, stale or missing a step |
| [BUGS.md](BUGS.md) | Product bugs, with repro steps (not filed as issues) |
| `screenshots/` | Ghostty TUI evidence (the one key hint is redacted) |
| `logs/` | Raw, redacted command logs |

This is review material. It can be dropped from the branch before merge, once
the findings are triaged.
