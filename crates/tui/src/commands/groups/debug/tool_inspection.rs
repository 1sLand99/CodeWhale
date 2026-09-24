use crate::commands::CommandResult;
use crate::commands::portable_reports::{render_tool_snapshot_json, render_tool_snapshot_text};
use crate::tui::app::AppAction;
use codewhale_command_contract::handler::CommandContexts;

pub(super) fn tools(contexts: CommandContexts<'_>, arg: Option<&str>) -> CommandResult {
    let mut parts = contexts.into_parts();
    let Some(diagnostics) = parts.debug_diagnostics.as_deref_mut() else {
        return CommandResult::error("Command capability unavailable: debug_diagnostics");
    };
    // The original availability check precedes format validation. No prior
    // request differs from an observed empty prepared catalog.
    let Some(snapshot) = diagnostics.tool_snapshot() else {
        return CommandResult::message(
            "Tool request snapshot unavailable — no model request has been captured for the latest turn.",
        );
    };
    match arg.unwrap_or("text").trim() {
        "" | "text" => CommandResult::action(AppAction::OpenTextPager {
            title: "Prepared Tool Request".to_string(),
            content: render_tool_snapshot_text(&snapshot),
        }),
        "json" => match render_tool_snapshot_json(&snapshot) {
            Ok(output) => CommandResult::action(AppAction::OpenTextPager {
                title: "Prepared Tool Request (JSON)".to_string(),
                content: output,
            }),
            Err(error) => CommandResult::error(format!(
                "tool request snapshot could not be serialized: {error}"
            )),
        },
        _ => CommandResult::error("usage: /tools [text|json]"),
    }
}
