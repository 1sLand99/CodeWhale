//! Child-agent requests waiting on the person (approvals C1).
//!
//! A child's approval card can be hidden (Esc), buried under another view, or
//! arrive while the parent is idle. This store is the one source for what is
//! still waiting: the footer row, `/agents` re-opening a card, and retiring a
//! card that was answered elsewhere all read it. It is keyed by approval id,
//! so parallel child calls each keep their own entry.

use std::time::Instant;

use crate::tools::subagent::SubAgentManager;
use crate::tui::app::App;
use crate::tui::approval::ApprovalOwner;
use codewhale_localization::MessageId;

/// Everything needed to re-raise a hidden child card.
#[derive(Debug, Clone)]
pub(crate) struct PendingChildRequest {
    pub agent_id: String,
    pub tool_name: String,
    pub description: String,
    pub input: serde_json::Value,
    pub approval_key: String,
    pub approval_grouping_key: String,
    pub intent_summary: Option<String>,
    pub requested_at: Instant,
}

/// The owning agent id of a child approval id
/// (`agent:{agent_id}:approval:{boot}:{n}`), or `None` for a parent id.
#[must_use]
pub(crate) fn child_agent_id(approval_id: &str) -> Option<&str> {
    if !SubAgentManager::is_child_approval_id(approval_id) {
        return None;
    }
    approval_id
        .strip_prefix("agent:")?
        .split_once(":approval:")
        .map(|(agent_id, _)| agent_id)
        .filter(|agent_id| !agent_id.is_empty())
}

/// Owner label for a child's card: stable agent label plus Fleet role.
pub(crate) fn owner_for(app: &mut App, agent_id: &str) -> ApprovalOwner {
    let label = app.ensure_agent_label(agent_id);
    let role = app
        .subagent_cache
        .iter()
        .find(|agent| agent.agent_id == agent_id)
        .map(|agent| agent.agent_type.as_str().to_string());
    ApprovalOwner {
        agent_id: agent_id.to_string(),
        label,
        role,
    }
}

/// Remember a child request that is now shown to the person.
pub(crate) fn record(app: &mut App, approval_id: &str, request: PendingChildRequest) {
    app.pending_child_requests
        .insert(approval_id.to_string(), request);
    app.needs_redraw = true;
}

/// The request was answered (here, on the web, or by the engine ending the
/// wait): forget it and retire its card wherever it sits in the stack.
pub(crate) fn resolve(app: &mut App, approval_id: &str) -> bool {
    let known = app.pending_child_requests.remove(approval_id).is_some();
    let removed_card = app.view_stack.remove_approval_by_id(approval_id);
    if known || removed_card {
        app.needs_redraw = true;
    }
    known || removed_card
}

/// A progress event that names a child approval id and is no longer waiting
/// means that wait ended (answered anywhere, cancelled, stopped): retire the
/// card and entry by identity.
pub(crate) fn observe_progress(
    app: &mut App,
    activity: &crate::core::events::AgentProgressEventMeta,
) -> bool {
    match activity.approval_id.as_deref() {
        Some(approval_id)
            if activity.worker_status
                != crate::tools::subagent::AgentWorkerStatus::WaitingForUser =>
        {
            resolve(app, approval_id)
        }
        _ => false,
    }
}

/// The agent finished or stopped: nothing it asked for is pending any more.
pub(crate) fn clear_for_agent(app: &mut App, agent_id: &str) {
    let ids: Vec<String> = app
        .pending_child_requests
        .iter()
        .filter(|(_, request)| request.agent_id == agent_id)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        resolve(app, &id);
    }
}

/// A different conversation owns none of this one's pending requests.
pub(crate) fn clear_all(app: &mut App) {
    let ids: Vec<String> = app.pending_child_requests.keys().cloned().collect();
    for id in ids {
        resolve(app, &id);
    }
}

/// One footer row per agent that is waiting on the person and whose card is
/// not the view on top: "Approval needed in {agent} — /agents".
pub(crate) fn footer_rows(app: &App) -> Vec<String> {
    let top = app.view_stack.top_approval_id();
    let mut agents: Vec<&str> = Vec::new();
    for (id, request) in &app.pending_child_requests {
        if top == Some(id.as_str()) || agents.contains(&request.agent_id.as_str()) {
            continue;
        }
        agents.push(request.agent_id.as_str());
    }
    agents
        .into_iter()
        .map(|agent_id| {
            let label = app
                .agent_label_map
                .get(agent_id)
                .cloned()
                .unwrap_or_else(|| agent_id.to_string());
            app.tr(MessageId::PendingApprovalInAgent)
                .replace("{agent}", &label)
        })
        .collect()
}

/// Re-raise the agent's hidden cards (oldest first) so the person can answer
/// them after `/agents` or "Go to agent". Cards already in the stack stay put.
pub(crate) fn repush_for_agent(
    app: &mut App,
    agent_id: &str,
    default_selection: crate::config::ApprovalDefaultSelection,
    timeout: Option<std::time::Duration>,
) -> bool {
    let mut waiting: Vec<(String, PendingChildRequest)> = app
        .pending_child_requests
        .iter()
        .filter(|(id, request)| {
            request.agent_id == agent_id && !app.view_stack.contains_approval_id(id)
        })
        .map(|(id, request)| (id.clone(), request.clone()))
        .collect();
    waiting.sort_by_key(|(_, request)| request.requested_at);
    let pushed = !waiting.is_empty();
    for (id, request) in waiting.into_iter().rev() {
        crate::tui::ui::push_approval_request_view(
            app,
            &id,
            &request.tool_name,
            &request.description,
            &request.input,
            &request.approval_key,
            &request.approval_grouping_key,
            request.intent_summary.as_deref(),
            default_selection,
            timeout,
        );
    }
    pushed
}
