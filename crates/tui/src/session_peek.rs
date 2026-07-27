//! Bounded, redacted, read-only transcript peek for the dashboard (#4397).
//!
//! The dashboard needs to show *what a saved session was about* without
//! becoming a second transcript viewer. Three constraints shape this module,
//! and all three are enforced here rather than in the client:
//!
//! * **Bounded on the wire.** The peek carries at most
//!   [`MAX_PEEK_ENTRIES`] entries of at most [`MAX_ENTRY_CHARS`] characters.
//!   Doing this client-side would mean shipping a multi-megabyte transcript to
//!   a browser in order to throw most of it away.
//! * **Redacted.** A saved transcript can contain an API key a user pasted, a
//!   token echoed by a tool, an `Authorization` header in a curl command. The
//!   dashboard is reachable over a LAN; a peek pane is not the place to
//!   re-emit those.
//! * **Read-only and non-live.** A peek is a recording. It carries no turn
//!   status, no "running" flag, nothing that could be mistaken for live state.
//!   Live state comes from a resumed thread and its SSE stream, never from
//!   here — see `runtime_web/app.mjs`'s reply-target rules.
//!
//! Tool payloads are summarised to a kind and a size, never inlined: a tool
//! result is the most likely place for both bulk and secrets.

use serde::Serialize;

use crate::models::ContentBlock;
use crate::session_manager::SavedSession;

/// Most entries a peek carries. The dashboard shows a tail, so this is "the
/// last N exchanges", which is what a peek is for.
pub const MAX_PEEK_ENTRIES: usize = 12;

/// Longest text any single entry carries.
pub const MAX_ENTRY_CHARS: usize = 400;

/// What produced an entry. Deliberately coarse — the peek is not a
/// reconstruction of the turn structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PeekEntryKind {
    User,
    Assistant,
    Reasoning,
    /// A tool call or result, summarised. Never the payload itself.
    Tool,
}

/// One bounded line of recorded conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PeekEntry {
    pub kind: PeekEntryKind,
    /// Already bounded and redacted. Safe to render as text — and only as
    /// text; the client inserts it with `textContent`, never `innerHTML`.
    pub text: String,
    /// True when [`Self::text`] was shortened.
    pub truncated: bool,
    /// True when at least one redaction was applied.
    pub redacted: bool,
}

/// A read-only view of a saved session.
///
/// Note what is absent: no turn status, no `active`, no `running`. A saved
/// session has none of those, and inventing them is the fabricated-live-state
/// failure this whole slice exists to avoid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionPeek {
    pub session_id: String,
    pub title: String,
    pub workspace: std::path::PathBuf,
    pub model: String,
    pub mode: String,
    pub archived: bool,
    pub message_count: usize,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Entries actually carried, oldest-first within the tail.
    pub entries: Vec<PeekEntry>,
    /// How many messages were dropped from the front to fit the bound. The
    /// client shows this rather than implying it has the whole conversation.
    pub omitted_before: usize,
    /// Always true: a peek is a recording of a saved session, never a live
    /// thread. Serialised so a client cannot mistake one payload for the
    /// other even by accident.
    pub live: bool,
}

/// Build a bounded, redacted peek from a loaded session.
#[must_use]
pub fn build_peek(session: &SavedSession, max_entries: usize) -> SessionPeek {
    let max_entries = max_entries.clamp(1, MAX_PEEK_ENTRIES);
    let total = session.messages.len();
    let start = total.saturating_sub(max_entries);

    let entries: Vec<PeekEntry> = session.messages[start..]
        .iter()
        .map(|message| {
            let kind = match message.role.as_str() {
                "user" => PeekEntryKind::User,
                _ => PeekEntryKind::Assistant,
            };
            entry_for_blocks(kind, &message.content)
        })
        .collect();

    SessionPeek {
        session_id: session.metadata.id.clone(),
        title: session.metadata.title.clone(),
        workspace: session.metadata.workspace.clone(),
        model: session.metadata.model.clone(),
        mode: session
            .metadata
            .mode
            .clone()
            .unwrap_or_else(|| "agent".to_string()),
        archived: session.metadata.archived,
        message_count: total,
        updated_at: session.metadata.updated_at,
        entries,
        omitted_before: start,
        live: false,
    }
}

fn entry_for_blocks(default_kind: PeekEntryKind, blocks: &[ContentBlock]) -> PeekEntry {
    let mut kind = default_kind;
    let mut parts: Vec<String> = Vec::new();

    for block in blocks {
        match block {
            ContentBlock::Text { text, .. } => parts.push(text.trim().to_string()),
            ContentBlock::Thinking { thinking, .. } => {
                kind = PeekEntryKind::Reasoning;
                parts.push(thinking.trim().to_string());
            }
            // Tool traffic is summarised, never inlined: it is the most
            // likely carrier of both bulk output and credentials.
            ContentBlock::ToolUse { name, .. } | ContentBlock::ServerToolUse { name, .. } => {
                kind = PeekEntryKind::Tool;
                parts.push(format!("[tool call: {name}]"));
            }
            ContentBlock::ToolResult { content, .. } => {
                kind = PeekEntryKind::Tool;
                parts.push(format!("[tool result: {} chars]", content.chars().count()));
            }
            // Structured tool results are JSON. Report their serialized size
            // rather than their shape: the size is the honest number, and any
            // field of the payload could be a credential.
            ContentBlock::ToolSearchToolResult { content, .. }
            | ContentBlock::CodeExecutionToolResult { content, .. } => {
                kind = PeekEntryKind::Tool;
                parts.push(format!(
                    "[tool result: {} chars]",
                    content.to_string().len()
                ));
            }
            ContentBlock::ImageUrl { .. } => {
                kind = PeekEntryKind::Tool;
                parts.push("[image]".to_string());
            }
        }
    }

    let joined = parts
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let (text, redacted) = redact(&joined);
    let (text, truncated) = bound(&text, MAX_ENTRY_CHARS);

    PeekEntry {
        kind,
        text,
        truncated,
        redacted,
    }
}

fn bound(text: &str, max_chars: usize) -> (String, bool) {
    if text.chars().count() <= max_chars {
        return (text.to_string(), false);
    }
    let kept: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    (format!("{kept}…"), true)
}

/// Placeholder substituted for anything that looks like a credential.
pub const REDACTED_PLACEHOLDER: &str = "[redacted]";

/// Mask credential-shaped substrings.
///
/// Conservative and shape-based: it does not try to understand the text, only
/// to recognise the handful of forms secrets usually take in a transcript.
/// Over-redacting a peek line is cheap; leaking a key over a LAN is not.
#[must_use]
pub fn redact(text: &str) -> (String, bool) {
    let mut out = String::with_capacity(text.len());
    let mut redacted = false;

    for token in text.split_inclusive(char::is_whitespace) {
        let trimmed = token.trim_end();
        let trailing = &token[trimmed.len()..];
        if looks_like_secret(trimmed) {
            out.push_str(REDACTED_PLACEHOLDER);
            out.push_str(trailing);
            redacted = true;
        } else if let Some(masked) = mask_assignment(trimmed) {
            out.push_str(&masked);
            out.push_str(trailing);
            redacted = true;
        } else {
            out.push_str(token);
        }
    }

    (out, redacted)
}

/// Known credential prefixes plus long opaque runs.
fn looks_like_secret(token: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "sk-",
        "sk_",
        "pk_",
        "ghp_",
        "gho_",
        "ghu_",
        "ghs_",
        "github_pat_",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "ASIA",
        "AIza",
        "hf_",
        "Bearer",
    ];
    if PREFIXES
        .iter()
        .any(|prefix| token.len() > prefix.len() && token.starts_with(prefix))
    {
        return true;
    }
    // A long unbroken run of base64/hex-ish characters is almost never prose.
    token.len() >= 32
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '_')
        && token.chars().any(|c| c.is_ascii_digit())
        && token.chars().any(|c| c.is_ascii_alphabetic())
}

/// `key=value`, `token: value`, `--password value` style assignments.
fn mask_assignment(token: &str) -> Option<String> {
    const KEYS: &[&str] = &[
        "api_key",
        "apikey",
        "api-key",
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
        "auth",
        "credential",
    ];
    let (name, sep_index) = token
        .find('=')
        .map(|i| (&token[..i], i))
        .or_else(|| token.find(':').map(|i| (&token[..i], i)))?;
    let normalized = name.trim_start_matches('-').to_ascii_lowercase();
    if !KEYS.contains(&normalized.as_str()) {
        return None;
    }
    if token[sep_index + 1..].trim().is_empty() {
        return None;
    }
    Some(format!(
        "{}{}{REDACTED_PLACEHOLDER}",
        name,
        &token[sep_index..=sep_index]
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Message;
    use crate::session_manager::create_saved_session_with_id_and_mode;

    fn text_block(text: &str) -> ContentBlock {
        ContentBlock::Text {
            text: text.to_string(),
            cache_control: None,
        }
    }

    fn session_with(messages: Vec<Message>) -> SavedSession {
        create_saved_session_with_id_and_mode(
            "peek-session".to_string(),
            &messages,
            "deepseek-chat",
            std::path::Path::new("/repo"),
            10,
            None,
            Some("agent"),
        )
    }

    fn user(text: &str) -> Message {
        Message {
            role: "user".to_string(),
            content: vec![text_block(text)],
        }
    }

    #[test]
    fn peek_is_bounded_in_entries_and_reports_what_it_dropped() {
        let messages: Vec<Message> = (0..40).map(|i| user(&format!("message {i}"))).collect();
        let peek = build_peek(&session_with(messages), MAX_PEEK_ENTRIES);

        assert_eq!(peek.entries.len(), MAX_PEEK_ENTRIES);
        assert_eq!(peek.message_count, 40);
        assert_eq!(peek.omitted_before, 40 - MAX_PEEK_ENTRIES);
        assert!(
            peek.entries.last().expect("tail").text.contains("39"),
            "the peek must be the tail, not the head"
        );
    }

    #[test]
    fn a_request_for_more_than_the_cap_still_gets_the_cap() {
        let messages: Vec<Message> = (0..100).map(|i| user(&format!("m{i}"))).collect();
        let peek = build_peek(&session_with(messages), usize::MAX);
        assert_eq!(peek.entries.len(), MAX_PEEK_ENTRIES);
    }

    #[test]
    fn long_entries_are_truncated_and_flagged() {
        let peek = build_peek(&session_with(vec![user(&"x".repeat(5_000))]), 4);
        let entry = &peek.entries[0];
        assert!(entry.truncated);
        assert!(entry.text.chars().count() <= MAX_ENTRY_CHARS);
    }

    #[test]
    fn credentials_are_redacted_out_of_peek_text() {
        for secret in [
            "sk-abcdefghijklmnopqrstuvwxyz123456",
            "ghp_abcdefghijklmnopqrstuvwxyz1234",
            "AKIAIOSFODNN7EXAMPLE",
        ] {
            let peek = build_peek(&session_with(vec![user(&format!("here: {secret}"))]), 4);
            let entry = &peek.entries[0];
            assert!(entry.redacted, "{secret} should have been redacted");
            assert!(
                !entry.text.contains(secret),
                "peek leaked {secret}: {}",
                entry.text
            );
            assert!(entry.text.contains(REDACTED_PLACEHOLDER));
        }
    }

    #[test]
    fn assignment_style_secrets_are_masked_but_keep_their_key() {
        let (masked, redacted) = redact("api_key=hunter2 and password:swordfish");
        assert!(redacted);
        assert!(masked.contains("api_key="));
        assert!(!masked.contains("hunter2"));
        assert!(!masked.contains("swordfish"));
    }

    #[test]
    fn ordinary_prose_is_not_redacted() {
        let (out, redacted) = redact("Please refactor the lane registry and update the docs.");
        assert!(!redacted);
        assert_eq!(
            out,
            "Please refactor the lane registry and update the docs."
        );
    }

    #[test]
    fn tool_payloads_are_summarised_never_inlined() {
        let message = Message {
            role: "assistant".to_string(),
            content: vec![
                ContentBlock::ToolUse {
                    id: "call-1".to_string(),
                    name: "read_file".to_string(),
                    input: serde_json::json!({ "path": "/etc/shadow" }),
                    caller: None,
                },
                ContentBlock::ToolResult {
                    tool_use_id: "call-1".to_string(),
                    content: "root:$6$verysecrethash".to_string(),
                    is_error: None,
                    content_blocks: None,
                },
            ],
        };
        let peek = build_peek(&session_with(vec![message]), 4);
        let entry = &peek.entries[0];

        assert_eq!(entry.kind, PeekEntryKind::Tool);
        assert!(entry.text.contains("[tool call: read_file]"));
        assert!(entry.text.contains("[tool result:"));
        assert!(
            !entry.text.contains("verysecrethash"),
            "tool output must never be inlined into a peek: {}",
            entry.text
        );
        assert!(
            !entry.text.contains("/etc/shadow"),
            "tool input must not be inlined either: {}",
            entry.text
        );
    }

    #[test]
    fn a_peek_never_claims_to_be_live() {
        let peek = build_peek(&session_with(vec![user("hello")]), 4);
        assert!(!peek.live, "a saved session is a recording, never live");
        let json = serde_json::to_value(&peek).expect("serialize");
        for forbidden in ["status", "running", "active", "turn"] {
            assert!(
                json.get(forbidden).is_none(),
                "peek payload must not carry a `{forbidden}` field a client could read as live state"
            );
        }
    }

    #[test]
    fn archive_state_rides_along_so_the_dashboard_need_not_guess() {
        let mut session = session_with(vec![user("hello")]);
        session.metadata.archived = true;
        assert!(build_peek(&session, 4).archived);
    }
}
