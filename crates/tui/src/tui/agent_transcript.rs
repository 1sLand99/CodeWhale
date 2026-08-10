//! The agent transcript surface — the primary destination for a child agent.
//!
//! v0.9.7 "one agent, one destination": activating an agent row anywhere
//! (Work strip, sidebar dossier, `/agents`) opens this surface directly. When
//! no transcript has been captured yet the same surface opens in an
//! explanatory state instead of bouncing to a different view, so one agent id
//! always resolves to one destination. The bounded Agent Details projection
//! remains reachable from here as the secondary action (Alt+V / ⌥V).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::{buffer::Buffer, layout::Rect};

use crate::tui::app::App;
use crate::tui::pager::PagerView;
use crate::tui::views::{ModalKind, ModalView, ViewAction, ViewEvent};

/// Pager-backed transcript view with a distinct close receipt and an explicit
/// secondary action into the bounded Agent Details projection.
pub(crate) struct AgentTranscriptView {
    pager: PagerView,
    agent_id: String,
}

impl AgentTranscriptView {
    fn new(title: String, body: &str, agent_id: impl Into<String>, width: u16) -> Self {
        let pager = PagerView::from_text(title, body, width.saturating_sub(2))
            .with_copy_text(body.to_string());
        Self {
            pager,
            agent_id: agent_id.into(),
        }
    }

    #[cfg(test)]
    pub(crate) fn body_text(&self) -> String {
        self.pager.body_text()
    }

    #[cfg(test)]
    pub(crate) fn title(&self) -> &str {
        self.pager.title()
    }
}

impl ModalView for AgentTranscriptView {
    fn kind(&self) -> ModalKind {
        ModalKind::Pager
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        // While `/`-search is typing, every printable key belongs to the
        // query; the chord and close keys below only apply outside it.
        if !self.pager.in_search_mode() {
            if matches!(key.code, KeyCode::Char('v' | 'V'))
                && key.modifiers.contains(KeyModifiers::ALT)
            {
                return ViewAction::Emit(ViewEvent::OpenAgentDetails {
                    agent_id: self.agent_id.clone(),
                });
            }
            if matches!(key.code, KeyCode::Esc | KeyCode::Left)
                || (key.code == KeyCode::Char('q') && key.modifiers.is_empty())
            {
                return ViewAction::EmitAndClose(ViewEvent::AgentTranscriptClosed {
                    agent_id: self.agent_id.clone(),
                });
            }
        }
        self.pager.handle_key(key)
    }

    fn handle_paste(&mut self, text: &str) -> bool {
        self.pager.handle_paste(text)
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> ViewAction {
        self.pager.handle_mouse(mouse)
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        self.pager.render(area, buf);
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Open the agent's transcript surface — the one destination every agent row
/// activation resolves to. Always opens: when no durable or resident
/// transcript exists yet, the surface explains why instead of routing the
/// user somewhere else.
pub(crate) fn open_agent_transcript(app: &mut App, agent_id: &str) {
    let transcript = crate::tui::mouse_ui::resolve_agent_transcript_text(app, agent_id);
    let display_name = crate::tui::agent_details::safe_agent_display_name(app, agent_id);
    // Platform glyph via display_chord (⌥V on macOS, Alt+V elsewhere) —
    // cap:verb, matching the Agent Details hint spelling.
    let chord = crate::tui::shell_key_routing::tool_details_chord();
    let hint = format!("Agent details: {chord}:details");
    let body = match transcript {
        Some(text) => format!("{hint}\n\n{text}"),
        None => format!(
            "{hint}\n\nNo transcript captured yet.\n\n\
             A worker's chat appears here once it exchanges its first \
             messages; finished workers keep a durable copy under the \
             workspace. If this agent just started, reopen after it makes \
             progress."
        ),
    };
    let width = app
        .viewport
        .last_transcript_area
        .map(|area| area.width)
        .unwrap_or(80);
    app.view_stack.push(AgentTranscriptView::new(
        format!("Agent transcript — {display_name}"),
        &body,
        agent_id,
        width,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use serde_json::json;
    use tempfile::tempdir;

    use crate::config::Config;
    use crate::tui::app::TuiOptions;

    fn test_app(workspace: PathBuf) -> App {
        App::new(
            TuiOptions {
                model: "test-model".to_string(),
                use_mouse_capture: true,
                max_subagents: 4,
                ..crate::test_support::test_tui_options(workspace)
            },
            &Config::default(),
        )
    }

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    fn open_with_resident_transcript(app: &mut App, agent_id: &str) {
        {
            let mut store = app
                .runtime_services
                .handle_store
                .try_lock()
                .expect("handle store");
            let _ = store.insert_json(
                format!("agent:{agent_id}"),
                "full_transcript",
                json!({
                    "message_count": 1,
                    "messages": [{
                        "role": "assistant",
                        "content": [{
                            "type": "text",
                            "text": "exact evidence",
                            "cache_control": null
                        }]
                    }]
                }),
            );
        }
        open_agent_transcript(app, agent_id);
    }

    #[test]
    fn transcript_is_the_destination_with_and_without_evidence() {
        let tmp = tempdir().expect("tempdir");
        let mut app = test_app(tmp.path().to_path_buf());
        let agent_id = "agent_destination";

        // Without any captured transcript the same surface opens in an
        // explanatory state — activation never dead-ends or reroutes.
        open_agent_transcript(&mut app, agent_id);
        let mut view = app.view_stack.pop().expect("transcript surface");
        let view = view
            .as_any_mut()
            .downcast_mut::<AgentTranscriptView>()
            .expect("agent transcript view");
        assert!(view.title().starts_with("Agent transcript — "));
        assert!(view.body_text().contains("No transcript captured yet"));
        let details_hint = format!(
            "Agent details: {}:details",
            crate::tui::shell_key_routing::tool_details_chord()
        );
        assert!(view.body_text().contains(&details_hint));

        // With resident evidence the same destination shows the exact chat.
        open_with_resident_transcript(&mut app, agent_id);
        let mut view = app.view_stack.pop().expect("transcript surface");
        let view = view
            .as_any_mut()
            .downcast_mut::<AgentTranscriptView>()
            .expect("agent transcript view");
        assert!(view.body_text().contains("exact evidence"));
        assert!(view.body_text().contains(&details_hint));
    }

    #[test]
    fn alt_v_requests_details_and_close_keys_emit_receipt() {
        let tmp = tempdir().expect("tempdir");
        let mut app = test_app(tmp.path().to_path_buf());
        let agent_id = "agent_secondary";
        open_with_resident_transcript(&mut app, agent_id);

        let events = app
            .view_stack
            .handle_key(key(KeyCode::Char('v'), KeyModifiers::ALT));
        assert!(matches!(
            events.as_slice(),
            [ViewEvent::OpenAgentDetails { agent_id: id }] if id == agent_id
        ));

        for code in [KeyCode::Esc, KeyCode::Left, KeyCode::Char('q')] {
            open_with_resident_transcript(&mut app, agent_id);
            let events = app.view_stack.handle_key(key(code, KeyModifiers::NONE));
            assert!(
                matches!(
                    events.as_slice(),
                    [ViewEvent::AgentTranscriptClosed { agent_id: id }] if id == agent_id
                ),
                "{code:?} must close the transcript with a receipt: {events:?}"
            );
        }
    }

    #[test]
    fn search_mode_keeps_q_and_chord_keys_as_query_input() {
        let tmp = tempdir().expect("tempdir");
        let mut app = test_app(tmp.path().to_path_buf());
        let agent_id = "agent_search";
        open_with_resident_transcript(&mut app, agent_id);

        assert!(
            app.view_stack
                .handle_key(key(KeyCode::Char('/'), KeyModifiers::NONE))
                .is_empty()
        );
        // `q` inside the search prompt is query text, not a close key.
        assert!(
            app.view_stack
                .handle_key(key(KeyCode::Char('q'), KeyModifiers::NONE))
                .is_empty()
        );
        assert!(!app.view_stack.is_empty(), "search 'q' must not close");
        // Esc exits search mode without closing the surface.
        assert!(
            app.view_stack
                .handle_key(key(KeyCode::Esc, KeyModifiers::NONE))
                .is_empty()
        );
        assert!(!app.view_stack.is_empty(), "search Esc must not close");
    }
}
