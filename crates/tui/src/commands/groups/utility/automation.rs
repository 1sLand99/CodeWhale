//! Automation commands: list/show/pause/resume/delete/run
//!
//! Operator surface over the durable `AutomationManager` (the model-visible
//! `automation` tool stays the authoring path — creation requires approval
//! and is deliberately not duplicated here). Actions run in the async UI
//! loop where the shared manager lock can be awaited.

use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::tui::app::{App, AppAction};

use crate::commands::CommandResult;

pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "automation",
    aliases: &["automations", "scheduled"],
    usage: "/automation [list|show <id>|pause <id>|resume <id>|delete <id>|run <id>]",
    description_id: MessageId::CmdAutomationDescription,
};

pub(in crate::commands) struct AutomationCmd;

impl RegisterCommand for AutomationCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }

    fn execute(app: &mut App, arg: Option<&str>) -> CommandResult {
        automation(app, arg)
    }
}

fn automation(_app: &mut App, args: Option<&str>) -> CommandResult {
    let raw = args.unwrap_or("").trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("list") {
        return CommandResult::action(AppAction::AutomationList);
    }

    let mut parts = raw.splitn(2, char::is_whitespace);
    let action = parts.next().unwrap_or("").to_ascii_lowercase();
    let remainder = parts.next().map(str::trim).filter(|s| !s.is_empty());

    match action.as_str() {
        "list" => CommandResult::action(AppAction::AutomationList),
        "show" | "status" => {
            let Some(id) = remainder else {
                return CommandResult::error("Usage: /automation show <id>");
            };
            CommandResult::action(AppAction::AutomationShow { id: id.to_string() })
        }
        "pause" => {
            let Some(id) = remainder else {
                return CommandResult::error("Usage: /automation pause <id>");
            };
            CommandResult::action(AppAction::AutomationPause { id: id.to_string() })
        }
        "resume" => {
            let Some(id) = remainder else {
                return CommandResult::error("Usage: /automation resume <id>");
            };
            CommandResult::action(AppAction::AutomationResume { id: id.to_string() })
        }
        "delete" | "remove" | "rm" => {
            let Some(id) = remainder else {
                return CommandResult::error("Usage: /automation delete <id>");
            };
            CommandResult::action(AppAction::AutomationDelete { id: id.to_string() })
        }
        "run" | "trigger" => {
            let Some(id) = remainder else {
                return CommandResult::error("Usage: /automation run <id>");
            };
            CommandResult::action(AppAction::AutomationRun { id: id.to_string() })
        }
        _ => CommandResult::error("Usage: /automation [list|show <id>|pause <id>|resume <id>|delete <id>|run <id>]"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::TuiOptions;
    use std::path::PathBuf;

    fn app() -> App {
        App::new(
            TuiOptions {
                use_alt_screen: false,
                max_subagents: 2,
                ..crate::test_support::test_tui_options(PathBuf::from("."))
            },
            &Config::default(),
        )
    }

    #[test]
    fn parses_list_show_and_actions() {
        let mut app = app();
        assert!(matches!(
            automation(&mut app, None).action,
            Some(AppAction::AutomationList)
        ));
        assert!(matches!(
            automation(&mut app, Some("list")).action,
            Some(AppAction::AutomationList)
        ));
        assert!(matches!(
            automation(&mut app, Some("show auto_1")).action,
            Some(AppAction::AutomationShow { id }) if id == "auto_1"
        ));
        assert!(matches!(
            automation(&mut app, Some("pause auto_1")).action,
            Some(AppAction::AutomationPause { id }) if id == "auto_1"
        ));
        assert!(matches!(
            automation(&mut app, Some("resume auto_1")).action,
            Some(AppAction::AutomationResume { id }) if id == "auto_1"
        ));
        assert!(matches!(
            automation(&mut app, Some("delete auto_1")).action,
            Some(AppAction::AutomationDelete { id }) if id == "auto_1"
        ));
        assert!(matches!(
            automation(&mut app, Some("run auto_1")).action,
            Some(AppAction::AutomationRun { id }) if id == "auto_1"
        ));
    }

    #[test]
    fn validates_missing_ids() {
        let mut app = app();
        for action in ["show", "pause", "resume", "delete", "run"] {
            let result = automation(&mut app, Some(action));
            assert!(result.message.is_some(), "{action} needs an id");
            assert!(result.action.is_none());
        }
    }
}
