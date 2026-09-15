//! Post-edit syntax gate for the file-mutating tools (#6204).
//!
//! Every tool that rewrites a file — `File write`/`edit`/`patch` in
//! [`super::file`] and [`super::apply_patch`], plus [`super::fim`] — routes its
//! post-edit content through [`guard_edit`] *before* the bytes reach the disk.
//! When the edit would leave a file unparseable, the write never happens and
//! the model gets a `line:column` error it can act on this turn instead of a
//! compiler failure one or more turns later.
//!
//! Rust is parsed by `syn`, which implements the real language grammar and
//! reports grammar-exact errors (as opposed to an error-tolerant CST that
//! builds a tree *around* malformed input).
//!
//! # The gate only rejects *newly* introduced breakage
//!
//! [`guard_edit`] fails open unless the file parsed **before** the edit and
//! fails to parse **after** it. That asymmetry is the whole safety argument:
//! repairing a file that is already broken — the single most common reason an
//! agent edits a source file at all — must never be blocked by a gate whose
//! job is to catch the edit that broke it. Creating a new file is likewise
//! ungated, since there is no "before" to have regressed.
//!
//! # Known limitations
//!
//! - **Grammar, not semantics.** A file that parses can still fail to compile;
//!   type errors stay the compiler's and the LSP hook's job
//!   (`core::engine::lsp_hooks`).
//! - **`syn` tracks the editions it knows.** Source using syntax newer than the
//!   pinned `syn` would be reported as a parse error. Because the gate requires
//!   the pre-edit file to have parsed, such a file is skipped entirely rather
//!   than becoming uneditable.
//! - **Extension-driven.** A Rust file that is not named `*.rs` is not checked;
//!   language detection by content is deliberately not attempted.
//! - **Bounded by [`MAX_CHECKED_BYTES`].** Larger files skip the check rather
//!   than spend edit-path latency on a multi-megabyte parse.

use std::fmt;
use std::path::Path;

use super::spec::ToolError;

/// Files larger than this skip the syntax gate.
///
/// Parsing is linear and fast, but the check sits on the interactive edit path
/// and runs twice on a rejection. 2 MiB covers every hand-written source file
/// in this workspace by a wide margin; past that the file is generated or
/// vendored, where a syntax verdict is worth less than the latency.
const MAX_CHECKED_BYTES: usize = 2 * 1024 * 1024;

/// A language the edit path can parse. Extensions outside this set fail open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SyntaxLanguage {
    Rust,
}

impl SyntaxLanguage {
    /// Pick a parser from the file extension, or `None` to skip the check.
    fn from_path(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "rs" => Some(Self::Rust),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
        }
    }
}

/// A parse failure, located precisely enough for the model to fix it directly.
#[derive(Debug, Clone)]
pub(super) struct SyntaxIssue {
    language: SyntaxLanguage,
    /// 1-based line, as every editor and compiler reports it.
    line: usize,
    /// 1-based column, likewise.
    column: usize,
    message: String,
}

impl fmt::Display for SyntaxIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} syntax error at line {}, column {}: {}",
            self.language.label(),
            self.line,
            self.column,
            self.message
        )
    }
}

/// Parse `source` as the language implied by `path`.
///
/// `None` means "no objection": the content parses, the extension is not one
/// we parse, or the file is too large to be worth checking.
pub(super) fn syntax_check(path: &Path, source: &str) -> Option<SyntaxIssue> {
    if source.len() > MAX_CHECKED_BYTES {
        return None;
    }
    match SyntaxLanguage::from_path(path)? {
        SyntaxLanguage::Rust => check_rust(source),
    }
}

/// Refuse an edit that would take a parseable file to an unparseable one.
///
/// `before` is the pre-edit content, or `None` when the edit creates the file.
/// Callers invoke this before writing, so a rejection leaves the file on disk
/// byte-for-byte untouched and there is nothing to roll back.
pub(super) fn guard_edit(
    path: &Path,
    display_path: &str,
    before: Option<&str>,
    after: &str,
) -> Result<(), ToolError> {
    let Some(issue) = syntax_check(path, after) else {
        return Ok(());
    };
    // Fail open on a file that was already broken (or is brand new): the gate
    // exists to catch the edit that *introduces* a syntax error, never to
    // strand a model that is repairing one.
    let Some(before) = before else {
        return Ok(());
    };
    if syntax_check(path, before).is_some() {
        return Ok(());
    }
    Err(ToolError::execution_failed(format!(
        "Edit refused: it would leave {display_path} unparseable — {issue}. Nothing was written; \
         the file is unchanged. Recovery: re-read the file with File action=\"read\", check the \
         replacement for unbalanced delimiters or a truncated block, and retry."
    )))
}

fn check_rust(source: &str) -> Option<SyntaxIssue> {
    let error = syn::parse_file(source).err()?;
    // A `syn::Error` can carry several diagnostics; the first is the earliest
    // and the one worth showing.
    let first = error.into_iter().next()?;
    let start = first.span().start();
    Some(SyntaxIssue {
        language: SyntaxLanguage::Rust,
        line: start.line,
        // `proc-macro2` columns are 0-based; editors and rustc are not.
        column: start.column.saturating_add(1),
        message: first.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn rust_path() -> PathBuf {
        PathBuf::from("src/lib.rs")
    }

    #[test]
    fn valid_rust_passes() {
        assert!(syntax_check(&rust_path(), "fn main() {}\n").is_none());
    }

    #[test]
    fn missing_brace_reports_line_and_column() {
        let issue = syntax_check(&rust_path(), "fn main() {\n    let x = 1;\n")
            .expect("unbalanced brace must be reported");
        assert_eq!(issue.language, SyntaxLanguage::Rust);
        assert!(issue.line >= 1, "{issue}");
        assert!(issue.column >= 1, "{issue}");
        let rendered = issue.to_string();
        assert!(rendered.contains("Rust syntax error at line"), "{rendered}");
    }

    #[test]
    fn unknown_extension_is_skipped() {
        assert!(syntax_check(Path::new("notes.txt"), "fn main() {").is_none());
    }

    #[test]
    fn oversized_source_is_skipped() {
        let huge = format!("fn main() {{{}", " ".repeat(MAX_CHECKED_BYTES));
        assert!(syntax_check(&rust_path(), &huge).is_none());
    }

    #[test]
    fn guard_rejects_newly_broken_rust() {
        let error = guard_edit(
            &rust_path(),
            "src/lib.rs",
            Some("fn main() {}\n"),
            "fn main() {\n",
        )
        .expect_err("an edit that breaks a parseable file must be refused");
        let message = error.to_string();
        assert!(message.contains("src/lib.rs"), "{message}");
        assert!(message.contains("Rust syntax error at line"), "{message}");
        assert!(message.contains("Nothing was written"), "{message}");
    }

    #[test]
    fn guard_allows_repairing_an_already_broken_file() {
        // Still broken after the edit, but it was broken before: a model
        // mid-repair must not be locked out.
        guard_edit(
            &rust_path(),
            "src/lib.rs",
            Some("fn main() {\n"),
            "fn main() {\n    let x = 1;\n",
        )
        .expect("pre-existing breakage must fail open");
    }

    #[test]
    fn guard_allows_creating_a_new_file() {
        guard_edit(&rust_path(), "src/lib.rs", None, "fn main() {\n")
            .expect("file creation has no prior state to regress");
    }

    #[test]
    fn guard_allows_a_valid_edit() {
        guard_edit(
            &rust_path(),
            "src/lib.rs",
            Some("fn main() {}\n"),
            "fn main() {\n    println!(\"hi\");\n}\n",
        )
        .expect("a syntactically valid edit must pass");
    }
}
