//! Untouched-source parity samples for the diagnostics slice (FEAT-029).
//!
//! The checked-in fixture was captured on baseline 922679d6c. This test only
//! reads it; it never updates expected bytes. Known limit: the first sample set
//! covers stable branches; timestamped reports and seeded pricing/telemetry
//! still require separate captured cases before diagnostics handlers are moved.

use super::{CommandResult, execute};
use crate::config::Config;
use crate::tui::app::{App, AppAction};
use codewhale_models::SystemPrompt;
use serde_json::{Value, json};

fn outcome(result: CommandResult) -> Value {
    let action = match result.action {
        None => Value::Null,
        Some(AppAction::FetchBalance) => json!({"type": "FetchBalance"}),
        Some(AppAction::CacheWarmup) => json!({"type": "CacheWarmup"}),
        Some(AppAction::OpenContextInspector) => json!({"type": "OpenContextInspector"}),
        Some(AppAction::OpenTextPager { title, content }) => {
            json!({"type": "OpenTextPager", "title": title, "content": content})
        }
        Some(AppAction::PreviewOutboundRequest {
            json,
            base_prompt_only,
            hypothetical_prompt,
        }) => json!({"type": "PreviewOutboundRequest", "json": json,
            "base_prompt_only": base_prompt_only, "hypothetical_prompt": hypothetical_prompt}),
        Some(other) => panic!("unexpected diagnostics action: {other:?}"),
    };
    json!({"message": result.message, "action": action, "is_error": result.is_error})
}

fn collect() -> Value {
    let cases = [
        "/balance",
        "/tokens",
        "/cost",
        "/cache",
        "/cache 0",
        "/cache stats",
        "/cache zones",
        "/cache inspect --verbose --json",
        "/cache warmup",
        "/cache nonsense",
        "/preview-request",
        "/dryrun json",
        "/preview_request --base-prompt",
        "/preview-request --json --prompt  keep  trailing   ",
        "/preview-request --base-prompt --json",
        "/preview-request --prompt",
        "/tools",
        "/tool-studio invalid",
        "/system",
        "/xitong",
        "/context",
        "/ctx nonsense",
    ];
    let mut samples = serde_json::Map::new();
    for command in cases {
        // App::new already protects its own settings transaction. Never wrap it
        // in with_test_state_io_lock: that mutex is non-reentrant.
        let workspace = tempfile::tempdir().expect("owned fixture workspace");
        let mut app = App::new(
            crate::test_support::test_tui_options(workspace.path()),
            &Config::default(),
        );
        app.ui_locale = codewhale_localization::Locale::En;
        app.api_provider = crate::config::ApiProvider::Deepseek;
        samples.insert(command.to_string(), outcome(execute(command, &mut app)));
    }
    for (label, prompt) in [
        ("text", SystemPrompt::Text("Example policy".into())),
        (
            "utf8_boundary",
            SystemPrompt::Text(format!("{}éEND", "a".repeat(499))),
        ),
    ] {
        let workspace = tempfile::tempdir().expect("owned fixture workspace");
        let mut app = App::new(
            crate::test_support::test_tui_options(workspace.path()),
            &Config::default(),
        );
        app.ui_locale = codewhale_localization::Locale::En;
        app.system_prompt = Some(prompt);
        samples.insert(
            format!("/system:{label}"),
            outcome(execute("/system", &mut app)),
        );
    }
    Value::Object(samples)
}

#[test]
fn stable_branches_match_untouched_baseline() {
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/diagnostics/stable-branches.json"))
            .expect("reviewed baseline fixture must parse");
    let actual = collect();
    assert_eq!(actual, expected, "diagnostics parity diverged");
}
