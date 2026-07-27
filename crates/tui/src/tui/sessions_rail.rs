//! The persistent Sessions rail (#2934).
//!
//! A bounded, workspace-scoped list of recent sessions that lives in the
//! sidebar panel stack alongside Work, Activity, Agents, and Context. It is a
//! *jump* affordance, not a second session browser: every row hands off to the
//! existing [`crate::tui::session_picker::SessionPickerView`], which already
//! owns preview, search, sort, rename, archive, delete, workspace-scope
//! toggling, and the resume contract. Duplicating any of that here would give
//! us two behaviours to keep in sync and one of them would eventually lie.
//!
//! Two properties this module exists to guarantee:
//!
//! * **No filesystem work per frame.** Rows are read once into a
//!   [`SessionsRailCache`] and reused until the TTL expires or a session
//!   lifecycle event invalidates them. A sidebar that re-listed a 50-session
//!   directory on every keystroke would be a visible stall.
//! * **No provider or network call.** Browsing sessions is offline by
//!   construction — the rail only ever reads already-persisted metadata.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::session_manager::{SessionListFilter, SessionManager, SessionMetadata};
use crate::session_projection::{
    DEFAULT_RAIL_ROWS, SessionQuery, SessionSortMode, SessionSummary, count_sessions,
    project_sessions,
};

/// How long cached rows stay warm. Long enough that scrolling and typing never
/// hit the disk, short enough that a session saved by this process or another
/// one shows up without the user doing anything.
pub const RAIL_CACHE_TTL: Duration = Duration::from_secs(20);

/// Rail rows plus the inputs they were computed from.
///
/// The inputs are stored so a workspace change or a different active session
/// invalidates the cache on its own, without every call site having to
/// remember to clear it.
#[derive(Debug, Clone)]
pub struct SessionsRailCache {
    rows: Vec<SessionSummary>,
    workspace: PathBuf,
    current_session_id: Option<String>,
    /// Total sessions in scope before the row cap, so the rail can say "8 of
    /// 31" rather than implying it is showing everything.
    total_in_scope: usize,
    /// Row budget this cache was built for. A resized sidebar changes the
    /// budget, so it participates in freshness — otherwise a grown panel would
    /// render short rows until the TTL happened to lapse.
    max_rows: usize,
    read_at: Instant,
    /// Set when the sessions directory could not be read. The rail renders
    /// this instead of an empty list, so a permissions or path problem is not
    /// silently displayed as "you have no sessions".
    error: Option<String>,
}

impl SessionsRailCache {
    #[must_use]
    pub fn rows(&self) -> &[SessionSummary] {
        &self.rows
    }

    #[must_use]
    pub fn total_in_scope(&self) -> usize {
        self.total_in_scope
    }

    #[must_use]
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Is this cache still usable for the given surface state?
    #[must_use]
    pub fn is_fresh(
        &self,
        workspace: &Path,
        current_session_id: Option<&str>,
        max_rows: usize,
        now: Instant,
    ) -> bool {
        self.workspace == workspace
            && self.current_session_id.as_deref() == current_session_id
            && self.max_rows == max_rows
            && now.duration_since(self.read_at) < RAIL_CACHE_TTL
    }
}

/// Build rail rows from an already-loaded metadata list.
///
/// Split out from [`load_rail_cache`] so the projection is testable without a
/// sessions directory, and so the rail and the picker demonstrably agree: both
/// go through [`project_sessions`].
#[must_use]
pub fn build_rail_cache(
    sessions: &[SessionMetadata],
    workspace: &Path,
    current_session_id: Option<&str>,
    max_rows: usize,
    now: Instant,
) -> SessionsRailCache {
    let scoped = SessionQuery::default()
        .with_filter(SessionListFilter::ActiveOnly)
        .with_sort(SessionSortMode::Recent)
        .scoped_to(workspace);

    // Count the full scoped set without building rows. Projecting with a huge
    // limit would silently re-clamp at `MAX_PROJECTED_SESSIONS` and under-report
    // the total, which is the one number this footer exists to tell the truth
    // about.
    let total_in_scope = count_sessions(sessions, &scoped);
    let rows = project_sessions(sessions, &scoped.with_limit(max_rows), current_session_id);

    SessionsRailCache {
        rows,
        workspace: workspace.to_path_buf(),
        current_session_id: current_session_id.map(str::to_string),
        total_in_scope,
        max_rows,
        read_at: now,
        error: None,
    }
}

/// Read the sessions directory and project rail rows.
///
/// Errors are captured into the cache rather than returned: a rail that cannot
/// read the store should say so once and keep rendering, not propagate a
/// failure into the render loop.
#[must_use]
pub fn load_rail_cache(
    workspace: &Path,
    current_session_id: Option<&str>,
    max_rows: usize,
) -> SessionsRailCache {
    let now = Instant::now();
    let listed = SessionManager::default_location().and_then(|manager| manager.list_sessions());

    match listed {
        Ok(sessions) => build_rail_cache(&sessions, workspace, current_session_id, max_rows, now),
        Err(err) => SessionsRailCache {
            rows: Vec::new(),
            workspace: workspace.to_path_buf(),
            current_session_id: current_session_id.map(str::to_string),
            total_in_scope: 0,
            max_rows,
            read_at: now,
            error: Some(err.to_string()),
        },
    }
}

/// The command a rail row dispatches when activated.
///
/// Opening the picker preselected on the row — rather than resuming inline —
/// is deliberate. Resume has draft, unsaved-work, and confirmation semantics
/// that the picker already implements; a rail row that resumed directly would
/// either re-implement them or quietly skip them.
#[must_use]
pub fn row_command(session_id: &str) -> String {
    format!("/sessions open {session_id}")
}

/// The command the rail's footer row dispatches: the full session browser.
#[must_use]
pub fn browse_all_command() -> &'static str {
    "/sessions list"
}

/// Default row budget for a sidebar of `height` rows.
///
/// Narrow and short terminals get fewer rows rather than a clipped panel; the
/// footer still reports the full in-scope count so nothing is silently
/// dropped.
#[must_use]
pub fn rows_for_height(height: u16) -> usize {
    // Two rows of chrome (border + title) and one footer row.
    let usable = usize::from(height).saturating_sub(3);
    usable.clamp(1, DEFAULT_RAIL_ROWS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn metadata(id: &str, title: &str, workspace: &Path, minutes_ago: i64) -> SessionMetadata {
        let ts = Utc::now() - chrono::Duration::minutes(minutes_ago);
        SessionMetadata {
            id: id.to_string(),
            title: title.to_string(),
            created_at: ts,
            updated_at: ts,
            message_count: 3,
            total_tokens: 10,
            model: "deepseek-chat".to_string(),
            model_provider: "deepseek".to_string(),
            model_provider_id: None,
            workspace: workspace.to_path_buf(),
            mode: Some("agent".to_string()),
            cost: Default::default(),
            parent_session_id: None,
            forked_from_message_count: None,
            cumulative_turn_secs: 0,
            archived: false,
        }
    }

    #[test]
    fn rail_is_scoped_to_the_workspace_and_capped() {
        let here = PathBuf::from("/repo-a");
        let there = PathBuf::from("/repo-b");
        let mut sessions: Vec<SessionMetadata> = (0..12)
            .map(|i| metadata(&format!("a{i}"), &format!("Here {i}"), &here, i))
            .collect();
        sessions.push(metadata("elsewhere", "There", &there, 1));

        let cache = build_rail_cache(&sessions, &here, None, 5, Instant::now());

        assert_eq!(cache.rows().len(), 5);
        assert_eq!(cache.total_in_scope(), 12);
        assert!(
            cache.rows().iter().all(|row| row.workspace == here),
            "rail must not leak another workspace's sessions"
        );
    }

    #[test]
    fn archived_sessions_never_appear_in_the_rail() {
        let here = PathBuf::from("/repo");
        let mut archived = metadata("gone", "Archived", &here, 1);
        archived.archived = true;
        let sessions = vec![archived, metadata("live", "Live", &here, 2)];

        let cache = build_rail_cache(&sessions, &here, None, 8, Instant::now());

        assert_eq!(
            cache
                .rows()
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["live"]
        );
        assert_eq!(cache.total_in_scope(), 1);
    }

    #[test]
    fn the_active_session_is_marked_current() {
        let here = PathBuf::from("/repo");
        let sessions = vec![metadata("a", "A", &here, 1), metadata("b", "B", &here, 2)];

        let cache = build_rail_cache(&sessions, &here, Some("b"), 8, Instant::now());

        let current: Vec<&str> = cache
            .rows()
            .iter()
            .filter(|r| r.is_current)
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(current, vec!["b"]);
    }

    #[test]
    fn cache_expires_on_ttl_workspace_session_or_row_budget_change() {
        let here = PathBuf::from("/repo");
        let now = Instant::now();
        let cache = build_rail_cache(&[metadata("a", "A", &here, 1)], &here, Some("a"), 8, now);

        assert!(cache.is_fresh(&here, Some("a"), 8, now));
        assert!(
            !cache.is_fresh(
                &here,
                Some("a"),
                8,
                now + RAIL_CACHE_TTL + Duration::from_secs(1)
            ),
            "TTL must expire the cache"
        );
        assert!(
            !cache.is_fresh(Path::new("/other"), Some("a"), 8, now),
            "a workspace change must invalidate the cache"
        );
        assert!(
            !cache.is_fresh(&here, Some("b"), 8, now),
            "switching sessions must invalidate the cache"
        );
        assert!(
            !cache.is_fresh(&here, Some("a"), 4, now),
            "a resized sidebar changes the row budget and must invalidate the cache"
        );
    }

    #[test]
    fn row_activation_opens_the_picker_rather_than_resuming_inline() {
        assert_eq!(row_command("abc123"), "/sessions open abc123");
        assert_eq!(browse_all_command(), "/sessions list");
    }

    #[test]
    fn narrow_and_short_sidebars_get_fewer_rows_but_never_zero() {
        assert_eq!(rows_for_height(0), 1);
        assert_eq!(rows_for_height(4), 1);
        assert_eq!(rows_for_height(7), 4);
        assert_eq!(rows_for_height(40), DEFAULT_RAIL_ROWS);
    }
}
