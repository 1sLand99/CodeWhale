//! Extension host tests.
//!
//! Unit tests (protocol corpus, registry rules, tool gating) need no Node.
//! Integration tests spawn the *committed* bundle under a real Node ≥22.19:
//! they skip with a printed reason when none is found, unless
//! `CODEWHALE_EXT_HOST_TESTS=1` is set (CI), where a missing Node fails.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::protocol::{
    self, OwnerRef, RegisterKind, RegisterParams, ToolSpecWire, parse_core_message,
    parse_host_message,
};
use super::registry::{OwnerRegistry, OwnerState};
use super::{ExtensionHostManager, ExtensionHostOptions, HostStatus};
use crate::plugins::PluginRegistry;
use crate::plugins::activation::TestPolicyGuard;
use crate::plugins::discovery::{DiscoveryConfig, discover_with_config};
use crate::tools::spec::{ApprovalRequirement, ToolContext, ToolError, ToolSpec};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/extension_host")
}

// ---------------------------------------------------------------------------
// Protocol
// ---------------------------------------------------------------------------

#[test]
fn protocol_corpus_parses_and_round_trips_in_both_directions() {
    let dir = fixtures_dir().join("protocol");
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("corpus dir")
        .map(|entry| entry.expect("entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();
    let (mut valid, mut invalid) = (0, 0);
    for path in entries {
        let case: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let frame = case["frame"].clone();
        let direction = case["direction"].as_str().unwrap();
        let expect_valid = case["valid"].as_bool().unwrap();
        let reencoded = match direction {
            "host_to_core" => parse_host_message(frame.clone()).map(|m| m.to_value()),
            "core_to_host" => parse_core_message(frame.clone()).map(|m| m.to_value()),
            other => panic!("unknown direction {other}"),
        };
        let name = path.file_name().unwrap().to_string_lossy();
        if expect_valid {
            let reencoded = reencoded.unwrap_or_else(|e| panic!("{name}: {e}"));
            assert_eq!(reencoded, frame, "{name} must round-trip exactly");
            let bytes = protocol::encode_frame(&frame).unwrap();
            assert_eq!(&bytes[..4], b"CWX1");
            valid += 1;
        } else {
            assert!(reencoded.is_err(), "{name} must be rejected");
            invalid += 1;
        }
    }
    assert!(
        valid >= 15 && invalid >= 8,
        "corpus too small: {valid} valid, {invalid} invalid"
    );
}

#[tokio::test]
async fn frames_decode_in_order_and_violations_are_typed() {
    let mut bytes = protocol::encode_frame(&json!({"a": 1})).unwrap();
    bytes.extend(protocol::encode_frame(&json!({"b": "ü"})).unwrap());
    let mut reader = bytes.as_slice();
    assert_eq!(
        protocol::read_frame(&mut reader).await.unwrap(),
        Some(json!({"a": 1}))
    );
    assert_eq!(
        protocol::read_frame(&mut reader).await.unwrap(),
        Some(json!({"b": "ü"}))
    );
    assert_eq!(protocol::read_frame(&mut reader).await.unwrap(), None);

    let mut bad = b"NOPE\0\0\0\0".as_slice();
    assert!(matches!(
        protocol::read_frame(&mut bad).await,
        Err(protocol::FrameError::BadMagic)
    ));
    let mut huge = Vec::from(*b"CWX1");
    huge.extend(((protocol::MAX_FRAME + 1) as u32).to_le_bytes());
    assert!(matches!(
        protocol::read_frame(&mut huge.as_slice()).await,
        Err(protocol::FrameError::TooLarge(_))
    ));
}

// ---------------------------------------------------------------------------
// Owner registry
// ---------------------------------------------------------------------------

fn fake_authority(plugin_id: &str) -> crate::plugins::types::PluginAuthority {
    crate::plugins::types::PluginAuthority {
        plugin_id: crate::plugins::types::PluginId(plugin_id.to_string()),
        plugin_name: plugin_id.to_string(),
        workspace: PathBuf::from("/w"),
        state_path: PathBuf::from("/s"),
        source_manifest: PathBuf::from("/m"),
        staged_manifest: PathBuf::from("/sm"),
        content_hash: format!("hash-{plugin_id}"),
        capability_hash: "cap".to_string(),
        state_generation: 1,
    }
}

fn register(registry: &mut OwnerRegistry, owner: &OwnerRef, name: &str) -> Result<u64, String> {
    registry.register_tool(&RegisterParams {
        owner: owner.clone(),
        kind: RegisterKind::Tool,
        spec: ToolSpecWire {
            name: name.to_string(),
            description: "d".to_string(),
            input_schema: json!({"type": "object", "properties": {}})
                .as_object()
                .unwrap()
                .clone(),
        },
    })
}

#[test]
fn registry_refuses_shadowing_and_foreign_names_and_undoes_exactly_one_entry() {
    let mut registry = OwnerRegistry::new();
    registry.set_native_names(["grep_files"]);
    let a = registry.begin_owner("a", "a", fake_authority("a"), "hash-a");
    let b = registry.begin_owner("b", "b", fake_authority("b"), "hash-b");

    // Built-ins (static, snapshot, and case-folded) and reserved prefixes.
    for name in [
        "read_file",
        "grep_files",
        "READ",
        "tool_search",
        "mcp_x_y",
        "ext_z",
    ] {
        assert!(
            register(&mut registry, &a, name).is_err(),
            "{name} must be refused"
        );
    }
    assert!(register(&mut registry, &a, "bad name").is_err());

    let first = register(&mut registry, &a, "shared_name").unwrap();
    // Another owner cannot take it, in any case.
    assert!(register(&mut registry, &b, "Shared_Name").is_err());
    // Same owner re-registering retires the old handle.
    let second = register(&mut registry, &a, "shared_name").unwrap();
    assert_ne!(first, second);
    registry.mark_active(&a);
    registry.unregister(&a, first); // stale: must not remove the newer entry
    assert_eq!(registry.live_tools().len(), 1);
    assert!(registry.is_live(second, &a));
    // A foreign owner cannot unregister it either.
    registry.unregister(&b, second);
    assert!(registry.is_live(second, &a));

    // A stale token is refused.
    let mut stale = a.clone();
    stale.owner_token = "not-the-token".to_string();
    assert!(register(&mut registry, &stale, "other").is_err());

    // Revocation is synchronous and total.
    assert_eq!(registry.revoke_owner("a"), Some(a.clone()));
    assert!(!registry.is_live(second, &a));
    assert!(registry.live_tools().is_empty());
    assert!(register(&mut registry, &a, "after_revoke").is_err());
}

#[test]
fn registry_enforces_schema_and_count_caps() {
    let mut registry = OwnerRegistry::new();
    let a = registry.begin_owner("a", "a", fake_authority("a"), "hash-a");
    let mut params = RegisterParams {
        owner: a.clone(),
        kind: RegisterKind::Tool,
        spec: ToolSpecWire {
            name: "big".to_string(),
            description: "x".repeat(super::registry::MAX_DESCRIPTION_BYTES + 1),
            input_schema: json!({"type": "object"}).as_object().unwrap().clone(),
        },
    };
    assert!(
        registry
            .register_tool(&params)
            .unwrap_err()
            .contains("description")
    );
    params.spec.description = "ok".to_string();
    params.spec.input_schema =
        json!({"type": "object", "description": "y".repeat(super::registry::MAX_SCHEMA_BYTES)})
            .as_object()
            .unwrap()
            .clone();
    assert!(
        registry
            .register_tool(&params)
            .unwrap_err()
            .contains("schema")
    );
    params.spec.input_schema = json!({"type": "string"}).as_object().unwrap().clone();
    assert!(
        registry
            .register_tool(&params)
            .unwrap_err()
            .contains("object")
    );
    for index in 0..super::registry::MAX_TOOLS_PER_OWNER {
        register(&mut registry, &a, &format!("t{index}")).unwrap();
    }
    assert!(
        register(&mut registry, &a, "one_too_many")
            .unwrap_err()
            .contains("at most")
    );
}

// ---------------------------------------------------------------------------
// Integration: real Node, real bundle
// ---------------------------------------------------------------------------

/// A Node for the integration tests, or `None` (skip) when there is none and
/// the tests were not explicitly required.
pub(crate) fn node_for_tests(test: &str) -> Option<PathBuf> {
    let resolution = crate::dependencies::resolve_node_for_extension_host(None);
    match resolution.selected {
        Some((path, _)) => Some(path),
        None if std::env::var_os("CODEWHALE_EXT_HOST_TESTS").is_some() => panic!(
            "{test}: CODEWHALE_EXT_HOST_TESTS is set but no Node ^22.19 || >=24 was found: {}",
            resolution.describe_rejections()
        ),
        None => {
            eprintln!(
                "skipping {test}: no Node ^22.19 || >=24 ({})",
                resolution.describe_rejections()
            );
            None
        }
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// Fixture plugins installed into a private user plugin dir, reviewed
/// (trusted) and enabled through the real registry. Callers must hold a
/// `TestPolicyGuard::extension_host(true)` on this thread.
pub(crate) struct FixturePlugins {
    _temp: tempfile::TempDir,
    pub config: DiscoveryConfig,
    pub root: PathBuf,
}

impl FixturePlugins {
    pub(crate) fn new(names: &[&str]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("project");
        let user = temp.path().join("user");
        std::fs::create_dir_all(&workspace).unwrap();
        for name in names {
            copy_dir(&fixtures_dir().join(name), &user.join(name));
        }
        let config = DiscoveryConfig {
            workspace: workspace.clone(),
            user_plugins_dir: user,
            workspace_plugins_dir: workspace.join(".codewhale/plugins"),
            builtin_plugin_dirs: Vec::new(),
            state_path: temp.path().join("state/plugin-state.json"),
        };
        let mut registry = discover_with_config(&config);
        for name in names {
            registry
                .trust(name)
                .unwrap_or_else(|e| panic!("trust {name}: {e}"));
            registry
                .enable(name)
                .unwrap_or_else(|e| panic!("enable {name}: {e}"));
        }
        let root = temp.path().join("home");
        let fixture = Self {
            _temp: temp,
            config,
            root,
        };
        let registry = fixture.registry();
        for name in names {
            assert!(
                registry.is_active(name),
                "{name} must be active under policy v4"
            );
        }
        fixture
    }

    pub(crate) fn registry(&self) -> Arc<PluginRegistry> {
        Arc::new(discover_with_config(&self.config))
    }

    pub(crate) fn disable(&self, name: &str) -> Arc<PluginRegistry> {
        let mut registry = discover_with_config(&self.config);
        registry.disable(name).unwrap();
        self.registry()
    }

    pub(crate) fn workspace(&self) -> &Path {
        &self.config.workspace
    }

    pub(crate) fn manager(&self, node: PathBuf) -> Arc<ExtensionHostManager> {
        Arc::new(ExtensionHostManager::new(ExtensionHostOptions {
            node_override: Some(node),
            root: Some(self.root.clone()),
        }))
    }
}

fn host_tool(manager: &ExtensionHostManager, workspace: &Path, name: &str) -> Arc<dyn ToolSpec> {
    let mut registry =
        crate::tools::registry::ToolRegistryBuilder::new().build(ToolContext::new(workspace));
    let installed = manager.install_tools(&mut registry);
    assert!(
        installed.contains(&name.to_string()),
        "{name} not installed: {installed:?}"
    );
    registry.get(name).unwrap()
}

fn rss_kib(pid: u32) -> Option<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .ok()?;
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

#[tokio::test]
async fn dsh_plugin_runs_end_to_end_behind_the_approval_gate() {
    let Some(node) = node_for_tests("dsh_plugin_runs_end_to_end_behind_the_approval_gate") else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["dsh-workspace-deps"]);
    let manager = fixture.manager(node);
    assert_eq!(
        manager.status(),
        HostStatus::Idle,
        "nothing starts before sync"
    );

    let started = Instant::now();
    manager.sync(fixture.registry()).await.unwrap();
    let elapsed = started.elapsed();
    let pid = manager.host_pid().expect("host running");
    eprintln!(
        "extension host: spawn + handshake + activation {:.1} ms; RSS {} KiB",
        elapsed.as_secs_f64() * 1000.0,
        rss_kib(pid).map_or_else(|| "?".to_string(), |kib| kib.to_string())
    );
    assert_eq!(
        manager.live_tool_names(),
        vec!["load_workspace_dependencies"]
    );
    assert_eq!(
        manager.owner_state(
            fixture
                .registry()
                .get("dsh-workspace-deps")
                .unwrap()
                .id
                .as_str()
        ),
        Some(OwnerState::Active)
    );

    let tool = host_tool(&manager, fixture.workspace(), "load_workspace_dependencies");
    assert_eq!(tool.registration_origin(), "extension:dsh-workspace-deps");
    // The plugin declares `presentCall: kind 'read'`; approval stays Required.
    assert_eq!(
        tool.approval_requirement_for(&json!({})),
        ApprovalRequirement::Required
    );
    assert!(!tool.is_read_only_for(&json!({})));
    assert!(tool.defer_loading());
    let context = ToolContext::new(fixture.workspace());
    let prepared = tool.prepare(json!({}), &context).unwrap();
    assert_eq!(prepared.approval, ApprovalRequirement::Required);
    assert!(
        prepared
            .description
            .contains("extension:dsh-workspace-deps")
    );

    let result = tool.execute(json!({}), &context).await.unwrap();
    assert!(result.success);
    let payload: Value = serde_json::from_str(&result.content).unwrap();
    assert_eq!(payload["pythonDistributions"]["numpy"], "2.1.0");
    assert!(payload["python"].as_str().unwrap().contains("dependencies"));
    manager.shutdown().await;
}

#[tokio::test]
async fn execute_tools_refuses_extension_tools_before_any_host_call() {
    let Some(node) = node_for_tests("execute_tools_refuses_extension_tools_before_any_host_call")
    else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["slow-tool"]);
    let manager = fixture.manager(node);
    manager.sync(fixture.registry()).await.unwrap();
    let mut registry = crate::tools::registry::ToolRegistryBuilder::new()
        .build(ToolContext::new(fixture.workspace()));
    manager.install_tools(&mut registry);
    let context = ToolContext::new(fixture.workspace());
    let started = Instant::now();
    let result = crate::tools::codemode::execute_tools_tool(
        &json!({"code": "return await tools.call('slow_wait', { ms: 5000 })"}),
        &registry,
        &context,
    )
    .await
    .unwrap();
    // Refused at the gate: had the call reached the host it would take 5 s.
    assert!(started.elapsed() < Duration::from_secs(4));
    assert!(!result.success, "{}", result.content);
    assert!(
        result.content.contains("can mutate") || result.content.contains("needs approval"),
        "{}",
        result.content
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn disabling_mid_call_revokes_at_once_and_teardown_waits_for_async_disposers() {
    let Some(node) = node_for_tests("disabling_mid_call") else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["slow-tool"]);
    let manager = fixture.manager(node);
    manager.sync(fixture.registry()).await.unwrap();
    let tool = host_tool(&manager, fixture.workspace(), "slow_wait");
    let context = ToolContext::new(fixture.workspace());
    let call = tokio::spawn(async move { tool.execute(json!({}), &context).await });
    tokio::time::sleep(Duration::from_millis(150)).await;

    let disabled = fixture.disable("slow-tool");
    // The revocation scan (a full re-hash of the staged tree) runs first;
    // the clock for the 500 ms bound starts when the registry drops the
    // handle, which is the moment revocation takes effect. A plain thread
    // watches for it, because this runtime is single-threaded (the policy
    // override is thread-local) and would only look when `sync` yields.
    let watcher = {
        let manager = Arc::clone(&manager);
        std::thread::spawn(move || {
            let started = Instant::now();
            while !manager.live_tool_names().is_empty() {
                assert!(
                    started.elapsed() < Duration::from_secs(10),
                    "revocation never happened"
                );
                std::thread::yield_now();
            }
            Instant::now()
        })
    };
    let sync_started = Instant::now();
    let sync = {
        let manager = Arc::clone(&manager);
        tokio::spawn(async move { manager.sync(disabled).await })
    };
    let outcome = tokio::time::timeout(Duration::from_secs(2), call)
        .await
        .expect("call resolves")
        .unwrap();
    let resolved_at = Instant::now();
    let revoked_at = watcher.join().unwrap();
    let call_resolved = resolved_at.saturating_duration_since(revoked_at);
    assert!(
        matches!(outcome, Err(ToolError::Cancelled { .. })),
        "{outcome:?}"
    );
    eprintln!(
        "extension host: in-flight call resolved as cancelled {:.1} ms after revocation",
        call_resolved.as_secs_f64() * 1000.0
    );
    assert!(
        call_resolved < Duration::from_millis(500),
        "{call_resolved:?}"
    );
    sync.await.unwrap().unwrap();
    let teardown = sync_started.elapsed();
    assert!(
        teardown >= Duration::from_millis(300),
        "ack must wait for the 300 ms async disposer (got {teardown:?})"
    );
    let diagnostics = manager.diagnostics();
    assert!(
        !diagnostics.iter().any(|d| d.contains("teardown")),
        "disposed with nothing leaked: {diagnostics:?}"
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn killed_host_fails_calls_with_a_typed_error_and_does_not_respawn() {
    let Some(node) = node_for_tests("killed_host") else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["slow-tool"]);
    let manager = fixture.manager(node);
    manager.sync(fixture.registry()).await.unwrap();
    assert_eq!(manager.spawn_attempts(), 1);
    let pid = manager.host_pid().unwrap();
    let tool = host_tool(&manager, fixture.workspace(), "slow_wait");
    let context = ToolContext::new(fixture.workspace());
    let call = tokio::spawn(async move { tool.execute(json!({}), &context).await });
    tokio::time::sleep(Duration::from_millis(150)).await;
    #[cfg(unix)]
    let status = std::process::Command::new("kill")
        .args(["-9", &pid.to_string()])
        .status()
        .unwrap();
    #[cfg(windows)]
    let status = std::process::Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .status()
        .unwrap();
    assert!(status.success());
    let outcome = tokio::time::timeout(Duration::from_secs(2), call)
        .await
        .expect("call resolves")
        .unwrap();
    match outcome {
        Err(ToolError::NotAvailable { message }) => {
            assert!(message.contains("extension host exited"), "{message}")
        }
        other => panic!("expected a typed not-available error, got {other:?}"),
    }
    // Let the exit watcher publish the failure.
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        matches!(manager.status(), HostStatus::Failed { .. }),
        "{:?}",
        manager.status()
    );
    assert!(manager.live_tool_names().is_empty());
    let report = super::render_status(&manager);
    assert!(report.contains("failed"), "{report}");
    manager.sync(fixture.registry()).await.ok();
    assert_eq!(manager.spawn_attempts(), 1, "no respawn within the session");
    assert!(matches!(manager.status(), HostStatus::Failed { .. }));
}

#[tokio::test]
async fn approval_providing_plugin_fails_activation_and_leaves_nothing_registered() {
    let Some(node) = node_for_tests("approval_providing_plugin") else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["refuses-approval", "clash-native"]);
    let manager = fixture.manager(node);
    manager.sync(fixture.registry()).await.unwrap();
    let registry = fixture.registry();
    for (name, needle) in [
        ("refuses-approval", "approval"),
        ("clash-native", "read_file"),
    ] {
        let id = registry.get(name).unwrap().id.as_str().to_string();
        match manager.owner_state(&id) {
            Some(OwnerState::Failed(reason)) => {
                assert!(reason.contains(needle), "{name}: {reason}")
            }
            other => panic!("{name}: expected failed activation, got {other:?}"),
        }
    }
    assert!(manager.live_tool_names().is_empty());
    // A failed activation of the same bytes is not retried every turn.
    let attempts = manager.spawn_attempts();
    manager.sync(fixture.registry()).await.unwrap();
    assert_eq!(manager.spawn_attempts(), attempts);
    manager.shutdown().await;
}

/// Stands in for a `~/.codewhale/tools` script tool.
struct FakeScriptTool;

#[async_trait::async_trait]
impl ToolSpec for FakeScriptTool {
    fn name(&self) -> &str {
        "fixture_script_tool"
    }
    fn registration_origin(&self) -> std::borrow::Cow<'_, str> {
        "plugin script fixture_script_tool".into()
    }
    fn description(&self) -> &str {
        "script"
    }
    fn input_schema(&self) -> Value {
        json!({"type": "object"})
    }
    fn capabilities(&self) -> Vec<crate::tools::spec::ToolCapability> {
        Vec::new()
    }
    async fn execute(
        &self,
        _input: Value,
        _context: &ToolContext,
    ) -> Result<crate::tools::spec::ToolResult, ToolError> {
        Ok(crate::tools::spec::ToolResult::success("from the script"))
    }
}

#[tokio::test]
async fn an_extension_named_like_a_script_tool_is_skipped_at_turn_build() {
    let Some(node) = node_for_tests("script_name_clash") else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["clash-script"]);
    let manager = fixture.manager(node);
    manager.sync(fixture.registry()).await.unwrap();
    assert_eq!(manager.live_tool_names(), vec!["fixture_script_tool"]);
    let mut registry = crate::tools::registry::ToolRegistryBuilder::new()
        .build(ToolContext::new(fixture.workspace()));
    registry.register(Arc::new(FakeScriptTool));
    let installed = manager.install_tools(&mut registry);
    assert!(installed.is_empty());
    assert_eq!(
        registry
            .get("fixture_script_tool")
            .unwrap()
            .registration_origin(),
        "plugin script fixture_script_tool",
        "the script tool is unaffected"
    );
    assert!(
        manager
            .diagnostics()
            .iter()
            .any(|d| d.contains("fixture_script_tool") && d.contains("skipped")),
        "{:?}",
        manager.diagnostics()
    );
    manager.shutdown().await;
}

#[tokio::test]
async fn with_no_native_plugin_the_host_is_never_spawned() {
    let _policy = TestPolicyGuard::extension_host(true);
    let temp = tempfile::tempdir().unwrap();
    let registry = Arc::new(PluginRegistry::empty(temp.path()));
    let manager = ExtensionHostManager::new(ExtensionHostOptions {
        node_override: None,
        root: Some(temp.path().join("home")),
    });
    manager.sync(registry).await.unwrap();
    assert_eq!(manager.spawn_attempts(), 0);
    assert_eq!(manager.status(), HostStatus::Idle);
    assert!(!temp.path().join("home").exists(), "nothing materialized");
}

async fn probe(tool: &Arc<dyn ToolSpec>, path: &Path, context: &ToolContext) -> Value {
    let result = tool
        .execute(json!({"path": path.to_string_lossy()}), context)
        .await
        .unwrap();
    serde_json::from_str(&result.content).unwrap()
}

#[tokio::test]
async fn sandboxed_host_cannot_read_codewhale_secrets_or_write_outside_its_data_dir() {
    let Some(node) = node_for_tests("sandboxed_host") else {
        return;
    };
    let _policy = TestPolicyGuard::extension_host(true);
    let fixture = FixturePlugins::new(&["secret-probe"]);
    // Created before launch: the deny-list records the canonical spelling of
    // paths that exist (macOS `/var` → `/private/var`).
    let secrets = fixture.root.join("secrets");
    std::fs::create_dir_all(&secrets).unwrap();
    let token = secrets.join("token");
    std::fs::write(&token, "s3cret-value").unwrap();
    let readable = fixture.root.join("readable.txt");
    std::fs::write(&readable, "plain").unwrap();

    let manager = fixture.manager(node);
    manager.sync(fixture.registry()).await.unwrap();
    let HostStatus::Ready { sandbox, .. } = manager.status() else {
        panic!("host not ready: {:?}", manager.status());
    };
    let Some(sandbox) = sandbox else {
        assert!(
            cfg!(windows) || crate::sandbox::get_platform_sandbox().is_none(),
            "an OS sandbox is available here, so the host must run under it"
        );
        eprintln!("skipping sandbox assertions: no OS sandbox for the host on this platform");
        manager.shutdown().await;
        return;
    };
    assert!(super::render_status(&manager).contains(&format!("{sandbox} sandbox")));

    let context = ToolContext::new(fixture.workspace());
    let read = host_tool(&manager, fixture.workspace(), "probe_read");
    let write = host_tool(&manager, fixture.workspace(), "probe_write");

    let plain = probe(&read, &readable, &context).await;
    assert_eq!(
        plain,
        json!({"ok": true, "text": "plain"}),
        "ordinary reads work"
    );
    let secret = probe(&read, &token, &context).await;
    assert_eq!(secret["ok"], false, "{secret}");
    assert!(!secret.to_string().contains("s3cret"), "{secret}");

    let data = fixture.root.join("extension-host/data/probe.txt");
    assert_eq!(probe(&write, &data, &context).await["ok"], true);
    // Outside the data dir and the temp dirs (the fixture itself lives under
    // TMPDIR, which the profile leaves writable): the crate's source dir,
    // unless the checkout itself sits in a temp dir.
    let crate_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
    let in_temp = [std::env::temp_dir(), PathBuf::from("/tmp")]
        .iter()
        .filter_map(|dir| std::fs::canonicalize(dir).ok())
        .any(|dir| crate_dir.starts_with(dir));
    if !in_temp {
        let escape = crate_dir.join(format!(".ext-host-probe-{}", uuid::Uuid::new_v4().simple()));
        let escaped = probe(&write, &escape, &context).await;
        let leaked = escape.exists();
        let _ = std::fs::remove_file(&escape);
        assert_eq!(escaped["ok"], false, "{escaped}");
        assert!(!leaked);
    }
    manager.shutdown().await;
}
