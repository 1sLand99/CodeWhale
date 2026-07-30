use super::*;

#[test]
fn raw_rate_and_quota_phrases_remain_coarse_rate_limit_diagnostics() {
    for message in [
        "Rate limit reached for gpt-4",
        "Too Many Requests",
        "HTTP 429 from upstream",
        "Your quota has been exceeded",
        "Authorization failed: You've reached your usage limit for this billing cycle",
    ] {
        assert_eq!(classify_error_message(message), ErrorCategory::RateLimit);
    }
}

#[test]
fn typed_llm_quota_envelope_is_non_recoverable_and_distinct_from_rate_limit() {
    let envelope = ErrorEnvelope::from(LlmError::from_http_response(
        429,
        r#"{"error":{"code":"insufficient_quota"}}"#,
    ));
    assert_eq!(envelope.category, ErrorCategory::RateLimit);
    assert_eq!(envelope.severity, ErrorSeverity::Error);
    assert!(!envelope.recoverable);
    assert_eq!(envelope.code, "llm_quota_exhausted");
}

#[test]
fn llm_auth_error_envelope_renders_context_without_secret() {
    let api_key = "tp-secret-token-plan-value";
    let envelope = ErrorEnvelope::from(LlmError::from_http_response_with_request_context(
        401,
        &format!("Invalid API Key: {api_key}"),
        Some("Xiaomi MiMo"),
        Some("https://token-plan-sgp.xiaomimimo.com/v1"),
        Some("mimo-v2.5"),
        Some("env"),
        Some(api_key),
    ));
    assert_eq!(envelope.category, ErrorCategory::Authentication);
    assert_eq!(envelope.severity, ErrorSeverity::Critical);
    assert!(!envelope.recoverable);
    for expected in [
        "provider: Xiaomi MiMo",
        "base URL authority: token-plan-sgp.xiaomimimo.com",
        "model: mimo-v2.5",
        "key source: env",
        "key fingerprint: tp-... (len=26)",
        "key type: Xiaomi MiMo Token Plan key",
    ] {
        assert!(envelope.message.contains(expected));
    }
    assert!(!envelope.message.contains(api_key));
    assert!(!envelope.message.contains("secret-token-plan-value"));
}
