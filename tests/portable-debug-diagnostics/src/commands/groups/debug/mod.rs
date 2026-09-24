//! Real portable debug source closure; no copy of command logic.

use crate::commands::CommandResult;

#[path = "../../../../../../crates/tui/src/commands/groups/debug/balance.rs"]
pub mod balance;
#[path = "../../../../../../crates/tui/src/commands/groups/debug/cache.rs"]
pub mod cache;
#[path = "../../../../../../crates/tui/src/commands/groups/debug/cache_format.rs"]
pub mod cache_format;
#[path = "../../../../../../crates/tui/src/commands/groups/debug/preview_request.rs"]
pub mod preview_request;
#[path = "../../../../../../crates/tui/src/commands/groups/debug/tokens.rs"]
pub mod tokens;
#[path = "../../../../../../crates/tui/src/commands/groups/debug/tool_inspection.rs"]
pub mod tool_inspection;

/// Expose the real leaf-owned registrations, making all eight original
/// metadata/handler implementations live in both test and normal builds.
pub fn portable_handlers() -> [(
    &'static codewhale_command_contract::metadata::CommandInfo,
    codewhale_command_contract::handler::CommandHandler<CommandResult>,
); 8] {
    use codewhale_command_contract::metadata::RegisterCommand;
    [
        (tokens::TokensCmd::info(), tokens::TokensCmd::handler()),
        (tokens::CostCmd::info(), tokens::CostCmd::handler()),
        (balance::BalanceCmd::info(), balance::BalanceCmd::handler()),
        (cache::CacheCmd::info(), cache::CacheCmd::handler()),
        (
            preview_request::PreviewRequestCmd::info(),
            preview_request::PreviewRequestCmd::handler(),
        ),
        (
            tool_inspection::ToolsCmd::info(),
            tool_inspection::ToolsCmd::handler(),
        ),
        (tokens::SystemCmd::info(), tokens::SystemCmd::handler()),
        (tokens::ContextCmd::info(), tokens::ContextCmd::handler()),
    ]
}

#[cfg(test)]
#[path = "../../../../../../crates/tui/src/commands/groups/debug/portable_tests.rs"]
mod portable_tests;

#[cfg(test)]
mod closure_tests {
    use super::portable_handlers;
    use codewhale_command_contract::handler::{CommandCapabilities as Caps, CommandHandler};

    #[test]
    fn independent_closure_includes_every_leaf_registration() {
        let handlers = portable_handlers();
        assert_eq!(
            handlers
                .iter()
                .map(|(info, _)| info.name)
                .collect::<Vec<_>>(),
            vec![
                "tokens",
                "cost",
                "balance",
                "cache",
                "preview-request",
                "tools",
                "system",
                "context",
            ]
        );
        for (info, handler) in handlers {
            match (info.name, handler) {
                ("preview-request", CommandHandler::Pure(pure)) => {
                    assert_eq!(info.aliases, &["dryrun", "preview_request"]);
                    assert!(pure(Some("json")).action.is_some());
                }
                ("tokens" | "cost" | "cache", CommandHandler::Contextual { capabilities, .. }) => {
                    assert_eq!(capabilities, Caps::DEBUG_DIAGNOSTICS | Caps::PRESENTATION);
                }
                (_, CommandHandler::Contextual { capabilities, .. }) => {
                    assert_eq!(capabilities, Caps::DEBUG_DIAGNOSTICS);
                }
                _ => panic!("wrong handler for /{}", info.name),
            }
        }
    }
}
