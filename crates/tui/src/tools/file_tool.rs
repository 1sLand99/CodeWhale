//! Canonical action-based wrapper for file system tools.
//!
//! The model sees one tool: `File` with an `action` parameter
//! (read | list | search_name | search_content | write | edit | patch).
//! The per-action legacy execution aliases were removed in v0.9.3.

use async_trait::async_trait;
use serde_json::{Value, json};

use super::apply_patch::ApplyPatchTool;
use super::canonical_action::required_action;
use super::file::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
use super::file_search::FileSearchTool;
use super::search::GrepFilesTool;
use super::spec::{
    ApprovalRequirement, ToolCapability, ToolContext, ToolError, ToolResult, ToolSpec,
};

/// Lift a parameter description out of the tool that implements the action.
///
/// Returns an empty string when the key is absent, which the
/// `wrapper_borrows_every_inner_description` test turns into a build-time
/// failure — a renamed inner parameter must not silently blank the only
/// description the model gets to read.
fn borrowed(schema: &Value, key: &str) -> String {
    schema
        .get("properties")
        .and_then(|properties| properties.get(key))
        .and_then(|property| property.get("description"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// `borrowed`, tagged with the action the parameter belongs to.
fn describe(schema: &Value, key: &str, action: &str) -> String {
    let base = borrowed(schema, key);
    if base.is_empty() {
        return String::new();
    }
    format!("{} — {action}.", base.trim_end_matches('.'))
}

pub struct FileTool {
    name: &'static str,
    forced_action: Option<&'static str>,
    allow_writes: bool,
    allow_patch: bool,
}

impl FileTool {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            forced_action: None,
            allow_writes: true,
            allow_patch: false,
        }
    }

    pub const fn with_patch(name: &'static str) -> Self {
        Self {
            name,
            forced_action: None,
            allow_writes: true,
            allow_patch: true,
        }
    }

    pub const fn read_only(name: &'static str) -> Self {
        Self {
            name,
            forced_action: None,
            allow_writes: false,
            allow_patch: false,
        }
    }

    pub const fn alias(name: &'static str, action: &'static str) -> Self {
        Self {
            name,
            forced_action: Some(action),
            allow_writes: true,
            allow_patch: true,
        }
    }

    /// The action set this instance actually dispatches, in schema order.
    ///
    /// The `enum` the model reads and the name list its errors quote are both
    /// built from this, so a mode that hides `write` can never advertise it or
    /// suggest it in a refusal.
    fn available_actions(&self) -> Vec<&'static str> {
        if let Some(forced) = self.forced_action {
            return vec![forced];
        }
        let mut actions = vec!["read", "list", "search_name", "search_content"];
        if self.allow_writes {
            actions.extend(["write", "edit"]);
        }
        if self.allow_patch {
            actions.push("patch");
        }
        actions
    }

    /// Policy-side action resolution.
    ///
    /// Approval, read-only, and parallel-safety predicates cannot fail, so a
    /// missing action resolves to the most restrictive answer they can give.
    /// `execute` does **not** share this fallback — see `required_action`.
    fn resolve_action<'a>(&self, input: &'a Value) -> &'a str {
        self.forced_action.unwrap_or_else(|| {
            input
                .get("action")
                .and_then(Value::as_str)
                .unwrap_or("read")
        })
    }

    /// Execution-side action resolution: `action` is required, as the schema
    /// has always said it is.
    fn required_action(&self, input: &Value) -> Result<String, ToolError> {
        if let Some(forced) = self.forced_action {
            return Ok(forced.to_string());
        }
        required_action(input, self.name, &self.available_actions())
    }

    fn strip_action(&self, input: Value) -> Result<Value, ToolError> {
        let mut input = input;
        if let Some(obj) = input.as_object_mut() {
            obj.remove("action");
            Ok(input)
        } else {
            Err(ToolError::invalid_input(
                "File tool input must be an object",
            ))
        }
    }
}

#[async_trait]
impl ToolSpec for FileTool {
    fn name(&self) -> &'static str {
        self.name
    }

    fn model_visible(&self) -> bool {
        self.name == "File"
    }

    fn description(&self) -> &'static str {
        "Read, list, search, write, edit, or patch workspace files. Use read before edit; edit performs one exact replacement, while patch is best for multi-hunk or multi-file changes. Read/list/search actions are parallel-safe and do not require approval. Available actions depend on the active mode and feature policy."
    }

    fn input_schema(&self) -> Value {
        let actions = self.available_actions();
        // Every per-action parameter description is borrowed from the tool
        // that implements the action rather than restated here. The wrapper is
        // the only schema the model ever reads, and the restated copy had gone
        // stale: it advertised `max_lines` "default 200" long after the real
        // default became 500-with-a-16KB-budget, and a `blame` action `File`
        // has never had. Borrowing makes that class of drift impossible.
        let read = ReadFileTool.input_schema();
        let name_search = FileSearchTool.input_schema();
        let content_search = GrepFilesTool.input_schema();
        let write = WriteFileTool.input_schema();
        let edit = EditFileTool.input_schema();
        let patch = ApplyPatchTool.input_schema();
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": actions,
                    "description": "Action to perform"
                },
                "path": {
                    "type": "string",
                    "description": format!(
                        "{} Optional for list and search (default: .).",
                        borrowed(&read, "path"),
                    )
                },
                "start_line": {
                    "type": "integer",
                    "description": describe(&read, "start_line", "action=read")
                },
                "max_lines": {
                    "type": "integer",
                    "description": describe(&read, "max_lines", "action=read")
                },
                "pages": {
                    "type": "string",
                    "description": describe(&read, "pages", "action=read")
                },
                "content": {
                    "type": "string",
                    "description": describe(&write, "content", "action=write")
                },
                "search": {
                    "type": "string",
                    "description": describe(&edit, "search", "action=edit")
                },
                "replace": {
                    "oneOf": [
                        { "type": "string", "description": describe(&edit, "replace", "action=edit") },
                        { "type": "array", "items": { "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } }, "required": ["path", "content"] }, "description": "Full-file replacements — action=patch." }
                    ]
                },
                "fuzz": {
                    "type": "integer",
                    "description": describe(&patch, "fuzz", "action=patch")
                },
                "query": {
                    "type": "string",
                    "description": describe(&name_search, "query", "action=search_name")
                },
                "pattern": {
                    "type": "string",
                    "description": describe(&content_search, "pattern", "action=search_content")
                },
                "limit": {
                    "type": "integer",
                    "description": describe(&name_search, "limit", "action=search_name")
                },
                "max_results": {
                    "type": "integer",
                    "description": format!(
                        "{} search_name: alias for `limit`.",
                        describe(&content_search, "max_results", "action=search_content"),
                    )
                },
                "extensions": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": describe(&name_search, "extensions", "action=search_name")
                },
                "include": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": describe(&content_search, "include", "action=search_content")
                },
                "exclude": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": format!(
                        "{} Also action=search_name.",
                        describe(&content_search, "exclude", "action=search_content"),
                    )
                },
                "context_lines": {
                    "type": "integer",
                    "description": describe(&content_search, "context_lines", "action=search_content")
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": describe(&content_search, "case_insensitive", "action=search_content")
                },
                "patch": {
                    "type": "string",
                    "description": "Unified diff patch content for action=patch"
                },
                "changes": {
                    "type": "array",
                    "items": { "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" } }, "required": ["path", "content"] },
                    "description": "Deprecated alias for replace in action=patch"
                },
                "create_if_missing": {
                    "type": "boolean",
                    "description": "Create files if missing for action=patch"
                }
            },
            "required": ["action"]
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        let mut capabilities = vec![ToolCapability::ReadOnly, ToolCapability::Sandboxable];
        let can_mutate = match self.forced_action {
            Some("write" | "edit" | "patch") => true,
            Some(_) => false,
            None => self.allow_writes || self.allow_patch,
        };
        if can_mutate {
            capabilities.extend([
                ToolCapability::WritesFiles,
                ToolCapability::RequiresApproval,
            ]);
        }
        capabilities
    }

    fn approval_requirement_for(&self, input: &Value) -> ApprovalRequirement {
        match self.resolve_action(input) {
            "read" | "list" | "search_name" | "search_content" => ApprovalRequirement::Auto,
            "write" | "edit" | "patch" => ApprovalRequirement::Suggest,
            _ => ApprovalRequirement::Auto,
        }
    }

    fn is_read_only_for(&self, input: &Value) -> bool {
        matches!(
            self.resolve_action(input),
            "read" | "list" | "search_name" | "search_content"
        )
    }

    fn supports_parallel_for(&self, input: &Value) -> bool {
        matches!(
            self.resolve_action(input),
            "read" | "list" | "search_name" | "search_content"
        )
    }

    fn starts_detached_for(&self, _input: &Value) -> bool {
        false
    }

    async fn execute(&self, input: Value, context: &ToolContext) -> Result<ToolResult, ToolError> {
        let action = self.required_action(&input)?;
        if matches!(action.as_str(), "write" | "edit") && !self.allow_writes {
            return Err(ToolError::not_available(format!(
                "File action=\"{action}\" is unavailable in the current mode; nothing was written. Available actions here: {}. Switch to Act mode (`/mode act`) for write-capable file work.",
                self.available_actions().join(", ")
            )));
        }
        if action == "patch" && !self.allow_patch {
            return Err(ToolError::not_available(format!(
                "File action=\"patch\" is unavailable because the patch feature is disabled; nothing was written. Available actions here: {}. Use action=\"edit\" for a single exact replacement, or action=\"write\" to replace the whole file.",
                self.available_actions().join(", ")
            )));
        }
        let input = self.strip_action(input)?;

        match action.as_str() {
            "read" => ReadFileTool.execute(input, context).await,
            "list" => ListDirTool.execute(input, context).await,
            // The cross-action spellings the wrapper advertises
            // (`max_results` on search_name, `query`/`limit` on
            // search_content) used to be copied here. They are alias-table
            // entries on the implementing tools now — one mechanism, applied
            // before the same unknown-parameter check every other action runs,
            // and a direct call to the inner tool behaves identically.
            "search_name" => FileSearchTool.execute(input, context).await,
            "search_content" => GrepFilesTool.execute(input, context).await,
            "write" => WriteFileTool.execute(input, context).await,
            "edit" => EditFileTool.execute(input, context).await,
            "patch" => ApplyPatchTool.execute(input, context).await,
            other => Err(ToolError::invalid_input(format!(
                "Unknown File action \"{other}\"; nothing was run. Pass one of: {}.",
                self.available_actions().join(", ")
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    fn tool() -> FileTool {
        FileTool::with_patch("File")
    }

    async fn err(tool: &FileTool, input: Value) -> String {
        let tmp = tempdir().expect("tempdir");
        let ctx = ToolContext::new(tmp.path().to_path_buf());
        tool.execute(input, &ctx)
            .await
            .expect_err("call must be refused")
            .to_string()
    }

    /// #5209's shape one level up: `File{path, content}` used to answer an
    /// intended write with the file's contents under a success receipt.
    #[tokio::test]
    async fn missing_action_is_refused_instead_of_silently_reading() {
        let message = err(&tool(), json!({"path": "a.rs", "content": "fn main() {}"})).await;
        assert!(
            message.contains("requires an `action`"),
            "must say what is missing: {message}"
        );
        assert!(
            message.contains("nothing was run"),
            "must deny having done work: {message}"
        );
        for action in [
            "read",
            "list",
            "search_name",
            "search_content",
            "write",
            "edit",
            "patch",
        ] {
            assert!(message.contains(action), "must name `{action}`: {message}");
        }
    }

    #[tokio::test]
    async fn non_string_action_is_refused_with_the_valid_values() {
        let message = err(&tool(), json!({"action": 3, "path": "a.rs"})).await;
        assert!(message.contains("to be a string"), "{message}");
        assert!(message.contains("read"), "{message}");
    }

    #[tokio::test]
    async fn unknown_action_names_the_actions_that_dispatch() {
        let message = err(&tool(), json!({"action": "str_replace", "path": "a.rs"})).await;
        assert!(
            message.contains("str_replace"),
            "must quote the bad value: {message}"
        );
        assert!(message.contains("nothing was run"), "{message}");
        assert!(
            message.contains("edit"),
            "must name the real action: {message}"
        );
    }

    /// A refusal must never advertise an action this instance cannot run.
    #[tokio::test]
    async fn read_only_instances_do_not_suggest_write_actions() {
        let message = err(&FileTool::read_only("File"), json!({"path": "a.rs"})).await;
        assert!(message.contains("read"), "{message}");
        assert!(
            !message.contains("write"),
            "read-only File must not offer write: {message}"
        );
        assert!(
            !message.contains("edit"),
            "read-only File must not offer edit: {message}"
        );
    }

    #[tokio::test]
    async fn disabled_write_refusal_states_what_is_available() {
        let message = err(
            &FileTool::read_only("File"),
            json!({"action": "write", "path": "a.rs", "content": "x"}),
        )
        .await;
        assert!(message.contains("nothing was written"), "{message}");
        assert!(message.contains("Available actions here"), "{message}");
    }

    /// The wrapper is the only schema the model reads. Every per-action
    /// description must come from the tool that implements the action, so the
    /// stale-copy drift that produced "default 200" cannot recur.
    #[test]
    fn wrapper_borrows_every_inner_description() {
        let schema = tool().input_schema();
        let properties = schema["properties"].as_object().expect("properties");
        for (name, property) in properties {
            let text = if name == "replace" {
                property.to_string()
            } else {
                property["description"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string()
            };
            assert!(
                !text.trim().is_empty(),
                "`{name}` lost its description — an inner parameter was probably renamed"
            );
        }
    }

    #[test]
    fn read_parameters_state_the_defaults_the_code_actually_uses() {
        let schema = tool().input_schema();
        let max_lines = schema["properties"]["max_lines"]["description"]
            .as_str()
            .expect("max_lines description");
        let inner = ReadFileTool.input_schema()["properties"]["max_lines"]["description"]
            .as_str()
            .expect("inner max_lines description")
            .to_string();
        assert!(
            max_lines.contains(inner.trim_end_matches('.')),
            "wrapper must quote the implementing tool verbatim: {max_lines}"
        );
        assert!(
            !max_lines.contains("200"),
            "the retired 200-line default must not be advertised: {max_lines}"
        );
        assert!(
            !max_lines.contains("blame"),
            "`File` has no blame action; blame lives on `Git`: {max_lines}"
        );
    }

    /// `fuzz` belongs to `patch` alone. `edit` read it into a discarded
    /// binding while the schema kept advertising it, and a live model read
    /// that as "an optional fuzzy-matching flag for the search" — a
    /// capability claim no code honored. The advertisement is gone; what
    /// remains must describe only the integer `patch` really uses.
    #[test]
    fn fuzz_is_advertised_only_for_patch() {
        let schema = tool().input_schema();
        assert_eq!(schema["properties"]["fuzz"]["type"], "integer");
        let fuzz = schema["properties"]["fuzz"]["description"]
            .as_str()
            .expect("fuzz description");
        assert!(fuzz.contains("action=patch"), "{fuzz}");
        assert!(
            !fuzz.contains("action=edit"),
            "edit no longer accepts fuzz: {fuzz}"
        );
        assert!(
            EditFileTool.input_schema()["properties"]
                .get("fuzz")
                .is_none(),
            "edit must not advertise a parameter it does not implement"
        );
    }

    /// `File` is in the active catalog in every mode, so its schema is re-sent
    /// on every turn of every session. Borrowing the implementing tools'
    /// descriptions buys accuracy with bytes; this bounds what that costs and
    /// prints the per-parameter breakdown when it trips, so the next increase
    /// is a decision rather than a drift.
    #[test]
    fn schema_stays_within_its_catalog_byte_budget() {
        const BUDGET_BYTES: usize = 3_000;

        let schema = FileTool::with_patch("File").input_schema();
        let mut rows: Vec<(usize, String)> = schema["properties"]
            .as_object()
            .expect("properties")
            .iter()
            .map(|(name, property)| (property.to_string().len(), name.clone()))
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(row.0));
        let total: usize = rows.iter().map(|(bytes, _)| bytes).sum();

        assert!(
            total <= BUDGET_BYTES,
            "File schema is {total} bytes against a {BUDGET_BYTES} budget; \
             trim explanation, keep instruction. Breakdown: {rows:?}"
        );
    }

    // === Per-action parameter validation ===
    //
    // #5209 taught `edit` to refuse a parameter it does not implement. Only
    // `edit` learned it, so every other action still dropped unknown keys and
    // answered anyway: a misspelled `start_line` on `read` returned the head
    // of the file with nothing in the response admitting the requested window
    // was never honored. These cover the refusal on every action, and — the
    // half that keeps a refusal honest — that each action still accepts its
    // full legitimate parameter set, optional names and aliases included.

    /// A workspace with one file, already read, so `edit` and `patch` are
    /// past their freshness precondition and reach parameter validation.
    async fn workspace() -> (tempfile::TempDir, ToolContext) {
        let tmp = tempdir().expect("tempdir");
        let ctx = ToolContext::new(tmp.path().to_path_buf());
        std::fs::write(tmp.path().join("doc.txt"), "alpha\nbeta\ngamma\n").expect("write");
        tool()
            .execute(json!({"action": "read", "path": "doc.txt"}), &ctx)
            .await
            .expect("seed read");
        (tmp, ctx)
    }

    /// The minimal call that dispatches for each action, in schema order.
    fn minimal_calls() -> Vec<(&'static str, Value)> {
        vec![
            ("read", json!({"action": "read", "path": "doc.txt"})),
            ("list", json!({"action": "list"})),
            (
                "search_name",
                json!({"action": "search_name", "query": "doc"}),
            ),
            (
                "search_content",
                json!({"action": "search_content", "pattern": "alpha"}),
            ),
            (
                "write",
                json!({"action": "write", "path": "new.txt", "content": "x\n"}),
            ),
            (
                "edit",
                json!({"action": "edit", "path": "doc.txt", "search": "alpha", "replace": "delta"}),
            ),
            (
                "patch",
                json!({"action": "patch", "path": "doc.txt", "patch": "@@ -1,1 +1,1 @@\n-alpha\n+delta\n"}),
            ),
        ]
    }

    fn with_key(mut input: Value, key: &str, value: Value) -> Value {
        input
            .as_object_mut()
            .expect("object")
            .insert(key.to_string(), value);
        input
    }

    /// The gap this closes. A parameter with no known meaning is refused by
    /// every action, not just `edit`, and the refusal carries the same four
    /// facts everywhere: what was wrong, what is allowed, what is required,
    /// and that nothing was done.
    #[tokio::test]
    async fn every_action_refuses_an_unknown_parameter() {
        for (action, call) in minimal_calls() {
            let (_tmp, ctx) = workspace().await;
            let message = tool()
                .execute(with_key(call, "bogus_param", json!(true)), &ctx)
                .await
                .expect_err("an unknown parameter must be refused")
                .to_string();
            assert!(
                message.contains("bogus_param"),
                "{action} must name the offending parameter: {message}"
            );
            assert!(
                message.contains(&format!("unexpected File {action} parameter")),
                "{action} must name the action it refused: {message}"
            );
            assert!(
                message.contains("Allowed parameters are"),
                "{action} must name the allowed set: {message}"
            );
            assert!(
                message.contains("Required:"),
                "{action} must name the required set: {message}"
            );
            assert!(
                message.contains(&format!("The {action} was not performed")),
                "{action} must deny having done the work: {message}"
            );
        }
    }

    /// The specific silent wrong answer that motivated this: a misspelled
    /// read window used to be dropped, and the head of the file came back
    /// under a success receipt as if it were the requested range.
    #[tokio::test]
    async fn a_misspelled_read_window_is_refused_rather_than_answered_with_the_head() {
        let (_tmp, ctx) = workspace().await;
        let message = tool()
            .execute(
                json!({"action": "read", "path": "doc.txt", "start_lien": 2}),
                &ctx,
            )
            .await
            .expect_err("a misspelled window must not silently return the head")
            .to_string();
        assert!(message.contains("start_lien"), "{message}");
        assert!(message.contains("`start_line`"), "{message}");
    }

    /// A refusal is only worth having if the legitimate call still lands.
    /// Every action's full parameter set — every optional name included —
    /// must survive validation.
    #[tokio::test]
    async fn every_action_accepts_its_full_legitimate_parameter_set() {
        let full: Vec<(&str, Value)> = vec![
            (
                "read",
                json!({"action": "read", "path": "doc.txt", "start_line": 1, "max_lines": 2, "pages": "1"}),
            ),
            ("list", json!({"action": "list", "path": "."})),
            (
                "search_name",
                json!({"action": "search_name", "query": "doc", "path": ".", "limit": 5,
                       "extensions": ["txt"], "exclude": ["target/**"]}),
            ),
            (
                "search_content",
                json!({"action": "search_content", "pattern": "alpha", "path": ".",
                       "include": ["*.txt"], "exclude": ["target/**"], "context_lines": 1,
                       "case_insensitive": true, "max_results": 5}),
            ),
            (
                "write",
                json!({"action": "write", "path": "new.txt", "content": "x\n"}),
            ),
            (
                "edit",
                json!({"action": "edit", "path": "doc.txt", "search": "alpha", "replace": "delta"}),
            ),
            (
                "patch",
                json!({"action": "patch", "path": "doc.txt",
                       "patch": "@@ -1,1 +1,1 @@\n-alpha\n+delta\n",
                       "fuzz": 3, "create_if_missing": false}),
            ),
        ];

        for (action, call) in full {
            let (_tmp, ctx) = workspace().await;
            let result = tool()
                .execute(call, &ctx)
                .await
                .unwrap_or_else(|error| panic!("{action} must accept its own parameters: {error}"));
            assert!(result.success, "{action}: {}", result.content);
        }
    }

    /// Validation runs *after* alias translation, so every cross-harness
    /// spelling the alias lane folds must still reach the action it names.
    /// A refusal that fired first would undo #5209's fix.
    #[tokio::test]
    async fn every_alias_survives_validation() {
        let aliased: Vec<(&str, Value)> = vec![
            // Path spellings, on every action that takes a path.
            ("read", json!({"action": "read", "file_path": "doc.txt"})),
            ("read", json!({"action": "read", "filePath": "doc.txt"})),
            ("list", json!({"action": "list", "file_path": "."})),
            (
                "search_name",
                json!({"action": "search_name", "query": "doc", "file_path": "."}),
            ),
            (
                "search_content",
                json!({"action": "search_content", "pattern": "alpha", "file_path": "."}),
            ),
            (
                "write",
                json!({"action": "write", "file_path": "new.txt", "content": "x\n"}),
            ),
            // Read-window spellings.
            (
                "read",
                json!({"action": "read", "path": "doc.txt", "offset": 2, "limit": 1}),
            ),
            (
                "read",
                json!({"action": "read", "path": "doc.txt", "line_offset": 2, "n_lines": 1}),
            ),
            (
                "read",
                json!({"action": "read", "path": "doc.txt", "num_lines": 1}),
            ),
            // Search spellings the wrapper advertises across both actions.
            (
                "search_name",
                json!({"action": "search_name", "query": "doc", "max_results": 5}),
            ),
            (
                "search_content",
                json!({"action": "search_content", "query": "alpha", "limit": 5}),
            ),
        ];

        for (action, call) in aliased {
            let (_tmp, ctx) = workspace().await;
            let result = tool()
                .execute(call.clone(), &ctx)
                .await
                .unwrap_or_else(|error| panic!("{action} must accept {call}: {error}"));
            assert!(result.success, "{action} / {call}: {}", result.content);
        }

        // Edit spellings need their own loop: each one mutates the file.
        for (search, replace) in [
            ("old_string", "new_string"),
            ("old_str", "new_str"),
            ("oldText", "newText"),
            ("old_text", "new_text"),
        ] {
            let (_tmp, ctx) = workspace().await;
            let result = tool()
                .execute(
                    json!({"action": "edit", "path": "doc.txt",
                           search: "alpha", replace: "delta"}),
                    &ctx,
                )
                .await
                .unwrap_or_else(|error| panic!("edit must accept {search}/{replace}: {error}"));
            assert!(result.success, "{search}/{replace}: {}", result.content);
        }
        let (_tmp, ctx) = workspace().await;
        let result = tool()
            .execute(
                json!({"action": "edit", "path": "doc.txt", "search": "alpha", "replacement": "delta"}),
                &ctx,
            )
            .await
            .expect("edit must accept `replacement`");
        assert!(result.success, "{}", result.content);
    }

    /// A parameter that belongs to a *different* action is still unknown to
    /// this one. Silently dropping it is how a model learns a call worked
    /// when the argument it cared about was discarded.
    #[tokio::test]
    async fn parameters_do_not_leak_between_actions() {
        for (action, call) in [
            (
                "read",
                json!({"action": "read", "path": "doc.txt", "case_insensitive": true}),
            ),
            (
                "write",
                json!({"action": "write", "path": "new.txt", "content": "x\n", "start_line": 2}),
            ),
            (
                "list",
                json!({"action": "list", "path": ".", "context_lines": 3}),
            ),
            (
                "search_name",
                json!({"action": "search_name", "query": "doc", "context_lines": 3}),
            ),
            (
                "search_content",
                json!({"action": "search_content", "pattern": "alpha", "extensions": ["txt"]}),
            ),
        ] {
            let (_tmp, ctx) = workspace().await;
            let message = tool()
                .execute(call, &ctx)
                .await
                .expect_err("another action's parameter must be refused")
                .to_string();
            assert!(
                message.contains(&format!("The {action} was not performed")),
                "{action}: {message}"
            );
        }
    }

    /// Every action's required names must be names it also allows, or a
    /// refusal would tell the model to pass something the action rejects.
    #[test]
    fn every_required_parameter_is_also_an_allowed_one() {
        use crate::tools::file::{
            EDIT_PARAMS, LIST_PARAMS, PATCH_PARAMS, READ_PARAMS, SEARCH_CONTENT_PARAMS,
            SEARCH_NAME_PARAMS, WRITE_PARAMS,
        };

        for params in [
            READ_PARAMS,
            WRITE_PARAMS,
            EDIT_PARAMS,
            LIST_PARAMS,
            SEARCH_NAME_PARAMS,
            SEARCH_CONTENT_PARAMS,
            PATCH_PARAMS,
        ] {
            params.assert_required_is_allowed();
        }
    }

    #[test]
    fn advertised_actions_match_the_actions_that_dispatch() {
        for (tool, expected) in [
            (FileTool::with_patch("File"), 7),
            (FileTool::new("File"), 6),
            (FileTool::read_only("File"), 4),
        ] {
            let schema = tool.input_schema();
            let advertised = schema["properties"]["action"]["enum"]
                .as_array()
                .expect("action enum")
                .len();
            assert_eq!(advertised, expected);
            assert_eq!(advertised, tool.available_actions().len());
        }
    }
}
