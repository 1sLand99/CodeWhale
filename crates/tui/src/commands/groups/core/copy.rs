//! `/copy` command — copy the last completed assistant response.

use crate::commands::traits::{CommandInfo, RegisterCommand};
use crate::localization::MessageId;
use crate::tui::app::App;
use crate::tui::history::HistoryCell;

use super::CommandResult;

pub(in crate::commands) const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "copy",
    aliases: &[],
    usage: "/copy",
    description_id: MessageId::CmdCopyDescription,
};

pub(in crate::commands) struct CopyCmd;

impl RegisterCommand for CopyCmd {
    fn info() -> &'static CommandInfo {
        &COMMAND_INFO
    }

    fn execute(app: &mut App, _arg: Option<&str>) -> CommandResult {
        execute_copy(app)
    }
}

fn last_completed_assistant_output(app: &App) -> Option<&str> {
    app.history.iter().rev().find_map(|cell| match cell {
        HistoryCell::Assistant {
            content,
            streaming: false,
        } if !content.trim().is_empty() => Some(content.as_str()),
        _ => None,
    })
}

fn execute_copy(app: &mut App) -> CommandResult {
    let Some(content) = last_completed_assistant_output(app).map(str::to_owned) else {
        return CommandResult::message(app.tr(MessageId::CmdCopyNoOutput).into_owned());
    };

    match app.clipboard.write_text(&content) {
        Ok(()) => CommandResult::message(
            app.tr(MessageId::CmdCopySuccess)
                .replace("{lines}", &content.lines().count().max(1).to_string()),
        ),
        Err(error) => CommandResult::error(
            app.tr(MessageId::CmdCopyFailed)
                .replace("{error}", &error.to_string()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::TuiOptions;
    use crate::tui::clipboard::ClipboardHandler;
    use std::path::PathBuf;

    fn test_app() -> App {
        App::new(
            TuiOptions {
                model: "deepseek-v4-flash".to_string(),
                ..crate::test_support::test_tui_options(PathBuf::from("."))
            },
            &Config::default(),
        )
    }

    #[test]
    fn copies_the_latest_completed_assistant_output_only() {
        let mut app = test_app();
        app.clipboard = ClipboardHandler::for_test(true, false);
        app.history = vec![
            HistoryCell::Assistant {
                content: "older".to_string(),
                streaming: false,
            },
            HistoryCell::Tool(crate::tui::history::ToolCell::Generic(
                crate::tui::history::GenericToolCell {
                    name: "read_file".to_string(),
                    status: crate::tui::history::ToolStatus::Success,
                    input_summary: None,
                    output: Some("tool output".to_string()),
                    prompts: None,
                    spillover_path: None,
                    output_summary: None,
                    is_diff: false,
                },
            )),
            HistoryCell::Assistant {
                content: "latest **answer**\nsecond line".to_string(),
                streaming: false,
            },
            HistoryCell::Assistant {
                content: "partial".to_string(),
                streaming: true,
            },
        ];

        let result = execute_copy(&mut app);

        assert_eq!(
            result.message.as_deref(),
            Some("Copied the last completed assistant response to the clipboard (2 lines)")
        );
        assert_eq!(
            app.clipboard.last_written_text(),
            Some("latest **answer**\nsecond line")
        );
    }

    #[test]
    fn skips_empty_and_non_assistant_history() {
        let mut app = test_app();
        app.history = vec![
            HistoryCell::Assistant {
                content: "  \n".to_string(),
                streaming: false,
            },
            HistoryCell::System {
                content: "system".to_string(),
            },
        ];

        let result = execute_copy(&mut app);

        assert_eq!(
            result.message.as_deref(),
            Some("No completed assistant response is available to copy")
        );
    }

    #[test]
    fn active_turn_does_not_change_which_completed_output_is_copied() {
        let mut app = test_app();
        app.clipboard = ClipboardHandler::for_test(true, false);
        app.history = vec![HistoryCell::Assistant {
            content: "completed before the active turn".to_string(),
            streaming: false,
        }];
        app.is_loading = true;

        let result = execute_copy(&mut app);

        assert!(!result.is_error);
        assert_eq!(
            app.clipboard.last_written_text(),
            Some("completed before the active turn")
        );
        assert!(app.is_loading);
    }
}
