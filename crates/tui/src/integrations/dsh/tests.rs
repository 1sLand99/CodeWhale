use std::path::PathBuf;

use super::detect::{
    DetectEnv, DshDetection, DshRunner, classify_version, detect, settings_namespaces,
};
use super::identity::{
    CodewhaleRouteIdentity, DshAdapter, DshPermissionMode, WireProtocol, dsh_reasoning_effort,
    map_identity, permission_mode_for, render_overlay,
};
use super::receipt::{DshReceiptDocument, DshReceiptEvent};
use super::*;

struct StubRunner {
    version: Option<(bool, String)>,
    help: String,
    fail: bool,
}

impl DshRunner for StubRunner {
    fn run(&self, _binary: &std::path::Path, args: &[&str]) -> std::io::Result<(bool, String)> {
        if self.fail {
            return Err(std::io::Error::other("cannot exec"));
        }
        match args {
            ["--version"] => Ok(self.version.clone().unwrap_or((false, String::new()))),
            ["--help"] => Ok((true, self.help.clone())),
            _ => Ok((false, String::new())),
        }
    }
}

fn verified_runner() -> StubRunner {
    StubRunner {
        version: Some((true, "0.1.0-rc.6\n".to_string())),
        help: "Options:\n  --profile <name>\n  --patch <path>\n".to_string(),
        fail: false,
    }
}

fn lab_env(with_dsh: bool) -> (tempfile::TempDir, DetectEnv) {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    if with_dsh {
        std::fs::write(bin.join("dsh"), "#!/bin/sh\necho 0.1.0-rc.6\n").unwrap();
    }
    let dsh_home = dir.path().join("dsh-home");
    let env = DetectEnv {
        path: Some(bin.into_os_string()),
        home: Some(dir.path().to_path_buf()),
        dsh_home: Some(dsh_home.into_os_string()),
    };
    (dir, env)
}

fn identity(
    provider: &str,
    model: &str,
    base_url: &str,
    protocol: WireProtocol,
) -> CodewhaleRouteIdentity {
    CodewhaleRouteIdentity {
        provider_id: provider.to_string(),
        provider_label: provider.to_uppercase(),
        model: model.to_string(),
        base_url: base_url.to_string(),
        protocol,
        api_key_env: Some(format!(
            "{}_API_KEY",
            provider.to_uppercase().replace('-', "_")
        )),
        keyless_local: false,
        reasoning_effort: None,
        sandbox_mode: None,
        approval_policy: None,
        yolo: false,
        workspace: "/ws".to_string(),
    }
}

#[test]
fn version_classification_is_exact_about_the_verified_line() {
    assert_eq!(
        classify_version("0.1.0-rc.6", true),
        DshCompatibility::Verified
    );
    assert!(matches!(
        classify_version("0.1.0-rc.7", true),
        DshCompatibility::NewerUnverified { .. }
    ));
    assert!(matches!(
        classify_version("0.1.0", true),
        DshCompatibility::NewerUnverified { .. }
    ));
    assert!(matches!(
        classify_version("0.2.0-rc.1", true),
        DshCompatibility::NewerUnverified { .. }
    ));
    assert!(matches!(
        classify_version("0.1.0-rc.3", true),
        DshCompatibility::Incompatible { .. }
    ));
    assert!(matches!(
        classify_version("0.0.1-rc.1", true),
        DshCompatibility::Incompatible { .. }
    ));
    assert!(matches!(
        classify_version("0.1.0-rc.6", false),
        DshCompatibility::Incompatible { .. }
    ));
    assert!(matches!(
        classify_version("nightly", true),
        DshCompatibility::Unparsed { .. }
    ));
}

#[test]
fn detection_reports_missing_offline_and_verified_without_writing() {
    let (dir, env) = lab_env(false);
    let d = detect(&env, &verified_runner());
    assert!(!d.installed());
    assert!(matches!(d.compatibility, DshCompatibility::Offline { .. }));
    assert!(!d.dsh_home_exists);
    assert!(d.dsh_home_from_env);

    let (dir2, env2) = lab_env(true);
    let d = detect(&env2, &verified_runner());
    assert!(d.installed());
    assert_eq!(d.version.as_deref(), Some("0.1.0-rc.6"));
    assert_eq!(d.compatibility, DshCompatibility::Verified);
    assert!(d.supports_patch);
    // Nothing was created under DSH_HOME by detection.
    assert!(!env2_home(&env2).exists());

    let offline = StubRunner {
        version: None,
        help: String::new(),
        fail: true,
    };
    let d = detect(&env2, &offline);
    assert!(matches!(d.compatibility, DshCompatibility::Offline { .. }));
    drop(dir);
    drop(dir2);
}

fn env2_home(env: &DetectEnv) -> PathBuf {
    PathBuf::from(env.dsh_home.clone().unwrap())
}

#[test]
fn detection_inventories_profiles_settings_and_credentials_presence_only() {
    let (_dir, env) = lab_env(true);
    let home = env2_home(&env);
    std::fs::create_dir_all(home.join("profiles/web")).unwrap();
    std::fs::create_dir_all(home.join("profiles/node_modules")).unwrap();
    std::fs::write(
        home.join("settings.yaml"),
        "ui-onboarding:\n  welcomeNoticeVersion: 1\nagent-default-model:\n  provider: deepseek-official\n  model: deepseek-v4-pro\n",
    )
    .unwrap();
    std::fs::write(
        home.join(".credentials.yaml"),
        "DEEPSEEK_API_KEY: not-a-real-key\n",
    )
    .unwrap();
    let d = detect(&env, &verified_runner());
    assert_eq!(d.profiles, vec!["web".to_string()]);
    assert_eq!(
        d.settings_namespaces,
        vec![
            "ui-onboarding".to_string(),
            "agent-default-model".to_string()
        ]
    );
    assert!(d.credentials_present);
    let json = serde_json::to_string(&d).unwrap();
    assert!(
        !json.contains("not-a-real-key"),
        "detection must never carry a credential value"
    );
}

#[test]
fn settings_namespace_scan_ignores_nested_keys_and_comments() {
    let ns = settings_namespaces(
        "# c\nllm-deepseek:\n  baseURL: x\n  models:\n    - id: y\nlocale: en\n- list\n",
    );
    assert_eq!(ns, vec!["llm-deepseek", "locale"]);
}

#[test]
fn reasoning_effort_maps_onto_dsh_tiers() {
    assert_eq!(dsh_reasoning_effort(None), None);
    assert_eq!(dsh_reasoning_effort(Some("off")), Some("off"));
    assert_eq!(dsh_reasoning_effort(Some("low")), Some("high"));
    assert_eq!(dsh_reasoning_effort(Some("high")), Some("high"));
    assert_eq!(dsh_reasoning_effort(Some("ultra")), Some("max"));
    assert_eq!(dsh_reasoning_effort(Some("max")), Some("max"));
    assert_eq!(dsh_reasoning_effort(Some("weird")), None);
}

#[test]
fn permission_never_broadens_without_explicit_confirmation() {
    let mut id = identity(
        "deepseek",
        "deepseek-v4-pro",
        "https://api.deepseek.com",
        WireProtocol::ChatCompletions,
    );
    assert_eq!(
        permission_mode_for(&id, false).0,
        DshPermissionMode::WorkspaceWrite
    );
    id.sandbox_mode = Some("read-only".to_string());
    assert_eq!(
        permission_mode_for(&id, false).0,
        DshPermissionMode::ReadOnly
    );
    id.sandbox_mode = Some("danger-full-access".to_string());
    let (mode, note) = permission_mode_for(&id, false);
    assert_eq!(mode, DshPermissionMode::WorkspaceWrite);
    assert!(note.unwrap().contains("--allow-full-access"));
    assert_eq!(
        permission_mode_for(&id, true).0,
        DshPermissionMode::DangerFullAccess
    );
    // Codewhale at workspace-write can never be lifted to full access.
    id.sandbox_mode = Some("workspace-write".to_string());
    assert_eq!(
        permission_mode_for(&id, true).0,
        DshPermissionMode::WorkspaceWrite
    );
}

#[test]
fn deepseek_route_maps_to_native_adapter_with_exact_identity() {
    let mut id = identity(
        "deepseek",
        "deepseek-v4-pro",
        "https://api.deepseek.com/beta",
        WireProtocol::ChatCompletions,
    );
    id.reasoning_effort = Some("ultra".to_string());
    let mapped = map_identity(&id, false);
    assert_eq!(mapped.adapter, DshAdapter::DeepseekNative);
    assert_eq!(mapped.dsh_reasoning_effort.as_deref(), Some("max"));
    let overlay = render_overlay(&mapped).unwrap();
    assert!(overlay.contains("provider: deepseek-official"));
    assert!(overlay.contains("model: 'deepseek-v4-pro'"));
    assert!(overlay.contains("baseURL: 'https://api.deepseek.com/beta'"));
    assert!(overlay.contains("reasoningEffort: max"));
    assert!(overlay.contains("DeepSeek Harness connected through Codewhale"));
    assert!(
        !overlay.contains("apiKeyEnv"),
        "native adapter resolves its own default key ref"
    );
}

#[test]
fn ollama_keyless_route_writes_no_credential_reference() {
    let mut id = identity(
        "ollama",
        "qwen3:8b",
        "http://127.0.0.1:11434/v1",
        WireProtocol::ChatCompletions,
    );
    id.keyless_local = true;
    let mapped = map_identity(&id, false);
    assert_eq!(
        mapped.adapter,
        DshAdapter::PiAiOpenAiCompatible {
            route_id: "codewhale-ollama".to_string()
        }
    );
    let overlay = render_overlay(&mapped).unwrap();
    assert!(overlay.contains("provider: 'codewhale-ollama'"));
    assert!(overlay.contains("api: openai-completions"));
    assert!(overlay.contains("baseURL: 'http://127.0.0.1:11434/v1'"));
    assert!(!overlay.contains("apiKeyEnv"));
    assert!(
        mapped
            .disclosures
            .iter()
            .any(|d| d.contains("Keyless local route"))
    );
}

#[test]
fn keyed_openai_compatible_route_names_only_the_env_var() {
    let secret = "sk-this-must-never-appear";
    let mut id = identity(
        "zai",
        "GLM-5.3",
        "https://api.z.ai/api/coding/paas/v4",
        WireProtocol::ChatCompletions,
    );
    id.api_key_env = Some("ZAI_API_KEY".to_string());
    id.reasoning_effort = Some("high".to_string());
    let mapped = map_identity(&id, false);
    let overlay = render_overlay(&mapped).unwrap();
    assert!(overlay.contains("apiKeyEnv: 'ZAI_API_KEY'"));
    assert!(!overlay.contains(secret));
    assert!(!overlay.contains("reasoningEffort"));
    let json = serde_json::to_string(&mapped).unwrap();
    assert!(!json.contains(secret));
    assert!(mapped.disclosures.iter().any(|d| d.contains("ZAI_API_KEY")));
    assert!(
        mapped
            .disclosures
            .iter()
            .any(|d| d.contains("Reasoning tier is not mapped"))
    );
}

#[test]
fn unsupported_protocols_and_credentialed_urls_are_refused() {
    let id = identity(
        "anthropic",
        "claude",
        "https://api.anthropic.com",
        WireProtocol::AnthropicMessages,
    );
    let mapped = map_identity(&id, false);
    assert!(matches!(mapped.adapter, DshAdapter::Unsupported { .. }));
    assert!(render_overlay(&mapped).is_none());
    let id = identity(
        "openai-codex",
        "gpt",
        "https://x/responses",
        WireProtocol::Responses,
    );
    assert!(!map_identity(&id, false).mappable());
    let id = identity(
        "custom",
        "m",
        "https://user:token@gateway/v1",
        WireProtocol::ChatCompletions,
    );
    let mapped = map_identity(&id, false);
    match mapped.adapter {
        DshAdapter::Unsupported { reason } => assert!(reason.contains("userinfo")),
        other => panic!("expected refusal, got {other:?}"),
    }
    let id = identity(
        "custom",
        "m",
        "https://gateway/v1?key=abc",
        WireProtocol::ChatCompletions,
    );
    assert!(!map_identity(&id, false).mappable());
}

#[test]
fn overlay_hash_is_deterministic_and_yaml_quotes_apostrophes() {
    let mut id = identity(
        "custom",
        "it's",
        "http://10.0.0.5:8000/v1",
        WireProtocol::ChatCompletions,
    );
    id.provider_label = "O'Brien Gateway".to_string();
    let a = render_overlay(&map_identity(&id, false)).unwrap();
    let b = render_overlay(&map_identity(&id, false)).unwrap();
    assert_eq!(sha256_hex(a.as_bytes()), sha256_hex(b.as_bytes()));
    assert!(a.contains("'it''s'"));
    assert!(a.contains("O''Brien"));
}

fn lab_paths() -> (tempfile::TempDir, DshPaths) {
    let dir = tempfile::tempdir().unwrap();
    let paths = DshPaths::under(&dir.path().join("codewhale-home"));
    (dir, paths)
}

fn detection_ok() -> DshDetection {
    let (_dir, env) = lab_env(true);
    let mut d = detect(&env, &verified_runner());
    d.binary = Some(PathBuf::from("/fake/dsh"));
    d
}

#[test]
fn connect_update_disable_enable_remove_lifecycle_writes_only_owned_files() {
    let (_dir, paths) = lab_paths();
    let detection = detection_ok();
    let id = identity(
        "deepseek",
        "deepseek-v4-flash",
        "https://api.deepseek.com",
        WireProtocol::ChatCompletions,
    );

    // Not connected yet.
    let report = compute_status(&paths, detection.clone(), Ok(id.clone()), false).unwrap();
    assert!(matches!(report.state, DshIntegrationState::Detected { .. }));
    assert!(launch_spec(&report, None, &[], std::path::Path::new("/ws")).is_err());

    let plan = super::plan(&paths, &detection, &id, "web", false, true).unwrap();
    assert!(plan.overlay_text.contains("deepseek-official"));
    let record = apply_plan(&paths, &detection, &plan, DshReceiptEvent::Connect).unwrap();
    assert!(paths.overlay.is_file());
    assert!(paths.skin.is_file());
    assert!(paths.receipt.is_file());
    assert_eq!(record.overlay_sha256, plan.overlay_sha256);

    let report = compute_status(&paths, detection.clone(), Ok(id.clone()), false).unwrap();
    assert!(
        matches!(report.state, DshIntegrationState::Connected { .. }),
        "{:?}",
        report.state
    );
    let spec = launch_spec(
        &report,
        None,
        &["--port".to_string(), "0".to_string()],
        std::path::Path::new("/ws"),
    )
    .unwrap();
    assert_eq!(spec.args[0], "--profile");
    assert_eq!(spec.args[1], "web");
    assert_eq!(spec.args[2], "--patch");
    assert!(spec.args[3].ends_with(OVERLAY_FILE));
    assert_eq!(spec.args[4..], ["--port", "0"]);
    assert_eq!(
        spec.env,
        vec![(
            "DSH_PERMISSION_MODE".to_string(),
            "workspace-write".to_string()
        )]
    );

    // Route drift → stale-config, launch refused.
    let mut moved = id.clone();
    moved.model = "deepseek-v4-pro".to_string();
    let report = compute_status(&paths, detection.clone(), Ok(moved.clone()), false).unwrap();
    assert!(
        matches!(report.state, DshIntegrationState::StaleConfig { .. }),
        "{:?}",
        report.state
    );
    let err = launch_spec(&report, None, &[], std::path::Path::new("/ws"))
        .unwrap_err()
        .to_string();
    assert!(err.contains("stale"), "{err}");

    // Update re-derives.
    let plan2 = super::plan(&paths, &detection, &moved, "web", false, false).unwrap();
    apply_plan(&paths, &detection, &plan2, DshReceiptEvent::Update).unwrap();
    let report = compute_status(&paths, detection.clone(), Ok(moved.clone()), false).unwrap();
    assert!(matches!(
        report.state,
        DshIntegrationState::Connected { .. }
    ));

    // Tampered overlay → stale.
    std::fs::write(&paths.overlay, "- id: x\n").unwrap();
    let report = compute_status(&paths, detection.clone(), Ok(moved.clone()), false).unwrap();
    assert!(matches!(
        report.state,
        DshIntegrationState::StaleConfig { .. }
    ));
    apply_plan(&paths, &detection, &plan2, DshReceiptEvent::Update).unwrap();

    // Disable / enable.
    set_disabled(&paths, true).unwrap();
    let report = compute_status(&paths, detection.clone(), Ok(moved.clone()), false).unwrap();
    assert!(matches!(report.state, DshIntegrationState::Disabled { .. }));
    assert!(launch_spec(&report, None, &[], std::path::Path::new("/ws")).is_err());
    set_disabled(&paths, false).unwrap();
    let report = compute_status(&paths, detection.clone(), Ok(moved.clone()), false).unwrap();
    assert!(matches!(
        report.state,
        DshIntegrationState::Connected { .. }
    ));

    // Remove: files gone, history kept, current cleared.
    let removed = remove(&paths).unwrap();
    assert!(removed.contains(&paths.overlay));
    assert!(!paths.overlay.exists());
    assert!(!paths.skin.exists());
    let doc = DshReceiptDocument::load(&paths.receipt).unwrap();
    assert!(doc.current.is_none());
    let events: Vec<_> = doc.history.iter().map(|e| e.event.as_str()).collect();
    assert_eq!(
        events,
        ["connect", "update", "update", "disable", "enable", "remove"]
    );
    let report = compute_status(&paths, detection, Ok(moved), false).unwrap();
    assert!(matches!(report.state, DshIntegrationState::Detected { .. }));
    // Every write stayed under the integration root.
    for entry in walk(&paths.root.parent().unwrap().parent().unwrap().to_path_buf()) {
        assert!(
            entry.starts_with(&paths.root),
            "unexpected file {}",
            entry.display()
        );
    }
}

fn walk(root: &PathBuf) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
    }
    out
}

#[test]
fn newer_dsh_reports_stale_version_but_stays_launchable() {
    let (_dir, paths) = lab_paths();
    let mut detection = detection_ok();
    let id = identity(
        "deepseek",
        "deepseek-v4-flash",
        "https://api.deepseek.com",
        WireProtocol::ChatCompletions,
    );
    let plan = super::plan(&paths, &detection, &id, "headless", false, false).unwrap();
    apply_plan(&paths, &detection, &plan, DshReceiptEvent::Connect).unwrap();
    detection.version = Some("0.1.0-rc.9".to_string());
    detection.compatibility = classify_version("0.1.0-rc.9", true);
    let report = compute_status(&paths, detection, Ok(id), false).unwrap();
    assert!(matches!(
        report.state,
        DshIntegrationState::StaleVersion { .. }
    ));
    assert!(report.state.launchable());
    let spec = launch_spec(&report, None, &[], std::path::Path::new("/ws")).unwrap();
    assert_eq!(spec.args[1], "headless");
}

#[test]
fn incompatible_and_missing_dsh_states_are_honest() {
    let (_dir, paths) = lab_paths();
    let mut detection = detection_ok();
    detection.version = Some("0.0.1-rc.1".to_string());
    detection.compatibility = classify_version("0.0.1-rc.1", true);
    let id = identity(
        "deepseek",
        "m",
        "https://api.deepseek.com",
        WireProtocol::ChatCompletions,
    );
    let report = compute_status(&paths, detection.clone(), Ok(id.clone()), false).unwrap();
    assert!(matches!(
        report.state,
        DshIntegrationState::Incompatible { .. }
    ));
    assert!(status_line(&report).starts_with("incompatible"));
    detection.binary = None;
    let report = compute_status(&paths, detection, Ok(id), false).unwrap();
    assert_eq!(report.state, DshIntegrationState::NotInstalled);
    assert!(status_line(&report).contains("not installed"));
}

#[test]
fn plan_discloses_shadowing_settings_namespaces() {
    let (_dir, paths) = lab_paths();
    let mut detection = detection_ok();
    detection.settings_namespaces = vec!["agent-default-model".to_string(), "locale".to_string()];
    let id = identity(
        "deepseek",
        "m",
        "https://api.deepseek.com",
        WireProtocol::ChatCompletions,
    );
    let plan = super::plan(&paths, &detection, &id, "web", false, false).unwrap();
    assert_eq!(plan.shadowing_namespaces, vec!["agent-default-model"]);
    assert!(plan.disclosures.iter().any(|d| d.contains("shadow")));
    assert!(
        plan.launch_command
            .contains("DSH_PERMISSION_MODE=workspace-write dsh --profile web --patch")
    );
}

#[test]
fn skin_css_is_generated_from_palette_and_labels_itself_unsupported() {
    let css = skin::skin_css();
    assert!(css.contains("--cw-surface-bg: #03070d"));
    assert!(css.contains("--cw-accent-action: #f6c453"));
    assert!(css.contains("--cw-water-surface: #102a45"));
    assert!(css.contains("--cw-water-middle: #0a1e33"));
    assert!(css.contains("--cw-water-deep: #061320"));
    assert!(css.contains("--cw-permission-full-access: #ff7a59"));
    assert!(css.contains("--cw-mode-plan: #b9dcec"));
    assert!(css.contains("--dsw-alias-bg-base: var(--cw-surface-bg)"));
    assert!(css.contains("prefers-reduced-motion: reduce"));
    assert!(css.contains("UNSUPPORTED OVERLAY"));
    assert!(css.contains("DeepSeek Harness connected through Codewhale"));
    assert!(css.contains("MIT"));
    assert!(css.contains("data:image/svg+xml"));
    let preview = skin::skin_preview_html();
    assert!(preview.contains("PREVIEW ONLY"));
}

#[test]
fn launch_strips_only_codewhale_injected_credentials() {
    let none = launch_env_strip_list(None, &["ZAI_API_KEY".to_string()]);
    assert_eq!(none, ["CODEWHALE_CLI_API_KEY", "DEEPSEEK_API_KEY_SOURCE"]);
    let cli = launch_env_strip_list(Some("cli"), &["ZAI_API_KEY".to_string()]);
    assert!(cli.contains(&"DEEPSEEK_API_KEY".to_string()));
    assert!(cli.contains(&"ZAI_API_KEY".to_string()));
    let env = launch_env_strip_list(Some("env"), &["ZAI_API_KEY".to_string()]);
    assert!(
        !env.contains(&"DEEPSEEK_API_KEY".to_string()),
        "a user's own env key is left alone"
    );
}
