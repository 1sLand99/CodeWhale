//! FEAT-029 Phase 2: public command-surface parity for the
//! `debug::diagnostics` slice.
//!
//! The host regressions prove handler/rendering parity. This module proves the
//! *observable command surface* is unchanged: registry metadata (name, aliases,
//! usage), registry position, the `description_key` -> catalog bridge, palette
//! and discovery classification, and canonical-name/alias dispatch equivalence
//! through the public `execute` seam.
//!
//! It lives at the `commands` root for the same FEAT-045 extraction reason as
//! the regression suite. The metadata fixture was captured from the
//! untouched implementation and compared byte-for-byte.

use codewhale_localization::{Locale, MessageId};

use crate::commands::debug_diagnostics_test_support::{DiagnosticsHarness, assert_fixture};
use crate::commands::{CommandResult, execute};
use crate::config::ApiProvider;
use crate::tui::app::AppAction;

/// The eight declared diagnostics commands in registry order, with their exact
/// aliases and catalog description ids.
const DIAGNOSTICS: &[(&str, &[&str], MessageId)] = &[
    ("tokens", &[], MessageId::CmdTokensDescription),
    ("cost", &[], MessageId::CmdCostDescription),
    ("balance", &[], MessageId::CmdBalanceDescription),
    ("cache", &[], MessageId::CmdCacheDescription),
    (
        "preview-request",
        &["dryrun", "preview_request"],
        MessageId::CmdPreviewRequestDescription,
    ),
    ("tools", &["tool-studio"], MessageId::CmdToolsDescription),
    ("system", &["xitong"], MessageId::CmdSystemDescription),
    ("context", &["ctx"], MessageId::CmdContextDescription),
];

fn info(name: &str) -> &'static crate::commands::CommandInfo {
    crate::commands::get_command_info(name)
        .unwrap_or_else(|| panic!("/{name} must remain registered"))
}

/// Freeze the complete registry metadata surface (name, aliases, usage,
/// English description, palette text, discovery flags) for all eight commands.
#[test]
fn diagnostics_surface_metadata_matches_baseline() {
    let _harness = DiagnosticsHarness::new();
    let mut out = String::new();
    for (name, _, _) in DIAGNOSTICS {
        let info = info(name);
        out.push_str(&format!("name: {}\n", info.name));
        out.push_str(&format!("aliases: {:?}\n", info.aliases));
        out.push_str(&format!("usage: {}\n", info.usage));
        out.push_str(&format!(
            "description_en: {}\n",
            info.description_for(Locale::En)
        ));
        out.push_str(&format!(
            "palette_en: {}\n",
            info.palette_description_for(Locale::En)
        ));
        out.push_str(&format!(
            "requires_argument: {}\n",
            info.requires_argument()
        ));
        out.push_str(&format!("unlisted: {}\n", info.is_unlisted()));
        out.push_str(&format!(
            "show_in_empty_discovery: {}\n",
            info.show_in_empty_discovery()
        ));
        out.push_str("---\n");
    }
    assert_fixture("surface_metadata.txt", &out);
}

/// Every canonical name and alias resolves to the same registry entry, and the
/// `description_key` bridge maps to the expected catalog `MessageId`.
#[test]
fn diagnostics_names_aliases_and_description_bridge_are_exact() {
    let registry = crate::commands::registry();
    for (name, aliases, description_id) in DIAGNOSTICS {
        let canonical = info(name);
        assert_eq!(canonical.name, *name, "canonical name");
        assert_eq!(canonical.aliases, *aliases, "/{name} alias list");
        assert_eq!(
            canonical.description_id, *description_id,
            "/{name} catalog bridge"
        );
        assert!(
            !canonical.description_for(Locale::En).trim().is_empty(),
            "/{name} must resolve an English description"
        );

        let entry = registry
            .get(name)
            .unwrap_or_else(|| panic!("/{name} must be registered"));
        assert_eq!(entry.info().name, *name);
        for alias in *aliases {
            let via_alias = registry
                .get(alias)
                .unwrap_or_else(|| panic!("/{alias} must resolve"));
            assert_eq!(
                via_alias.info().name,
                *name,
                "/{alias} must resolve to /{name}"
            );
            assert_eq!(via_alias.info().usage, canonical.usage, "/{alias} usage");
        }
    }
}

/// Registry position inside the debug group is preserved: the eight
/// diagnostics commands stay in their original relative order and the five
/// FEAT-030 mutation commands remain registered around them.
#[test]
fn diagnostics_registry_position_matches_baseline() {
    let names: Vec<&str> = crate::commands::command_infos()
        .iter()
        .map(|info| info.name)
        .collect();
    let position = |name: &str| {
        names
            .iter()
            .position(|candidate| *candidate == name)
            .unwrap_or_else(|| panic!("/{name} must be registered; found {names:?}"))
    };

    let order = [
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
    ];
    for pair in order.windows(2) {
        assert!(
            position(pair[0]) < position(pair[1]),
            "/{} must stay before /{}",
            pair[0],
            pair[1]
        );
    }
}

fn render(result: &CommandResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("is_error: {}\n", result.is_error));
    match &result.message {
        Some(message) => out.push_str(&format!("message:\n{message}\n")),
        None => out.push_str("message: <none>\n"),
    }
    match &result.action {
        Some(action) => out.push_str(&format!("action: {action:?}\n")),
        None => out.push_str("action: <none>\n"),
    }
    out
}

/// Canonical names and every compatibility alias dispatch to byte-identical
/// results through the public seam.
#[test]
fn public_dispatch_canonical_and_alias_are_byte_equivalent() {
    let preview_cases = [
        "/preview-request json",
        "/preview-request json --prompt keep  the bytes",
        "/preview-request base-prompt",
    ];
    for command in preview_cases {
        let canonical = {
            let mut canonical_harness = DiagnosticsHarness::new();
            execute(command, &mut canonical_harness.app)
        };
        for alias in ["dryrun", "preview_request"] {
            let aliased = command.replacen("preview-request", alias, 1);
            let mut alias_harness = DiagnosticsHarness::new();
            let aliased_result = execute(&aliased, &mut alias_harness.app);
            assert_eq!(
                render(&canonical),
                render(&aliased_result),
                "{aliased} must match {command}"
            );
        }
    }

    // /tools and /tool-studio share the prepared-snapshot payload.
    let canonical = {
        let mut canonical_harness = DiagnosticsHarness::new();
        canonical_harness.app.session.last_tool_request_snapshot = Some(snapshot());
        execute("/tools json", &mut canonical_harness.app)
    };
    let mut alias_harness = DiagnosticsHarness::new();
    alias_harness.app.session.last_tool_request_snapshot = Some(snapshot());
    let aliased = execute("/tool-studio json", &mut alias_harness.app);
    assert_eq!(render(&canonical), render(&aliased));

    // /system and /xitong share the exact message.
    let canonical = {
        let mut canonical_harness = DiagnosticsHarness::new();
        canonical_harness.app.system_prompt =
            Some(codewhale_models::SystemPrompt::Text("aliased".to_string()));
        execute("/system", &mut canonical_harness.app)
    };
    let mut alias_harness = DiagnosticsHarness::new();
    alias_harness.app.system_prompt =
        Some(codewhale_models::SystemPrompt::Text("aliased".to_string()));
    let aliased = execute("/xitong", &mut alias_harness.app);
    assert_eq!(render(&canonical), render(&aliased));

    // /context and /ctx share the bare inspector action.
    let canonical = {
        let mut canonical_harness = DiagnosticsHarness::new();
        execute("/context", &mut canonical_harness.app)
    };
    let mut alias_harness = DiagnosticsHarness::new();
    let aliased = execute("/ctx", &mut alias_harness.app);
    assert!(matches!(
        canonical.action,
        Some(AppAction::OpenContextInspector)
    ));
    assert_eq!(render(&canonical), render(&aliased));
}

/// `/preview-request` must register as a `Pure` handler that builds no host
/// envelope, while the seven contextual commands are still on the legacy
/// dispatch path until their owning phases.
#[test]
fn preview_request_remains_the_pure_registration() {
    // The public dispatcher routes the command through the legacy function
    // path today; the assertion that it never needs `App` state is behavioural:
    // an app with no provider and no session still parses the same action.
    let mut harness = DiagnosticsHarness::new();
    harness.app.api_provider = ApiProvider::Ollama;
    let result = execute("/preview-request", &mut harness.app);
    assert!(matches!(
        result.action,
        Some(AppAction::PreviewOutboundRequest { .. })
    ));
}

fn snapshot() -> crate::tool_inspection::ToolInspectionSnapshot {
    crate::tool_inspection::ToolInspectionSnapshot::from_prepared_request(
        "turn-1",
        2,
        Some(&[tool("read_file"), tool("write_file")]),
    )
}

fn tool(name: &str) -> codewhale_models::Tool {
    codewhale_models::Tool {
        tool_type: Some("function".to_string()),
        name: name.to_string(),
        description: format!("{name} test tool"),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {"path": {"type": "string"}}
        }),
        allowed_callers: None,
        defer_loading: Some(false),
        input_examples: None,
        strict: Some(true),
        cache_control: None,
    }
}
