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
