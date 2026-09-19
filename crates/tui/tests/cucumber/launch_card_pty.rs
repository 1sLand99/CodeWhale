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
    let workspace = make_sealed_workspace().unwrap();
    std::fs::write(workspace.home().join(".codewhale/.onboarded"), "").unwrap();
    let trust = workspace.workspace().join(".deepseek");
    std::fs::create_dir_all(&trust).unwrap();
    std::fs::write(trust.join("trusted"), "").unwrap();
    let sessions = workspace.home().join(".codewhale/sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    let session = serde_json::json!({
        "schema_version": 1,
        "metadata": {
            "id": "11111111-2222-4333-8444-555555555555",
            "title": TITLE,
            "created_at": "2026-09-19T00:00:00Z",
            "updated_at": "2026-09-19T00:00:00Z",
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
        sessions.join("11111111-2222-4333-8444-555555555555.json"),
        serde_json::to_vec(&session).unwrap(),
    )
    .unwrap();
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
        click_text(&mut tui, TITLE);
        wait(&mut tui, "Resume");
        tui.wait_for_idle(Duration::from_millis(200), WAIT).unwrap();
        tui.send(keys::key::enter()).unwrap();
        // No pointer motion follows Enter: the accepted action must run now.
        wait(&mut tui, SAVED_TEXT);
        tui.shutdown();
    }
}

#[test]
fn launch_mcp_summary_opens_manager_by_click_and_keyboard() {
    for (rows, cols) in SIZES {
        let (_workspace, mut tui) = start(rows, cols, true);
        wait(&mut tui, "MCP");
        click_text(&mut tui, "MCP");
        wait(&mut tui, "Extensions");
        wait(&mut tui, "launch-proof");
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
