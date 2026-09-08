//! Consumer regressions for operator-visible routes (#5915/#5955).
use super::*;

#[tokio::test]
async fn roster_matches_actual_start_receipts_and_refreshes_live_role_defaults() {
    let _env = crate::test_support::lock_test_env();
    let root = tempdir().unwrap();
    let (client, calls, _) = delayed_chat_client(Duration::ZERO, "done").await;
    let config = crate::config::Config {
        api_key: Some("test-key".into()),
        base_url: Some(client.base_url().into()),
        subagents: Some(crate::config::SubagentsConfig {
            worker_model: Some("deepseek-v4-flash".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let manager = new_shared_subagent_manager(root.path().to_path_buf(), 8);
    let context = ToolContext::new(root.path()).with_state_namespace("roster-route-consumer");
    let mut runtime = SubAgentRuntime::new(
        client,
        "deepseek-v4-pro".into(),
        context.clone(),
        false,
        None,
        manager.clone(),
    )
    .with_api_config(config);
    runtime
        .role_models
        .insert("general".into(), "deepseek-v4-pro".into());
    let tool = AgentTool::new(manager.clone(), runtime);
    let query = tool
        .execute(json!({"action":"roster"}), &context)
        .await
        .unwrap();
    let roster: Value = serde_json::from_str(&query.content).unwrap();
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "discovery must not send inference"
    );
    let rows = roster["members"].as_array().unwrap();
    assert_eq!(rows.len(), 8);
    assert_eq!(
        rows[0]["route"]["model"], "deepseek-v4-flash",
        "live config supersedes launch default"
    );
    for row in rows {
        assert!(row["route"].is_object(), "route missing: {row}");
        assert_eq!(row["route"]["reachability"], "unverified");
        let role = row["role"].as_str().unwrap();
        let mut request = json!({"action":"start", "type":role, "prompt":"Say done."});
        if role == "custom" {
            request["allowed_tools"] = json!(["Read"]);
        }
        let started = tool.execute(request, &context).await.unwrap();
        let metadata = started.metadata.as_ref().unwrap();
        let receipt = &metadata["child_route"];
        for (discovery, dispatch) in [
            ("provider", "provider_id"),
            ("model", "model_id"),
            ("reasoning_effort", "effective_reasoning"),
            ("source", "route_source"),
        ] {
            assert_eq!(
                row["route"][discovery], receipt[dispatch],
                "{role}: {discovery}"
            );
        }
        manager
            .write()
            .await
            .cancel_agent(metadata["agent_id"].as_str().unwrap())
            .unwrap();
    }
}

#[tokio::test]
async fn roster_preserves_unknown_and_non_metered_costs_and_invalid_role_errors() {
    let _env = crate::test_support::lock_test_env();
    let _live = crate::provider_lake::lock_live_snapshot();
    crate::provider_lake::clear_live_snapshot();
    for (provider, model, vendor, expected_cost, reason) in [
        ("deepseek", "deepseek-v4-flash", None, "paid", None),
        (
            "openrouter",
            "qwen/qwen3.7-plus",
            Some("cerebras"),
            "unknown",
            Some("routing_dependent_price"),
        ),
        (
            "ollama",
            "fixture-local-model",
            None,
            "not_money_metered",
            Some("not_money_metered"),
        ),
    ] {
        let root = tempdir().unwrap();
        let mut config = crate::config::Config {
            provider: Some(provider.into()),
            ..Default::default()
        };
        let selected = config.provider_config_for_mut(ApiProvider::parse(provider).unwrap());
        selected.api_key = Some("roster-private-fixture-key".into());
        selected.model = Some(model.into());
        selected.vendor = vendor.map(str::to_string);
        let client = DeepSeekClient::new(&config).unwrap();
        let manager = new_shared_subagent_manager(root.path().to_path_buf(), 1);
        let runtime = SubAgentRuntime::new(
            client,
            model.into(),
            ToolContext::new(root.path()),
            false,
            None,
            manager,
        )
        .with_api_config(config);
        let row =
            resolved_role_roster_entry(&runtime, &spawn_roster(&runtime), &FleetRole::Worker).await;
        assert_eq!(
            row["route"]["cost_class"], expected_cost,
            "{provider}: {row}"
        );
        assert_eq!(row["route"]["unpriced_reason"], json!(reason));
        assert!(!row.to_string().contains("roster-private-fixture-key"));
    }
    let mut runtime = stub_runtime();
    runtime
        .role_models
        .insert("general".into(), "invalid\nmodel".into());
    let row =
        resolved_role_roster_entry(&runtime, &spawn_roster(&runtime), &FleetRole::Worker).await;
    assert!(row["route"].is_null());
    assert!(row["route_error"].as_str().is_some());
    let other =
        resolved_role_roster_entry(&runtime, &spawn_roster(&runtime), &FleetRole::Reviewer).await;
    assert!(
        other["route"].is_object(),
        "one bad role must not hide other routes: {other}"
    );
}

#[tokio::test]
async fn advertised_task_route_overrides_reach_start_and_foreign_models_fail_before_admission() {
    let _env = crate::test_support::lock_test_env();
    let root = tempdir().unwrap();
    let (client, _, _) = delayed_chat_client(Duration::ZERO, "done").await;
    let config = crate::config::Config {
        api_key: Some("test-key".into()),
        base_url: Some(client.base_url().into()),
        ..Default::default()
    };
    let manager = new_shared_subagent_manager(root.path().to_path_buf(), 2);
    let context = ToolContext::new(root.path()).with_state_namespace("explicit-task-route");
    let runtime = SubAgentRuntime::new(
        client,
        "deepseek-v4-flash".into(),
        context.clone(),
        false,
        None,
        manager.clone(),
    )
    .with_api_config(config);
    let tool = AgentTool::new(manager.clone(), runtime);
    let schema = tool.input_schema();
    for field in ["model", "model_strength", "thinking"] {
        assert!(schema["properties"].get(field).is_some());
    }
    let started = tool.execute(json!({"action":"start", "type":"explore", "prompt":"Say done.", "model":"deepseek-v4-pro", "model_strength":"faster", "thinking":"high"}), &context).await.unwrap();
    let metadata = started.metadata.as_ref().unwrap();
    let receipt = &metadata["child_route"];
    assert_eq!(receipt["model_id"], "deepseek-v4-pro");
    assert_eq!(receipt["route_source"], "task.model");
    assert_eq!(receipt["effective_reasoning"], "high");
    manager
        .write()
        .await
        .cancel_agent(metadata["agent_id"].as_str().unwrap())
        .unwrap();
    let error = tool.execute(json!({"action":"start", "type":"explore", "prompt":"Say done.", "model":"claude-fable-5"}), &context).await.unwrap_err();
    assert!(error.to_string().contains("provider"), "{error}");
}

struct ProjectProfilesGuard(bool);
impl ProjectProfilesGuard {
    fn enabled() -> Self {
        let previous = crate::fleet::roster::project_agent_profiles_enabled();
        crate::fleet::roster::set_project_agent_profiles_enabled(true);
        Self(previous)
    }
}
impl Drop for ProjectProfilesGuard {
    fn drop(&mut self) {
        crate::fleet::roster::set_project_agent_profiles_enabled(self.0);
    }
}

#[tokio::test]
async fn saved_profile_discovery_and_actual_start_share_current_instructions_route_and_trust() {
    let _env = crate::test_support::lock_test_env();
    let root = tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("state"));
    let _project = ProjectProfilesGuard::enabled();
    let profile_dir = root.path().join(".codewhale/agents");
    std::fs::create_dir_all(&profile_dir).unwrap();
    let profile = profile_dir.join("bug-hunter.toml");
    let (client, calls, bodies) = delayed_chat_client(Duration::ZERO, "done").await;
    let config = crate::config::Config {
        api_key: Some("test-key".into()),
        base_url: Some(client.base_url().into()),
        subagents: Some(crate::config::SubagentsConfig {
            explorer_model: Some("invalid\nrole-default".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let manager = new_shared_subagent_manager(root.path().to_path_buf(), 2);
    let context = ToolContext::new(root.path()).with_state_namespace("saved-profile-consumer");
    let runtime = SubAgentRuntime::new(
        client,
        "deepseek-v4-flash".into(),
        context.clone(),
        false,
        None,
        manager.clone(),
    )
    .with_api_config(config);
    let tool = AgentTool::new(manager.clone(), runtime);
    for (model, instruction) in [
        ("deepseek-v4-pro", "Inspect only changed parser branches."),
        ("deepseek-v4-flash", "Inspect the new queue consumer."),
    ] {
        std::fs::write(&profile, format!("id = \"bug-hunter\"\nbase_role = \"scout\"\nmodel = \"{model}\"\nreasoning_effort = \"high\"\npersona = \"{instruction}\"\n")).unwrap();
        let before = calls.load(Ordering::SeqCst);
        let discovered = tool
            .execute(json!({"action":"roster"}), &context)
            .await
            .unwrap();
        assert_eq!(
            calls.load(Ordering::SeqCst),
            before,
            "roster must never infer"
        );
        let roster: Value = serde_json::from_str(&discovered.content).unwrap();
        let row = roster["profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["member_id"] == "bug-hunter")
            .unwrap();
        assert_eq!(row["route"]["model"], model);
        assert_eq!(row["route"]["reasoning_effort"], "high");
        let started = tool
            .execute(
                json!({"profile":"bug-hunter", "prompt":"Inspect the assigned slice."}),
                &context,
            )
            .await
            .unwrap();
        let meta = started.metadata.as_ref().unwrap();
        let receipt = &meta["child_route"];
        assert_eq!(receipt["resolved_profile_id"], "bug-hunter");
        assert_eq!(receipt["profile_origin"], "project");
        assert_eq!(receipt["model_id"], row["route"]["model"]);
        assert_eq!(
            receipt["effective_reasoning"],
            row["route"]["reasoning_effort"]
        );
        assert_eq!(receipt["route_source"], "agent_profile.model");
        tokio::time::timeout(Duration::from_secs(5), async {
            while calls.load(Ordering::SeqCst) == before {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("local provider receives the saved profile prompt");
        let body = bodies.lock().unwrap().last().unwrap().clone();
        assert!(
            body.to_string().contains(instruction),
            "saved instructions must reach the actual request"
        );
        let id = meta["agent_id"].as_str().unwrap();
        let mut guard = manager.write().await;
        let worker = guard.worker_records.get(id).unwrap();
        assert!(
            worker
                .spec
                .launch_manifest
                .as_ref()
                .unwrap()
                .prompt
                .contains(instruction)
        );
        assert!(!worker.spec.runtime_profile.permissions.write);
        if guard.agents[id].status == SubAgentStatus::Running {
            guard.cancel_agent(id).unwrap();
        }
    }
    crate::fleet::roster::set_project_agent_profiles_enabled(false);
    let error = tool
        .execute(
            json!({"profile":"bug-hunter", "prompt":"Inspect."}),
            &context,
        )
        .await
        .unwrap_err();
    assert!(
        error.to_string().contains("Unknown Fleet role/profile"),
        "{error}"
    );
}

#[tokio::test]
async fn saved_provider_pin_reaches_actual_request_and_conflicts_fail_before_admission() {
    let _env = crate::test_support::lock_test_env();
    let root = tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("state"));
    let _project = ProjectProfilesGuard::enabled();
    let profile_dir = root.path().join(".codewhale/agents");
    std::fs::create_dir_all(&profile_dir).unwrap();
    std::fs::write(profile_dir.join("router-review.toml"), "id = \"router-review\"\nbase_role = \"reviewer\"\nprovider = \"openrouter\"\nmodel = \"qwen/qwen3.7-plus\"\nreasoning_effort = \"low\"\n").unwrap();
    let (client, calls, bodies) = delayed_chat_client(Duration::ZERO, "done").await;
    let mut config = crate::config::Config {
        api_key: Some("test-key".into()),
        base_url: Some(client.base_url().into()),
        ..Default::default()
    };
    let router = config.provider_config_for_mut(ApiProvider::Openrouter);
    router.api_key = Some("test-router-key".into());
    router.base_url = Some(client.base_url().into());
    router.vendor = Some("cerebras".into());
    let manager = new_shared_subagent_manager(root.path().to_path_buf(), 2);
    let context = ToolContext::new(root.path()).with_state_namespace("saved-provider-consumer");
    let runtime = SubAgentRuntime::new(
        client,
        "deepseek-v4-flash".into(),
        context.clone(),
        false,
        None,
        manager.clone(),
    )
    .with_api_config(config);
    let tool = AgentTool::new(manager.clone(), runtime);
    for extra in [
        json!({"model":"deepseek-v4-pro"}),
        json!({"model_strength":"faster"}),
        json!({"type":"builder"}),
    ] {
        let mut input = json!({"profile":"router-review", "prompt":"Inspect."});
        input
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(tool.execute(input, &context).await.is_err());
    }
    assert!(manager.read().await.agents.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let discovered = tool
        .execute(json!({"action":"roster"}), &context)
        .await
        .unwrap();
    let roster: Value = serde_json::from_str(&discovered.content).unwrap();
    let row = roster["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["member_id"] == "router-review")
        .unwrap();
    assert_eq!(row["route"]["provider"], "openrouter");
    assert_eq!(row["route"]["openrouter_vendor"], "cerebras");
    assert_eq!(row["route"]["cost_class"], "unknown");
    let started = tool
        .execute(
            json!({"profile":"router-review", "prompt":"Say done.", "thinking":"high"}),
            &context,
        )
        .await
        .unwrap();
    let meta = started.metadata.as_ref().unwrap();
    assert_eq!(meta["child_route"]["provider_id"], "openrouter");
    assert_eq!(meta["child_route"]["model_id"], "qwen/qwen3.7-plus");
    assert_eq!(meta["child_route"]["effective_reasoning"], "high");
    tokio::time::timeout(Duration::from_secs(5), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("local provider fixture receives child request");
    let body = bodies.lock().unwrap()[0].clone();
    assert_eq!(body["model"], "qwen/qwen3.7-plus");
    assert_eq!(body["provider"]["order"], json!(["cerebras"]));
    assert_eq!(body["provider"]["allow_fallbacks"], false);
    assert!(
        !serde_json::to_string(meta)
            .unwrap()
            .contains("test-router-key")
    );
    let id = meta["agent_id"].as_str().unwrap();
    if manager.read().await.agents[id].status == SubAgentStatus::Running {
        manager.write().await.cancel_agent(id).unwrap();
    }
}

#[tokio::test]
async fn saved_profile_cannot_widen_parent_posture_or_depth_and_missing_provider_fails_closed() {
    let _env = crate::test_support::lock_test_env();
    let root = tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("state"));
    let (client, calls, _) = delayed_chat_client(Duration::ZERO, "done").await;
    let mut profile = codewhale_config::FleetProfile::default();
    profile.role.name = "builder".into();
    profile.model = Some("deepseek-v4-flash".into());
    profile.delegation.max_spawn_depth = Some(0);
    profile.permissions.allow_shell = true;
    profile.permissions.trust = true;
    let mut config = crate::config::Config {
        api_key: Some("test-key".into()),
        base_url: Some(client.base_url().into()),
        ..Default::default()
    };
    let mut fleet = codewhale_config::FleetConfigToml::default();
    fleet
        .profiles
        .insert("bounded-builder".into(), profile.clone());
    profile.provider = Some("unconfigured-private-route".into());
    fleet.profiles.insert("missing-route".into(), profile);
    config.fleet = Some(fleet);
    let manager = new_shared_subagent_manager(root.path().to_path_buf(), 2);
    let context = ToolContext::new(root.path()).with_state_namespace("saved-profile-ceiling");
    let mut runtime = SubAgentRuntime::new(
        client,
        "deepseek-v4-flash".into(),
        context.clone(),
        false,
        None,
        manager.clone(),
    )
    .with_api_config(config);
    runtime.worker_profile = WorkerRuntimeProfile::for_role(FleetRole::Scout);
    runtime.worker_profile.shell = ShellPolicy::None;
    let tool = AgentTool::new(manager.clone(), runtime);
    assert!(
        tool.execute(
            json!({"profile":"missing-route", "prompt":"Inspect."}),
            &context
        )
        .await
        .is_err()
    );
    assert!(manager.read().await.agents.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let started = tool
        .execute(
            json!({"profile":"bounded-builder", "prompt":"Inspect only.", "max_depth":2}),
            &context,
        )
        .await
        .unwrap();
    let id = started.metadata.as_ref().unwrap()["agent_id"]
        .as_str()
        .unwrap();
    let mut guard = manager.write().await;
    let worker = guard.worker_records.get(id).unwrap();
    assert!(!worker.spec.runtime_profile.permissions.write);
    assert_eq!(worker.spec.runtime_profile.shell, ShellPolicy::None);
    assert_eq!(worker.spec.runtime_profile.max_spawn_depth, 0);
    guard.cancel_agent(id).unwrap();
}

#[tokio::test]
async fn selected_fleet_capability_and_broken_selection_refuse_actual_start() {
    use crate::fleet::store::{FleetFile, FleetScope, save_fleet, set_selected};
    let _env = crate::test_support::lock_test_env();
    let root = tempdir().unwrap();
    let _home = crate::test_support::EnvVarGuard::set("CODEWHALE_HOME", root.path().join("state"));
    let (client, calls, _) = delayed_chat_client(Duration::ZERO, "done").await;
    let config = crate::config::Config {
        api_key: Some("test-key".into()),
        base_url: Some(client.base_url().into()),
        ..Default::default()
    };
    let mut fleet = FleetFile::new("Capability fixture".into(), None).unwrap();
    fleet.members.push(serde_json::from_value(json!({
        "id":"visual-review", "role":"reviewer", "provider":"deepseek", "model":"deepseek-v4-flash", "requires":["vision"]
    })).unwrap());
    let path = save_fleet(&fleet, FleetScope::Workspace, root.path()).unwrap();
    set_selected(&fleet.name, FleetScope::Workspace, root.path()).unwrap();
    let manager = new_shared_subagent_manager(root.path().to_path_buf(), 1);
    let context =
        ToolContext::new(root.path()).with_state_namespace("selected-capability-consumer");
    let runtime = SubAgentRuntime::new(
        client,
        "deepseek-v4-flash".into(),
        context.clone(),
        false,
        None,
        manager.clone(),
    )
    .with_api_config(config);
    let tool = AgentTool::new(manager.clone(), runtime);
    let error = tool
        .execute(
            json!({"profile":"visual-review", "prompt":"Inspect image."}),
            &context,
        )
        .await
        .unwrap_err();
    assert!(error.to_string().contains("requires vision"), "{error}");
    assert!(manager.read().await.agents.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    std::fs::write(path, "this is not a Fleet document").unwrap();
    let error = tool
        .execute(json!({"type":"reviewer", "prompt":"Inspect."}), &context)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Selected"), "{error}");
    assert!(manager.read().await.agents.is_empty());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
