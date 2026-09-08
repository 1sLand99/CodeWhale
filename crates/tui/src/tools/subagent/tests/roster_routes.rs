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
        let row = resolved_role_roster_entry(&runtime, &FleetRole::Worker).await;
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
    let row = resolved_role_roster_entry(&runtime, &FleetRole::Worker).await;
    assert!(row["route"].is_null());
    assert!(row["route_error"].as_str().is_some());
    let other = resolved_role_roster_entry(&runtime, &FleetRole::Reviewer).await;
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
