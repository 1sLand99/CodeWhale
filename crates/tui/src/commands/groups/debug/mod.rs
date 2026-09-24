//! Debug command area: token/cost introspection, cache tooling, undo/retry,
//! and the change log.

mod balance;
mod cache;
pub(in crate::commands) mod cache_format;
mod change;
mod preview_request;
mod receipts;
pub(in crate::commands) mod tokens;
mod tool_inspection;
mod undo;

#[cfg(test)]
mod portable_tests;
#[cfg(test)]
pub(in crate::commands) mod tests;

use crate::commands::CommandResult;
use crate::commands::traits::{
    Command, CommandGroup, CommandInfo, ContextualCommand, FunctionCommand,
};
use crate::tui::app::App;
use codewhale_localization::MessageId;

pub struct DebugCommands;

impl CommandGroup for DebugCommands {
    fn commands(&self) -> &'static [Box<dyn Command>] {
        cached_command_list!(vec![
            Box::new(
                ContextualCommand::from_contract::<tokens::TokensCmd>()
                    .expect("tokens registration")
            ),
            Box::new(
                ContextualCommand::from_contract::<tokens::CostCmd>().expect("cost registration")
            ),
            Box::new(FunctionCommand::new(&RECEIPTS_INFO, run_receipts)),
            Box::new(
                ContextualCommand::from_contract::<balance::BalanceCmd>()
                    .expect("balance registration")
            ),
            Box::new(
                ContextualCommand::from_contract::<cache::CacheCmd>().expect("cache registration")
            ),
            Box::new(
                ContextualCommand::from_contract::<preview_request::PreviewRequestCmd>()
                    .expect("preview registration")
            ),
            Box::new(
                ContextualCommand::from_contract::<tool_inspection::ToolsCmd>()
                    .expect("tools registration")
            ),
            Box::new(FunctionCommand::new(&CHANGE_INFO, run_change)),
            Box::new(
                ContextualCommand::from_contract::<tokens::SystemCmd>()
                    .expect("system registration")
            ),
            Box::new(
                ContextualCommand::from_contract::<tokens::ContextCmd>()
                    .expect("context registration")
            ),
            Box::new(FunctionCommand::new(&EDIT_INFO, run_edit)),
            Box::new(FunctionCommand::new(&DIFF_INFO, run_diff)),
            Box::new(FunctionCommand::new(&UNDO_INFO, run_undo)),
            Box::new(FunctionCommand::new(&RETRY_INFO, run_retry)),
        ])
    }
}

static RECEIPTS_INFO: CommandInfo = CommandInfo {
    name: "receipts",
    aliases: &["receipt"],
    usage: "/receipts [json] [<turn>]",
    description_id: MessageId::CmdReceiptsDescription,
};
static CHANGE_INFO: CommandInfo = CommandInfo {
    name: "change",
    aliases: &[],
    usage: "/change [version]",
    description_id: MessageId::CmdChangeDescription,
};
static EDIT_INFO: CommandInfo = CommandInfo {
    name: "edit",
    aliases: &[],
    usage: "/edit",
    description_id: MessageId::CmdEditDescription,
};
static DIFF_INFO: CommandInfo = CommandInfo {
    name: "diff",
    aliases: &[],
    usage: "/diff",
    description_id: MessageId::CmdDiffDescription,
};
static UNDO_INFO: CommandInfo = CommandInfo {
    name: "undo",
    aliases: &[],
    usage: "/undo",
    description_id: MessageId::CmdUndoDescription,
};
static RETRY_INFO: CommandInfo = CommandInfo {
    name: "retry",
    aliases: &["chongshi"],
    usage: "/retry",
    description_id: MessageId::CmdRetryDescription,
};

fn run_registered(app: &mut App, name: &str, arg: Option<&str>) -> CommandResult {
    dispatch(app, name, arg).expect("registered debug command should dispatch")
}

fn run_receipts(app: &mut App, arg: Option<&str>) -> CommandResult {
    run_registered(app, "receipts", arg)
}
fn run_change(app: &mut App, arg: Option<&str>) -> CommandResult {
    run_registered(app, "change", arg)
}
fn run_edit(app: &mut App, arg: Option<&str>) -> CommandResult {
    run_registered(app, "edit", arg)
}
fn run_diff(app: &mut App, arg: Option<&str>) -> CommandResult {
    run_registered(app, "diff", arg)
}
fn run_undo(app: &mut App, arg: Option<&str>) -> CommandResult {
    run_registered(app, "undo", arg)
}
fn run_retry(app: &mut App, arg: Option<&str>) -> CommandResult {
    run_registered(app, "retry", arg)
}

pub(in crate::commands) fn dispatch(
    app: &mut App,
    command: &str,
    arg: Option<&str>,
) -> Option<CommandResult> {
    let result = match command {
        "receipts" | "receipt" => receipts::receipts(app, arg),
        "change" => change::change(app, arg),
        "edit" => undo::edit(app),
        "diff" => undo::diff(app),
        "undo" => {
            // Try surgical patch-undo first; fall back to conversation undo
            // when there is no snapshot-level change to revert. Never fall
            // through when the trusted-mode gate refused the file rollback —
            // that would silently crop the conversation instead of telling
            // the user to switch modes.
            let result = undo::patch_undo(app);
            // `CommandResult::error` decorates its text with an `Error: `
            // prefix, so the repo-unavailable arm below never matched and the
            // fallback it names was dead: `/undo` dumped the raw gate error
            // and left the conversation untouched. Match the undecorated text.
            let declined = result
                .message
                .as_deref()
                .map(|m| m.strip_prefix("Error: ").unwrap_or(m));
            if declined.is_none_or(|m| {
                m.starts_with("No snapshots found")
                    || m.starts_with("No older tool or pre-turn")
                    || m.starts_with("No undoable snapshot")
                    || m.starts_with(undo::SNAPSHOT_REPO_UNAVAILABLE_PREFIX)
            }) {
                // "Nothing to revert" is already honest; an unavailable repo
                // is not — the conversation-only result must say the files
                // were left alone, and why.
                let unavailable = declined
                    .filter(|m| m.starts_with(undo::SNAPSHOT_REPO_UNAVAILABLE_PREFIX))
                    .map(str::to_owned);
                let mut fallback = undo::undo_conversation(app);
                if let Some(reason) = unavailable {
                    let note = format!("{}\n{reason}", undo::FILES_NOT_REVERTED_NOTE);
                    fallback.message = Some(match fallback.message.take() {
                        Some(message) => format!("{message}\n{note}"),
                        None => note,
                    });
                }
                fallback
            } else {
                result
            }
        }
        "retry" | "chongshi" => undo::retry(app),
        _ => return None,
    };
    Some(result)
}
