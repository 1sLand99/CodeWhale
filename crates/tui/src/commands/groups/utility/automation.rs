//! Operator controls for durable scheduled automations.

use crate::commands::CommandResult;
use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::tui::app::{App, AppAction, AutomationAction};

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

    fn execute(_app: &mut App, arg: Option<&str>) -> CommandResult {
        automation(arg)
    }
}

fn automation(args: Option<&str>) -> CommandResult {
    let raw = args.unwrap_or("").trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("list") {
        return action(AutomationAction::List);
    }

    let mut parts = raw.splitn(2, char::is_whitespace);
    let verb = parts.next().unwrap_or("").to_ascii_lowercase();
    let id = parts.next().map(str::trim).filter(|id| !id.is_empty());

    let (make_action, usage): (fn(String) -> AutomationAction, &str) = match verb.as_str() {
        "list" => return action(AutomationAction::List),
        "show" | "status" => (AutomationAction::Show, "/automation show <id>"),
        "pause" => (AutomationAction::Pause, "/automation pause <id>"),
        "resume" => (AutomationAction::Resume, "/automation resume <id>"),
        "delete" | "remove" | "rm" => (AutomationAction::Delete, "/automation delete <id>"),
        "run" | "trigger" => (AutomationAction::Run, "/automation run <id>"),
        _ => return CommandResult::error(format!("Usage: {}", COMMAND_INFO.usage)),
    };

    let Some(id) = id else {
        return CommandResult::error(format!("Usage: {usage}"));
    };
    action(make_action(id.to_string()))
}

fn action(action: AutomationAction) -> CommandResult {
    CommandResult::action(AppAction::Automation(action))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(args: Option<&str>) -> Option<AutomationAction> {
        match automation(args).action {
            Some(AppAction::Automation(action)) => Some(action),
            _ => None,
        }
    }

    #[test]
    fn parses_list_show_and_mutations() {
        assert_eq!(parsed(None), Some(AutomationAction::List));
        assert_eq!(parsed(Some("list")), Some(AutomationAction::List));
        assert_eq!(
            parsed(Some("show auto_1")),
            Some(AutomationAction::Show("auto_1".to_string()))
        );
        assert_eq!(
            parsed(Some("pause auto_1")),
            Some(AutomationAction::Pause("auto_1".to_string()))
        );
        assert_eq!(
            parsed(Some("resume auto_1")),
            Some(AutomationAction::Resume("auto_1".to_string()))
        );
        assert_eq!(
            parsed(Some("delete auto_1")),
            Some(AutomationAction::Delete("auto_1".to_string()))
        );
        assert_eq!(
            parsed(Some("run auto_1")),
            Some(AutomationAction::Run("auto_1".to_string()))
        );
    }

    #[test]
    fn accepts_operator_aliases() {
        assert_eq!(
            parsed(Some("status auto_1")),
            Some(AutomationAction::Show("auto_1".to_string()))
        );
        assert_eq!(
            parsed(Some("rm auto_1")),
            Some(AutomationAction::Delete("auto_1".to_string()))
        );
        assert_eq!(
            parsed(Some("trigger auto_1")),
            Some(AutomationAction::Run("auto_1".to_string()))
        );
    }

    #[test]
    fn validates_missing_ids_and_unknown_actions() {
        for verb in ["show", "pause", "resume", "delete", "run", "unknown"] {
            let result = automation(Some(verb));
            assert!(result.message.is_some(), "{verb} should show usage");
            assert!(result.action.is_none());
        }
    }
}
