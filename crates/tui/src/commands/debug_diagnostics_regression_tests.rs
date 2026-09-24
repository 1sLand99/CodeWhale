//! Untouched-source parity samples for the diagnostics slice (FEAT-029).
//!
//! The checked-in fixture was captured on baseline 922679d6c. This test only
//! reads it; it never updates expected bytes. Known limit: the first sample set
//! covers stable branches; timestamped reports and seeded pricing/telemetry
//! still require separate captured cases before diagnostics handlers are moved.

use super::traits::{CommandDiscovery, CommandGroup};
use super::{CommandResult, execute};
use crate::config::Config;
use crate::tui::app::{App, AppAction};
use codewhale_models::{SystemBlock, SystemPrompt};
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
    for (label, command) in [
        ("balance_unsupported", "/balance"),
        ("tokens_reported_telemetry", "/tokens"),
        ("cost_priced_zero", "/cost"),
        ("cost_legacy_unknown", "/cost"),
        ("system_blocks", "/system"),
    ] {
        let workspace = tempfile::tempdir().expect("owned fixture workspace");
        let mut app = App::new(
            crate::test_support::test_tui_options(workspace.path()),
            &Config::default(),
        );
        app.ui_locale = codewhale_localization::Locale::En;
        app.api_provider = crate::config::ApiProvider::Deepseek;
        match label {
            "balance_unsupported" => app.api_provider = crate::config::ApiProvider::OpenaiCodex,
            "tokens_reported_telemetry" => {
                app.session.total_tokens = 1234;
                app.session.last_prompt_tokens = Some(100);
                app.session.last_completion_tokens = Some(25);
                app.session.last_prompt_cache_hit_tokens = Some(70);
                app.session.last_prompt_cache_miss_tokens = Some(30);
                app.session.total_cache_write_tokens = 250;
            }
            "cost_priced_zero" => app.session.cost_priced_turns = 1,
            "cost_legacy_unknown" => app.session.cost_coverage_unknown_legacy = true,
            "system_blocks" => {
                app.system_prompt = Some(SystemPrompt::Blocks(vec![
                    SystemBlock {
                        block_type: "text".into(),
                        text: "First block".into(),
                        cache_control: None,
                    },
                    SystemBlock {
                        block_type: "text".into(),
                        text: "Second block".into(),
                        cache_control: None,
                    },
                ]));
            }
            _ => unreachable!("fixture case list is exhaustive"),
        }
        samples.insert(label.into(), outcome(execute(command, &mut app)));
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

#[test]
fn registration_metadata_matches_untouched_baseline() {
    use codewhale_localization::MessageId;

    let commands = super::groups::debug::DebugCommands.commands();
    let names: Vec<_> = commands.iter().map(|command| command.info().name).collect();
    assert_eq!(
        names,
        [
            "tokens",
            "cost",
            "balance",
            "cache",
            "preview-request",
            "tools",
            "change",
            "system",
            "context",
            "edit",
            "diff",
            "undo",
            "retry",
        ],
        "diagnostics must keep their original positions among mutation commands"
    );

    let expected = [
        (
            0,
            "tokens",
            &[][..],
            "/tokens",
            MessageId::CmdTokensDescription,
            CommandDiscovery::Primary,
        ),
        (
            1,
            "cost",
            &[][..],
            "/cost",
            MessageId::CmdCostDescription,
            CommandDiscovery::Primary,
        ),
        (
            2,
            "balance",
            &[][..],
            "/balance",
            MessageId::CmdBalanceDescription,
            CommandDiscovery::Advanced,
        ),
        (
            3,
            "cache",
            &[][..],
            "/cache [count|inspect|stats|zones|warmup]",
            MessageId::CmdCacheDescription,
            CommandDiscovery::Advanced,
        ),
        (
            4,
            "preview-request",
            &["dryrun", "preview_request"][..],
            "/preview-request [json] [--prompt <text>]",
            MessageId::CmdPreviewRequestDescription,
            CommandDiscovery::Advanced,
        ),
        (
            5,
            "tools",
            &["tool-studio"][..],
            "/tools [text|json]",
            MessageId::CmdToolsDescription,
            CommandDiscovery::Advanced,
        ),
        (
            7,
            "system",
            &["xitong"][..],
            "/system",
            MessageId::CmdSystemDescription,
            CommandDiscovery::Advanced,
        ),
        (
            8,
            "context",
            &["ctx"][..],
            "/context [report|json|prompt-json|summary]",
            MessageId::CmdContextDescription,
            CommandDiscovery::Advanced,
        ),
    ];
    for (index, name, aliases, usage, description_id, discovery) in expected {
        let info = commands[index].info();
        assert_eq!(info.name, name);
        assert_eq!(info.aliases, aliases, "aliases for {name}");
        assert_eq!(info.usage, usage, "usage for {name}");
        assert_eq!(
            info.description_id, description_id,
            "description for {name}"
        );
        assert_eq!(info.discovery(), discovery, "discovery for {name}");
    }
}
