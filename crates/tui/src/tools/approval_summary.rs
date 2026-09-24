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
        "run_verifiers" => verifiers_summary(input),
        "run_tests" => match text("args") {
            Some(args) => format!("Run `cargo test {}`", clip(args)),
            None => "Run the project's tests".to_string(),
        },
        name if name.starts_with("mcp_") => mcp_summary(name, input, workspace),
        name => match argument_hint(input, workspace) {
            Some(hint) => format!("{}: {hint}", humanize(name)),
            None => format!("Use the {name} tool"),
        },
    }
}

/// `run_verifiers{commands}` spawns arbitrary programs, so the heading names
/// what will run rather than the tool that runs it.
fn verifiers_summary(input: &Value) -> String {
    let commands = input
        .get("commands")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let command_line = |command: &Value| {
        let program = command.get("program").and_then(Value::as_str)?.trim();
        if program.is_empty() {
            return None;
        }
        // The full program path stays in the preview below the heading; the
        // heading names the program the way a person would.
        let shown = Path::new(program)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(program);
        let mut line = shown.to_string();
        for arg in command
            .get("args")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            line.push(' ');
            line.push_str(arg);
        }
        Some(line)
    };
    let label = |command: &Value| {
        command
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
            .or_else(|| command_line(command))
    };
    match commands {
        [] => "Run the project's checks".to_string(),
        [one] => match command_line(one) {
            Some(line) => format!("Run `{}`", clip(&line)),
            None => "Run a check".to_string(),
        },
        many => {
            let names: Vec<String> = many.iter().take(2).filter_map(label).collect();
            let rest = many.len().saturating_sub(names.len());
            let more = if rest > 0 {
                format!(" (+{rest} more)")
            } else {
                String::new()
            };
            format!(
                "Run {} checks: {}{more}",
                many.len(),
                clip(&names.join(", "))
            )
        }
    }
}

/// The first argument that says what a tool acts on, for tools without a
/// dedicated line.
fn argument_hint(input: &Value, workspace: Option<&Path>) -> Option<String> {
    const KEYS: [&str; 10] = [
        "path", "file", "url", "command", "query", "q", "name", "app", "title", "target",
    ];
    KEYS.iter().find_map(|key| {
        let raw = input.get(*key)?.as_str()?.trim();
        if raw.is_empty() {
            return None;
        }
        Some(match *key {
            "path" | "file" => relative_path(raw, workspace),
            "url" => url_display(raw, workspace),
            "command" => format!("`{}`", clip(raw)),
            "query" | "q" => format!("'{}'", clip(raw)),
            _ => clip(raw),
        })
    })
}

/// `create_issue` → `Create issue`.
fn humanize(name: &str) -> String {
    let spaced = name.replace(['_', '-', '.'], " ");
    let mut chars = spaced.trim().chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => name.to_string(),
    }
}

/// A URL as a person would name it: a `file://` URL inside the workspace is
/// its relative path, anything else is the URL itself.
fn url_display(raw: &str, workspace: Option<&Path>) -> String {
    let local = raw
        .strip_prefix("file://localhost/")
        .map(|rest| format!("/{rest}"))
        .or_else(|| raw.strip_prefix("file:///").map(|rest| format!("/{rest}")));
    match local {
        Some(path) => {
            let path = path
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_string();
            let decoded = urlencoding::decode(&path)
                .map(|decoded| decoded.into_owned())
                .unwrap_or(path);
            relative_path(&decoded, workspace)
        }
        None => clip(raw),
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

fn mcp_summary(name: &str, input: &Value, workspace: Option<&Path>) -> String {
    // `mcp_<server>_<tool>`; server names may themselves hold `_`, so this is
    // presentation only and never a policy decision.
    let rest = name.trim_start_matches("mcp_");
    let (server, tool) = match mcp_server_and_tool(rest) {
        Some(parts) => parts,
        None => return format!("Use {rest}"),
    };
    let server = server_display(server);
    let text = |key: &str| {
        input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    };
    let target = |keys: &[&str]| keys.iter().find_map(|key| text(key)).map(clip);
    // Verbs that say what will happen on the person's computer, whichever
    // server provides them; the server still closes the line.
    let action = match tool {
        "browser" | "browser_start" | "browser_navigate" | "browser_click" | "browser_type"
        | "browser_screenshot" | "browser_status" | "browser_stop" => {
            let verb = tool
                .strip_prefix("browser_")
                .or_else(|| text("action"))
                .unwrap_or("start");
            Some(match (verb, text("url")) {
                ("start" | "navigate", Some(url)) => format!(
                    "Open {} in a controlled browser",
                    url_display(url, workspace)
                ),
                ("start", None) => "Start a controlled browser".to_string(),
                ("click", _) => match text("selector") {
                    Some(selector) => {
                        format!("Click `{}` in the controlled browser", clip(selector))
                    }
                    None => "Click in the controlled browser".to_string(),
                },
                ("type", _) => match text("text") {
                    Some(typed) => format!("Type '{}' in the controlled browser", clip(typed)),
                    None => "Type in the controlled browser".to_string(),
                },
                ("screenshot", _) => "Screenshot the controlled browser page".to_string(),
                ("status", _) => "Check the controlled browser".to_string(),
                ("stop", _) => "Close the controlled browser tab".to_string(),
                _ => "Use the controlled browser".to_string(),
            })
        }
        "open_application" => Some(match target(&["name", "bundle_id", "url"]) {
            Some(app) => format!("Open {app}"),
            None => "Open an application".to_string(),
        }),
        "kill_app" => Some(match target(&["name", "bundle_id"]) {
            Some(app) => format!("Quit {app}"),
            None => "Quit an application".to_string(),
        }),
        "list_apps" => Some("List apps on this computer".to_string()),
        "list_windows" => Some("List windows on this computer".to_string()),
        "screenshot" => Some("Take a screenshot".to_string()),
        "get_app_state" => Some("Read an app's screen contents".to_string()),
        "app_script" => Some("Run a script in an app".to_string()),
        "request_access" => Some("Check computer-control access".to_string()),
        "type" => Some(match text("text") {
            Some(typed) => format!("Type '{}'", clip(typed)),
            None => "Type text".to_string(),
        }),
        "key" => Some(match text("key").or_else(|| text("keys")) {
            Some(key) => format!("Press {}", clip(key)),
            None => "Press a key".to_string(),
        }),
        _ => None,
    };
    let action = action.unwrap_or_else(|| match argument_hint(input, workspace) {
        Some(hint) => format!("{}: {hint}", humanize(tool)),
        None => humanize(tool),
    });
    format!("{action} ({server})")
}

/// Split `<server>_<tool>`. A plugin-qualified server
/// (`plugin-<len>-<plugin>-<server>`) is length-prefixed, so its end is known
/// even when the name holds `_`.
fn mcp_server_and_tool(rest: &str) -> Option<(&str, &str)> {
    if let Some((_, server_and_tool)) = crate::mcp::split_qualified_plugin_server_name(rest)
        && let Some((_, tool)) = server_and_tool.split_once('_')
        && !tool.is_empty()
    {
        let server_len = rest.len() - tool.len() - 1;
        return Some((&rest[..server_len], tool));
    }
    match rest.split_once('_') {
        Some((server, tool)) if !server.is_empty() && !tool.is_empty() => Some((server, tool)),
        _ => None,
    }
}

/// A server as a person would name it: an included plugin shows its title
/// (`plugin-12-computer-use-computer` → `Computer Use`), never the wire key.
fn server_display(server: &str) -> String {
    match crate::mcp::split_qualified_plugin_server_name(server) {
        Some((plugin, _)) => plugin
            .split(['-', '_'])
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().chain(chars).collect::<String>())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
        None => server.to_string(),
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
            "Create issue (github)"
        );
        assert_eq!(
            approval_summary("mcp_github_create_issue", &json!({"title": "Fix it"}), None),
            "Create issue: Fix it (github)"
        );
        assert_eq!(
            approval_summary("some_tool", &json!({"a": 1}), None),
            "Use the some_tool tool"
        );
    }

    /// Desktop QA 2026-09-23 bug 3: the card read "Use browser from
    /// plugin-12-computer-use-computer" for opening a file in the workspace.
    #[test]
    fn computer_use_browser_names_the_page_not_the_wire_key() {
        let workspace = Path::new("/w/demo/field-notes");
        let summary = approval_summary(
            "mcp_plugin-12-computer-use-computer_browser",
            &json!({"action": "start", "url": "file:///w/demo/field-notes/field-guide.html"}),
            Some(workspace),
        );
        assert_eq!(
            summary,
            "Open field-guide.html in a controlled browser (Computer Use)"
        );
        assert!(!summary.contains("plugin-12"), "{summary}");
        assert_eq!(
            approval_summary(
                "mcp_codewhale-cu_browser_navigate",
                &json!({"url": "http://127.0.0.1:8000/field%20guide.html"}),
                None,
            ),
            "Open http://127.0.0.1:8000/field%20guide.html in a controlled browser (codewhale-cu)"
        );
        assert_eq!(
            approval_summary("mcp_codewhale-cu_list_apps", &json!({}), None),
            "List apps on this computer (codewhale-cu)"
        );
        assert_eq!(
            approval_summary(
                "mcp_plugin-12-computer-use-computer_open_application",
                &json!({"name": "Safari"}),
                None,
            ),
            "Open Safari (Computer Use)"
        );
        // A `_` inside a plugin-qualified server name does not split it.
        assert_eq!(
            approval_summary("mcp_plugin-6-my_kit-srv_do_thing", &json!({}), None),
            "Do thing (My Kit)"
        );
        // A file URL outside the workspace stays absolute and decoded.
        assert_eq!(
            approval_summary(
                "mcp_codewhale-cu_browser",
                &json!({"action": "navigate", "url": "file:///etc/my%20hosts"}),
                Some(workspace),
            ),
            "Open /etc/my hosts in a controlled browser (codewhale-cu)"
        );
    }

    /// Desktop QA 2026-09-23 bug 3: `Run{action:"verifiers", commands}` read
    /// "Use the run_verifiers tool" while it was about to launch Chrome.
    #[test]
    fn run_verifiers_names_what_will_run() {
        let chrome = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
        assert_eq!(
            approval_summary(
                "Run",
                &json!({"action": "verifiers", "commands": [{
                    "name": "print-render",
                    "program": chrome,
                    "args": ["--headless", "--print-to-pdf=out.pdf", "field-guide.html"]
                }]}),
                None,
            ),
            "Run `Google Chrome --headless --print-to-pdf=out.pdf field-guide.html`"
        );
        assert_eq!(
            approval_summary(
                "Run",
                &json!({"action": "verifiers", "commands": [
                    {"name": "list-apps", "program": "/bin/ls", "args": ["/Applications"]},
                    {"name": "print-render", "program": chrome, "args": ["--headless"]},
                    {"name": "third", "program": "true"}
                ]}),
                None,
            ),
            "Run 3 checks: list-apps, print-render (+1 more)"
        );
        assert_eq!(
            approval_summary("run_verifiers", &json!({"level": "quick"}), None),
            "Run the project's checks"
        );
        assert_eq!(
            approval_summary(
                "Run",
                &json!({"action": "tests", "args": "-p tui approval"}),
                None
            ),
            "Run `cargo test -p tui approval`"
        );
    }
}
