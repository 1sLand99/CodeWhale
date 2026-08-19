//! Coherent shell grammar for the underwater TUI.
//!
//! This module owns phase, responsive density, the empty-state composition,
//! and the compact header/footer fact budget. Product data still belongs to
//! [`App`]; this is only its terminal projection. Keeping these decisions in
//! one place prevents the default UI from drifting back into a header +
//! sidebar + dashboard + footer composition with four owners for one fact.

use std::borrow::Cow;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::config::HeaderItem;
use crate::localization::{Locale, MessageId, tr};
use crate::palette::{ChromeInk, chrome_style};
use crate::tui::{
    app::{App, AppMode, OnboardingState},
    approval::ApprovalMode,
    footer_ui::format_token_count_compact,
    views::ModalKind,
};

/// Responsive density tier. It changes how much truth is shown, never the
/// underlying state grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellTier {
    Compact,
    Normal,
    Wide,
}

const LAUNCH_ROWS: [(MessageId, &str); 6] = [
    (MessageId::LaunchMenuWork, "Enter"),
    (MessageId::LaunchMenuChat, "C"),
    (MessageId::LaunchMenuResumeSession, "Ctrl+R"),
    (MessageId::LaunchMenuNewWorktree, "Ctrl+N"),
    (MessageId::LaunchMenuChangelog, "Ctrl+L"),
    (MessageId::LaunchMenuQuit, "Ctrl+Q"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchAction {
    None,
    NewSession,
    NewChat,
    CreateWorktree(String),
    Resume,
    Changelog,
    Quit,
}

impl LaunchAction {
    /// Session-only mode selected by a launch choice. The event loop applies
    /// this with `App::set_mode`, never the startup-default-writing selector.
    #[must_use]
    pub const fn session_mode(&self) -> Option<AppMode> {
        match self {
            Self::NewSession => Some(AppMode::Agent),
            Self::NewChat => Some(AppMode::Plan),
            _ => None,
        }
    }
}

/// Translate launch-menu input into one product action. Direct reliable keys
/// and row navigation share this path, so the printed key column cannot drift
/// away from the handler.
pub fn handle_launch_key(
    launch: &mut crate::tui::app::LaunchState,
    key: KeyEvent,
    locale: Locale,
) -> LaunchAction {
    if let Some(input) = launch.worktree_input.as_mut() {
        return match key.code {
            KeyCode::Esc => {
                launch.worktree_input = None;
                launch.status = None;
                LaunchAction::None
            }
            KeyCode::Enter => {
                let name = input.trim().to_string();
                launch.worktree_input = None;
                LaunchAction::CreateWorktree(name)
            }
            KeyCode::Backspace => {
                input.pop();
                LaunchAction::None
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                launch.worktree_input = None;
                launch.status = None;
                LaunchAction::None
            }
            KeyCode::Char(ch)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) =>
            {
                input.push(ch);
                LaunchAction::None
            }
            _ => LaunchAction::None,
        };
    }

    let direct = match key.code {
        KeyCode::Char('c') | KeyCode::Char('C')
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER) =>
        {
            Some(1)
        }
        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(2),
        KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(3),
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(4),
        KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(5),
        _ => None,
    };
    if let Some(selected) = direct {
        launch.selected = selected;
    } else {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                launch.selected = launch.selected.saturating_sub(1);
                return LaunchAction::None;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                launch.selected = (launch.selected + 1).min(LAUNCH_ROWS.len() - 1);
                return LaunchAction::None;
            }
            KeyCode::Enter => {}
            _ => return LaunchAction::None,
        }
    }

    match launch.selected {
        0 => LaunchAction::NewSession,
        1 => LaunchAction::NewChat,
        2 => LaunchAction::Resume,
        3 if launch.worktree_available => {
            launch.worktree_input = Some(String::new());
            launch.status = Some(tr(locale, MessageId::LaunchWorktreePrompt).into_owned());
            LaunchAction::None
        }
        3 => {
            launch.status = Some(tr(locale, MessageId::LaunchWorktreeNeedsGit).into_owned());
            LaunchAction::None
        }
        4 => LaunchAction::Changelog,
        5 => LaunchAction::Quit,
        _ => LaunchAction::None,
    }
}

impl ShellTier {
    #[must_use]
    pub fn for_area(area: Rect) -> Self {
        if area.width < 60 || area.height < 16 {
            Self::Compact
        } else if area.width < 110 || area.height < 30 {
            Self::Normal
        } else {
            Self::Wide
        }
    }

    #[must_use]
    pub fn for_chrome_width(width: u16) -> Self {
        if width < 60 {
            Self::Compact
        } else if width < 110 {
            Self::Normal
        } else {
            Self::Wide
        }
    }
}

/// Perceptual session phase. Every treatment reads from this same enum so a
/// footer cannot say `idle` while the transcript is asking for approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellPhase {
    Idle,
    Typing,
    Working,
    /// A live verification pass (tests/checks/lints). Same clock family as
    /// `Working` but rendered as the metered braille tick — checking, not
    /// searching (ocean state model).
    Verifying,
    Waiting,
    Approval,
    Done,
    Failed,
}

/// The one truthful verb shown while a turn is live. This deliberately stays
/// smaller than the tool taxonomy: the phase strip only needs to distinguish
/// hidden reasoning, read-shaped exploration, other tool use, verification,
/// and generic model work. It never exposes reasoning text or tool arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LiveActivityKind {
    Working,
    Compacting,
    AutoCompacting,
    Reasoning,
    Reading,
    UsingTool,
    Verifying,
}

/// Bounded projection of live turn activity. Completed entries are ignored,
/// so an `ActiveCell` retained until `TurnComplete` cannot keep the shell in a
/// false working state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LiveActivity {
    kind: LiveActivityKind,
    running_tools: usize,
}

impl LiveActivity {
    #[must_use]
    pub(crate) fn from_app(app: &App) -> Self {
        let tools = running_tool_facts(app);
        let kind = if app
            .active_compaction
            .as_ref()
            .is_some_and(|compaction| compaction.auto)
        {
            LiveActivityKind::AutoCompacting
        } else if app.active_compaction.is_some() {
            LiveActivityKind::Compacting
        } else if tools.verifying {
            LiveActivityKind::Verifying
        } else if tools.count > 0 && tools.all_reading {
            LiveActivityKind::Reading
        } else if tools.count > 0 {
            LiveActivityKind::UsingTool
        } else if app.streaming_thinking_active_entry.is_some() {
            LiveActivityKind::Reasoning
        } else {
            LiveActivityKind::Working
        };
        Self {
            kind,
            running_tools: tools.count,
        }
    }

    #[must_use]
    pub(crate) fn kind(self) -> LiveActivityKind {
        self.kind
    }

    #[must_use]
    pub(crate) fn running_tool_count(self) -> usize {
        self.running_tools
    }

    #[must_use]
    fn is_explicit(self) -> bool {
        !matches!(self.kind, LiveActivityKind::Working)
    }

    #[must_use]
    fn label(self, locale: Locale) -> Cow<'static, str> {
        match self.kind {
            LiveActivityKind::Working => tr(locale, MessageId::PhaseWorking),
            LiveActivityKind::Compacting => tr(locale, MessageId::ContextManualCompacting),
            LiveActivityKind::AutoCompacting => tr(locale, MessageId::ContextAutoCompacting),
            LiveActivityKind::Reasoning => tr(locale, MessageId::PhaseReasoning),
            LiveActivityKind::Reading => tr(locale, MessageId::PhaseReading),
            LiveActivityKind::UsingTool => tr(locale, MessageId::PhaseUsingTool),
            LiveActivityKind::Verifying => tr(locale, MessageId::PhaseVerifying),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct RunningToolFacts {
    count: usize,
    all_reading: bool,
    verifying: bool,
}

impl Default for RunningToolFacts {
    fn default() -> Self {
        Self {
            count: 0,
            all_reading: true,
            verifying: false,
        }
    }
}

impl RunningToolFacts {
    fn observe(&mut self, reading: bool, verifying: bool) {
        self.count = self.count.saturating_add(1);
        self.all_reading &= reading;
        self.verifying |= verifying;
    }
}

const WORKING_BUBBLE_FRAMES: [&str; 8] = ["⠀", "⢀", "⣀", "⣄", "⣤", "⣦", "⣶", "⣿"];
const COMPLETION_BREATH_MS: u128 = 800;
const COMPLETION_RELEASE_MS: u128 = 560;
/// Signal Cut hero mark. The Whale Teams roster (CWC 2026-08-15) reads
/// head-left, blunt nose, swept dorsal on an arched back, a short tail stock
/// that stays body mass (`▙▄▄▞`) and rises into the attached crown fluke
/// `▚△▞`. The fluke's notch `△` sits directly above the rising stock tip
/// `▞`, so the tail reads as one continuous animal instead of a bar with a
/// shape floating past it. The belly carries one cyan current cut. The glyph
/// vocabulary is the one `whales::art` uses for the six-role portraits.
const IDLE_WHALE_SPOUT_ROW: &str = "    ˚";
const IDLE_WHALE_ROWS: [&str; 3] = ["  ▗▄▄▟▄▄▄▄▄▖  ▚△▞", " ▐█·████████▙▄▄▞", "  ▝▀▀▀▀▀▀▀▀▘"];

/// Soft variant: same silhouette, one body cell shorter, blush around the eye
/// and a sparkle beside the spout.
const UWU_IDLE_WHALE_SPOUT_ROW: &str = "    ˚✦";
const UWU_IDLE_WHALE_ROWS: [&str; 3] = ["  ▗▄▄▟▄▄▄▄▖  ▚△▞", " ▐█░·░█████▙▄▄▞", "  ▝▀▀▀▀▀▀▀▘"];

/// The belly row is the mark's cyan current cut, not gold body mass; it holds
/// still while the caustic sweep travels across the gold rows above it.
const IDLE_WHALE_CURRENT_ROW: usize = 2;

const IDLE_SHIMMER_CYCLE_MS: u128 = 4_000;
const IDLE_SHIMMER_SWEEP_FRACTION: f32 = 0.32;
const IDLE_SHIMMER_BAND_HALF_WIDTH: f32 = 0.38;
const IDLE_SHIMMER_STRENGTH: f32 = 0.33;

/// The build-version string the header renders. Since #5245 an unstamped
/// local build reports `0.9.4 (dev)` while CI/release carries a sha, so the
/// header's width choreography (which lengths of version stamp fit at which
/// terminal width) is environment-dependent. Tests that assert on those
/// width breakpoints override this to a fixed value so they measure the
/// layout, not the ambient build's sha length.
fn shell_build_version() -> Cow<'static, str> {
    #[cfg(test)]
    {
        if let Some(version) = tests::build_version_override() {
            return Cow::Owned(version);
        }
    }
    Cow::Borrowed(env!("DEEPSEEK_BUILD_VERSION"))
}

impl ShellPhase {
    #[must_use]
    pub fn from_app(app: &App) -> Self {
        Self::from_app_with_activity(app, LiveActivity::from_app(app))
    }

    #[must_use]
    pub(crate) fn from_app_with_activity(app: &App, activity: LiveActivity) -> Self {
        if matches!(
            app.view_stack.top_kind(),
            Some(ModalKind::Approval | ModalKind::Elevation | ModalKind::UserInput)
        ) {
            return Self::Approval;
        }
        if matches!(
            activity.kind(),
            LiveActivityKind::Compacting | LiveActivityKind::AutoCompacting
        ) {
            // A typed CompactionStarted event is newer and more specific than
            // a prior turn's failed projection. Keep the recovery operation
            // visible until its matching terminal event arrives.
            return Self::Working;
        }
        if app.turn_error_posted
            || matches!(app.runtime_turn_status.as_deref(), Some("failed" | "error"))
        {
            return Self::Failed;
        }
        if app.pending_user_input_prompt.is_some()
            || app
                .task_panel
                .iter()
                .any(|task| matches!(task.status.as_str(), "waiting" | "needs_user"))
        {
            return Self::Waiting;
        }
        if app.is_loading
            || matches!(app.runtime_turn_status.as_deref(), Some("in_progress"))
            || activity.is_explicit()
        {
            if activity.kind() == LiveActivityKind::Verifying {
                return Self::Verifying;
            }
            return Self::Working;
        }
        if !app.input.is_empty() {
            return Self::Typing;
        }
        if matches!(app.runtime_turn_status.as_deref(), Some("completed")) {
            return Self::Done;
        }
        Self::Idle
    }

    #[must_use]
    pub fn label(self, locale: Locale) -> Cow<'static, str> {
        match self {
            Self::Idle => tr(locale, MessageId::PhaseIdle),
            Self::Typing => tr(locale, MessageId::PhaseDraft),
            Self::Working => tr(locale, MessageId::PhaseWorking),
            Self::Verifying => tr(locale, MessageId::PhaseVerifying),
            Self::Waiting | Self::Approval => tr(locale, MessageId::PhaseWaitingOnYou),
            Self::Done => tr(locale, MessageId::PhaseDone),
            Self::Failed => tr(locale, MessageId::PhaseFailed),
        }
    }

    #[must_use]
    pub fn color(self, app: &App) -> Color {
        phase_ink(self).color(&app.ui_theme)
    }
}

/// Status-bar phase ink. Failure red is only `Failed`.
#[must_use]
pub(crate) fn phase_ink(phase: ShellPhase) -> ChromeInk {
    match phase {
        ShellPhase::Idle => ChromeInk::Metadata,
        ShellPhase::Done => ChromeInk::Outcome,
        ShellPhase::Typing => ChromeInk::Identity,
        // Verifying shares the live seafoam hue; the tick-vs-bubble
        // marker carries the checking/searching distinction.
        ShellPhase::Working | ShellPhase::Verifying => ChromeInk::Active,
        ShellPhase::Waiting | ShellPhase::Approval => ChromeInk::Waiting,
        ShellPhase::Failed => ChromeInk::Failure,
    }
}

/// Exhaustive on purpose: a new [`AppMode`] must be handed a Policy ink
/// deliberately rather than inheriting act's by falling through a wildcard.
fn header_mode_ink(mode: AppMode) -> ChromeInk {
    match mode {
        AppMode::Plan => ChromeInk::PolicyPlan,
        AppMode::Operate => ChromeInk::PolicyOperate,
        // YOLO stays Policy, not Failure — the header must not spend red
        // on a selected mode. It wears the act badge because `mode_label`
        // resolves it to act; the posture it implies is the permission
        // chip's Cognition ink, not this one.
        AppMode::Agent | AppMode::Auto | AppMode::Yolo => ChromeInk::PolicyAct,
    }
}

fn header_permission_ink(mode: ApprovalMode) -> ChromeInk {
    match mode {
        ApprovalMode::Suggest | ApprovalMode::Never => ChromeInk::PermissionAsk,
        ApprovalMode::Auto => ChromeInk::PermissionAutoReview,
        ApprovalMode::Bypass => ChromeInk::PermissionFullAccess,
    }
}

fn header_fg(app: &App, ink: ChromeInk) -> Style {
    chrome_style(&app.ui_theme, ink)
}

/// Summarize only tools whose lifecycle is actually `Running`. A read label
/// is earned only when every running entry is read/exploration-shaped; mixed
/// work stays the neutral `using tool`. Verification wins because it is the
/// existing stronger promise made by the phase strip.
fn running_tool_facts(app: &App) -> RunningToolFacts {
    use crate::tui::history::{HistoryCell, ToolCell, ToolStatus};
    use crate::tui::widgets::tool_card::{ToolFamily, tool_family_for_name};

    let mut facts = RunningToolFacts::default();
    let Some(active) = app.active_cell.as_ref() else {
        return facts;
    };
    for cell in active.entries() {
        let HistoryCell::Tool(tool) = cell else {
            continue;
        };
        match tool {
            ToolCell::Exec(exec) if exec.status == ToolStatus::Running => {
                facts.observe(false, exec_is_verification(&exec.command));
            }
            ToolCell::Generic(generic) if generic.status == ToolStatus::Running => {
                let family = tool_family_for_name(&generic.name);
                facts.observe(
                    matches!(family, ToolFamily::Read | ToolFamily::Find),
                    family == ToolFamily::Verify || generic.name == "read_lints",
                );
            }
            ToolCell::Exploring(exploring) => {
                for entry in &exploring.entries {
                    if entry.status == ToolStatus::Running {
                        facts.observe(true, false);
                    }
                }
            }
            ToolCell::WebSearch(search) if search.status == ToolStatus::Running => {
                facts.observe(true, false);
            }
            other if other.status() == Some(ToolStatus::Running) => {
                facts.observe(false, false);
            }
            _ => {}
        }
    }
    facts
}

fn exec_is_verification(command: &str) -> bool {
    let trimmed = command.trim_start();
    let mut tokens = trimmed.split_whitespace();
    let first = tokens.next().unwrap_or("");
    let second = tokens.next().unwrap_or("");
    match first {
        "cargo" => matches!(second, "test" | "check" | "clippy" | "nextest"),
        "go" => matches!(second, "test" | "vet"),
        "npm" | "pnpm" | "yarn" | "bun" => matches!(second, "test" | "lint" | "check"),
        "make" => matches!(second, "test" | "check" | "lint"),
        "python" | "python3" => trimmed.contains("-m pytest") || trimmed.contains("-m unittest"),
        "pytest" | "jest" | "vitest" | "tsc" | "eslint" | "ruff" | "mypy" | "clippy-driver"
        | "golangci-lint" | "shellcheck" => true,
        _ => false,
    }
}

fn completion_elapsed_ms(app: &App) -> Option<u128> {
    if !app.motion_policy().allows_decorative() {
        return None;
    }
    app.ocean_completion_started_at
        .map(|started| started.elapsed().as_millis())
        .filter(|elapsed| *elapsed < COMPLETION_BREATH_MS)
}

#[cfg(test)]
pub(crate) fn phase_marker(app: &App, phase: ShellPhase) -> (&'static str, Cow<'static, str>) {
    phase_marker_with_activity(app, phase, LiveActivity::from_app(app))
}

/// Truthful window-title activity verb for the OSC-0 whale animation.
///
/// Uses short English fragments (with fixed-width ellipsis) so alt-tabbed
/// sessions stay legible without depending on the full localized phase strip.
#[must_use]
pub(crate) fn title_activity_verb(app: &App) -> &'static str {
    let activity = LiveActivity::from_app(app);
    let phase = ShellPhase::from_app_with_activity(app, activity);
    match phase {
        ShellPhase::Waiting | ShellPhase::Approval => "waiting on you…",
        ShellPhase::Verifying => "verifying…",
        ShellPhase::Done => "done",
        ShellPhase::Failed => "failed",
        ShellPhase::Typing => "drafting…",
        ShellPhase::Idle => "idle",
        ShellPhase::Working => match activity.kind() {
            LiveActivityKind::Compacting | LiveActivityKind::AutoCompacting => {
                "compacting context…"
            }
            LiveActivityKind::Reasoning => "reasoning…",
            LiveActivityKind::Reading => "reading…",
            LiveActivityKind::UsingTool => "using tool…",
            LiveActivityKind::Verifying => "verifying…",
            LiveActivityKind::Working => "working…",
        },
    }
}

/// Push the current shell phase into the terminal title whale animation.
pub(crate) fn sync_title_activity(app: &App) {
    crate::tui::notifications::set_title_motion_enabled(
        app.motion_policy().allows_decorative() && app.status_indicator != "off",
    );
    // Keep the `[title] …` window-title prefix in step with the session and
    // config defaults; change detection inside makes this free when nothing
    // moved.
    crate::tui::notifications::set_title_prefix(app.window_title_prefix());
    if app.is_loading
        || matches!(
            ShellPhase::from_app(app),
            ShellPhase::Working
                | ShellPhase::Verifying
                | ShellPhase::Waiting
                | ShellPhase::Approval
                | ShellPhase::Typing
        )
    {
        crate::tui::notifications::set_title_activity_verb(title_activity_verb(app));
    }
}

pub(crate) fn phase_marker_with_activity(
    app: &App,
    phase: ShellPhase,
    activity: LiveActivity,
) -> (&'static str, Cow<'static, str>) {
    let locale = app.ui_locale;
    match phase {
        ShellPhase::Idle => ("·", phase.label(locale)),
        ShellPhase::Typing => ("›", phase.label(locale)),
        ShellPhase::Working => {
            // The footer and the live tool card share one wall-clock cadence,
            // so the two primary liveness marks never look like unrelated
            // spinners. The shared helper also preserves the 400ms
            // "motion is earned" delay and reduced/still fallback.
            let policy = app.motion_policy();
            let animated = crate::tui::spinner::braille_spinner_frame(app.turn_started_at, false);
            let earned = app.turn_started_at.is_none_or(|started| {
                started.elapsed().as_millis()
                    >= u128::from(crate::tui::spinner::LIVE_MARKER_DELAY_MS)
            });
            let frame = policy.spinner_glyph(animated, earned);
            (frame, activity.label(locale))
        }
        ShellPhase::Verifying => {
            // Metered braille tick on the shared live clock — checking, not
            // searching. Reduced motion holds the legible mid frame.
            let policy = app.motion_policy();
            let animated = crate::tui::spinner::verification_tick_frame(app.turn_started_at, false);
            let earned = app.turn_started_at.is_none_or(|started| {
                started.elapsed().as_millis()
                    >= u128::from(crate::tui::spinner::LIVE_MARKER_DELAY_MS)
            });
            let frame = policy.spinner_glyph(animated, earned);
            (frame, phase.label(locale))
        }
        ShellPhase::Waiting | ShellPhase::Approval => ("◆", phase.label(locale)),
        ShellPhase::Done => match completion_elapsed_ms(app) {
            Some(elapsed) if elapsed < COMPLETION_RELEASE_MS => {
                let index = ((elapsed / 140) as usize + 4).min(WORKING_BUBBLE_FRAMES.len() - 1);
                (
                    WORKING_BUBBLE_FRAMES[index],
                    tr(locale, MessageId::PhaseFinishing),
                )
            }
            _ => (crate::tui::glyphs::DONE, phase.label(locale)),
        },
        ShellPhase::Failed => (crate::tui::glyphs::FAILED, phase.label(locale)),
    }
}

fn mode_label(locale: Locale, mode: AppMode) -> Cow<'static, str> {
    match mode {
        AppMode::Agent | AppMode::Auto | AppMode::Yolo => tr(locale, MessageId::ChipModeAct),
        AppMode::Plan => tr(locale, MessageId::ChipModePlan),
        AppMode::Operate => tr(locale, MessageId::ChipModeOperate),
    }
}

/// Permission chip words. This maps from the typed [`ApprovalMode`] state —
/// never from the English `permission_chip_label()` strings — so localizing
/// (or rewording) the upstream chip labels can never silently break the chip.
fn permission_label(app: &App) -> Cow<'static, str> {
    let locale = app.ui_locale;
    if app.mode == AppMode::Plan {
        return tr(locale, MessageId::ChipPermissionReadOnly);
    }
    let approval = match app.approval_mode {
        ApprovalMode::Suggest => tr(locale, MessageId::ChipPermissionAsk),
        ApprovalMode::Auto => tr(locale, MessageId::ChipPermissionAuto),
        // Keep the effective permission explicit. `bypass` is an
        // implementation detail and, more importantly, can imply that
        // repository law no longer applies. Full Access never bypasses
        // constitution rules. This is **tool-approval posture**, not
        // filesystem scope — see filesystem_scope_label.
        ApprovalMode::Bypass => tr(locale, MessageId::ChipPermissionFullAccess),
        ApprovalMode::Never => tr(locale, MessageId::ChipPermissionNever),
    };
    // Append filesystem scope so "Full Access" (approval) is never confused
    // with unrestricted disk writes.
    let fs = filesystem_scope_label(app);
    Cow::Owned(format!("{approval} · {fs}"))
}

/// Always-legible effective filesystem scope for the shell chrome.
#[must_use]
fn filesystem_scope_label(app: &App) -> Cow<'static, str> {
    // Spelled out because the old `fs:` prefix read as an unexplained
    // acronym (user report, 2026-07-23): this chip states which files the
    // session may write.
    let policy = crate::core::authority::sandbox_policy_for_turn(
        app.mode,
        app.approval_mode,
        app.configured_sandbox_mode.as_deref(),
        &app.workspace,
    );
    // A policy is an intent; enforcement needs a backend. On default Linux
    // (bubblewrap is opt-in) and on all Windows there is none, and this chip
    // used to say "files: workspace" while nothing restricted anything
    // (2026-08-04 audit). Say "unenforced" rather than name a boundary that
    // is not applied. `DangerFullAccess` is already honest, and
    // `ExternalSandbox` is enforced by the external runner, not by us.
    let unenforced = app.sandbox_backend.is_none()
        && !matches!(
            policy,
            crate::sandbox::SandboxPolicy::DangerFullAccess
                | crate::sandbox::SandboxPolicy::ExternalSandbox { .. }
        );
    match policy {
        crate::sandbox::SandboxPolicy::ReadOnly if unenforced => {
            Cow::Borrowed("files: read-only (unenforced)")
        }
        crate::sandbox::SandboxPolicy::ReadOnly => Cow::Borrowed("files: read-only"),
        crate::sandbox::SandboxPolicy::DangerFullAccess => Cow::Borrowed("files: full disk"),
        crate::sandbox::SandboxPolicy::ExternalSandbox { .. } => {
            Cow::Borrowed("files: external sandbox")
        }
        crate::sandbox::SandboxPolicy::WorkspaceWrite { .. } if unenforced => {
            Cow::Borrowed("files: workspace (unenforced)")
        }
        crate::sandbox::SandboxPolicy::WorkspaceWrite { .. } => Cow::Borrowed("files: workspace"),
    }
}

fn span_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.width()).sum()
}

fn truncate_to_width(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width <= 3 {
        return ".".repeat(width);
    }
    let mut result = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width + 1 > width {
            break;
        }
        result.push(ch);
        used += ch_width;
    }
    result.push('…');
    result
}

fn render_launch_line(area: Rect, buf: &mut Buffer, y: u16, spans: Vec<Span<'static>>) {
    if y >= area.height {
        return;
    }
    Paragraph::new(Line::from(spans)).render(
        Rect {
            x: area.x,
            y: area.y.saturating_add(y),
            width: area.width,
            height: 1,
        },
        buf,
    );
}

fn render_launch_content_line(
    area: Rect,
    buf: &mut Buffer,
    y: u16,
    inset: u16,
    spans: Vec<Span<'static>>,
) {
    if y >= area.height {
        return;
    }
    let inset = inset.min(area.width / 2);
    Paragraph::new(Line::from(spans)).render(
        Rect {
            x: area.x.saturating_add(inset),
            y: area.y.saturating_add(y),
            width: area.width.saturating_sub(inset.saturating_mul(2)),
            height: 1,
        },
        buf,
    );
}

fn launch_has_detail(area: Rect) -> bool {
    area.width >= 60 && area.height >= 22
}

fn launch_content_start(_area: Rect) -> u16 {
    // Keep the decision block anchored just below the shell header at every
    // detailed size. Vertically centering it made a wide terminal look like
    // an old fixed-height menu floating in decorative emptiness.
    3
}

fn launch_row_y(area: Rect, index: usize) -> u16 {
    const DETAIL_ROW_OFFSETS: [u16; 6] = [4, 7, 11, 12, 15, 16];
    let start = launch_content_start(area);
    if launch_has_detail(area) {
        start.saturating_add(DETAIL_ROW_OFFSETS[index])
    } else {
        start.saturating_add(u16::try_from(index).unwrap_or(0))
    }
}

fn launch_workspace_name(app: &App) -> String {
    app.workspace
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map_or_else(
            || crate::utils::display_path(&app.workspace),
            str::to_string,
        )
}

/// Render the distinct pre-session choice state. This screen contains no
/// transcript, composer, dashboard, or post-launch whale: each row dispatches
/// to real session/worktree machinery before the idle ocean is entered.
pub fn render_launch_screen(area: Rect, buf: &mut Buffer, app: &App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    Block::default()
        .style(Style::default().bg(app.ui_theme.surface_bg))
        .render(area, buf);
    let width = usize::from(area.width);
    let version = format!("v{}", shell_build_version());
    let workspace_budget = width.saturating_sub(version.width() + 6);
    let workspace = truncate_to_width(
        &crate::utils::display_path(&app.workspace),
        workspace_budget,
    );
    let mut header = vec![
        Span::styled(
            "cw",
            Style::default()
                .fg(app.ui_theme.accent_primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(workspace, Style::default().fg(app.ui_theme.text_muted)),
    ];
    let gap = width.saturating_sub(span_width(&header) + version.width());
    header.push(Span::raw(" ".repeat(gap)));
    header.push(Span::styled(
        version,
        Style::default().fg(app.ui_theme.text_hint),
    ));
    render_launch_line(area, buf, 0, header);
    if area.height > 1 {
        render_launch_line(
            area,
            buf,
            1,
            vec![Span::styled(
                "─".repeat(width),
                Style::default().fg(app.ui_theme.border),
            )],
        );
    }

    if launch_has_detail(area) {
        let content_start = launch_content_start(area);
        render_launch_content_line(
            area,
            buf,
            content_start,
            2,
            vec![Span::styled(
                tr(app.ui_locale, MessageId::LaunchStartTitle).into_owned(),
                Style::default()
                    .fg(app.ui_theme.text_body)
                    .add_modifier(Modifier::BOLD),
            )],
        );
        let workspace_id = if app.launch.worktree_available {
            MessageId::LaunchWorkspaceGitReady
        } else {
            MessageId::LaunchWorkspaceFolderReady
        };
        render_launch_content_line(
            area,
            buf,
            content_start.saturating_add(1),
            2,
            vec![Span::styled(
                tr(app.ui_locale, workspace_id).replace("{name}", &launch_workspace_name(app)),
                Style::default().fg(app.ui_theme.text_soft),
            )],
        );
        let provider_id = if app.onboarding_needs_api_key {
            MessageId::LaunchProviderSetupNeeded
        } else {
            MessageId::LaunchProviderConfigured
        };
        render_launch_content_line(
            area,
            buf,
            content_start.saturating_add(2),
            2,
            vec![Span::styled(
                tr(app.ui_locale, provider_id).into_owned(),
                Style::default().fg(if app.onboarding_needs_api_key {
                    app.ui_theme.warning
                } else {
                    app.ui_theme.success
                }),
            )],
        );
        for (row, description_id) in [
            (launch_row_y(area, 0), MessageId::LaunchWorkDescription),
            (launch_row_y(area, 1), MessageId::LaunchChatDescription),
        ] {
            render_launch_content_line(
                area,
                buf,
                row.saturating_add(1),
                4,
                vec![Span::styled(
                    tr(app.ui_locale, description_id).into_owned(),
                    Style::default().fg(app.ui_theme.text_muted),
                )],
            );
        }
        for (row, heading_id) in [
            (launch_row_y(area, 2), MessageId::LaunchGroupContinue),
            (launch_row_y(area, 4), MessageId::LaunchGroupMore),
        ] {
            render_launch_content_line(
                area,
                buf,
                row.saturating_sub(1),
                2,
                vec![Span::styled(
                    tr(app.ui_locale, heading_id).into_owned(),
                    Style::default()
                        .fg(app.ui_theme.text_hint)
                        .add_modifier(Modifier::BOLD),
                )],
            );
        }
    }

    for (index, (label_id, key)) in LAUNCH_ROWS.iter().enumerate() {
        let y = launch_row_y(area, index);
        if y >= area.height.saturating_sub(3) {
            break;
        }
        let selected = app.launch.selected == index;
        let mut label = tr(app.ui_locale, *label_id).into_owned();
        if index == 3 && !app.launch.worktree_available {
            label.push_str(&format!(
                " · {}",
                tr(app.ui_locale, MessageId::LaunchMenuUnavailable)
            ));
        }
        if index == 2 {
            label.push_str(&format!(
                " · {}",
                tr(app.ui_locale, MessageId::LaunchMenuSavedCount)
                    .replace("{count}", &app.launch.workspace_session_count.to_string())
            ));
        }
        let prefix = if selected { "▸ " } else { "  " };
        let key_width = key.width();
        let content_width = width.saturating_sub(4);
        let label_budget = content_width.saturating_sub(prefix.width() + key_width + 2);
        let label = truncate_to_width(&label, label_budget);
        let fill = content_width.saturating_sub(prefix.width() + label.width() + key_width);
        let row_style = if selected {
            crate::tui::menu_style::theme_selected_row_style(&app.ui_theme)
        } else if index == 3 && !app.launch.worktree_available {
            Style::default().fg(app.ui_theme.text_dim)
        } else {
            Style::default().fg(app.ui_theme.text_body)
        };
        let key_style = if selected {
            row_style
        } else {
            Style::default().fg(app.ui_theme.text_hint)
        };
        render_launch_content_line(
            area,
            buf,
            y,
            2,
            vec![
                Span::styled(prefix, row_style),
                Span::styled(label, row_style),
                Span::styled(" ".repeat(fill), row_style),
                Span::styled(*key, key_style),
            ],
        );
    }

    if area.height < 3 {
        return;
    }
    let rule_y = area.height.saturating_sub(3);
    render_launch_line(
        area,
        buf,
        rule_y,
        vec![Span::styled(
            "─".repeat(width),
            Style::default().fg(app.ui_theme.border),
        )],
    );
    let prompt = if let Some(input) = app.launch.worktree_input.as_deref() {
        format!(
            "{}  {}{}",
            tr(app.ui_locale, MessageId::LaunchWorktreeNameLabel),
            input,
            if app.low_motion { "_" } else { "▌" }
        )
    } else if let Some(status) = app.launch.status.as_deref() {
        status.to_string()
    } else if area.width < 60 {
        format!(
            "j/k:{} · Enter:{}",
            tr(app.ui_locale, MessageId::LaunchHintMove),
            tr(app.ui_locale, MessageId::LaunchHintOpen)
        )
    } else {
        tr(app.ui_locale, MessageId::LaunchTipFlags).into_owned()
    };
    render_launch_line(
        area,
        buf,
        area.height.saturating_sub(2),
        vec![Span::styled(
            truncate_to_width(&prompt, width),
            Style::default().fg(if app.launch.status.is_some() {
                app.ui_theme.text_muted
            } else {
                app.ui_theme.text_hint
            }),
        )],
    );

    let workspace_kind = tr(
        app.ui_locale,
        if app.launch.worktree_available {
            MessageId::LaunchWorkspaceGitShort
        } else {
            MessageId::LaunchWorkspaceFolderShort
        },
    );
    let provider = tr(
        app.ui_locale,
        if app.onboarding_needs_api_key {
            MessageId::LaunchProviderSetupShort
        } else {
            MessageId::LaunchProviderConfiguredShort
        },
    );
    let status = format!(
        "{} · {workspace_kind} · {provider}",
        launch_workspace_name(app)
    );
    render_launch_line(
        area,
        buf,
        area.height.saturating_sub(1),
        vec![Span::styled(
            truncate_to_width(&status, width),
            Style::default().fg(app.ui_theme.text_dim),
        )],
    );
}

/// Record the launch row rects immediately after the launch frame is painted.
/// The coordinates mirror the renderer's responsive row placement exactly.
pub fn record_launch_row_areas(area: Rect, launch: &mut crate::tui::app::LaunchState) {
    launch.row_areas.clear();
    for index in 0..LAUNCH_ROWS.len() {
        let y = launch_row_y(area, index);
        if y >= area.height.saturating_sub(3) {
            break;
        }
        launch.row_areas.push(Rect {
            x: area.x.saturating_add(2),
            y: area.y.saturating_add(y),
            width: area.width.saturating_sub(4),
            height: 1,
        });
    }
}

fn compact_tokens(tokens: i64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.0}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn compact_effort_label(label: &str) -> &'static str {
    let effective = label
        .rsplit_once('→')
        .map_or(label, |(_, effective)| effective);
    let effective = effective
        .rsplit_once(':')
        .map_or(effective, |(_, effective)| effective)
        .trim()
        .to_ascii_lowercase();
    match effective.as_str() {
        "off" => "o",
        "low" => "l",
        "med" | "medium" => "m",
        "high" => "h",
        "max" | "maximum" | "xhigh" => "x",
        "auto" => "a",
        _ => "·",
    }
}

fn session_token_breakdown(app: &App) -> Option<Span<'static>> {
    app.header_items.contains(&HeaderItem::Tokens).then(|| {
        Span::styled(
            format!(
                "{} in · {} cch · {} out",
                format_token_count_compact(u64::from(app.session.total_input_tokens)),
                format_token_count_compact(u64::from(app.session.total_cache_hit_tokens)),
                format_token_count_compact(u64::from(app.session.total_output_tokens)),
            ),
            header_fg(app, ChromeInk::Info),
        )
    })
}

/// Append one right-hand chrome element, inserting the two-space separator
/// only between elements so an absent element never leaves trailing padding.
fn push_chrome(spans: &mut Vec<Span<'static>>, span: Span<'static>) {
    if !spans.is_empty() {
        spans.push(Span::raw("  "));
    }
    spans.push(span);
}

/// Render the one-line shell header. Route, mode, requested/effective effort,
/// permission, active-agent count, and context each have exactly one owner.
pub fn render_header(area: Rect, buf: &mut Buffer, app: &App) {
    let git_status = crate::tui::git_status::cached_status();
    render_header_with_git_status(area, buf, app, &git_status);
}

fn render_header_with_git_status(
    area: Rect,
    buf: &mut Buffer,
    app: &App,
    git_status: &crate::tui::git_status::GitStatusSnapshot,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let tier = ShellTier::for_chrome_width(area.width);
    Block::default()
        .style(Style::default().bg(app.ui_theme.header_bg))
        .render(area, buf);

    let (effective_provider, effective_model) = app.effective_route_identity_display();
    let route_label = format!("{effective_provider} · {effective_model}");
    let effort_label = app.reasoning_effort_display_label();
    let mode_color = header_mode_ink(app.mode).color(&app.ui_theme);
    // Match the composer's warm top edge exactly: Ask amber, Auto-Review
    // Signal Gold, and Full Access coral.
    let permission_color = header_permission_ink(app.approval_mode).color(&app.ui_theme);
    let dim = header_fg(app, ChromeInk::MetadataDim);
    // `status_indicator` owns the single header mark. It used to be filtered
    // against the literal "cw" because the header also hardcoded a leading
    // "cw" span, and `header_status_indicator_frame` collapses `cw`, the
    // legacy `whale` opt-in, and unknown values onto that same mark — so the
    // filter silently discarded three of the setting's four documented values
    // and left `off` with nothing to turn off (#5512). There is one mark now,
    // and this setting decides what occupies it.
    let status_indicator = crate::tui::widgets::header_status_indicator_frame(
        (!app.low_motion && app.fancy_animations)
            .then_some(app.turn_started_at)
            .flatten(),
        &app.status_indicator,
    );
    let mut left = Vec::new();
    if let Some(indicator) = status_indicator {
        left.push(Span::styled(
            indicator,
            header_fg(app, ChromeInk::Identity).add_modifier(Modifier::BOLD),
        ));
        left.push(Span::raw("  "));
    }
    left.extend([
        Span::styled(route_label.clone(), header_fg(app, ChromeInk::Metadata)),
        Span::styled(" · ", dim),
        Span::styled(
            mode_label(app.ui_locale, app.mode),
            Style::default().fg(mode_color),
        ),
        Span::styled(" · ", dim),
        Span::styled(effort_label.clone(), header_fg(app, ChromeInk::Info)),
    ]);
    // Permission is safety state, not optional chrome. Compact terminals shed
    // route detail and the context meter, but keep mode, effective effort, and
    // the effective posture.
    left.push(Span::styled(" · ", dim));
    left.push(Span::styled(
        permission_label(app),
        Style::default().fg(permission_color),
    ));
    // Active-goal chip (#39): the ocean shell has no sidebar, so the topbar
    // is the only always-on surface where a goal set via `create_goal` can
    // live. Objective truncated to a fixed budget; terminal goals render
    // nothing. The cramped-layout rebuild below keeps the chip in `suffix`.
    let goal_chip =
        crate::tui::footer_ui::active_goal_chip_state(app).map(|(objective, paused)| {
            let budget = if paused { 22 } else { 26 };
            let flat = objective.trim().replace(['\n', '\r'], " ");
            let text = if paused {
                format!("goal paused {}", truncate_to_width(&flat, budget))
            } else {
                format!("goal {}", truncate_to_width(&flat, budget))
            };
            let color = if paused {
                ChromeInk::Attention.color(&app.ui_theme)
            } else {
                ChromeInk::Active.color(&app.ui_theme)
            };
            (text, color)
        });
    if let Some((text, color)) = &goal_chip {
        left.push(Span::styled(" · ", dim));
        left.push(Span::styled(
            text.clone(),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
    }
    // Workflow-run chip (#5040): the same `WorkflowPanel::top_bar_chip` the
    // classic header shows, so a collapsed run stays visible on the ocean
    // shell too. No workflow panel means no chip. The cramped-layout rebuild
    // below keeps the chip in `suffix` alongside the goal chip.
    let workflow_chip = app
        .workflow_panel
        .as_ref()
        .map(|panel| (panel.top_bar_chip(), ChromeInk::Info.color(&app.ui_theme)));
    if let Some((text, color)) = &workflow_chip {
        left.push(Span::styled(" · ", dim));
        left.push(Span::styled(
            text.clone(),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
    }
    // Update-available chip (#14): a quiet, persistent affordance set once by
    // the startup version check. Gets the workflow chip's treatment: last in
    // the left cluster, the route label yields its budget first, and the chip
    // drops cleanly when even a minimal chip cannot fit — never a modal,
    // never mid-chip clipping.
    let update_chip = app
        .update_available
        .as_ref()
        .map(|label| (label.clone(), ChromeInk::Attention.color(&app.ui_theme)));
    if let Some((text, color)) = &update_chip {
        left.push(Span::styled(" · ", dim));
        left.push(Span::styled(
            text.clone(),
            Style::default().fg(*color).add_modifier(Modifier::BOLD),
        ));
    }

    let context_meter = (tier != ShellTier::Compact)
        .then(|| crate::tui::ui::context_usage_snapshot(app))
        .flatten()
        .map(|(used, max, percent)| {
            let filled = ((percent / 100.0) * 5.0).ceil().clamp(0.0, 5.0) as usize;
            Span::styled(
                format!(
                    "{}/{} [{}{}] {:.0}%",
                    compact_tokens(used),
                    compact_tokens(i64::from(max)),
                    "▰".repeat(filled),
                    "▱".repeat(5usize.saturating_sub(filled)),
                    percent
                ),
                header_fg(app, ChromeInk::Info),
            )
        });
    let token_breakdown = (tier != ShellTier::Compact)
        .then(|| session_token_breakdown(app))
        .flatten();
    let token_breakdown_requested = token_breakdown.is_some();
    let version = (tier == ShellTier::Wide).then(|| {
        Span::styled(
            format!("v{}", shell_build_version()),
            header_fg(app, ChromeInk::MetadataHint),
        )
    });
    // Cached repository/worktree status only — never probe from the render path.
    // Background refresh is scheduled from the event loop / idle ticks.
    let git_label = crate::tui::git_status::chrome_label(git_status).map(|label| {
        let max_width = match tier {
            ShellTier::Compact => 24,
            ShellTier::Normal => 36,
            ShellTier::Wide => 52,
        };
        Span::styled(
            truncate_to_width(&label, max_width),
            header_fg(app, crate::tui::git_status::chrome_ink()),
        )
    });

    // Baseline right-hand chrome: git, context meter, version. Exact route
    // identity outranks this auxiliary chrome when the full line cannot fit.
    let mut right = Vec::new();
    if let Some(git_label) = git_label.clone() {
        push_chrome(&mut right, git_label);
    }
    if let Some(context_meter) = context_meter.clone() {
        push_chrome(&mut right, context_meter);
    }
    if let Some(version) = version.clone() {
        push_chrome(&mut right, version);
    }

    let minimum_effort = if tier == ShellTier::Compact {
        compact_effort_label(&effort_label).to_string()
    } else {
        effort_label.clone()
    };
    // The mark leads the header and carries its own two-space gutter, so it
    // costs `width + 2` when present and nothing at all when `off` (#5512).
    let indicator_width = status_indicator.map_or(0, |indicator| indicator.width() + 2);
    let minimum_left_width = indicator_width
        .saturating_add(3 + mode_label(app.ui_locale, app.mode).width())
        .saturating_add(3 + minimum_effort.width())
        .saturating_add(3 + permission_label(app).width());
    let available = usize::from(area.width);
    // The optional token breakdown is the only elidable element: it is added
    // between the git label and the context meter when the terminal is wide
    // enough to keep the whole baseline plus the guaranteed-left minimum.
    if let Some(token_breakdown) = token_breakdown {
        let mut enhanced_right = Vec::new();
        if let Some(git_label) = git_label.clone() {
            push_chrome(&mut enhanced_right, git_label);
        }
        push_chrome(&mut enhanced_right, token_breakdown);
        if let Some(context_meter) = context_meter.clone() {
            push_chrome(&mut enhanced_right, context_meter);
        }
        if let Some(version) = version.clone() {
            push_chrome(&mut enhanced_right, version);
        }
        let enhanced_width = span_width(&enhanced_right);
        let gap = usize::from(enhanced_width > 0);
        if minimum_left_width
            .saturating_add(gap)
            .saturating_add(enhanced_width)
            <= available
        {
            right = enhanced_right;
        }
    }

    // Provider + model are routing truth. Shed auxiliary right-hand chrome in
    // least-important-first order before shortening that identity on a normal
    // 100+ column shell. Narrow shells keep the context meter, and an explicit
    // token-breakdown opt-in keeps its documented width priority.
    let full_left_width = span_width(&left);
    let route_identity_priority = available >= 100
        && !token_breakdown_requested
        && app.api_provider == crate::config::ApiProvider::Custom;
    if route_identity_priority
        && full_left_width
            .saturating_add(usize::from(!right.is_empty()))
            .saturating_add(span_width(&right))
            > available
    {
        right.clear();
        if let Some(context_meter) = context_meter.clone() {
            push_chrome(&mut right, context_meter);
        }
        if let Some(version) = version.clone() {
            push_chrome(&mut right, version);
        }
    }
    if route_identity_priority
        && full_left_width
            .saturating_add(usize::from(!right.is_empty()))
            .saturating_add(span_width(&right))
            > available
    {
        right.clear();
        if let Some(context_meter) = context_meter {
            push_chrome(&mut right, context_meter);
        }
    }
    if route_identity_priority
        && full_left_width
            .saturating_add(usize::from(!right.is_empty()))
            .saturating_add(span_width(&right))
            > available
    {
        right.clear();
    }

    let right_width = span_width(&right);
    let left_budget = available.saturating_sub(right_width + usize::from(right_width > 0));
    if span_width(&left) > left_budget {
        let mode = mode_label(app.ui_locale, app.mode);
        let permission = permission_label(app);
        let effort = if tier == ShellTier::Compact {
            compact_effort_label(&effort_label).to_string()
        } else {
            effort_label.clone()
        };
        let mut suffix = vec![
            Span::styled(" · ", dim),
            Span::styled(mode, Style::default().fg(mode_color)),
            Span::styled(" · ", dim),
            Span::styled(effort, header_fg(app, ChromeInk::Info)),
            Span::styled(" · ", dim),
            Span::styled(permission, Style::default().fg(permission_color)),
        ];
        // The goal chip survives cramped layouts too — it is operator state,
        // not decoration. The route label yields its budget first (down to
        // nothing, as it always has); below that the goal itself truncates,
        // and when even a minimal chip cannot fit it drops rather than
        // clipping mid-word (#39).
        // Same accounting as the baseline pass: the mark leads and owns its
        // gutter, so it is `width + 2` present and 0 when `off` (#5512).
        let indicator_width = status_indicator.map_or(0, |indicator| indicator.width() + 2);
        let base_fixed = indicator_width.saturating_add(span_width(&suffix));
        if let Some((text, color)) = &goal_chip {
            let goal_room = left_budget.saturating_sub(base_fixed).saturating_sub(3);
            if goal_room >= 8 {
                suffix.push(Span::styled(" · ", dim));
                suffix.push(Span::styled(
                    truncate_to_width(text, goal_room),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ));
            }
        }
        // The workflow chip (#5040) is operator state too, so it gets the
        // goal chip's treatment: whatever room remains after the chips ahead
        // of it, clean truncation, and a clean drop when even a minimal chip
        // cannot fit. The route label still yields its budget first.
        if let Some((text, color)) = &workflow_chip {
            let workflow_room = left_budget
                .saturating_sub(indicator_width.saturating_add(span_width(&suffix)))
                .saturating_sub(3);
            if workflow_room >= 8 {
                suffix.push(Span::styled(" · ", dim));
                suffix.push(Span::styled(
                    truncate_to_width(text, workflow_room),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ));
            }
        }
        // The update chip (#14) gets the same treatment, last in line: it is
        // useful, but it yields to every piece of operator state ahead of it.
        if let Some((text, color)) = &update_chip {
            let update_room = left_budget
                .saturating_sub(indicator_width.saturating_add(span_width(&suffix)))
                .saturating_sub(3);
            if update_room >= 8 {
                suffix.push(Span::styled(" · ", dim));
                suffix.push(Span::styled(
                    truncate_to_width(text, update_room),
                    Style::default().fg(*color).add_modifier(Modifier::BOLD),
                ));
            }
        }
        let fixed_width = indicator_width.saturating_add(span_width(&suffix));
        let route_budget = left_budget.saturating_sub(fixed_width);
        left = Vec::new();
        if let Some(indicator) = status_indicator {
            left.push(Span::styled(
                indicator,
                header_fg(app, ChromeInk::Identity).add_modifier(Modifier::BOLD),
            ));
            left.push(Span::raw("  "));
        }
        left.push(Span::styled(
            truncate_to_width(&route_label, route_budget),
            header_fg(app, ChromeInk::Metadata),
        ));
        left.extend(suffix);
    }
    let left_width = span_width(&left);
    let gap = available.saturating_sub(left_width + right_width);
    left.push(Span::raw(" ".repeat(gap)));
    left.extend(right);
    let title_area = Rect { height: 1, ..area };
    Paragraph::new(Line::from(left)).render(title_area, buf);
    if area.height > 1 {
        let rule_area = Rect {
            y: area.y.saturating_add(1),
            height: 1,
            ..area
        };
        Paragraph::new(Line::from(Span::styled(
            "─".repeat(usize::from(area.width)),
            Style::default().fg(app.ui_theme.border),
        )))
        .render(rule_area, buf);
    }
}

/// Render the fixed one-line phase band.
///
/// Ocean placement (above vs below the composer) is owned by
/// [`crate::tui::phase_strip`]; this entry point only paints the band so
/// classic callers and tests keep a stable name.
pub fn render_footer(area: Rect, buf: &mut Buffer, app: &mut App) {
    crate::tui::phase_strip::render(area, buf, app);
}

/// The transcript rows the idle brand mark needs before it will draw at all.
///
/// This is [`ShellTier::for_area`]'s `Compact` floor, named so the *layout*
/// can honour it before the frame is split. Anything that reserves rows above
/// the transcript must subtract against this constant rather than guess, or
/// the reservation and the render gate drift and the mark is evicted by
/// chrome that was sized without knowing the mark existed.
pub(crate) const AMBIENT_MIN_CHAT_HEIGHT: u16 = 16;
/// Companion column floor, same reasoning as [`AMBIENT_MIN_CHAT_HEIGHT`].
pub(crate) const AMBIENT_MIN_CHAT_WIDTH: u16 = 60;

/// Build the post-launch idle composition: brand, workspace context, Fleet,
/// help, and the orchestration trio (`/workflow /goal /auto`).
///
/// Expressed in terms of the ambient floor constants so the layout rule that
/// reserves the rows and the gate that spends them cannot disagree. (The old
/// spelling also tested `height >= 14 && width >= 28`, which was dead: the
/// tier check already demands 16 rows and 60 columns.)
#[must_use]
pub(crate) fn empty_state_mark_visible(area: Rect) -> bool {
    area.height >= AMBIENT_MIN_CHAT_HEIGHT && area.width >= AMBIENT_MIN_CHAT_WIDTH
}

#[must_use]
pub(crate) fn decorative_shell_motion_enabled(app: &App) -> bool {
    app.motion_policy().allows_decorative()
        && !app.attention_hold_active()
        && app.onboarding == OnboardingState::None
        && !app.launch.visible
        && app.view_stack.is_empty()
}

#[must_use]
fn idle_mark_animation_enabled(app: &App) -> bool {
    decorative_shell_motion_enabled(app) && matches!(ShellPhase::from_app(app), ShellPhase::Idle)
}

/// Raised-cosine caustic band for the idle whale. The 4s cycle spends roughly
/// 1.3s crossing the mark and parks off-screen for the remainder, so the brand
/// has a clear moment of life without becoming looping chrome.
#[must_use]
fn idle_mark_shine_opacity(diagonal: f32, elapsed_ms: u128) -> f32 {
    let cycle_progress = (elapsed_ms % IDLE_SHIMMER_CYCLE_MS) as f32 / IDLE_SHIMMER_CYCLE_MS as f32;
    let sweep_progress = (cycle_progress / IDLE_SHIMMER_SWEEP_FRACTION).min(1.0);
    let band_position =
        -IDLE_SHIMMER_BAND_HALF_WIDTH + sweep_progress * (1.0 + 2.0 * IDLE_SHIMMER_BAND_HALF_WIDTH);
    let distance = (diagonal - band_position).abs();
    if distance >= IDLE_SHIMMER_BAND_HALF_WIDTH {
        return 0.0;
    }
    let raised_cosine =
        0.5 * (1.0 + (std::f32::consts::PI * distance / IDLE_SHIMMER_BAND_HALF_WIDTH).cos());
    IDLE_SHIMMER_STRENGTH * raised_cosine
}

#[must_use]
fn idle_mark_color(base: Color, highlight: Color, opacity: f32) -> Color {
    if opacity <= 0.0 {
        return base;
    }
    match (base, highlight) {
        (Color::Rgb(..), Color::Rgb(..)) => crate::palette::blend(highlight, base, opacity),
        // Named/terminal-owned colors cannot be blended truthfully. Hold the
        // stable brand color instead of flashing the entire mark at full ink.
        _ => base,
    }
}

fn idle_whale_is_uwu(app: &App) -> bool {
    app.ui_theme.name == "uwu"
}

fn idle_whale_spout_row(app: &App) -> &'static str {
    if idle_whale_is_uwu(app) {
        UWU_IDLE_WHALE_SPOUT_ROW
    } else {
        IDLE_WHALE_SPOUT_ROW
    }
}

fn idle_whale_rows(app: &App) -> [&'static str; 3] {
    if idle_whale_is_uwu(app) {
        UWU_IDLE_WHALE_ROWS
    } else {
        IDLE_WHALE_ROWS
    }
}

/// Signal Current cyan owns the spout and the belly cut. It resolves through
/// the same Whale Teams ink the `/fleet` portraits use, so every theme gets
/// the brand cyan lifted to the secondary-chrome contrast floor rather than a
/// per-theme guess.
fn idle_whale_current_color(app: &App) -> Color {
    crate::tui::whales::WhaleInk::from_theme(&app.ui_theme).current
}

fn idle_whale_row_spans(
    text: &'static str,
    row: usize,
    elapsed_ms: u128,
    animated: bool,
    base: Color,
    highlight: Color,
    eye: Color,
) -> Vec<Span<'static>> {
    let rows = IDLE_WHALE_ROWS.len() as f32;
    let cols = IDLE_WHALE_ROWS
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1) as f32;
    let mut spans = Vec::new();
    let mut run = String::new();
    let mut run_color = None;

    for (column, ch) in text.chars().enumerate() {
        let diagonal = (column as f32 + (rows - 1.0 - row as f32)) / (cols + rows);
        let color = if matches!(ch, '·' | '░' | '✦' | '△') {
            // Soft uwu blush/sparkle and the quiet crown-fluke center use the
            // eye/sakura channel; classic otherwise only has the eye dot.
            eye
        } else if animated {
            idle_mark_color(
                base,
                highlight,
                idle_mark_shine_opacity(diagonal, elapsed_ms),
            )
        } else {
            base
        };
        if run_color != Some(color) {
            if let Some(previous) = run_color {
                spans.push(Span::styled(
                    std::mem::take(&mut run),
                    Style::default().fg(previous),
                ));
            }
            run_color = Some(color);
        }
        run.push(ch);
    }
    if let Some(previous) = run_color {
        spans.push(Span::styled(run, Style::default().fg(previous)));
    }
    spans
}

#[must_use]
fn idle_whale_block_width(spout: &str, rows: &[&str]) -> usize {
    std::iter::once(spout)
        .chain(rows.iter().copied())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

pub fn empty_state_lines(app: &App, area: Rect) -> Vec<Line<'static>> {
    if area.width == 0 || area.height == 0 {
        return Vec::new();
    }
    let width = usize::from(area.width);
    let tier = ShellTier::for_area(area);
    let mut lines = vec![Line::from(""); usize::from(area.height / 4)];
    if empty_state_mark_visible(area) {
        let animated = idle_mark_animation_enabled(app);
        let elapsed_ms = app.ocean_started_at.elapsed().as_millis();
        let spout = idle_whale_spout_row(app);
        let rows = idle_whale_rows(app);
        let current = idle_whale_current_color(app);
        let mut mark = vec![vec![Span::styled(spout, Style::default().fg(current))]];
        // Soft uwu: sakura blush/sparkle glyphs; classic keeps body peach + text eye.
        let highlight = if idle_whale_is_uwu(app) {
            app.ui_theme.accent_primary
        } else {
            app.ui_theme.text_body
        };
        mark.extend(rows.iter().enumerate().map(|(row, text)| {
            // The belly cut is water, not chrome: it holds the flat brand cyan
            // while the caustic sweep travels across the gold body above it.
            let is_current = row == IDLE_WHALE_CURRENT_ROW;
            idle_whale_row_spans(
                text,
                row,
                elapsed_ms,
                animated && !is_current,
                if is_current {
                    current
                } else {
                    app.ui_theme.accent_action
                },
                app.ui_theme.text_body,
                highlight,
            )
        }));
        // The spout, head, belly, peduncle, and flukes are one drawing. Give
        // every row the same outer inset so the authored offsets survive;
        // centering each row independently shears the silhouette apart.
        let block_inset =
            " ".repeat(width.saturating_sub(idle_whale_block_width(spout, &rows)) / 2);
        for row in mark {
            let mut spans = vec![Span::raw(block_inset.clone())];
            spans.extend(row);
            lines.push(Line::from(spans));
        }
        lines.push(Line::from(""));
    }

    let identity = crate::tui::workspace_context::identity_from_context(
        &app.workspace,
        app.workspace_context.as_deref(),
    );
    let workspace = crate::utils::display_path(&app.workspace);
    let branch = identity.branch.as_deref().map_or_else(
        || tr(app.ui_locale, MessageId::EmptyStateNoGit),
        |branch| Cow::Owned(branch.to_string()),
    );
    let context = if tier == ShellTier::Compact {
        branch.into_owned()
    } else {
        format!(
            "{workspace} · {branch} · {} {}",
            tr(app.ui_locale, MessageId::EmptyStateMcpLabel),
            app.mcp_configured_count
        )
    };
    let brand = "Codewhale";
    let brand_inset = " ".repeat(width.saturating_sub(brand.width()) / 2);
    lines.push(Line::from(Span::styled(
        format!("{brand_inset}{brand}"),
        Style::default()
            .fg(app.ui_theme.text_body)
            .add_modifier(Modifier::BOLD),
    )));
    let context = truncate_to_width(&context, width);
    let inset = " ".repeat(width.saturating_sub(context.width()) / 2);
    lines.push(Line::from(Span::styled(
        format!("{inset}{context}"),
        Style::default().fg(app.ui_theme.text_soft),
    )));
    if area.height >= 6 {
        lines.push(Line::from(""));
        let (fleet_label, fleet_action) = if app.onboarding_needs_api_key {
            // `--skip-onboarding` can expose the launch shell without a usable
            // provider route. Do not claim that Fleet is ready in that state;
            // point at the boundary that can actually make it runnable.
            (
                tr(app.ui_locale, MessageId::EmptyStateFleetLabel),
                "/provider",
            )
        } else {
            // Built-in roles are immediately usable with the active route.
            // Keep this truth at every responsive tier so `/fleet setup`
            // reads as optional customization instead of required setup.
            (
                tr(app.ui_locale, MessageId::EmptyStateFleetSetupLabel),
                "/fleet setup",
            )
        };
        let fleet = format!("{fleet_label}  {fleet_action}");
        let inset = " ".repeat(width.saturating_sub(fleet.width()) / 2);
        lines.push(Line::from(vec![
            Span::raw(inset),
            Span::styled(
                fleet_label.into_owned(),
                Style::default().fg(app.ui_theme.text_soft),
            ),
            Span::raw("  "),
            Span::styled(
                fleet_action,
                Style::default()
                    .fg(app.ui_theme.accent_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        if area.height >= 7 {
            let help_connector = tr(app.ui_locale, MessageId::EmptyStateHelpConnector);
            let help_command = format!("/help {help_connector} Ctrl+K");
            let help_hint = tr(app.ui_locale, MessageId::EmptyStateHelpHint);
            let help = format!("{help_command} {help_hint}");
            let inset = " ".repeat(width.saturating_sub(help.width()) / 2);
            lines.push(Line::from(vec![
                Span::raw(inset),
                Span::styled(
                    help_command,
                    Style::default()
                        .fg(app.ui_theme.accent_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::styled(
                    help_hint.into_owned(),
                    Style::default().fg(app.ui_theme.text_soft),
                ),
            ]));
        }
        if area.height >= 8 {
            let orch_label = tr(app.ui_locale, MessageId::EmptyStateOrchestrationLabel);
            let orch_commands = crate::commands::traits::orchestration_slash_hint();
            let orch = format!("{orch_label}  {orch_commands}");
            let inset = " ".repeat(width.saturating_sub(orch.width()) / 2);
            lines.push(Line::from(vec![
                Span::raw(inset),
                Span::styled(
                    orch_label.into_owned(),
                    Style::default().fg(app.ui_theme.text_soft),
                ),
                Span::raw("  "),
                Span::styled(
                    orch_commands,
                    Style::default()
                        .fg(app.ui_theme.accent_primary)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        tui::app::{LaunchState, TuiOptions},
    };
    use std::{
        cell::RefCell,
        path::PathBuf,
        time::{Duration, Instant},
    };

    thread_local! {
        static BUILD_VERSION_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
    }

    /// Read the header's version-string override (see [`shell_build_version`]).
    pub(super) fn build_version_override() -> Option<String> {
        BUILD_VERSION_OVERRIDE.with(|cell| cell.borrow().clone())
    }

    /// Pin the header's version stamp for the current test thread so width
    /// choreography is measured against a fixed length, not the ambient
    /// build's sha (which is `(dev)` locally and a sha on CI since #5245).
    /// The default fixture mirrors a sha-stamped build's width.
    struct BuildVersionGuard;

    impl BuildVersionGuard {
        fn set(version: &str) -> Self {
            BUILD_VERSION_OVERRIDE.with(|cell| *cell.borrow_mut() = Some(version.to_string()));
            Self
        }
    }

    impl Drop for BuildVersionGuard {
        fn drop(&mut self) {
            BUILD_VERSION_OVERRIDE.with(|cell| *cell.borrow_mut() = None);
        }
    }

    /// An enforced-sandbox marker for this platform, or `None` where the
    /// enum has no enforced variant. Only identity matters here: the header
    /// reads `sandbox_backend.is_some()`, never which backend it is.
    fn enforced_backend() -> Option<crate::sandbox::SandboxType> {
        #[cfg(target_os = "macos")]
        {
            Some(crate::sandbox::SandboxType::MacosSeatbelt)
        }
        #[cfg(all(target_os = "linux", not(target_env = "ohos")))]
        {
            Some(crate::sandbox::SandboxType::LinuxBubblewrap)
        }
        #[cfg(target_os = "windows")]
        {
            Some(crate::sandbox::SandboxType::Windows)
        }
        #[cfg(not(any(
            target_os = "macos",
            target_os = "windows",
            all(target_os = "linux", not(target_env = "ohos"))
        )))]
        {
            None
        }
    }

    fn test_app() -> App {
        let mut app = App::new(
            TuiOptions {
                model: "deepseek-v4-flash".to_string(),
                start_in_agent_mode: true,
                ..crate::test_support::test_tui_options(PathBuf::from("."))
            },
            &Config::default(),
        );
        // `filesystem_scope_label` is deliberately honest about enforcement:
        // with no OS sandbox backend it appends " (unenforced)" (all Windows,
        // and Linux where bubblewrap is absent — it is opt-in). That is 12
        // extra columns in the permission chip, which both changes the exact
        // chip text and eats the width budget the cramped-layout assertions
        // below are calibrated against. Header rendering is not a probe of
        // the host's sandbox availability, so pin the backend and keep these
        // tests platform-stable; `permission_chip_says_unenforced_without_a_
        // backend` covers the `None` rendering explicitly.
        app.sandbox_backend = enforced_backend();
        app
    }

    fn launch() -> LaunchState {
        LaunchState {
            visible: true,
            selected: 0,
            worktree_input: None,
            status: None,
            workspace_session_count: 2,
            worktree_available: true,
            row_areas: Vec::new(),
        }
    }

    #[test]
    fn window_title_prefix_prefers_session_over_config_default() {
        let mut app = test_app();
        // Nothing configured: no prefix at all.
        assert_eq!(app.window_title_prefix(), None);

        // Config default alone.
        app.title_default = Some("workspace-x".to_string());
        assert_eq!(app.window_title_prefix(), Some("workspace-x"));

        // Session-level `/title` wins over the config default.
        app.window_title = Some("task-7".to_string());
        assert_eq!(app.window_title_prefix(), Some("task-7"));

        // Clearing the session title falls back to the default.
        app.window_title = None;
        assert_eq!(app.window_title_prefix(), Some("workspace-x"));

        // Whitespace-only titles count as unset.
        app.window_title = Some("   ".to_string());
        assert_eq!(app.window_title_prefix(), None);
        app.window_title = None;
    }

    #[test]
    fn sync_title_activity_pushes_the_resolved_prefix() {
        // The render-loop sync must reach the notifications layer so the
        // terminal title actually carries the prefix.
        let _guard = crate::tui::notifications::title_prefix_test_lock();
        crate::tui::notifications::set_title_prefix(None);
        let mut app = test_app();
        app.window_title = Some("sync-check".to_string());
        sync_title_activity(&app);
        assert_eq!(
            crate::tui::notifications::title_prefix_slot()
                .lock()
                .unwrap()
                .as_str(),
            "sync-check"
        );
        app.window_title = None;
        sync_title_activity(&app);
        assert_eq!(
            crate::tui::notifications::title_prefix_slot()
                .lock()
                .unwrap()
                .as_str(),
            ""
        );
    }

    #[test]
    fn launch_row_hitboxes_follow_responsive_render_rows() {
        let mut launch = launch();
        record_launch_row_areas(Rect::new(3, 2, 80, 24), &mut launch);
        assert_eq!(launch.row_areas.len(), 6);
        assert_eq!(launch.row_areas[0], Rect::new(5, 9, 76, 1));
        assert_eq!(launch.row_areas[5], Rect::new(5, 21, 76, 1));

        record_launch_row_areas(Rect::new(3, 2, 40, 10), &mut launch);
        assert_eq!(launch.row_areas.len(), 4);
        assert_eq!(launch.row_areas[0], Rect::new(5, 5, 36, 1));
    }

    fn launch_render(app: &App, width: u16, height: u16) -> (Buffer, String) {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        render_launch_screen(area, &mut buf, app);
        let text = (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n");
        (buf, text)
    }

    #[test]
    fn compact_launch_keeps_every_choice_and_readiness_fact_visible() {
        let mut app = test_app();
        app.workspace = PathBuf::from("/tmp/codewhale");
        app.launch = launch();
        app.onboarding_needs_api_key = false;

        let (buf, text) = launch_render(&app, 40, 12);

        for expected in [
            "Work · current folder",
            "Chat · read-only",
            "Resume session · 2 saved",
            "New worktree",
            "Changelog",
            "Quit",
            "Enter",
            "Ctrl+R",
            "Ctrl+N",
            "Ctrl+L",
            "Ctrl+Q",
            "codewhale · Git · provider set",
        ] {
            assert!(text.contains(expected), "missing {expected:?}:\n{text}");
        }
        assert!(
            text.contains('▸'),
            "selection needs a non-color cue:\n{text}"
        );
        assert_eq!(buf[(2, 3)].bg, app.ui_theme.selection_bg);
        assert_eq!(
            buf[(4, 3)].bg,
            app.ui_theme.selection_bg,
            "the selected label must sit inside its selection band"
        );
        assert!(
            !text.contains("deepseek-v4-flash"),
            "launch readiness must not become model marketing:\n{text}"
        );
    }

    #[test]
    fn normal_and_wide_launch_add_decision_context() {
        let mut app = test_app();
        app.workspace = PathBuf::from("/tmp/codewhale");
        app.launch = launch();
        app.onboarding_needs_api_key = false;

        for (width, height) in [(80, 24), (120, 36)] {
            let (buf, text) = launch_render(&app, width, height);
            for expected in [
                "Start here",
                "Workspace · codewhale · Git workspace",
                "Provider · configured",
                "Use this folder with local tools; changes follow your approval policy.",
                "Conversation and planning only; no file changes.",
                "Continue",
                "More",
            ] {
                assert!(
                    text.contains(expected),
                    "{width}x{height} missing {expected:?}:\n{text}"
                );
            }
            assert!(
                !text.contains("deepseek-v4-flash"),
                "{width}x{height}:\n{text}"
            );
            assert_eq!(
                buf[(4, 7)].bg,
                app.ui_theme.selection_bg,
                "{width}x{height} selected label escaped its row"
            );
        }
    }

    #[test]
    fn launch_provider_copy_reports_setup_without_route_or_secret_detail() {
        let mut app = test_app();
        app.workspace = PathBuf::from("/tmp/codewhale");
        app.launch = launch();
        app.onboarding_needs_api_key = true;

        let (_, compact) = launch_render(&app, 40, 12);
        assert!(compact.contains("provider setup"), "{compact}");
        let (_, normal) = launch_render(&app, 80, 24);
        assert!(normal.contains("Provider · setup needed"), "{normal}");
        assert!(!normal.contains("deepseek-v4-flash"), "{normal}");
    }

    #[test]
    fn all_complete_locales_keep_six_compact_launch_targets() {
        let mut app = test_app();
        app.workspace = PathBuf::from("/tmp/codewhale");
        app.launch = launch();
        for locale in Locale::shipped_complete() {
            app.ui_locale = *locale;
            let (_, text) = launch_render(&app, 40, 12);
            for key in ["Enter", "C", "Ctrl+R", "Ctrl+N", "Ctrl+L", "Ctrl+Q"] {
                assert!(
                    text.contains(key),
                    "{} compact launch lost {key}:\n{text}",
                    locale.tag()
                );
            }
        }
    }

    fn footer_text(app: &mut App) -> String {
        let area = Rect::new(0, 0, 100, 1);
        let mut buf = Buffer::empty(area);
        render_footer(area, &mut buf, app);
        (0..area.width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    /// #5512: the header status indicator never rendered for three of its
    /// four documented values. `header_status_indicator_frame` collapses
    /// `cw`, the legacy `whale` opt-in, and unknown values onto the `cw`
    /// mark, and the header then filtered that exact string out because it
    /// also hardcoded a leading `cw` span — so `cw`, `whale`, `off`, and a
    /// typo all produced byte-identical headers and `off` had nothing to
    /// turn off. There is one mark now and the setting owns it.
    #[test]
    fn status_indicator_setting_changes_the_header_mark() {
        let render = |value: &str| {
            let mut app = test_app();
            app.status_indicator = value.to_string();
            header_text(&app, 120)
        };

        let cw = render("cw");
        let whale = render("whale");
        let dots = render("dots");
        let off = render("off");
        let unknown = render("not-a-real-value");

        assert!(cw.starts_with("cw  "), "cw must lead with the mark: {cw:?}");
        assert_eq!(whale, cw, "legacy whale opt-in normalizes onto the cw mark");
        assert_eq!(unknown, cw, "unknown values fall back to the cw mark");

        assert!(
            !off.starts_with("cw"),
            "`off` must actually remove the mark: {off:?}"
        );
        assert_ne!(off, cw, "`off` must differ from `cw` (#5512)");

        assert_ne!(dots, cw, "`dots` must differ from `cw` (#5512)");
        assert!(
            !dots.starts_with("cw"),
            "`dots` replaces the mark rather than sitting beside it: {dots:?}"
        );

        // Every documented value still renders a single-line header that
        // keeps the operator state the layout guarantees.
        for (value, rendered) in [
            ("cw", &cw),
            ("whale", &whale),
            ("dots", &dots),
            ("off", &off),
        ] {
            assert!(
                !rendered.contains('\n'),
                "{value} header must stay one line"
            );
            let lowered = rendered.to_ascii_lowercase();
            assert!(
                lowered.contains("work"),
                "{value} lost the mode: {rendered:?}"
            );
            assert!(
                lowered.contains("ask"),
                "{value} lost the posture: {rendered:?}"
            );
        }
    }

    /// The mark must not be duplicated: the header carries exactly one.
    #[test]
    fn header_carries_exactly_one_status_mark() {
        let mut app = test_app();
        app.status_indicator = "cw".to_string();
        let rendered = header_text(&app, 120);
        let trimmed = rendered.trim_end();
        assert_eq!(
            trimmed.matches("cw").count(),
            1,
            "exactly one cw mark belongs in the header: {trimmed:?}"
        );
    }

    /// The compact-layout rebuild has its own span and width accounting, so
    /// #5512 stays fixed only if the selected mark survives that path too.
    /// Exercise every representative layout width from the TUI contract,
    /// including the 40-column floor and both normal/wide transitions.
    #[test]
    fn status_indicator_owns_one_leading_slot_at_every_supported_width() {
        for width in [40, 60, 80, 100, 140] {
            let render = |value: &str| {
                let mut app = test_app();
                app.status_indicator = value.to_string();
                header_text(&app, width)
            };

            let cw = render("cw");
            let whale = render("whale");
            let dots = render("dots");
            let off = render("off");

            assert!(
                cw.starts_with("cw  "),
                "{width}-column header lost the cw mark: {cw:?}"
            );
            assert_eq!(
                cw.matches("cw").count(),
                1,
                "{width}-column header duplicated the cw mark: {cw:?}"
            );
            assert_eq!(
                whale, cw,
                "{width}-column legacy whale setting must use the cw mark"
            );
            assert!(
                dots.starts_with("◍  "),
                "{width}-column header lost the idle dots mark: {dots:?}"
            );
            assert_eq!(
                dots.matches('◍').count(),
                1,
                "{width}-column header duplicated the dots mark: {dots:?}"
            );
            assert!(
                !off.starts_with("cw  ") && !off.starts_with("◍  "),
                "{width}-column off setting retained a status mark: {off:?}"
            );
            assert_ne!(
                off, cw,
                "{width}-column off setting must reclaim the mark slot"
            );
        }
    }

    fn header_text(app: &App, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        render_header(area, &mut buf, app);
        (0..width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    fn header_text_with_git_status(
        app: &App,
        width: u16,
        git_status: &crate::tui::git_status::GitStatusSnapshot,
    ) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        render_header_with_git_status(area, &mut buf, app, git_status);
        (0..width).map(|x| buf[(x, 0)].symbol()).collect()
    }

    #[test]
    fn header_surfaces_repository_and_worktree_without_wrapping() {
        let app = test_app();
        let git_status = crate::tui::git_status::GitStatusSnapshot {
            root: Some("/repo/.cw-worktrees/feature".into()),
            repository_name: Some("repo".into()),
            branch: Some("feature".into()),
            dirty: true,
            ..Default::default()
        };

        let wide = header_text_with_git_status(&app, 130, &git_status);
        assert!(
            wide.contains("repo/feature · feature*"),
            "wide header: {wide:?}"
        );

        let narrow = header_text_with_git_status(&app, 60, &git_status);
        assert!(!narrow.contains('\n'), "narrow header must stay one line");
        assert!(
            narrow.to_ascii_lowercase().contains("work"),
            "mode: {narrow:?}"
        );
        assert!(
            narrow.to_ascii_lowercase().contains("ask"),
            "permission: {narrow:?}"
        );
    }

    #[test]
    fn header_git_chrome_uses_metadata_ink_not_failure() {
        let app = test_app();
        let git_status = crate::tui::git_status::GitStatusSnapshot {
            root: Some("/repo/.cw-worktrees/feature".into()),
            repository_name: Some("repo".into()),
            branch: Some("feature".into()),
            dirty: true,
            ..Default::default()
        };
        let area = Rect::new(0, 0, 130, 1);
        let mut buf = Buffer::empty(area);
        render_header_with_git_status(area, &mut buf, &app, &git_status);
        let rendered = (0..area.width)
            .map(|x| buf[(x, 0)].symbol())
            .collect::<String>();
        let label_byte = rendered
            .find("repo/feature")
            .expect("repo/worktree label should render");
        let label_x = rendered[..label_byte].width() as u16;
        assert_eq!(
            buf[(label_x, 0)].fg,
            crate::palette::ChromeInk::Metadata.color(&app.ui_theme)
        );
        assert_ne!(
            buf[(label_x, 0)].fg,
            crate::palette::ChromeInk::Failure.color(&app.ui_theme)
        );
    }

    #[test]
    fn header_keeps_known_worktree_when_branch_is_unknown() {
        let app = test_app();
        let git_status = crate::tui::git_status::GitStatusSnapshot {
            root: Some("/repo/.cw-worktrees/feature".into()),
            repository_name: Some("repo".into()),
            branch: None,
            dirty: true,
            ..Default::default()
        };

        let wide = header_text_with_git_status(&app, 130, &git_status);
        assert!(wide.contains("repo/feature*"), "wide header: {wide:?}");
        assert!(
            !wide.contains("repo/feature ·"),
            "unknown branch must not gain an invented ref: {wide:?}"
        );

        let narrow = header_text_with_git_status(&app, 60, &git_status);
        assert!(!narrow.contains('\n'), "narrow header must stay one line");
        assert!(narrow.to_ascii_lowercase().contains("work"), "{narrow:?}");
        assert!(narrow.to_ascii_lowercase().contains("ask"), "{narrow:?}");
    }

    /// The real YOLO posture is mode + bypassed approvals, so pin both spans
    /// it paints. Neither the selected mode nor the Full Access chip may
    /// borrow Failure red: mode is Policy, permission is Cognition.
    #[test]
    fn header_yolo_mode_does_not_spend_failure_red() {
        let mut app = test_app();
        app.mode = AppMode::Yolo;
        app.approval_mode = ApprovalMode::Bypass;
        let area = Rect::new(0, 0, 120, 1);
        let mut buf = Buffer::empty(area);
        // Render against an explicit empty snapshot so the repo segment
        // cannot drift the spans this test locates.
        render_header_with_git_status(
            area,
            &mut buf,
            &app,
            &crate::tui::git_status::GitStatusSnapshot::default(),
        );
        let rendered = (0..area.width)
            .map(|x| buf[(x, 0)].symbol())
            .collect::<String>();
        let fg_at = |needle: &str| {
            let byte = rendered
                .rfind(needle)
                .unwrap_or_else(|| panic!("{needle:?} should render: {rendered:?}"));
            buf[(rendered[..byte].width() as u16, 0)].fg
        };

        let red = crate::palette::ChromeInk::Failure.color(&app.ui_theme);
        let mode_fg = fg_at(mode_label(app.ui_locale, app.mode).as_ref());
        assert_eq!(
            mode_fg,
            crate::palette::ChromeInk::PolicyAct.color(&app.ui_theme)
        );
        assert_ne!(mode_fg, red);

        let permission = permission_label(&app);
        let permission_fg = fg_at(permission.as_ref());
        assert_eq!(
            permission_fg,
            crate::palette::ChromeInk::PermissionFullAccess.color(&app.ui_theme)
        );
        assert_eq!(
            crate::palette::ChromeInk::PermissionFullAccess.family(),
            crate::palette::SemanticFamily::Cognition
        );
    }

    #[test]
    fn configured_session_tokens_follow_underwater_header_width_priority() {
        // Pin the version stamp: the Wide-tier breakpoints below are
        // calibrated to a sha-length stamp, which #5245 no longer guarantees
        // on a local build.
        let _version = BuildVersionGuard::set("0.9.4 (000000000000)");
        let mut app = test_app();
        app.header_items = vec![HeaderItem::Tokens];
        app.session.total_input_tokens = 18_000;
        app.session.total_cache_hit_tokens = 12_000;
        app.session.total_output_tokens = 6_000;
        app.session.last_prompt_tokens = Some(48_000);

        // The optional chip is the only elidable element. It appears once the
        // terminal can hold it alongside the whole baseline right-hand chrome
        // plus the guaranteed-left minimum (brand, mode, effort, permission +
        // filesystem scope). Route detail — already the first thing this header
        // truncates under pressure — is what yields the space.
        //
        // The Wide tier re-adds the version stamp to the baseline, but the
        // route detail yields before the complete optional chip.
        for (width, should_show_tokens, should_show_context) in [
            (40, false, false),
            (60, false, true),
            (80, false, true),
            (93, false, true),
            (94, true, true),
            (100, true, true),
            (110, false, true),
            (130, true, true),
        ] {
            let header = header_text(&app, width);
            assert_eq!(
                header.contains("18.0k in · 12.0k cch · 6.0k out"),
                should_show_tokens,
                "unexpected token visibility at width {width}: {header:?}",
            );
            assert_eq!(
                header.contains('%'),
                should_show_context,
                "unexpected context visibility at width {width}: {header:?}",
            );
            assert!(
                header.to_ascii_lowercase().contains("work"),
                "mode must survive at width {width}: {header:?}",
            );
            assert!(
                header.to_ascii_lowercase().contains("ask"),
                "permission must survive at width {width}: {header:?}",
            );
        }
    }

    #[test]
    fn underwater_header_shows_update_chip_only_when_update_available() {
        // The startup version check sets the label once; the chip then rides
        // the right-hand chrome until the session ends (#14).
        let mut app = test_app();
        app.update_available = Some("↑ v0.9.5".to_string());
        for width in [96, 130] {
            let header = header_text(&app, width);
            assert!(
                header.contains("↑ v0.9.5"),
                "update chip missing at width {width}: {header:?}"
            );
        }
        // Under width pressure the chip yields cleanly — never clipped
        // mid-chip, never evicting the mode/permission posture.
        let narrow = header_text(&app, 60);
        assert!(
            !narrow.contains('↑'),
            "update chip must drop when the line has no room: {narrow:?}"
        );
        assert!(
            narrow.to_ascii_lowercase().contains("ask"),
            "permission must survive at width 60: {narrow:?}"
        );

        // Up to date (or the check never ran): silent.
        let app = test_app();
        let header = header_text(&app, 130);
        assert!(
            !header.contains('↑'),
            "no update chip without an available update: {header:?}"
        );
    }

    #[test]
    fn underwater_header_keeps_session_tokens_opt_in() {
        let mut app = test_app();
        app.header_items.clear();
        app.session.total_input_tokens = 18_000;
        app.session.total_cache_hit_tokens = 12_000;
        app.session.total_output_tokens = 6_000;
        app.session.last_prompt_tokens = Some(48_000);

        let normal_header = header_text(&app, 60);
        let wide_header = header_text(&app, 110);

        assert!(
            !normal_header.contains("18.0k in"),
            "header: {normal_header:?}"
        );
        assert!(
            normal_header.contains('%'),
            "context meter missing: {normal_header:?}"
        );
        assert!(
            wide_header.contains('%'),
            "context meter missing: {wide_header:?}"
        );
        assert!(
            wide_header.contains(&format!("v{}", shell_build_version())),
            "version missing: {wide_header:?}"
        );
    }

    #[test]
    fn compact_header_keeps_mode_and_effective_permission() {
        let mut app = test_app();
        app.mode = AppMode::Operate;
        app.approval_mode = ApprovalMode::Bypass;
        app.reasoning_effort = crate::tui::app::ReasoningEffort::Low;
        app.model = "provider/model-with-a-deliberately-long-route-name".to_string();

        let header = header_text(&app, 40);

        assert!(header.starts_with("cw"), "brand missing: {header:?}");
        assert!(
            header.to_ascii_lowercase().contains("operate"),
            "mode missing: {header:?}"
        );
        assert!(
            header.contains("Full Access"),
            "permission posture missing: {header:?}"
        );
        assert!(
            header.contains(" · l · Full Access"),
            "effective effort missing: {header:?}"
        );
    }

    #[test]
    fn ocean_header_renders_active_goal_and_hides_it_when_unset_or_terminal() {
        // #39: the ocean shell has no sidebar, so the topbar is the surface
        // that must show a goal the moment `create_goal` sets it.
        let mut app = test_app();
        let idle = header_text(&app, 120);
        assert!(
            !idle.contains("goal"),
            "no goal chip without an active goal: {idle:?}"
        );

        app.hunt.quarry = Some("Ship the v0.9.4 release train".to_string());
        let hunting = header_text(&app, 120);
        assert!(
            hunting.contains("goal Ship the v0.9.4"),
            "active goal missing from ocean topbar: {hunting:?}"
        );

        app.hunt.verdict = crate::tui::app::HuntVerdict::Hunted;
        let done = header_text(&app, 120);
        assert!(
            !done.contains("goal"),
            "terminal goal must not linger in the topbar: {done:?}"
        );
    }

    #[test]
    fn ocean_header_names_a_paused_goal() {
        let mut app = test_app();
        app.paused_quarry = Some("Audit the fleet roster".to_string());
        let header = header_text(&app, 120);
        assert!(
            header.contains("goal paused Audit the"),
            "paused goal must say so: {header:?}"
        );
    }

    #[test]
    fn ocean_header_keeps_goal_chip_in_cramped_layouts() {
        let mut app = test_app();
        app.model = "provider/model-with-a-deliberately-long-route-name".to_string();
        app.hunt.quarry = Some("Ship it".to_string());
        // Width pressure forces the cramped rebuild: the route yields first
        // and the goal chip survives whole.
        let header = header_text(&app, 80);
        assert!(
            header.contains("goal Ship it"),
            "goal chip must survive width pressure: {header:?}"
        );
        // When even a minimal chip cannot fit alongside mode, effort, and
        // permission, it drops cleanly instead of clipping mid-word.
        let narrow = header_text(&app, 48);
        assert!(
            !narrow.contains("goal"),
            "unsupportable goal chip must drop, not clip: {narrow:?}"
        );
    }

    #[test]
    fn ocean_header_renders_running_workflow_chip_and_hides_it_when_idle() {
        // #5040: a collapsed workflow run must stay visible in the ocean
        // topbar — the same chip the classic header shows.
        let mut app = test_app();
        let idle = header_text(&app, 120);
        assert!(
            !idle.contains("wf "),
            "no workflow chip without a run: {idle:?}"
        );

        app.workflow_panel = Some(crate::tui::widgets::workflow_panel::WorkflowPanel::new(
            "wf_1", "ship it", 0,
        ));
        let running = header_text(&app, 120);
        assert!(
            running.contains("wf running"),
            "running workflow chip missing from ocean topbar: {running:?}"
        );
    }

    #[test]
    fn ocean_header_keeps_completed_workflow_status_visible() {
        let mut app = test_app();
        let mut panel =
            crate::tui::widgets::workflow_panel::WorkflowPanel::new("wf_2", "ship it", 1_000);
        panel.lifecycle = crate::tui::widgets::workflow_panel::WorkflowPanelLifecycle::Succeeded;
        panel.completed_at_ms = Some(61_000);
        app.workflow_panel = Some(panel);
        let header = header_text(&app, 120);
        assert!(
            header.contains("wf success"),
            "completed workflow status missing: {header:?}"
        );
    }

    #[test]
    fn ocean_header_keeps_workflow_chip_in_cramped_layouts() {
        let mut app = test_app();
        app.model = "provider/model-with-a-deliberately-long-route-name".to_string();
        app.workflow_panel = Some(crate::tui::widgets::workflow_panel::WorkflowPanel::new(
            "wf_3", "ship it", 0,
        ));
        // Width pressure forces the cramped rebuild: the route yields first
        // and the workflow chip survives whole.
        let header = header_text(&app, 80);
        assert!(
            header.contains("wf running"),
            "workflow chip must survive width pressure: {header:?}"
        );
        // When even a minimal chip cannot fit alongside mode, effort, and
        // permission, it drops cleanly instead of clipping mid-word.
        let narrow = header_text(&app, 48);
        assert!(
            !narrow.contains("wf "),
            "unsupportable workflow chip must drop, not clip: {narrow:?}"
        );
    }

    #[test]
    fn header_labels_follow_the_ask_amber_auto_gold_full_access_coral_ramp() {
        for width in [40, 100] {
            for (approval_mode, expected_label) in [
                (ApprovalMode::Suggest, "ask"),
                (ApprovalMode::Auto, "auto"),
                (ApprovalMode::Bypass, "Full Access"),
            ] {
                let mut app = test_app();
                app.approval_mode = approval_mode;
                let expected_color = match approval_mode {
                    ApprovalMode::Suggest | ApprovalMode::Never => app.ui_theme.permission_ask,
                    ApprovalMode::Auto => app.ui_theme.permission_auto_review,
                    ApprovalMode::Bypass => app.ui_theme.permission_full_access,
                };
                let label = permission_label(&app).into_owned();
                assert!(
                    label.starts_with(expected_label) && label.contains("files:"),
                    "{approval_mode:?}: {label}"
                );
                let area = Rect::new(0, 0, width, 1);
                let mut buf = Buffer::empty(area);

                render_header(area, &mut buf, &app);

                let rendered = (0..width).map(|x| buf[(x, 0)].symbol()).collect::<String>();
                // `auto` can also appear earlier as a route/mode label. The
                // permission posture owns the rightmost occurrence.
                let label_byte = rendered
                    .rfind(expected_label)
                    .expect("permission label should render");
                let label_x = rendered[..label_byte].width() as u16;
                assert_eq!(buf[(label_x, 0)].fg, expected_color, "{approval_mode:?}");
            }
        }
    }

    #[test]
    fn permission_chip_reports_the_same_effective_scope_as_execution() {
        let mut app = test_app();
        app.approval_mode = ApprovalMode::Bypass;
        assert_eq!(
            permission_label(&app),
            Cow::Borrowed("Full Access · files: full disk")
        );

        app.configured_sandbox_mode = Some("workspace-write".to_string());
        assert_eq!(
            permission_label(&app),
            Cow::Borrowed("Full Access · files: workspace")
        );

        app.mode = AppMode::Plan;
        app.configured_sandbox_mode = Some("danger-full-access".to_string());
        assert_eq!(permission_label(&app), Cow::Borrowed("read only"));
    }

    /// The other half of the chip contract: a policy is an intent, and
    /// without a backend nothing applies it. On those platforms the chip must
    /// say so rather than name a boundary that is not enforced (2026-08-04
    /// audit). `DangerFullAccess` is already honest and stays unqualified.
    #[test]
    fn permission_chip_says_unenforced_without_a_backend() {
        let mut app = test_app();
        app.sandbox_backend = None;

        app.approval_mode = ApprovalMode::Bypass;
        app.configured_sandbox_mode = Some("workspace-write".to_string());
        assert_eq!(
            permission_label(&app),
            Cow::Borrowed("Full Access · files: workspace (unenforced)")
        );

        app.configured_sandbox_mode = Some("danger-full-access".to_string());
        assert_eq!(
            permission_label(&app),
            Cow::Borrowed("Full Access · files: full disk")
        );
    }

    #[test]
    fn normal_header_keeps_requested_effective_effort_before_route_detail() {
        let mut app = test_app();
        app.mode = AppMode::Operate;
        app.approval_mode = ApprovalMode::Bypass;
        app.reasoning_effort = crate::tui::app::ReasoningEffort::Low;
        app.model = "provider/model-with-a-deliberately-long-route-name".to_string();

        let header = header_text(&app, 80);

        // First-party DeepSeek maps low -> low (8c5370a56: the wire documents
        // [low, high, max] and has no medium), so requested and effective agree
        // and the header renders the tier alone. Assert the absence of the
        // arrow too: bare `contains("low")` would also pass on the old
        // `low→high` rendering, which is the regression this pins against.
        assert!(header.contains("low"), "effort missing: {header:?}");
        assert!(
            !header.contains("low→"),
            "requested and effective agree on first-party DeepSeek; no arrow expected: {header:?}"
        );
        assert!(
            header.to_ascii_lowercase().contains("operate"),
            "mode missing: {header:?}"
        );
        assert!(
            header.contains("Full Access"),
            "permission posture missing: {header:?}"
        );
    }

    #[test]
    fn compact_header_never_shows_a_whale_emoji_even_for_legacy_settings() {
        // The whale emoji header chip is retired (2026-07-23): a persisted
        // "whale" opt-in renders the typographic mark, and no header width
        // squeeze may reintroduce the emoji beside the model/mode chips.
        let mut app = test_app();
        app.status_indicator = "whale".to_string();
        app.model = "provider/model-with-a-deliberately-long-route-name".to_string();

        let header = header_text(&app, 40);

        assert!(
            !header.contains('🐳') && !header.contains('🐋'),
            "whale emoji must stay out of the header: {header:?}"
        );
        assert!(header.contains("cw"), "cw mark missing: {header:?}");
    }

    #[test]
    fn header_shows_exact_named_custom_provider() {
        let mut app = test_app();
        app.set_provider_identity(crate::config::ApiProvider::Custom, "lm-studio");
        app.model = "local-code-model".to_string();

        let header = header_text(&app, 100);

        assert!(
            header.contains("lm-studio · local-code-model"),
            "{header:?}"
        );
        assert!(!header.contains("Custom ·"), "{header:?}");
    }

    /// The footer consumes the toast system, not the legacy status sink: an
    /// informational acknowledgement must leave on its own instead of
    /// becoming permanent idle chrome.
    #[test]
    fn footer_notices_expire_instead_of_becoming_permanent_chrome() {
        let mut app = test_app();
        app.status_message = Some("Auto-compaction enabled".to_string());

        let fresh = footer_text(&mut app);
        assert!(
            fresh.contains("Auto-compaction enabled"),
            "a fresh notice should surface once: {fresh}"
        );

        for toast in &mut app.status_toasts {
            toast.created_at = Instant::now() - Duration::from_secs(60);
        }
        let later = footer_text(&mut app);
        assert!(
            !later.contains("Auto-compaction"),
            "an informational acknowledgement must expire without user action: {later}"
        );
        assert!(
            later.contains("idle"),
            "the stable phase fact survives the expiry: {later}"
        );
    }

    /// Errors are sticky: they outlive the informational TTL window and stay
    /// until their own resolution window passes, then expire on their own.
    #[test]
    fn footer_errors_outlive_informational_acknowledgements() {
        let mut app = test_app();
        app.status_message = Some("Provider request failed: timeout".to_string());

        let fresh = footer_text(&mut app);
        assert!(fresh.contains("failed"), "error notice missing: {fresh}");

        if let Some(sticky) = app.sticky_status.as_mut() {
            assert_eq!(
                sticky.ttl_ms,
                Some(crate::tui::app::App::STICKY_ERROR_TTL_MS)
            );
            sticky.created_at = Instant::now() - Duration::from_secs(6);
        } else {
            panic!("an error must be promoted to the sticky slot");
        }
        let held = footer_text(&mut app);
        assert!(
            held.contains("failed"),
            "errors must hold past the informational window: {held}"
        );

        if let Some(sticky) = app.sticky_status.as_mut() {
            sticky.created_at = Instant::now()
                - Duration::from_millis(crate::tui::app::App::STICKY_ERROR_TTL_MS + 1);
        }
        let expired = footer_text(&mut app);
        assert!(
            !expired.contains("failed"),
            "sticky errors must expire after their TTL: {expired}"
        );
    }

    #[test]
    fn sticky_error_clears_when_composer_gets_input() {
        let mut app = test_app();
        app.set_sticky_status(
            "workflow failed: script error",
            crate::tui::app::StatusToastLevel::Error,
            None,
        );
        assert!(app.sticky_status.is_some());
        app.insert_char('x');
        assert!(
            app.sticky_status.is_none(),
            "composer activity must dismiss sticky error chrome"
        );
    }

    #[test]
    fn launch_rows_and_direct_keys_share_actions() {
        let mut state = launch();
        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                Locale::En,
            ),
            LaunchAction::NewSession
        );
        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE),
                Locale::En,
            ),
            LaunchAction::NewChat
        );
        assert_eq!(state.selected, 1);
        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT),
                Locale::En,
            ),
            LaunchAction::NewChat
        );

        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
                Locale::En,
            ),
            LaunchAction::Resume
        );
        assert_eq!(state.selected, 2);

        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
                Locale::En,
            ),
            LaunchAction::Changelog
        );
        assert_eq!(state.selected, 4);
    }

    #[test]
    fn launch_work_and_chat_modes_are_session_only_and_restore_policy() {
        let mut app = test_app();
        let initial_policy = (app.allow_shell, app.trust_mode, app.approval_mode);
        let pending_defaults = app.startup_defaults.pending_len();

        assert_eq!(LaunchAction::NewChat.session_mode(), Some(AppMode::Plan));
        let _ = app.set_mode(LaunchAction::NewChat.session_mode().unwrap());
        assert_eq!(app.mode, AppMode::Plan);
        assert_eq!(app.startup_defaults.pending_len(), pending_defaults);

        assert_eq!(
            LaunchAction::NewSession.session_mode(),
            Some(AppMode::Agent)
        );
        let _ = app.set_mode(LaunchAction::NewSession.session_mode().unwrap());
        assert_eq!(app.mode, AppMode::Agent);
        assert_eq!(
            (app.allow_shell, app.trust_mode, app.approval_mode),
            initial_policy,
            "Work must restore the configured Agent policy instead of broadening permissions"
        );
        assert_eq!(
            app.startup_defaults.pending_len(),
            pending_defaults,
            "launch choices must not persist a startup-default change"
        );
    }

    #[test]
    fn worktree_action_collects_a_name_before_creation() {
        let mut state = launch();
        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
                Locale::En,
            ),
            LaunchAction::None
        );
        for ch in "repair-pty".chars() {
            assert_eq!(
                handle_launch_key(
                    &mut state,
                    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
                    Locale::En,
                ),
                LaunchAction::None
            );
        }
        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
                Locale::En,
            ),
            LaunchAction::CreateWorktree("repair-pty".to_string())
        );
    }

    #[test]
    fn unavailable_worktree_is_truthful_and_non_destructive() {
        let mut state = launch();
        state.worktree_available = false;
        assert_eq!(
            handle_launch_key(
                &mut state,
                KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL),
                Locale::En,
            ),
            LaunchAction::None
        );
        assert!(state.worktree_input.is_none());
        assert_eq!(
            state.status.as_deref(),
            Some("New worktree requires a Git repository.")
        );
    }

    #[test]
    fn phase_markers_make_motion_and_attention_explicit() {
        let mut app = test_app();

        app.runtime_turn_status = Some("in_progress".to_string());
        app.turn_started_at = Some(Instant::now() - Duration::from_millis(1_300));
        let (working, label) = phase_marker(&app, ShellPhase::from_app(&app));
        assert!(crate::tui::spinner::BRAILLE_SPINNER_FRAMES.contains(&working));
        assert_eq!(label, "working");

        app.low_motion = true;
        app.turn_started_at = Some(Instant::now() - Duration::from_secs(9));
        assert_eq!(
            phase_marker(&app, ShellPhase::Working).0,
            WORKING_BUBBLE_FRAMES[4]
        );

        app.runtime_turn_status = None;
        app.runtime_turn_status = Some("failed".to_string());
        let (marker, label) = phase_marker(&app, ShellPhase::from_app(&app));
        assert_eq!(marker, "✕");
        assert_eq!(label, "failed");
    }

    #[test]
    fn compaction_activity_owns_phase_label_for_its_full_lifecycle() {
        let mut app = test_app();
        app.is_loading = true;
        app.is_compacting = true;
        app.turn_error_posted = true;
        app.runtime_turn_status = Some("failed".to_string());
        app.active_compaction = Some(crate::tui::app::ActiveCompaction {
            id: "compact-auto".to_string(),
            auto: true,
        });

        assert_eq!(
            LiveActivity::from_app(&app).kind(),
            LiveActivityKind::AutoCompacting
        );
        let phase = ShellPhase::from_app(&app);
        assert_eq!(phase, ShellPhase::Working);
        assert_eq!(
            phase_marker(&app, phase).1,
            "Context automatically compacting…"
        );
        let auto_label = "Context automatically compacting…".to_string();
        app.status_message = Some(auto_label.clone());
        app.last_status_message_seen = Some(auto_label.clone());
        app.push_status_toast(
            auto_label,
            crate::tui::app::StatusToastLevel::Info,
            Some(4_000),
        );
        for toast in &mut app.status_toasts {
            toast.created_at = Instant::now() - Duration::from_secs(5);
        }
        let footer = footer_text(&mut app);
        assert!(
            footer.contains("Context automatically compacting…"),
            "the lifecycle phase must outlive the start toast: {footer}"
        );

        app.active_compaction = Some(crate::tui::app::ActiveCompaction {
            id: "compact-manual".to_string(),
            auto: false,
        });
        assert_eq!(
            LiveActivity::from_app(&app).kind(),
            LiveActivityKind::Compacting
        );
        assert_eq!(phase_marker(&app, phase).1, "Compacting context…");
    }

    #[test]
    fn live_activity_is_truthful_prioritized_and_ignores_stale_tools() {
        use crate::tui::active_cell::ActiveCell;
        use crate::tui::history::{
            ExploringCell, ExploringEntry, GenericToolCell, HistoryCell, ToolCell, ToolStatus,
        };

        let generic = |name: &str, status: ToolStatus| {
            HistoryCell::Tool(ToolCell::Generic(GenericToolCell {
                name: name.to_string(),
                status,
                input_summary: None,
                output: None,
                prompts: None,
                spillover_path: None,
                output_summary: None,
                is_diff: false,
            }))
        };
        let reading = || {
            HistoryCell::Tool(ToolCell::Exploring(ExploringCell {
                entries: vec![ExploringEntry {
                    label: "Reading src/lib.rs".to_string(),
                    status: ToolStatus::Running,
                }],
            }))
        };

        let mut app = test_app();

        // A completed tool may remain in the active group until TurnComplete,
        // but it cannot manufacture liveness on its own.
        let mut stale = ActiveCell::new();
        stale.push_tool("done", generic("write_file", ToolStatus::Success));
        app.active_cell = Some(stale);
        assert_eq!(
            LiveActivity::from_app(&app).kind(),
            LiveActivityKind::Working
        );
        assert_eq!(ShellPhase::from_app(&app), ShellPhase::Idle);

        // Only the explicit streaming pointer earns the reasoning label. No
        // configured effort, elapsed clock, or generic loading inference is
        // involved.
        let mut active = ActiveCell::new();
        let thinking = active.push_thinking(HistoryCell::Thinking {
            content: "private reasoning must not reach the strip".to_string(),
            streaming: true,
            duration_secs: None,
        });
        app.active_cell = Some(active);
        app.streaming_thinking_active_entry = Some(thinking);
        assert_eq!(
            LiveActivity::from_app(&app).kind(),
            LiveActivityKind::Reasoning
        );
        let (_, label) = phase_marker(&app, ShellPhase::from_app(&app));
        assert_eq!(label, "reasoning");
        assert!(!label.contains("private"));

        // A running read wins over a stale thinking pointer.
        app.active_cell
            .as_mut()
            .expect("active cell")
            .push_tool("read", reading());
        let activity = LiveActivity::from_app(&app);
        assert_eq!(activity.kind(), LiveActivityKind::Reading);
        assert_eq!(activity.running_tool_count(), 1);
        assert_eq!(phase_marker(&app, ShellPhase::Working).1, "reading");

        // Mixed tool work is not mislabeled as a pure read pass.
        app.active_cell
            .as_mut()
            .expect("active cell")
            .push_tool("write", generic("write_file", ToolStatus::Running));
        let activity = LiveActivity::from_app(&app);
        assert_eq!(activity.kind(), LiveActivityKind::UsingTool);
        assert_eq!(activity.running_tool_count(), 2);
        assert_eq!(phase_marker(&app, ShellPhase::Working).1, "using tool");

        // Verification remains the strongest live promise.
        app.active_cell
            .as_mut()
            .expect("active cell")
            .push_tool("verify", generic("run_verifiers", ToolStatus::Running));
        assert_eq!(
            LiveActivity::from_app(&app).kind(),
            LiveActivityKind::Verifying
        );
        assert_eq!(ShellPhase::from_app(&app), ShellPhase::Verifying);
    }

    #[test]
    fn live_activity_marker_freezes_for_reduced_or_still_and_has_ascii_fallback() {
        let mut app = test_app();
        app.runtime_turn_status = Some("in_progress".to_string());
        app.turn_started_at = Some(Instant::now() - Duration::from_secs(5));

        app.low_motion = true;
        let reduced = phase_marker(&app, ShellPhase::Working).0;
        assert_eq!(reduced, crate::tui::spinner::BRAILLE_SPINNER_STILL_FRAME);

        app.low_motion = false;
        app.fancy_animations = false;
        let fancy_off = phase_marker(&app, ShellPhase::Working).0;
        assert_eq!(fancy_off, crate::tui::spinner::LIVE_STATIC_MARKER);

        let mut cell = ratatui::buffer::Cell::default();
        cell.set_symbol(fancy_off);
        crate::tui::color_compat::adapt_cell_symbol_for_ascii(&mut cell);
        assert_eq!(cell.symbol(), ">");
        assert!(cell.symbol().is_ascii());
    }

    #[test]
    fn idle_whale_caustic_sweeps_then_parks_offscreen() {
        assert_eq!(idle_mark_shine_opacity(0.5, 0), 0.0);
        assert!(
            idle_mark_shine_opacity(0.5, 640) > 0.32,
            "the raised-cosine band should reach its peak near mid-sweep"
        );
        assert_eq!(
            idle_mark_shine_opacity(0.5, 2_000),
            0.0,
            "the caustic must rest offscreen between sweeps"
        );
    }

    #[test]
    fn idle_whale_caustic_preserves_text_width_and_has_a_static_fallback() {
        let base = Color::Rgb(246, 196, 83);
        let highlight = Color::Rgb(246, 242, 232);
        let text = IDLE_WHALE_ROWS[0];
        let moving = idle_whale_row_spans(text, 0, 640, true, base, highlight, highlight);
        let parked = idle_whale_row_spans(text, 0, 2_000, true, base, highlight, highlight);
        let frozen_a = idle_whale_row_spans(text, 0, 640, false, base, highlight, highlight);
        let frozen_b = idle_whale_row_spans(text, 0, 2_000, false, base, highlight, highlight);

        let content = |spans: &[Span<'_>]| {
            spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        };
        let colors =
            |spans: &[Span<'_>]| spans.iter().map(|span| span.style.fg).collect::<Vec<_>>();

        for spans in [&moving, &parked, &frozen_a, &frozen_b] {
            assert_eq!(content(spans), text);
            assert_eq!(span_width(spans), text.width());
        }
        assert_ne!(colors(&moving), colors(&parked));
        assert_eq!(colors(&frozen_a), colors(&frozen_b));
    }

    #[test]
    fn idle_whale_rows_share_one_centered_block_without_losing_authored_offsets() {
        let mut app = test_app();
        app.ui_theme = crate::palette::ThemeId::Whale.ui_theme();
        app.low_motion = true;
        let width = 60usize;
        let rendered = empty_state_lines(&app, Rect::new(0, 0, width as u16, 16))
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let block_width = idle_whale_block_width(IDLE_WHALE_SPOUT_ROW, &IDLE_WHALE_ROWS);
        let block_inset = (width - block_width) / 2;

        assert_eq!(
            block_width, 17,
            "the crown-fluke mark should stay quiet at 60 cols"
        );
        for row in std::iter::once(IDLE_WHALE_SPOUT_ROW).chain(IDLE_WHALE_ROWS) {
            let line = rendered
                .iter()
                .find(|line| line.trim_start() == row.trim_start())
                .unwrap_or_else(|| panic!("missing authored whale row {row:?}"));
            let rendered_inset = line.chars().take_while(|ch| *ch == ' ').count();
            let authored_inset = row.chars().take_while(|ch| *ch == ' ').count();

            assert_eq!(
                rendered_inset - authored_inset,
                block_inset,
                "row drifted out of the shared silhouette: {line:?}"
            );
            assert!(
                line.width() <= block_inset + block_width,
                "row escaped the centered mark block: {line:?}"
            );
        }
    }

    #[test]
    fn uwu_idle_whale_uses_its_own_centered_block_width() {
        let mut app = test_app();
        app.ui_theme = crate::palette::ThemeId::Uwu.ui_theme();
        app.low_motion = true;
        let width = 60usize;
        let rendered = empty_state_lines(&app, Rect::new(0, 0, width as u16, 16))
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>();
        let block_width = idle_whale_block_width(UWU_IDLE_WHALE_SPOUT_ROW, &UWU_IDLE_WHALE_ROWS);
        let block_inset = (width - block_width) / 2;

        assert_eq!(block_width, 16);
        for row in std::iter::once(UWU_IDLE_WHALE_SPOUT_ROW).chain(UWU_IDLE_WHALE_ROWS) {
            let line = rendered
                .iter()
                .find(|line| line.trim_start() == row.trim_start())
                .unwrap_or_else(|| panic!("missing authored uwu whale row {row:?}"));
            let rendered_inset = line.chars().take_while(|ch| *ch == ' ').count();
            let authored_inset = row.chars().take_while(|ch| *ch == ' ').count();

            assert_eq!(
                rendered_inset - authored_inset,
                block_inset,
                "uwu row drifted out of its own centered silhouette: {line:?}"
            );
        }
    }

    #[test]
    fn idle_whale_has_a_recognizable_ascii_safe_silhouette() {
        let ascii_row = |row: &str| {
            let mut rendered = String::new();
            for ch in row.chars() {
                let mut cell = ratatui::buffer::Cell::default();
                cell.set_symbol(&ch.to_string());
                crate::tui::color_compat::adapt_cell_symbol_for_ascii(&mut cell);
                rendered.push_str(cell.symbol());
            }
            rendered
        };
        let rows = std::iter::once(IDLE_WHALE_SPOUT_ROW)
            .chain(IDLE_WHALE_ROWS)
            .map(ascii_row)
            .collect::<Vec<_>>();

        assert_eq!(
            rows,
            [
                "    o",
                r"  .########.  \^/",
                " |#.###########/",
                "  .########.",
            ]
        );
        assert!(rows.iter().all(|row| row.is_ascii()));
    }

    #[test]
    fn reduced_motion_keeps_the_whole_idle_mark_still_and_cursorless() {
        let mut app = test_app();
        app.low_motion = true;
        app.fancy_animations = true;
        app.cursor_position = 7;
        app.ocean_started_at = Instant::now() - Duration::from_secs(2);
        let first = empty_state_lines(&app, Rect::new(0, 0, 100, 30));

        app.ocean_started_at = Instant::now() - Duration::from_secs(11);
        let second = empty_state_lines(&app, Rect::new(0, 0, 100, 30));

        assert_eq!(first, second, "reduced motion must freeze mark and shine");
        assert_eq!(
            app.cursor_position, 7,
            "the empty-state decoration must leave cursor ownership to the composer"
        );
    }

    #[test]
    fn idle_whale_uses_the_human_brand_role_not_focus_blue() {
        let mut app = test_app();
        app.low_motion = true;
        let lines = empty_state_lines(&app, Rect::new(0, 0, 100, 30));
        let colors = lines
            .iter()
            .flat_map(|line| line.spans.iter())
            .filter_map(|span| span.style.fg)
            .collect::<Vec<_>>();

        assert!(colors.contains(&app.ui_theme.accent_action));
        assert_ne!(app.ui_theme.accent_action, app.ui_theme.accent_primary);
    }

    #[test]
    fn idle_whale_caustic_obeys_motion_policy_and_attention_stillness() {
        let mut app = test_app();
        app.launch.visible = false;
        app.low_motion = false;
        app.fancy_animations = true;
        assert!(idle_mark_animation_enabled(&app));

        app.low_motion = true;
        assert!(!idle_mark_animation_enabled(&app));

        app.low_motion = false;
        app.fancy_animations = false;
        assert!(!idle_mark_animation_enabled(&app));

        app.fancy_animations = true;
        app.ocean_treatment = crate::tui::ocean::OceanTreatment::Flat;
        assert!(idle_mark_animation_enabled(&app));

        app.ocean_treatment = crate::tui::ocean::OceanTreatment::Ombre;
        app.launch.visible = true;
        assert!(!idle_mark_animation_enabled(&app));

        app.launch.visible = false;
        app.view_stack
            .push(crate::tui::views::HelpView::new_for_locale(app.ui_locale));
        assert!(!idle_mark_animation_enabled(&app));
    }

    #[test]
    fn verifying_phase_meters_a_tick_for_test_runs_only() {
        use crate::tui::active_cell::ActiveCell;
        use crate::tui::history::{ExecCell, ExecSource, HistoryCell, ToolCell, ToolStatus};

        let running_exec = |command: &str| {
            HistoryCell::Tool(ToolCell::Exec(ExecCell {
                command: command.to_string(),
                status: ToolStatus::Running,
                output: None,
                live_output: None,
                shell_task_id: None,
                owner_agent_id: None,
                owner_agent_name: None,
                started_at: None,
                duration_ms: None,
                stale_elapsed_since_output_ms: None,
                source: ExecSource::Assistant,
                interaction: None,
                output_summary: None,
            }))
        };

        let mut app = test_app();
        app.runtime_turn_status = Some("in_progress".to_string());
        app.turn_started_at = Some(Instant::now() - Duration::from_secs(3));

        // A live test run reads as `verifying`. Reduced motion keeps the
        // semantic label while sharing the calm, static live-work marker.
        let mut active = ActiveCell::new();
        active.push_tool("exec-1", running_exec("cargo test -p codewhale-tui"));
        app.active_cell = Some(active);
        assert_eq!(ShellPhase::from_app(&app), ShellPhase::Verifying);
        app.low_motion = true;
        let (marker, label) = phase_marker(&app, ShellPhase::Verifying);
        assert_eq!(marker, crate::tui::spinner::BRAILLE_SPINNER_STILL_FRAME);
        assert_eq!(label, "verifying");
        app.low_motion = false;

        // An ordinary build stays `working` — checking must not lie.
        let mut active = ActiveCell::new();
        active.push_tool("exec-2", running_exec("cargo build --release"));
        app.active_cell = Some(active);
        assert_eq!(ShellPhase::from_app(&app), ShellPhase::Working);

        // Verifying is a live phase: strip sits above the composer and
        // shares the live seafoam hue.
        assert!(
            crate::tui::phase_strip::PhaseStripPlacement::for_phase(ShellPhase::Verifying)
                .is_above_composer()
        );
        assert_eq!(
            ShellPhase::Verifying.color(&app),
            app.ui_theme.status_working
        );
    }

    #[test]
    fn attention_and_failure_keep_distinct_semantic_hues() {
        let app = test_app();
        assert_eq!(ShellPhase::Waiting.color(&app), app.ui_theme.accent_action);
        assert_eq!(ShellPhase::Approval.color(&app), app.ui_theme.accent_action);
        assert_eq!(ShellPhase::Failed.color(&app), app.ui_theme.error_fg);
        assert_ne!(
            ShellPhase::Waiting.color(&app),
            ShellPhase::Failed.color(&app)
        );
    }

    #[test]
    fn completion_releases_once_then_settles_to_checkmark() {
        let mut app = test_app();
        app.runtime_turn_status = Some("completed".to_string());
        app.low_motion = false;
        app.fancy_animations = true;
        app.ocean_completion_started_at = Some(Instant::now() - Duration::from_millis(120));

        let (marker, label) = phase_marker(&app, ShellPhase::from_app(&app));
        assert_ne!(marker, "✓");
        assert_eq!(label, "finishing");

        app.ocean_completion_started_at = Some(Instant::now() - Duration::from_millis(700));
        let (marker, label) = phase_marker(&app, ShellPhase::Done);
        assert_eq!(marker, "✓");
        assert_eq!(label, "done");

        app.low_motion = true;
        app.ocean_completion_started_at = Some(Instant::now());
        let (marker, label) = phase_marker(&app, ShellPhase::Done);
        assert_eq!(marker, "✓");
        assert_eq!(label, "done");
    }

    #[test]
    fn draft_phase_beats_stale_completion_status() {
        let mut app = test_app();
        app.runtime_turn_status = Some("completed".to_string());

        assert_eq!(ShellPhase::from_app(&app), ShellPhase::Done);

        app.input = "next task".to_string();
        assert_eq!(ShellPhase::from_app(&app), ShellPhase::Typing);
    }
}
