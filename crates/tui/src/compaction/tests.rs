use super::*;

fn report(error: &anyhow::Error) -> String {
    report_compaction_failure("Auto-compaction failed", "compact_fixture", true, error)
}

#[test]
fn untyped_usage_limit_text_never_becomes_quota_exhaustion() {
    let error = anyhow::anyhow!(
        "[auth] Authorization failed: You've reached your usage limit for this billing cycle"
    );
    let message = report(&error);
    assert!(message.contains("provider rate limit blocked compaction"));
    assert!(!message.contains("quota exhausted"));
}

#[test]
fn typed_quota_renders_quota_and_is_not_transient() {
    let error = anyhow::Error::new(crate::llm_client::LlmError::from_http_response(
        429,
        r#"{"error":{"code":"insufficient_quota"}}"#,
    ))
    .context("summary request failed");
    assert_eq!(
        report(&error),
        "Auto-compaction failed: provider plan quota exhausted — switch provider/model or renew the provider plan"
    );
    assert!(!is_transient_error(&error));
    assert!(!should_retry_cache_aligned_with_formatted(&error));
}

#[test]
fn typed_rate_limit_stays_transient_and_does_not_become_quota() {
    let error = anyhow::Error::new(crate::llm_client::LlmError::RateLimited {
        message: "Too Many Requests".into(),
        retry_after: None,
    });
    assert!(report(&error).contains("provider rate limit blocked compaction"));
    assert!(is_transient_error(&error));
    assert!(should_retry_cache_aligned_with_formatted(&error));
}

#[test]
fn unknown_diagnostic_is_preserved_safely() {
    let error = anyhow::anyhow!("summary response was structurally empty");
    assert_eq!(
        report(&error),
        "Auto-compaction failed: summary response was structurally empty"
    );
}

#[test]
fn untyped_transient_and_deterministic_classification_remains_compatible() {
    for message in [
        "Connection timeout",
        "429 Too Many Requests",
        "503 Service Unavailable",
        "network error: connection refused",
    ] {
        assert!(is_transient_error(&anyhow::anyhow!(message)), "{message}");
    }
    for message in [
        "401 Unauthorized: Invalid API key",
        "Failed to parse JSON response",
        "Invalid request: missing required field",
    ] {
        assert!(!is_transient_error(&anyhow::anyhow!(message)), "{message}");
    }
    assert_eq!(
        classify_compaction_failure(&anyhow::anyhow!(
            "prompt is too long for this model's context window"
        )),
        CompactionFailureKind::ContextOverflow
    );
}

fn pressure_fixture() -> Vec<Message> {
    (0..30)
        .map(|index| Message {
            role: if index % 2 == 0 { "user" } else { "assistant" }.to_string(),
            content: vec![ContentBlock::Text {
                text: "x".repeat(8_000),
                cache_control: None,
            }],
        })
        .collect()
}

fn oversized_tool_pair(id: &str, content: String) -> Vec<Message> {
    vec![
        Message {
            role: "assistant".to_string(),
            content: vec![ContentBlock::ToolUse {
                id: id.to_string(),
                name: "read_file".to_string(),
                input: serde_json::json!({"path": "src/compaction.rs"}),
                caller: None,
            }],
        },
        Message {
            role: "user".to_string(),
            content: vec![ContentBlock::ToolResult {
                tool_use_id: id.to_string(),
                content,
                is_error: None,
                content_blocks: None,
            }],
        },
    ]
}

#[test]
fn pinned_tool_result_local_pruning_is_reclaimable() {
    let mut messages =
        oversized_tool_pair("old-read", "error: ".to_string() + &"x".repeat(300_000));
    messages.extend(pressure_fixture());
    let external_pins = [0, 1];
    let full_pressure = estimate_input_tokens_conservative(&messages, None);
    let mut projected = messages.clone();
    let pruned_bytes = prune_tool_results_until(&mut projected, KEEP_RECENT_MESSAGES, |_, _| false);
    let projected_pressure = estimate_input_tokens_conservative(&projected, None);
    assert!(
        pruned_bytes > 250_000,
        "fixture must prune the pinned result"
    );
    assert!(projected_pressure < full_pressure);

    let config = CompactionConfig {
        token_threshold: projected_pressure + (full_pressure - projected_pressure) / 2,
        ..Default::default()
    };
    assert!(compaction_pressure_reached(&messages, None, &config));
    assert!(!compaction_pressure_reached(&projected, None, &config));
    assert!(should_compact(
        &messages,
        None,
        &PreparedCompactionEnvelope::new(config, None),
        None,
        Some(&external_pins),
        None,
    ));
}

#[test]
fn retained_tool_result_cap_is_used_in_successor_floor() {
    let mut messages = pressure_fixture();
    messages.extend(oversized_tool_pair(
        "recent-read",
        "z".repeat(RETAINED_TOOL_RESULT_MAX_CHARS * 4),
    ));
    let plan = plan_compaction(&messages, None, KEEP_RECENT_MESSAGES, None, None);
    let base_config = CompactionConfig::default();
    let prepared = PreparedCompactionEnvelope::new(base_config.clone(), None);
    let retained_floor =
        estimate_retained_floor_conservative(&messages, None, &plan, &prepared, None);
    let raw_pinned_tokens = plan
        .pinned_indices
        .iter()
        .map(|&index| {
            let message = &messages[index];
            estimate_tokens_for_message(message, message_has_tool_use(message))
        })
        .sum::<usize>()
        .saturating_mul(3)
        .div_ceil(2);
    assert!(
        retained_floor < raw_pinned_tokens,
        "the retained-message cap must reduce the nominal pinned floor"
    );

    let config = CompactionConfig {
        token_threshold: retained_floor + 1,
        ..base_config
    };
    assert!(compaction_pressure_reached(&messages, None, &config));
    assert!(raw_pinned_tokens >= config.token_threshold);
    assert!(should_compact(
        &messages,
        None,
        &PreparedCompactionEnvelope::new(config, None),
        None,
        None,
        None,
    ));
}

#[test]
fn exact_unbounded_reanchor_controls_reclaimability() {
    let messages = pressure_fixture();
    let plan = plan_compaction(&messages, None, KEEP_RECENT_MESSAGES, None, None);
    let mut config = CompactionConfig::default();
    let without_reanchor = PreparedCompactionEnvelope::new(config.clone(), None);
    let retained_floor =
        estimate_retained_floor_conservative(&messages, None, &plan, &without_reanchor, None);
    let pressure = estimate_input_tokens_conservative(&messages, None);
    assert!(
        retained_floor < pressure,
        "fixture must have reclaimable pressure"
    );
    config.token_threshold = retained_floor + (pressure - retained_floor) / 2;

    let without_reanchor = PreparedCompactionEnvelope::new(config.clone(), None);
    assert!(should_compact(
        &messages,
        None,
        &without_reanchor,
        None,
        None,
        None,
    ));

    let external = "x".repeat(300_000);
    let reanchor = SystemPrompt::Text(format!(
        "{}\n- `shell:{external}` - active - exact owner\n{}",
        crate::work_graph::ACTIVE_OPERATION_SUMMARY_START,
        crate::work_graph::ACTIVE_OPERATION_SUMMARY_END,
    ));
    assert!(
        estimate_system_tokens_conservative(Some(&reanchor)) > 4_096,
        "fixture must exceed the retired fixed reserve"
    );
    let with_exact_reanchor = PreparedCompactionEnvelope::new(config, Some(reanchor));
    assert!(
        !should_compact(&messages, None, &with_exact_reanchor, None, None, None,),
        "an unbounded exact reanchor must make an unreclaimable successor ineligible"
    );
}

#[test]
fn stale_installed_reanchor_is_replaced_not_double_counted() {
    let messages = pressure_fixture();
    let stale_external = "y".repeat(300_000);
    let base_system = SystemPrompt::Text("stable base prompt".to_string());
    let current_system = SystemPrompt::Text(format!(
        "stable base prompt\n{}\n- `shell:{stale_external}` - active - stale owner\n{}",
        crate::work_graph::ACTIVE_OPERATION_SUMMARY_START,
        crate::work_graph::ACTIVE_OPERATION_SUMMARY_END,
    ));
    let current_reanchor = SystemPrompt::Text(format!(
        "{}\n- `shell:current` - active - current owner\n{}",
        crate::work_graph::ACTIVE_OPERATION_SUMMARY_START,
        crate::work_graph::ACTIVE_OPERATION_SUMMARY_END,
    ));
    let plan = plan_compaction(&messages, None, KEEP_RECENT_MESSAGES, None, None);
    let mut config = CompactionConfig::default();
    let prepared = PreparedCompactionEnvelope::new(config.clone(), Some(current_reanchor.clone()));
    let retained_floor = estimate_retained_floor_conservative(
        &messages,
        Some(&current_system),
        &plan,
        &prepared,
        None,
    );
    let base_retained_floor =
        estimate_retained_floor_conservative(&messages, Some(&base_system), &plan, &prepared, None);
    assert_eq!(
        retained_floor, base_retained_floor,
        "the stale installed reanchor must be stripped before sizing its exact replacement"
    );
    let pressure = estimate_input_tokens_conservative(&messages, Some(&current_system));
    assert!(
        retained_floor < pressure,
        "stale reanchor must create pressure"
    );
    config.token_threshold = retained_floor + 1;

    let prepared = PreparedCompactionEnvelope::new(config, Some(current_reanchor));
    assert!(should_compact(
        &messages,
        Some(&current_system),
        &prepared,
        None,
        None,
        None,
    ));
}
