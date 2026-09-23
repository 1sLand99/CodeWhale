//! One plain-language line per gated tool call (E6).
//!
//! An approval card used to carry the model's raw JSON arguments and a static
//! tool description. [`approval_summary`] derives a short sentence from the
//! tool name and its arguments alone — never from model prose — so every
//! client can show the same first line ("Search the web for 'espresso'") and
//! put the raw arguments behind it. Paths are shown relative to the
//! workspace when they sit inside it.

use std::path::Path;

use serde_json::Value;

/// Longest quoted argument a summary carries before it is cut with `…`.
const MAX_QUOTED_CHARS: usize = 80;

/// Summarize a gated tool call for an approval prompt.
#[must_use]
pub fn approval_summary(tool_name: &str, input: &Value, workspace: Option<&Path>) -> String {
    let name = crate::tools::canonical_action::canonical_action_alias(tool_name, input);
    let text = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let path = |key: &str| text(key).map(|raw| relative_path(raw, workspace));

    match name {
        "exec_shell" | "task_shell_start" => match text("command") {
            Some(command) => format!("Run `{}`", clip(command)),
            None => "Run a shell command".to_string(),
        },
        "exec_shell_wait" | "exec_wait" => "Wait for a running shell command".to_string(),
        "exec_shell_interact" | "exec_interact" => {
            "Send input to a running shell command".to_string()
        }
        "exec_shell_cancel" => "Stop a running shell command".to_string(),
        "write_file" => match path("path") {
            Some(path) => format!("Write {path}"),
            None => "Write a file".to_string(),
        },
        "edit_file" | "fim_edit" => match path("path") {
            Some(path) => format!("Edit {path}"),
            None => "Edit a file".to_string(),
        },
        "apply_patch" => patch_summary(input, workspace),
        "read_file" => match path("path") {
            Some(path) => format!("Read {path}"),
            None => "Read a file".to_string(),
        },
        "list_dir" => match path("path") {
            Some(path) => format!("List {path}"),
            None => "List the workspace".to_string(),
        },
        "fetch_url" | "web.fetch" | "web_fetch" => match text("url") {
            Some(url) => format!("Fetch {}", clip(url)),
            None => "Fetch a web page".to_string(),
        },
        "web_search" => match text("query").or_else(|| text("q")) {
            Some(query) => format!("Search the web for '{}'", clip(query)),
            None => "Search the web".to_string(),
        },
        "web.run" => web_run_summary(input),
        name if name.starts_with("mcp_") => mcp_summary(name),
        name => format!("Use the {name} tool"),
    }
}

fn web_run_summary(input: &Value) -> String {
    let first = |key: &str, field: &str| {
        input
            .get(key)
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .and_then(|item| item.get(field))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let count = |key: &str| input.get(key).and_then(Value::as_array).map_or(0, Vec::len);
    let more = |key: &str| match count(key) {
        0 | 1 => String::new(),
        n => format!(" (+{} more)", n - 1),
    };
    if let Some(query) = first("search_query", "q") {
        return format!(
            "Search the web for '{}'{}",
            clip(&query),
            more("search_query")
        );
    }
    if let Some(query) = first("image_query", "q") {
        return format!(
            "Search the web for images of '{}'{}",
            clip(&query),
            more("image_query")
        );
    }
    if let Some(target) = first("open", "ref_id") {
        return format!("Open {}{}", clip(&target), more("open"));
    }
    if count("click") > 0 {
        return "Follow a link on an opened page".to_string();
    }
    if let Some(pattern) = first("find", "pattern") {
        return format!("Find '{}' on an opened page", clip(&pattern));
    }
    if count("screenshot") > 0 {
        return "Take a screenshot of an opened page".to_string();
    }
    "Browse the web".to_string()
}

fn patch_summary(input: &Value, workspace: Option<&Path>) -> String {
    let Ok(preflight) = crate::tools::apply_patch::preflight_apply_patch(input) else {
        return "Apply a patch".to_string();
    };
    let mut paths: Vec<String> = preflight
        .touched_files
        .iter()
        .map(|raw| relative_path(raw, workspace))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    match paths.as_slice() {
        [] => "Apply a patch".to_string(),
        [one] => format!("Edit {one}"),
        [first, rest @ ..] => format!("Edit {first} and {} more file(s)", rest.len()),
    }
}

fn mcp_summary(name: &str) -> String {
    // `mcp_<server>_<tool>`; server names may themselves hold `_`, so this is
    // presentation only and never a policy decision.
    let rest = name.trim_start_matches("mcp_");
    match rest.split_once('_') {
        Some((server, tool)) if !server.is_empty() && !tool.is_empty() => {
            format!("Use {tool} from {server}")
        }
        _ => format!("Use {rest}"),
    }
}

/// Show `raw` relative to `workspace` when it names a path inside it.
fn relative_path(raw: &str, workspace: Option<&Path>) -> String {
    let candidate = Path::new(raw);
    if let Some(workspace) = workspace
        && candidate.has_root()
        && let Ok(relative) = candidate.strip_prefix(workspace)
    {
        let shown = relative.display().to_string();
        return if shown.is_empty() {
            ".".to_string()
        } else {
            clip(&shown)
        };
    }
    // A relative path is already workspace-relative; drop a leading `./`.
    if let Ok(relative) = candidate.strip_prefix(".")
        && !relative.as_os_str().is_empty()
    {
        return clip(&relative.display().to_string());
    }
    clip(raw)
}

fn clip(value: &str) -> String {
    // Keep line breaks visible: `a\nb` joined with a space would read as one
    // command with arguments on an approval card.
    let single_line = value
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ⏎ ");
    if single_line.chars().count() <= MAX_QUOTED_CHARS {
        return single_line;
    }
    let mut clipped: String = single_line.chars().take(MAX_QUOTED_CHARS - 1).collect();
    clipped.push('…');
    clipped
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn web_run_search_names_the_query() {
        let summary = approval_summary(
            "web.run",
            &json!({"search_query": [{"q": "best espresso machine"}, {"q": "reviews"}]}),
            None,
        );
        assert_eq!(
            summary,
            "Search the web for 'best espresso machine' (+1 more)"
        );
    }

    #[test]
    fn file_paths_are_workspace_relative() {
        let workspace = Path::new("/work/repo");
        assert_eq!(
            approval_summary(
                "write_file",
                &json!({"path": "/work/repo/notes/espresso.md", "content": "x"}),
                Some(workspace),
            ),
            "Write notes/espresso.md"
        );
        // Outside the workspace stays absolute, so the card never hides where
        // a write lands.
        assert_eq!(
            approval_summary("edit_file", &json!({"path": "/etc/hosts"}), Some(workspace)),
            "Edit /etc/hosts"
        );
        // The model-facing `File{action}` family resolves to the same line.
        assert_eq!(
            approval_summary(
                "File",
                &json!({"action": "write", "path": "/work/repo/a.txt"}),
                Some(workspace),
            ),
            "Write a.txt"
        );
    }

    #[test]
    fn shell_and_fallbacks_are_plain_and_bounded() {
        assert_eq!(
            approval_summary(
                "exec_shell",
                &json!({"command": "cargo  test  -p tui"}),
                None
            ),
            "Run `cargo test -p tui`"
        );
        assert_eq!(
            approval_summary(
                "exec_shell",
                &json!({"command": "cargo test\n\nrm -rf target"}),
                None
            ),
            "Run `cargo test ⏎ rm -rf target`",
            "a second command line never reads as arguments of the first"
        );
        let long = "x".repeat(500);
        let summary = approval_summary("exec_shell", &json!({ "command": long }), None);
        assert!(summary.chars().count() < 100, "{summary}");
        assert_eq!(
            approval_summary("mcp_github_create_issue", &json!({}), None),
            "Use create_issue from github"
        );
        assert_eq!(
            approval_summary("some_tool", &json!({"a": 1}), None),
            "Use the some_tool tool"
        );
    }
}
