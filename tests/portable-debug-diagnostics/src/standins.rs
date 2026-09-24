//! Faithful, data-only D8 transport stand-ins; no executable host helpers.
//! Keep discriminants and error prefix aligned with the TUI's AppAction and
//! CommandResult until FEAT-037 gives these shapes a shared home.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppAction {
    FetchBalance,
    CacheWarmup,
    PreviewOutboundRequest {
        json: bool,
        base_prompt_only: bool,
        hypothetical_prompt: Option<String>,
    },
    OpenTextPager {
        title: String,
        content: String,
    },
    OpenContextInspector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandResult {
    pub message: Option<String>,
    pub action: Option<AppAction>,
    pub is_error: bool,
}

impl CommandResult {
    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            message: Some(msg.into()),
            action: None,
            is_error: false,
        }
    }

    pub fn action(action: AppAction) -> Self {
        Self {
            message: None,
            action: Some(action),
            is_error: false,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            message: Some(format!("Error: {}", msg.into())),
            action: None,
            is_error: true,
        }
    }
}
