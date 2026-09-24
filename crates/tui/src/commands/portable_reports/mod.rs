//! Portable diagnostics formatting, shared by commands and non-command callers.
//! Request construction and classification stay with their existing TUI owners.

mod tool_snapshot;

pub use tool_snapshot::{render_tool_snapshot_json, render_tool_snapshot_text};
