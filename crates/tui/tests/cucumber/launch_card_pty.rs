//! Launch actions must work through the real input loop, including Enter
//! after a mouse click. All state is sealed; no provider is contacted.

use std::time::Duration;

use super::qa_harness;
use qa_harness::harness::{Harness, SealedWorkspace, make_sealed_workspace};
use qa_harness::keys;

const WAIT: Duration = Duration::from_secs(15);
const SIZES: [(u16, u16); 5] = [(12, 40), (16, 60), (24, 80), (32, 100), (40, 140)];
const TITLE: &str = "Recent proof";
const SAVED_TEXT: &str = "Restored conversation proof";

fn start(rows: u16, cols: u16, with_mcp: bool) -> (SealedWorkspace, Harness) {
    start_titled(rows, cols, with_mcp, TITLE)
}

fn start_titled(rows: u16, cols: u16, with_mcp: bool, title: &str) -> (SealedWorkspace, Harness) {
    start_with_titles(rows, cols, with_mcp, &[title])
}

fn start_with_titles(
    rows: u16,
    cols: u16,
    with_mcp: bool,
    titles: &[&str],
) -> (SealedWorkspace, Harness) {
    let workspace = make_sealed_workspace().unwrap();
    std::fs::write(workspace.home().join(".codewhale/.onboarded"), "").unwrap();
    let trust = workspace.workspace().join(".deepseek");
    std::fs::create_dir_all(&trust).unwrap();
    std::fs::write(trust.join("trusted"), "").unwrap();
    let sessions = workspace.home().join(".codewhale/sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    for (index, title) in titles.iter().enumerate() {
        let id = format!(
            "11111111-2222-4333-8444-{:012}",
            555555555555u64 + index as u64
        );
        let session = serde_json::json!({
            "schema_version": 1,
            "metadata": {
                "id": id,
                "title": title,
                "created_at": "2026-09-19T00:00:00Z",
                "updated_at": format!("2026-09-19T00:00:{:02}Z", 59usize.saturating_sub(index)),
                "message_count": 1,
                "total_tokens": 0,
                "model": "deepseek-flash",
                "model_provider": "deepseek",
                "workspace": workspace.workspace()
            },
            "messages": [{"role": "user", "content": [{"type": "text", "text": SAVED_TEXT}]}],
            "system_prompt": null
        });
        std::fs::write(
            sessions.join(format!("{id}.json")),
            serde_json::to_vec(&session).unwrap(),
        )
        .unwrap();
    }
    if with_mcp {
        // A local failing server gives the summary a real row without any network.
        let mcp = serde_json::json!({"mcpServers": {"launch-proof": {
            "command": "/usr/bin/false", "required": true
        }}});
        std::fs::write(
            workspace.home().join(".codewhale/mcp.json"),
            serde_json::to_vec(&mcp).unwrap(),
        )
        .unwrap();
    }

    let mut tui = Harness::builder(Harness::cargo_bin("codewhale-tui"))
        .cwd(workspace.workspace())
        .clear_env()
        .seal_home(workspace.home())
        .env("CODEWHALE_DISABLE_MODELS_DEV_FETCH", "1")
        .env("CODEWHALE_NO_UPDATE_CHECK", "1")
        .env("NO_ANIMATIONS", "1")
        .env("COLORTERM", "truecolor")
        .args([
            "--workspace",
            workspace.workspace().to_str().unwrap(),
            "--no-project-config",
            "--fresh",
            "--mouse-capture",
        ])
        .size(rows, cols)
        .spawn()
        .unwrap();
    wait(&mut tui, "Choose your model provider");
    tui.send(keys::key::ctrl('o')).unwrap();
    wait(&mut tui, "You're ready.");
    tui.send(keys::key::enter()).unwrap();
    wait(&mut tui, "New session");
    tui.wait_for_idle(Duration::from_millis(300), WAIT).unwrap();
    tui.send(keys::key::ctrl('u')).unwrap();
    tui.wait_for_idle(Duration::from_millis(200), WAIT).unwrap();
    (workspace, tui)
}

fn wait(tui: &mut Harness, text: &str) {
    if let Err(error) = tui.wait_for(|frame| frame.contains(text), WAIT) {
        panic!("waiting for {text:?}: {error}\n{}", tui.diagnostics());
    }
}

fn click_text(tui: &mut Harness, text: &str) {
    tui.pump();
    let (row, col) = tui
        .frame()
        .find_text(text)
        .unwrap_or_else(|| panic!("missing click target {text:?}\n{}", tui.diagnostics()));
    tui.send(keys::mouse::click(row, col)).unwrap();
}

#[test]
fn launch_recent_click_then_enter_resumes_without_another_mouse_event() {
    for (rows, cols) in SIZES {
        let (_workspace, mut tui) = start(rows, cols, false);
        wait(&mut tui, TITLE);
        capture(&mut tui, "home");
        tui.send(keys::key::down()).unwrap();
        tui.send(keys::key::down()).unwrap();
        capture(&mut tui, "selected");
        click_text(&mut tui, TITLE);
        wait(&mut tui, "Resume");
        tui.wait_for_idle(Duration::from_millis(200), WAIT).unwrap();
        capture(&mut tui, "confirm");
        tui.send(keys::key::enter()).unwrap();
        // No pointer motion follows Enter: the accepted action must run now.
        wait(&mut tui, SAVED_TEXT);
        assert!(
            !tui.frame().contains("Session loaded from"),
            "resume should not add a technical path receipt to the conversation"
        );
        capture(&mut tui, "conversation");
        tui.shutdown();
    }
}

#[test]
fn launch_mcp_summary_opens_manager_by_click_and_keyboard() {
    for (rows, cols) in SIZES {
        let (_workspace, mut tui) = start(rows, cols, true);
        wait(&mut tui, "MCP");
        capture(&mut tui, "home-mcp");
        click_text(&mut tui, "MCP");
        wait(&mut tui, "Extensions");
        wait(&mut tui, "launch-proof");
        capture(&mut tui, "mcp");
        tui.send(keys::key::esc()).unwrap();
        wait(&mut tui, "New session");
        // New session, the recent row (or compact See all), then MCP.
        for _ in 0..3 {
            tui.send(keys::key::down()).unwrap();
        }
        tui.send(keys::key::enter()).unwrap();
        wait(&mut tui, "Extensions");
        wait(&mut tui, "launch-proof");
        tui.shutdown();
    }
}

#[test]
fn launch_resume_buttons_support_mouse_cancel_and_keyboard_choice() {
    for (rows, cols) in SIZES {
        let (_workspace, mut tui) = start(rows, cols, false);
        click_text(&mut tui, TITLE);
        wait(&mut tui, "resume");
        click_text(&mut tui, "cancel");
        wait(&mut tui, "New session");
        assert!(!tui.frame().contains(SAVED_TEXT));
        click_text(&mut tui, TITLE);
        wait(&mut tui, "resume");
        tui.send(keys::key::tab()).unwrap();
        tui.send(keys::key::enter()).unwrap();
        wait(&mut tui, "New session");
        assert!(!tui.frame().contains(SAVED_TEXT));
        click_text(&mut tui, TITLE);
        wait(&mut tui, "resume");
        click_text(&mut tui, "resume");
        wait(&mut tui, SAVED_TEXT);
        tui.shutdown();
    }
}

/// Optional review evidence from the real PTY, keeping cell colors rather
/// than relying on symbol-only goldens. The viewer supplies terminal fonts.
fn capture(tui: &mut Harness, name: &str) {
    let Some(directory) = std::env::var_os("QA_LAUNCH_CAPTURE_DIR") else {
        return;
    };
    tui.wait_for_idle(Duration::from_millis(200), WAIT).unwrap();
    let frame = tui.frame();
    let directory = std::path::PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{name}-{}x{}.json", frame.cols(), frame.rows()));
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&frame.capture_cells()).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "opt-in visual evidence; writes only with QA_LAUNCH_CAPTURE_DIR"]
fn workbench_settings_visual_evidence() {
    assert!(std::env::var_os("QA_LAUNCH_CAPTURE_DIR").is_some());
    for (command, title, name) in [
        ("/model", "route ·", "models"),
        ("/provider", "Provider", "providers"),
        ("/fleet", "Coordinator", "fleet"),
        ("/plugin", "Extensions", "plugins"),
        ("/config", "Config", "settings"),
    ] {
        for (rows, cols) in SIZES {
            let (_workspace, mut tui) = start(rows, cols, true);
            tui.paste(command).unwrap();
            tui.wait_for_idle(Duration::from_millis(300), WAIT).unwrap();
            tui.send(keys::key::enter()).unwrap();
            wait(&mut tui, title);
            capture(&mut tui, name);
            if name == "providers" {
                tui.send(keys::key::alt('v')).unwrap();
                wait(&mut tui, "DeepSeek · Open details");
                capture(&mut tui, "provider-details");
                tui.send(keys::key::esc()).unwrap();
                wait(&mut tui, "Provider");
            }
            tui.shutdown();
        }
    }
}

#[test]
#[ignore = "opt-in populated launch evidence; fixture sessions, no provider calls"]
fn workbench_populated_home_visual_evidence() {
    assert!(std::env::var_os("QA_LAUNCH_CAPTURE_DIR").is_some());
    for (rows, cols) in SIZES {
        let (_workspace, mut tui) = start_with_titles(
            rows,
            cols,
            true,
            &[
                "Polish the release notes",
                "Investigate a provider timeout",
                "Review the plugin setup flow",
            ],
        );
        capture(&mut tui, "home-populated");
        tui.send(keys::key::down()).unwrap();
        tui.send(keys::key::down()).unwrap();
        capture(&mut tui, "home-populated-selected");
        tui.shutdown();
    }
}

#[test]
fn launch_long_resume_title_preserves_warning_and_truthful_enter_hint() {
    let title = "Investigate provider timeouts and connection failures across multiple accounts, preserve the original credentials, and verify every saved session can still be restored after the upgrade";
    for (rows, cols) in SIZES {
        let (_workspace, mut tui) = start_titled(rows, cols, false, title);
        click_text(&mut tui, "Investigate provider");
        wait(&mut tui, "resume");
        tui.wait_for_idle(Duration::from_millis(200), WAIT).unwrap();
        capture(&mut tui, "confirm-long");
        let text = tui
            .frame()
            .text()
            .chars()
            .map(|ch| {
                if ('\u{2500}'..='\u{257f}').contains(&ch) {
                    ' '
                } else {
                    ch
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            text.contains("This replaces the current context with that session's history."),
            "{text}"
        );
        tui.send(keys::key::tab()).unwrap();
        tui.wait_for_idle(Duration::from_millis(200), WAIT).unwrap();
        let text = tui.frame().text();
        assert!(text.contains("cancel  Enter"), "{text}");
        assert!(!text.contains("resume  Enter"), "{text}");
        capture(&mut tui, "confirm-cancel");
        tui.send(keys::key::enter()).unwrap();
        wait(&mut tui, "New session");
        assert!(!tui.frame().contains(SAVED_TEXT));
        tui.shutdown();
    }
}
