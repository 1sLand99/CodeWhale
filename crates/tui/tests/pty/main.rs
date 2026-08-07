//! Consolidated harness for real-PTY scenarios.
//!
//! Every test here boots the real `codewhale-tui` binary inside a
//! `portable-pty` session and drives it through `support/qa_harness`. They are
//! `#[cfg(unix)]` and contend for the terminal, so they serialize on a
//! per-harness mutex already. One binary links `rio-vt` + `portable-pty` once
//! instead of four times. See `crates/tui/tests/README.md`.

mod qa_pty;
mod release_runtime_qa;
mod terminal_matrix_qa;
mod work_bar_subagents_pty;
