//! Static intent extraction for RLM code blocks.
//!
//! When the Python REPL is the model's primary surface, one block can perform
//! many operations that would each have been a separate, individually approved
//! tool call. Prompting per call makes the surface unusable; approving a blind
//! block makes it unsafe. This module is the middle path: read the block
//! before it runs and report what it intends to do, so a single approval can
//! still be an informed one.
//!
//! **It fails toward disclosure.** A binding called with a literal argument is
//! reported exactly. A binding called with a computed argument — an f-string, a
//! variable, a loop — cannot be known before execution, so it is recorded as
//! [`Undecidable`](IntentKind::Undecidable) rather than omitted or guessed. A
//! manifest that under-reports is worse than no manifest, because it converts
//! "I don't know" into "nothing will happen".
//!
//! This is deliberately *not* a Python parser. It recognizes call sites of the
//! known bindings and classifies their first argument. Anything it cannot read
//! confidently becomes an undecidable entry, which the caller must gate at the
//! call itself.

use std::collections::BTreeSet;

/// A binding that reaches a gated capability. Kept as an explicit list rather
/// than "anything that looks like a call" so adding a capability to the REPL
/// is a deliberate act that shows up in review.
const GATED_BINDINGS: &[(&str, IntentKind)] = &[
    ("bash", IntentKind::Command),
    ("shell", IntentKind::Command),
    ("run", IntentKind::Command),
    ("edit", IntentKind::FileWrite),
    ("write", IntentKind::FileWrite),
    ("apply_patch", IntentKind::FileWrite),
    ("agent", IntentKind::Agent),
    ("sub_query", IntentKind::Agent),
    ("fetch", IntentKind::Network),
    ("web_fetch", IntentKind::Network),
];

/// What a single call site intends to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntentKind {
    Command,
    FileWrite,
    Agent,
    Network,
    /// A gated binding whose argument could not be read statically.
    Undecidable,
}

impl IntentKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Command => "run command",
            Self::FileWrite => "write file",
            Self::Agent => "spawn agent",
            Self::Network => "network fetch",
            Self::Undecidable => "unknown until it runs",
        }
    }
}

/// One intended operation found in a block.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Intent {
    pub kind: IntentKind,
    /// The binding that was called, e.g. `bash`.
    pub binding: String,
    /// The literal first argument when it could be read; `None` when the call
    /// computes it, which is exactly the case the caller must gate at runtime.
    pub detail: Option<String>,
}

/// Everything a block intends, ready to render as one approval prompt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockIntent {
    pub intents: Vec<Intent>,
}

impl BlockIntent {
    /// True when every gated call was read statically, so the manifest is a
    /// complete description of the block and one approval can stand for all
    /// of it. False means at least one call must still be gated when reached.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        !self
            .intents
            .iter()
            .any(|intent| intent.kind == IntentKind::Undecidable)
    }

    /// True when the block reaches nothing gated and needs no approval at all
    /// — the common case for pure inspection over context.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.intents.is_empty()
    }

    /// A short, human-facing summary: one line per distinct operation, with
    /// counts. Bounded so a pathological block cannot flood the prompt.
    #[must_use]
    pub fn summary_lines(&self, max_lines: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut seen: BTreeSet<(IntentKind, Option<String>)> = BTreeSet::new();
        for intent in &self.intents {
            let key = (intent.kind, intent.detail.clone());
            if !seen.insert(key) {
                continue;
            }
            let line = match (&intent.detail, intent.kind) {
                (Some(detail), IntentKind::Undecidable) => {
                    format!("{}: {detail} (computed at runtime)", intent.binding)
                }
                (Some(detail), kind) => format!("{}: {detail}", kind.label()),
                (None, kind) => format!("{}: {} (argument computed)", kind.label(), intent.binding),
            };
            lines.push(line);
            if lines.len() >= max_lines {
                let remaining = self.intents.len().saturating_sub(lines.len());
                if remaining > 0 {
                    lines.push(format!("… and {remaining} more"));
                }
                break;
            }
        }
        lines
    }
}

/// Read a block and report what it intends.
///
/// Comments and string bodies are skipped so a binding name mentioned in prose
/// or inside a quoted string is not reported as a call — over-reporting trains
/// people to approve without reading, which is its own failure.
#[must_use]
pub fn scan(code: &str) -> BlockIntent {
    let mut intents = Vec::new();
    for (binding, kind) in GATED_BINDINGS {
        for position in call_sites(code, binding) {
            let detail = first_literal_argument(&code[position..]);
            let kind = if detail.is_some() {
                *kind
            } else {
                IntentKind::Undecidable
            };
            intents.push(Intent {
                kind,
                binding: (*binding).to_string(),
                detail: detail.map(|value| truncate(&value, 80)),
            });
        }
    }
    intents.sort();
    BlockIntent { intents }
}

/// Byte offsets just past `name(` for each call site outside comments and
/// strings.
fn call_sites(code: &str, name: &str) -> Vec<usize> {
    let bytes = code.as_bytes();
    let mut sites = Vec::new();
    let mut index = 0usize;
    let mut in_string: Option<u8> = None;
    let mut in_comment = false;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_comment {
            if byte == b'\n' {
                in_comment = false;
            }
            index += 1;
            continue;
        }
        if let Some(quote) = in_string {
            if byte == b'\\' {
                index += 2;
                continue;
            }
            if byte == quote {
                in_string = None;
            }
            index += 1;
            continue;
        }
        match byte {
            b'#' => {
                in_comment = true;
                index += 1;
                continue;
            }
            b'"' | b'\'' => {
                in_string = Some(byte);
                index += 1;
                continue;
            }
            _ => {}
        }
        if code[index..].starts_with(name) {
            let before_is_ident = index > 0 && is_ident_byte(bytes[index - 1]);
            let after = index + name.len();
            // `foo.bash(` is a method on something else, not our binding.
            let is_attribute = index > 0 && bytes[index - 1] == b'.';
            if !before_is_ident
                && !is_attribute
                && bytes.get(after).copied() == Some(b'(')
                && code.is_char_boundary(after + 1)
            {
                sites.push(after + 1);
            }
        }
        index += 1;
    }
    sites
}

const fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The first argument when it is a plain string literal. f-strings, variables,
/// concatenations, and calls all return `None` — those are undecidable, and
/// saying so is the point.
fn first_literal_argument(after_paren: &str) -> Option<String> {
    let trimmed = after_paren.trim_start();
    // An f-string's value depends on runtime state even though it is quoted.
    if trimmed.starts_with('f') {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let quote = *bytes.first()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let mut value = String::new();
    let mut index = 1usize;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'\\' {
            let next = *bytes.get(index + 1)?;
            value.push(next as char);
            index += 2;
            continue;
        }
        if byte == quote {
            // Reject `"a" + b` — the call site is only partly literal, so the
            // full argument is not knowable.
            let rest = trimmed[index + 1..].trim_start();
            if rest.starts_with('+') || rest.starts_with('%') {
                return None;
            }
            return Some(value);
        }
        value.push(byte as char);
        index += 1;
    }
    None
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let kept: String = value.chars().take(max.saturating_sub(1)).collect();
    format!("{kept}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pure_inspection_block_needs_no_approval() {
        let intent = scan("total = len(content)\nprint(total)\nFINAL(total)");
        assert!(intent.is_empty(), "{intent:?}");
        assert!(intent.is_complete());
    }

    #[test]
    fn literal_calls_are_reported_exactly() {
        let intent = scan("bash(\"cargo test\")\nedit(\"src/lib.rs\")");
        let details: Vec<_> = intent
            .intents
            .iter()
            .map(|i| (i.kind, i.detail.clone()))
            .collect();
        assert!(details.contains(&(IntentKind::Command, Some("cargo test".to_string()))));
        assert!(details.contains(&(IntentKind::FileWrite, Some("src/lib.rs".to_string()))));
        assert!(intent.is_complete(), "all arguments were literal");
    }

    /// The case the whole module exists for: an argument the block computes
    /// cannot be described in advance, so it must be declared unknown rather
    /// than omitted. Omitting it would let a block that runs arbitrary
    /// commands present itself as doing nothing.
    #[test]
    fn computed_arguments_are_undecidable_not_omitted() {
        for code in [
            "bash(cmd)",
            "bash(f\"cargo test {name}\")",
            "bash(\"cargo \" + verb)",
            "bash(build_command())",
        ] {
            let intent = scan(code);
            assert_eq!(intent.intents.len(), 1, "{code}: {intent:?}");
            assert_eq!(intent.intents[0].kind, IntentKind::Undecidable, "{code}");
            assert!(
                !intent.is_complete(),
                "{code} must force a runtime gate: {intent:?}"
            );
        }
    }

    /// Over-reporting is its own failure: if the manifest lists operations the
    /// block never performs, people stop reading it.
    #[test]
    fn bindings_named_in_comments_and_strings_are_not_calls() {
        let intent = scan(
            "# bash(\"rm -rf /\") is what we are NOT doing\n\
             note = \"call bash(\\\"ls\\\") later\"\n\
             print(note)",
        );
        assert!(intent.is_empty(), "{intent:?}");
    }

    #[test]
    fn attributes_and_longer_identifiers_are_not_our_bindings() {
        let intent = scan("subprocess.run(\"ls\")\nrerun(\"x\")\nmy_bash(\"y\")");
        assert!(intent.is_empty(), "{intent:?}");
    }

    #[test]
    fn summary_is_bounded_and_deduplicated() {
        let code = (0..40)
            .map(|_| "bash(\"cargo test\")")
            .collect::<Vec<_>>()
            .join("\n");
        let intent = scan(&code);
        assert_eq!(intent.intents.len(), 40);
        let lines = intent.summary_lines(5);
        assert!(lines.len() <= 5, "{lines:?}");
        assert!(lines[0].contains("cargo test"), "{lines:?}");
    }

    #[test]
    fn agent_and_network_bindings_are_classified() {
        let intent = scan("agent(\"review the diff\")\nfetch(\"https://example.com\")");
        let kinds: Vec<_> = intent.intents.iter().map(|i| i.kind).collect();
        assert!(kinds.contains(&IntentKind::Agent), "{intent:?}");
        assert!(kinds.contains(&IntentKind::Network), "{intent:?}");
    }
}
