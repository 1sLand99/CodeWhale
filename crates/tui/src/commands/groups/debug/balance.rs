//! Balance: query the active provider's remaining prepaid credit.

use crate::tui::app::AppAction;
use codewhale_command_contract::facets::CommandDebugDiagnosticsContext;
use codewhale_command_contract::handler::CommandContexts;

use super::CommandResult;

/// Query provider account balance / credits.
pub fn balance(contexts: CommandContexts<'_>) -> CommandResult {
    let mut parts = contexts.into_parts();
    let Some(diagnostics) = parts.debug_diagnostics.as_deref_mut() else {
        return CommandResult::error("Command capability unavailable: debug_diagnostics");
    };
    balance_portable(diagnostics)
}

fn balance_portable(diagnostics: &mut dyn CommandDebugDiagnosticsContext) -> CommandResult {
    let projection = diagnostics.balance_projection();
    if !projection.supports_balance_api {
        return CommandResult::message(format!(
            "Balance check is not supported for {} yet. Check the provider dashboard for account balance details.",
            projection.provider_display_name
        ));
    }
    CommandResult::action(AppAction::FetchBalance)
}
