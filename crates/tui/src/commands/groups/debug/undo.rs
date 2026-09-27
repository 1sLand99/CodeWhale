//! Undo, retry, edit, and diff commands.

use crate::dependencies::{ExternalTool, Git};
use crate::tui::app::{App, AppAction};
use crate::tui::history::HistoryCell;
use codewhale_models::ContentBlock;
use std::path::PathBuf;

use super::CommandResult;

/// Opening of the [`patch_undo`] message that means the snapshot repo could
/// not be opened at all — as opposed to "there is nothing to revert". The
/// dispatcher must carry this into the conversation-only fallback: dropping it
/// left `/undo` reporting `Removed N message(s)` while the workspace files it
/// implied were reverted were never touched.
pub(in crate::commands) const SNAPSHOT_REPO_UNAVAILABLE_PREFIX: &str = "Snapshot repo unavailable";

/// Told to the user whenever conversation-only undo runs because the snapshot
/// repo was unavailable.
pub(in crate::commands) const FILES_NOT_REVERTED_NOTE: &str =
    "Workspace files were NOT reverted — only the conversation was rolled back.";

/// Remove last message pair (user + assistant).
///
/// This is the old `/undo` behaviour — it removes the most recent
/// user+assistant conversation pair from history and API messages.
/// The new `/undo` first tries to revert workspace files via
/// [`patch_undo`]; if no snapshots are available it falls back to
/// this function.
pub fn undo_conversation(app: &mut App) -> CommandResult {
    // Remove from display history (up to the last user message)
    let mut removed_count = 0;
    while !app.history.is_empty() {
        let last_is_user = matches!(app.history.last(), Some(HistoryCell::User { .. }));
        app.pop_history();
        removed_count += 1;
        if last_is_user {
            break;
        }
    }

    // Remove from API messages
    while let Some(last) = app.api_messages.last() {
        if last.role == "user" {
            app.pop_api_message();
            break;
        }
        app.pop_api_message();
    }

    if removed_count > 0 {
        // Keep tool/index mappings consistent after truncation.
        app.tool_cells.clear();
        app.tool_details_by_cell.clear();
        app.exploring_entries.clear();
        app.ignored_tool_calls.clear();
        app.mark_history_updated();
        CommandResult::message(format!("Removed {removed_count} message(s)"))
    } else {
        CommandResult::message("Nothing to undo")
    }
}

pub(crate) fn prune_undone_tool_context(app: &mut App, tool_id: &str) {
    if let Some(history_idx) = app.tool_cells.get(tool_id).copied() {
        app.truncate_history_to(history_idx);
    }

    let Some((msg_idx, block_idx)) =
        app.api_messages
            .iter()
            .enumerate()
            .find_map(|(msg_idx, msg)| {
                msg.content
                    .iter()
                    .position(
                        |block| matches!(block, ContentBlock::ToolUse { id, ..} if id == tool_id),
                    )
                    .map(|block_idx| (msg_idx, block_idx))
            })
    else {
        return;
    };

    let kept_blocks = app.api_messages[msg_idx].content[..block_idx].to_vec();
    let kept_tool_ids: std::collections::HashSet<String> = kept_blocks
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse { id, .. } => Some(id.clone()),
            _ => None,
        })
        .collect();

    if kept_blocks.is_empty() {
        app.truncate_api_messages(msg_idx);
        return;
    }
    // Re-inserted tool results keep the stamp they earned, so the journal's
    // timeline survives the prune.
    let preserved_tool_results: Vec<_> =
        app.api_messages_stamped()
            .skip(msg_idx + 1)
            .take_while(|(msg, _)| {
                msg.role == "user"
                    && !msg.content.is_empty()
                    && msg
                        .content
                        .iter()
                        .all(|block| tool_result_id(block).is_some())
            })
            .filter(|(msg, _)| {
                msg.role == "user"
                    && !msg.content.is_empty()
                    && msg.content.iter().all(|block| {
                        tool_result_id(block).is_some_and(|id| kept_tool_ids.contains(id))
                    })
            })
            .map(|(msg, stamp)| (msg.clone(), stamp))
            .collect();
    app.truncate_api_messages(msg_idx + 1);
    app.api_messages_mut()[msg_idx].content = kept_blocks;
    for (message, stamp) in preserved_tool_results {
        app.push_api_message_stamped(message, stamp);
    }
}

fn prune_undone_turn_context(app: &mut App) {
    if let Some(history_idx) = app
        .history
        .iter()
        .rposition(|cell| matches!(cell, HistoryCell::User { .. }))
    {
        app.truncate_history_to(history_idx);
    }

    if let Some(api_idx) = app.api_messages.iter().rposition(|msg| msg.role == "user") {
        app.truncate_api_messages(api_idx);
    }
}

fn tool_result_id(block: &ContentBlock) -> Option<&String> {
    match block {
        ContentBlock::ToolResult { tool_use_id, .. }
        | ContentBlock::ToolSearchToolResult { tool_use_id, .. }
        | ContentBlock::CodeExecutionToolResult { tool_use_id, .. } => Some(tool_use_id),
        _ => None,
    }
}

/// Deepest fork chain [`snapshot_owners`] follows. A chain this long is
/// already unusual; the bound only stops a corrupt lineage from looping.
const MAX_FORK_ANCESTORS: usize = 32;

/// A session whose restore points this conversation owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::commands) struct SnapshotOwner {
    /// Session tag the snapshots carry.
    pub(in crate::commands) session_id: String,
    /// Newest snapshot time (Unix seconds) owned from this session: `None`
    /// for the current session, the fork time for a session it was forked
    /// from. The source keeps working after the fork, and its later
    /// snapshots are not the fork's.
    pub(in crate::commands) until: Option<i64>,
}

impl SnapshotOwner {
    fn owns(&self, snapshot: &crate::snapshot::Snapshot) -> bool {
        snapshot.session_id.as_deref() == Some(self.session_id.as_str())
            && self.until.is_none_or(|until| snapshot.timestamp <= until)
    }
}

/// The sessions whose restore points `/undo` may use: the current session,
/// and for a fork each session it was forked from, up to the fork. A fork
/// copies its source's turns, so the snapshots those turns took (tagged
/// with the source's id) are the fork's too, as the Runtime's thread-owned
/// restore points are (#6621).
///
/// Lineage the saved sessions cannot prove ends the chain: fewer owners
/// means fewer restorable steps, never someone else's.
pub(in crate::commands) fn snapshot_owners(app: &App) -> Vec<SnapshotOwner> {
    let Some(current) = app.current_session_id.clone() else {
        return Vec::new();
    };
    let manager = crate::session_manager::SessionManager::default_location().ok();
    let load = |id: &str| {
        manager
            .as_ref()
            .and_then(|manager| manager.load_session_metadata_by_id(id).ok())
    };
    let mut metadata = app
        .current_session_metadata
        .clone()
        .filter(|metadata| metadata.id == current)
        .or_else(|| load(&current));
    let mut owners = vec![SnapshotOwner {
        session_id: current,
        until: None,
    }];
    while let Some(child) = metadata.take() {
        let Some(parent) = child.parent_session_id.clone() else {
            break;
        };
        if owners.len() > MAX_FORK_ANCESTORS
            || owners.iter().any(|owner| owner.session_id == parent)
        {
            break;
        }
        let forked_at = child.created_at.timestamp();
        let until = owners
            .last()
            .and_then(|owner| owner.until)
            .map_or(forked_at, |child_until| child_until.min(forked_at));
        metadata = load(&parent);
        owners.push(SnapshotOwner {
            session_id: parent,
            until: Some(until),
        });
    }
    owners
}

/// Labels a `/undo` step starts at: before one tool call, or before a turn.
fn is_undo_step_label(label: &str) -> bool {
    label.starts_with("tool:") || label.starts_with("pre-turn:")
}

/// Labels of the restore points an engine takes for a turn. A step runs from
/// one of them to the next one the conversation owns.
fn is_restore_point_label(label: &str) -> bool {
    is_undo_step_label(label) || label.starts_with("post-tool:") || label.starts_with("post-turn:")
}

/// One `/undo` step, planned but not applied.
pub(in crate::commands) struct UndoStep {
    /// Restore point the step started at.
    pub(in crate::commands) target: crate::snapshot::Snapshot,
    /// Tree the step ended at: the next restore point this conversation
    /// owns, or, for the newest step, a snapshot of the workspace as it is
    /// now. Trees, not commit ids, because a prune rewrites commit ids.
    pub(in crate::commands) end: crate::snapshot::SnapshotId,
    /// The paths the step changed that are still as it left them.
    pub(in crate::commands) restore: Vec<PathBuf>,
}

/// Why no step can be undone.
pub(in crate::commands) enum UndoRefusal {
    /// Nothing this conversation owns is left to undo.
    Nothing(String),
    /// A step is there, but undoing it would clobber later work, or the
    /// snapshot repo failed; nothing was changed.
    Refused(Box<CommandResult>),
}

/// Find the newest step of `snapshots` (newest first) that `owners` own and
/// that is not undone yet, and the paths undoing it restores.
///
/// A step is scoped to the paths that changed between its restore point and
/// the next one: edits to any other file (the user's, another session's)
/// are never touched. A path the step changed that changed again since is
/// refused rather than overwritten. A step whose paths are all back at its
/// restore point is already undone, so `/undo` walks back one tool call (or
/// turn) at a time (#384).
///
/// Known limits: the TUI records no per-tool receipts (the Runtime's
/// `post-tool:` spans and declared write paths), so a step owns everything
/// that changed between its restore point and the next one this
/// conversation owns, including a write another session made in that window.
/// The newest step, when no later restore point exists yet, ends at the
/// workspace as it is now.
pub(in crate::commands) fn plan_undo_step(
    repo: &crate::snapshot::SnapshotRepo,
    snapshots: Vec<crate::snapshot::Snapshot>,
    owners: &[SnapshotOwner],
) -> Result<UndoStep, UndoRefusal> {
    let owned: Vec<crate::snapshot::Snapshot> = snapshots
        .into_iter()
        .filter(|snapshot| is_restore_point_label(&snapshot.label))
        .filter(|snapshot| owners.iter().any(|owner| owner.owns(snapshot)))
        .collect();
    if !owned
        .iter()
        .any(|snapshot| is_undo_step_label(&snapshot.label))
    {
        return Err(UndoRefusal::Nothing(
            "No undoable snapshots for the current session — nothing to revert.".to_string(),
        ));
    }

    let compare_failed = |error: std::io::Error| {
        UndoRefusal::Refused(Box::new(
            if error.kind() == std::io::ErrorKind::InvalidInput {
                CommandResult::message(format!(
                    "A path the undone step changed cannot be restored file by file: {error}. \
                 Nothing was changed; use /restore for a whole-workspace rollback."
                ))
            } else {
                CommandResult::error(format!("Failed to compare snapshot: {error}"))
            },
        ))
    };

    for (index, target) in owned.iter().enumerate() {
        if !is_undo_step_label(&target.label) {
            continue;
        }
        let end = match index.checked_sub(1) {
            Some(newer) => owned[newer].tree.clone(),
            // The newest step has no later restore point (the turn is still
            // running, stopped early, or its post-turn snapshot has not
            // landed): the workspace now is the only record of its end.
            None => {
                if repo
                    .work_tree_matches_snapshot(&target.tree)
                    .map_err(compare_failed)?
                {
                    continue;
                }
                let short = &target.id.as_str()[..target.id.as_str().len().min(12)];
                repo.take_snapshot(&format!("pre-restore:{short}"), None)
                    .map_err(|error| {
                        UndoRefusal::Refused(Box::new(CommandResult::error(format!(
                            "Failed to snapshot the workspace before undo: {error}"
                        ))))
                    })?
                    .tree
            }
        };
        let changed = repo
            .changed_paths_between(&target.tree, &end)
            .map_err(compare_failed)?;
        let mut restore = Vec::new();
        let mut changed_since = Vec::new();
        'paths: for path in changed {
            if repo
                .path_matches_snapshot(&end, &path)
                .map_err(compare_failed)?
            {
                restore.push(path);
                continue;
            }
            // Back at the step's start, or at an older restore point that an
            // earlier `/undo` walked it back to: already undone.
            for older in &owned[index..] {
                if repo
                    .path_matches_snapshot(&older.tree, &path)
                    .map_err(compare_failed)?
                {
                    continue 'paths;
                }
            }
            changed_since.push(path.display().to_string());
        }
        if !changed_since.is_empty() {
            return Err(UndoRefusal::Refused(Box::new(CommandResult::message(
                format!(
                    "Refusing to undo snapshot '{}': {} changed after it, and undoing would overwrite \
                 that change. Nothing was changed; revert those files yourself, or use /restore \
                 for a whole-workspace rollback.",
                    target.label,
                    changed_since.join(", ")
                ),
            ))));
        }
        if restore.is_empty() {
            // Already undone, or the step changed nothing: keep walking back.
            continue;
        }
        return Ok(UndoStep {
            target: target.clone(),
            end,
            restore,
        });
    }
    Err(UndoRefusal::Nothing(
        "No undoable snapshot differs from the current workspace — nothing to revert.".to_string(),
    ))
}

/// Revert the most recent write tool (apply_patch/edit_file/write_file) or turn.
///
/// Opens the side-git snapshot repo and finds the newest `tool:*` or
/// `pre-turn:*` restore point this conversation owns (see
/// [`snapshot_owners`]) whose step is not undone yet, then restores only the
/// files that step changed (see [`plan_undo_step`]). Falls back to
/// conversation undo when no snapshots exist.
///
/// Posts a `HistoryCell::System` entry so the user can see what was
/// reverted in the transcript.
pub fn patch_undo(app: &mut App) -> CommandResult {
    let workspace = app.workspace.clone();

    let repo = match crate::snapshot::SnapshotRepo::open_or_init(&workspace) {
        Ok(r) => r,
        Err(e) => {
            return CommandResult::error(format!(
                "{SNAPSHOT_REPO_UNAVAILABLE_PREFIX} for {}: {e}",
                workspace.display(),
            ));
        }
    };

    // The whole store: an older restore point that is still stored must not
    // be mistaken for a pruned one.
    let snapshots = match repo.list(usize::MAX) {
        Ok(s) => s,
        Err(e) => {
            return CommandResult::error(format!("Failed to list snapshots: {e}"));
        }
    };

    if snapshots.is_empty() {
        return CommandResult::message("No snapshots found to undo — nothing to revert.");
    }

    // Automatic file rollback is allowed only when ownership is provable.
    // Untagged legacy snapshots and snapshots from another conversation may
    // describe unrelated user work in this same workspace, so fail closed
    // and let the command dispatcher fall back to conversation-only undo.
    let owners = snapshot_owners(app);
    if owners.is_empty() {
        return CommandResult::message(
            "No undoable snapshot is tagged for the current session — nothing to revert.",
        );
    }

    let step = match plan_undo_step(&repo, snapshots, &owners) {
        Ok(step) => step,
        Err(UndoRefusal::Nothing(message)) => return CommandResult::message(message),
        Err(UndoRefusal::Refused(result)) => return *result,
    };
    let target = &step.target;

    // Restoring workspace files is a mutation. Apply the trust gate only
    // after finding a real, owned step so chat-only `/undo` can still fall
    // back to conversation history in ordinary mode.
    if !(app.yolo || app.trust_mode) {
        return CommandResult::message(
            "Refusing to undo workspace files outside trusted mode.\n\
             Run `/trust on` or select Full Access with Shift+Tab, then re-run `/undo`.",
        );
    }

    let plan: Vec<(PathBuf, crate::snapshot::SnapshotId)> = step
        .restore
        .iter()
        .map(|path| (path.clone(), target.tree.clone()))
        .collect();
    let backup_short = &target.id.as_str()[..target.id.as_str().len().min(12)];
    let outcomes =
        match repo.restore_path_plan(&plan, &format!("pre-restore:{backup_short}"), true, || {
            // Re-verify after the safety snapshot, immediately before the
            // first write: a change that landed meanwhile is refused.
            for path in &step.restore {
                if !repo.path_matches_snapshot(&step.end, path)? {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        format!(
                            "'{}' changed while the undo was being prepared; nothing was changed.",
                            path.display()
                        ),
                    ));
                }
            }
            Ok(())
        }) {
            Ok(outcomes) => outcomes,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return CommandResult::message(e.to_string());
            }
            Err(e) => return CommandResult::error(format!("Restore failed: {e}")),
        };

    if let Some(tool_id) = target.label.strip_prefix("tool:") {
        prune_undone_tool_context(app, tool_id);
    } else if target.label.starts_with("pre-turn:") {
        prune_undone_turn_context(app);
    }

    let short = &target.id.as_str()[..target.id.as_str().len().min(8)];
    let lines: Vec<String> = outcomes
        .iter()
        .map(|outcome| format!("{} {}", outcome.action.as_str(), outcome.path.display()))
        .collect();
    let summary = format!(
        "Restored {} file(s) to snapshot '{}' ({}):\n{}",
        outcomes.len(),
        target.label,
        short,
        lines.join("\n")
    );

    // Post a system cell so the reverted state is visible in the transcript.
    app.push_history_cell(HistoryCell::System {
        content: format!(
            "/undo reverted workspace files to snapshot '{}' ({})",
            target.label, short
        ),
    });

    CommandResult::with_message_and_action(
        summary,
        AppAction::SyncSession {
            session_id: app.current_session_id.clone(),
            messages: app.api_messages.as_ref().clone(),
            system_prompt: app.system_prompt.clone(),
            model: app.model.clone(),
            workspace: app.workspace.clone(),
            mode: app.mode,
        },
    )
}

/// Load the last user message back into the composer for editing.
///
/// Searches `app.history` for the most recent `HistoryCell::User`, copies its
/// content into `app.input`, and positions the cursor at the end so the user
/// can edit and press Enter to resubmit. The original exchange stays visible
/// in the transcript.
pub fn edit(app: &mut App) -> CommandResult {
    let last_user = app.history.iter().rev().find_map(|cell| match cell {
        HistoryCell::User { content } => Some(content.clone()),
        _ => None,
    });

    match last_user {
        Some(content) => {
            app.input = content;
            app.cursor_position = app.input.chars().count();
            app.edit_in_progress = true;
            CommandResult::message(
                "Last message loaded into composer — edit and press Enter to resubmit",
            )
        }
        None => CommandResult::message("No previous message to edit"),
    }
}

/// Show git diff output since session start.
///
/// Runs `git diff --stat` and `git diff --name-only` in the workspace
/// directory. Displays which files have changed and a stat summary. If no
/// changes exist or git fails, returns an appropriate message.
pub fn diff(app: &mut App) -> CommandResult {
    let workspace = app.workspace.clone();

    let Some(mut name_only_cmd) = Git::command() else {
        return CommandResult::error("git not found on PATH");
    };
    let Some(mut stat_cmd) = Git::command() else {
        return CommandResult::error("git not found on PATH");
    };
    let name_only_output = name_only_cmd
        .args(["diff", "--name-only"])
        .current_dir(&workspace)
        .output();
    let stat_output = stat_cmd
        .args(["diff", "--stat"])
        .current_dir(&workspace)
        .output();

    match (name_only_output, stat_output) {
        (Ok(name_only), Ok(stat)) => {
            let name_stdout = String::from_utf8_lossy(&name_only.stdout);
            let stat_stdout = String::from_utf8_lossy(&stat.stdout);

            if name_stdout.trim().is_empty() {
                return CommandResult::message("No changes since session start");
            }

            let files: Vec<&str> = name_stdout.lines().filter(|l| !l.is_empty()).collect();
            let file_count = files.len();
            let file_list = files.join("\n");

            // Detect rename entries (e.g. "foo -> bar") and exclude them
            // from the file-count header so the user sees only actual
            // modifications.
            let renamed_count = files.iter().filter(|f| f.contains(" -> ")).count();
            let summary = if renamed_count > 0 {
                format!("Changed files ({file_count}, {renamed_count} renamed):\n{file_list}")
            } else {
                format!("Changed files ({file_count}):\n{file_list}")
            };

            let stat_str = stat_stdout.trim();
            let mut message = summary;
            if !stat_str.is_empty() {
                message.push_str("\n\n── Stat ──\n");
                message.push_str(stat_str);
            }
            CommandResult::message(message)
        }
        (Err(e), _) | (_, Err(e)) => {
            CommandResult::message(format!("Git diff failed — is this a git repository?\n{e}"))
        }
    }
}

/// Retry last request - remove last exchange and re-send the user's message
pub fn retry(app: &mut App) -> CommandResult {
    let last_user_input = app.history.iter().rev().find_map(|cell| match cell {
        HistoryCell::User { content } => Some(content.clone()),
        _ => None,
    });

    match last_user_input {
        Some(input) => {
            undo_conversation(app);
            let display_input = if input.len() > 50 {
                let truncate_at = input
                    .char_indices()
                    .take_while(|(i, _)| *i <= 50)
                    .last()
                    .map_or(0, |(i, _)| i);
                format!("{}...", &input[..truncate_at])
            } else {
                input.clone()
            };
            CommandResult::with_message_and_action(
                format!("Retrying: {display_input}"),
                AppAction::SendMessage(input),
            )
        }
        None => CommandResult::error("No previous request to retry"),
    }
}
