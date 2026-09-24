//! Original pure renderers shared with TUI non-command consumers.

#[path = "../../../../../crates/tui/src/commands/portable_reports/context.rs"]
mod context;
#[path = "../../../../../crates/tui/src/commands/portable_reports/money.rs"]
mod money;
#[path = "../../../../../crates/tui/src/commands/portable_reports/tool_snapshot.rs"]
mod tool_snapshot;

pub use context::{
    context_report_json, format_context_report, format_context_summary, prompt_context_json,
};
pub use money::format_cost_amount_precise;
pub use tool_snapshot::{render_tool_snapshot_json, render_tool_snapshot_text};
