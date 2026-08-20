//! Live phase band for the underwater shell.
//!
//! The HTML reference attaches activity to the transcript and leaves the
//! composer as the final stable object. That means live phases
//! (working / waiting / approval / failed / done) render **above** the
//! composer, while idle and typing keep a quiet phase line beneath it.
//!
//! This module only decides Ocean placement and paints the one-line band. The
//! Classic shell it used to defer to was removed in 0.9.4 — see the migration
//! shim note at `crates/tui/src/tui/ocean.rs:35` — so there is no
//! footer-below-composer fallback path left.

use std::borrow::Cow;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

use crate::localization::{MessageId, tr};
use crate::palette::{ChromeInk, UiTheme};
use crate::tui::{
    app::App,
    underwater::{LiveActivity, ShellPhase, ShellTier, phase_marker_with_activity},
};

/// Where the phase band sits relative to the composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStripPlacement {
    /// Live activity: phase sits on the transcript side of the prompt.
    AboveComposer,
    /// Idle / drafting: quiet phase under the prompt.
    BelowComposer,
}

impl PhaseStripPlacement {
    /// Live phases stay above the composer so the prompt is the bottom
    /// stable object. Idle and typing keep the quiet footer under `❯`.
    #[must_use]
    pub fn for_phase(phase: ShellPhase) -> Self {
        match phase {
            ShellPhase::Working
            | ShellPhase::Verifying
            | ShellPhase::Waiting
            | ShellPhase::Approval
            | ShellPhase::Failed
            | ShellPhase::Done => Self::AboveComposer,
            ShellPhase::Idle | ShellPhase::Typing => Self::BelowComposer,
        }
    }

    #[must_use]
    pub fn is_above_composer(self) -> bool {
        matches!(self, Self::AboveComposer)
    }
}

/// Fixed one-row reservation for the phase band.
#[must_use]
pub fn height() -> u16 {
    1
}

fn span_width(spans: &[Span<'_>]) -> usize {
    spans.iter().map(|span| span.content.width()).sum()
}

/// Compact working detail for the phase band: `×N` for tools or `1m 15s`
/// while the model is thinking.
/// Kept quieter than the classic footer's verbose tool-status line so the
/// transcript owns the ledger and the strip only names the live pulse.
fn working_detail(app: &App, activity: LiveActivity) -> Option<String> {
    let running = activity.running_tool_count();
    let secs = app
        .turn_started_at
        .map(|started| started.elapsed().as_secs());
    match (running, secs) {
        (0, Some(secs)) if secs > 0 => Some(crate::elapsed::format_elapsed_secs(secs)),
        (n, Some(_)) if n > 0 => Some(format!("×{n}")),
        (n, None) if n > 0 => Some(format!("×{n}")),
        _ => None,
    }
}

fn session_cache_hit_percentage(app: &App) -> Option<u8> {
    let hit = u64::from(app.session.total_cache_hit_tokens);
    let miss = u64::from(app.session.total_cache_miss_tokens);
    let total = hit + miss;
    if total == 0 {
        return None;
    }

    // Round to the nearest whole percent. Widen before adding so sessions
    // with saturated u32 telemetry counters can never render above 100%.
    Some(((hit * 100 + total / 2) / total) as u8)
}

/// Route identity for the rail, shed field by field until it fits `budget`.
///
/// The old version composed the full `provider · model · effort` label and
/// then `truncate_to_width`'d it to a fixed 24/44/64 columns, which happily
/// rendered `deepseek-v4-flash-prev…`. A clipped model name is worse than no
/// model name: routes share prefixes, so the ellipsis is the rail admitting
/// it will not tell you which model is answering. Shed the qualifiers
/// instead — provider first, then effort — and if the bare model name still
/// does not fit, shed the whole group. `/model` and `/status` own the full
/// route either way.
fn route_identity_fields(app: &App, tier: ShellTier, budget: usize) -> Option<Vec<String>> {
    let (provider, model) = app.effective_route_identity_display();
    let effort = app.reasoning_effort_display_label();
    if model.is_empty() {
        return None;
    }
    let mut candidates: Vec<Vec<String>> = Vec::new();
    if tier != ShellTier::Compact && !provider.is_empty() && !effort.is_empty() {
        // The smallest shell never repeats the provider: model and effort are
        // the two facts that change what comes back.
        candidates.push(vec![provider, model.clone(), effort.clone()]);
    }
    if !effort.is_empty() {
        candidates.push(vec![model.clone(), effort]);
    }
    candidates.push(vec![model]);
    candidates.into_iter().find(|fields| {
        let width = fields.iter().map(|field| field.width()).sum::<usize>()
            + fields.len().saturating_sub(1) * ITEM_SEPARATOR_WIDTH;
        width <= budget
    })
}

/// Paint an identity group: the model name one step brighter than the
/// qualifiers that narrow it, so the field a mid-session glance is looking
/// for is the field the eye lands on. Two weights, no new colour.
fn identity_spans(fields: &[String], model_index: usize, theme: &UiTheme) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(fields.len() * 2);
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(
                ITEM_SEPARATOR,
                Style::default().fg(ChromeInk::MetadataDim.color(theme)),
            ));
        }
        let ink = if index == model_index {
            ChromeInk::MetadataValue
        } else {
            ChromeInk::Metadata
        };
        spans.push(Span::styled(
            field.clone(),
            Style::default().fg(ink.color(theme)),
        ));
    }
    spans
}

/// Split a notice at its joints, coarsest first.
///
/// A rail notice is prose, and prose has joints. Cutting at a joint keeps
/// every word that survives true; cutting mid-phrase and hanging an ellipsis
/// off the end only advertises that the row lost the argument. Sentence stops
/// are the joint we want; the inner marks are the fallback for a one-sentence
/// notice that is still too long for a narrow rail — losing the second half
/// of `Auto-denied exec_shell: denied earlier` beats losing the warning.
fn notice_clauses<'a>(text: &'a str, marks: &[char]) -> Vec<&'a str> {
    let mut clauses = Vec::new();
    let mut start = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((idx, ch)) = chars.next() {
        if !marks.contains(&ch) {
            continue;
        }
        // Full-width marks carry no trailing space, so they break on sight.
        // ASCII marks only break before whitespace, which keeps `0.9.11`,
        // `docs/TELEMETRY.md`, and `https://…` in one piece.
        let breaks = !ch.is_ascii() || chars.peek().is_none_or(|(_, next)| next.is_whitespace());
        if !breaks {
            continue;
        }
        let end = idx + ch.len_utf8();
        let clause = text[start..end].trim();
        if !clause.is_empty() {
            clauses.push(clause);
        }
        start = end;
    }
    let rest = text[start..].trim();
    if !rest.is_empty() {
        clauses.push(rest);
    }
    clauses
}

/// Sentence stops — the joint a notice prefers to be cut at.
const SENTENCE_MARKS: [char; 7] = ['.', '!', '?', '…', '。', '！', '？'];
/// Inner joints, used only when one sentence still will not fit the rail.
const CLAUSE_MARKS: [char; 8] = [';', ':', ',', '—', '；', '：', '，', '、'];

fn join_while_fitting(clauses: &[&str], budget: usize) -> Option<String> {
    let mut fitted = String::new();
    for clause in clauses {
        // A full-width stop already carries its own breathing room; putting
        // a Latin space after `。` is a typographic accent in the wrong
        // language.
        let space = usize::from(!fitted.is_empty() && fitted.ends_with(|ch: char| ch.is_ascii()));
        let candidate = fitted.width() + space + clause.width();
        if candidate > budget {
            break;
        }
        if space == 1 {
            fitted.push(' ');
        }
        fitted.push_str(clause);
    }
    // A phrase that ends on `:` or `;` is still telling you more is coming —
    // the same lie an ellipsis tells. Cut the mark and let the phrase stand.
    let fitted = fitted
        .trim_end_matches(|ch| CLAUSE_MARKS.contains(&ch) || ch == ' ')
        .to_string();
    (!fitted.is_empty()).then_some(fitted)
}

/// Fit a notice into `budget` by dropping whole trailing clauses.
///
/// Returns `None` only when not even the first inner phrase fits, and the
/// rail then says nothing rather than dangling a stump. Notices get first
/// call on the row: identity and the ledger chips have already stood down by
/// the time this is asked, and the key hints stand down after it if that is
/// what the notice needs.
fn fit_notice(text: &str, budget: usize) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || budget == 0 {
        return None;
    }
    if text.width() <= budget {
        return Some(text.to_string());
    }
    let sentences = notice_clauses(text, &SENTENCE_MARKS);
    if let Some(fitted) = join_while_fitting(&sentences, budget) {
        return Some(fitted);
    }
    let first = sentences.first().copied().unwrap_or(text);
    join_while_fitting(&notice_clauses(first, &CLAUSE_MARKS), budget)
}

/// Toasts share the footer rail, so their typed level must resolve through
/// the same closed status-bar grammar as the phase marker around them.
fn status_toast_ink(level: crate::tui::app::StatusToastLevel) -> ChromeInk {
    match level {
        crate::tui::app::StatusToastLevel::Info => ChromeInk::Info,
        crate::tui::app::StatusToastLevel::Success => ChromeInk::Outcome,
        crate::tui::app::StatusToastLevel::Warning => ChromeInk::Attention,
        crate::tui::app::StatusToastLevel::Error => ChromeInk::Failure,
    }
}

/// Paint the one-line phase rail.
///
/// The rail speaks four kinds of fact and they are not peers:
///
/// * **live state** — the phase marker, in the phase accent. Always present.
/// * **a notice** — a transient thing the session is telling you. When one is
///   live it takes the identity slot and the standing facts stand down; they
///   are back in a few seconds and the notice is not.
/// * **route identity** — provider, model, effort. Standing background truth.
/// * **key hints** — right-aligned, dimmest, the only group that is optional
///   twice over.
///
/// Groups are divided by a blank gutter and by ink family; peers *inside* a
/// group keep the ` · `. Nothing on the rail is ever truncated: every group
/// either fits whole or sheds a whole field.
pub fn render(area: Rect, buf: &mut Buffer, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let status_toast = app.active_status_toast();
    let activity = LiveActivity::from_app(app);
    let phase = ShellPhase::from_app_with_activity(app, activity);
    let tier = ShellTier::for_chrome_width(area.width);
    // Quiet chrome background — never paint the full row in phase accent.
    Block::default()
        .style(Style::default().bg(app.ui_theme.footer_bg))
        .render(area, buf);

    // Compact left rail: one accent cell + marker + verb (not a full-width band).
    let rail_color = phase.color(app);
    let (marker, phase_label) = phase_marker_with_activity(app, phase, activity);
    let phase_style = Style::default().fg(rail_color).add_modifier(
        if matches!(phase, ShellPhase::Waiting | ShellPhase::Approval) {
            Modifier::BOLD
        } else {
            Modifier::empty()
        },
    );
    let mut left = vec![
        Span::styled("▌", phase_style),
        Span::styled(marker, phase_style),
        Span::raw(" "),
        Span::styled(phase_label.clone(), phase_style),
    ];

    if tier != ShellTier::Compact && matches!(phase, ShellPhase::Working | ShellPhase::Verifying) {
        if let Some(detail) = working_detail(app, activity) {
            left.push(Span::styled(
                ITEM_SEPARATOR,
                Style::default().fg(ChromeInk::MetadataDim.color(&app.ui_theme)),
            ));
            left.push(Span::styled(
                detail,
                Style::default().fg(ChromeInk::Active.color(&app.ui_theme)),
            ));
        }
        left.push(Span::styled(
            ITEM_SEPARATOR,
            Style::default().fg(ChromeInk::MetadataDim.color(&app.ui_theme)),
        ));
        // `Esc to interrupt` is a key hint that happens to sit next to the
        // thing it interrupts. It reads in the hint weight the right-hand
        // chords use, not in the separator weight it used to share.
        left.push(Span::styled(
            tr(app.ui_locale, MessageId::FooterHintEscInterrupt).into_owned(),
            Style::default().fg(ChromeInk::MetadataHint.color(&app.ui_theme)),
        ));
    }

    // Live phases keep the strip quiet: no detail-key chorus competing with
    // the ledger. Idle/typing may advertise keys on the quiet footer.
    // Hints come from shell_key_routing so advertised chords match handlers;
    // bare letters are never advertised — the composer owns printable keys.
    let mut right_text: Cow<'static, str> =
        if PhaseStripPlacement::for_phase(phase).is_above_composer() {
            Cow::Borrowed("")
        } else {
            use crate::tui::shell_key_routing::{ShellBindingId, binding, footer_action_hints};
            let hint_keys = tr(app.ui_locale, MessageId::FooterHintKeys);
            let hint_output = tr(app.ui_locale, MessageId::FooterHintOutput);
            Cow::Owned(match tier {
                ShellTier::Compact => {
                    format!("{}:{hint_keys}", binding(ShellBindingId::Help).footer_chord)
                }
                // Wide used to add `/context:context`. The rail advertises
                // chords you cannot discover any other way; a slash command
                // announces itself the moment you type `/`, so it was paying
                // eighteen columns of a 24-row screen to tell you something
                // the composer already tells you. The rail now reads the same
                // at 80 columns as at 200.
                ShellTier::Normal | ShellTier::Wide => footer_action_hints(false)
                    .replace("{output}", hint_output.as_ref())
                    .replace("{keys}", hint_keys.as_ref()),
            })
        };

    // `← for agents · ↓ to manage`: advertise only while the empty
    // composer owns those keys and a worker exists. Focused surfaces, modals,
    // attachments, and draft text keep the arrows' local meaning.
    let agent_hints = (tier != ShellTier::Compact
        // Slash and mention menus require non-empty trigger text, which the
        // shared predicate rejects; dispatch additionally passes its exact
        // post-completion menu ownership into the same predicate.
        && crate::tui::agent_focus::shell_shortcuts_available(app, false))
    .then(|| crate::tui::agent_focus::footer_agent_hints(app));
    right_text = match agent_hints {
        Some(hints) if !right_text.is_empty() => {
            Cow::Owned(format!("{hints}{ITEM_SEPARATOR}{right_text}"))
        }
        // A settled turn keeps the strip above the composer without the key
        // chorus; the two agent keys still apply there, so they stay visible.
        Some(hints) if phase == ShellPhase::Done => Cow::Owned(hints),
        _ => right_text,
    };

    let available = usize::from(area.width);
    let phase_width = span_width(&left);
    let hint_width = right_text.width();
    let hint_cost = if hint_width == 0 {
        0
    } else {
        hint_width + HINT_GAP
    };

    let notice = (tier != ShellTier::Compact)
        .then_some(status_toast)
        .flatten()
        .filter(|toast| {
            // Completion may land in the same event drain as an approval
            // denial. Keep unresolved attention/error receipts visible after
            // `done`; only routine informational completion copy yields to the
            // stable done marker.
            let survives_completion = matches!(
                toast.level,
                crate::tui::app::StatusToastLevel::Warning
                    | crate::tui::app::StatusToastLevel::Error
            );
            (phase != ShellPhase::Done || survives_completion)
                && !toast.text.trim().is_empty()
                && toast.text.trim() != phase_label.as_ref()
        })
        .and_then(|toast| {
            // Fit the notice against the whole row it could have, then keep
            // the key hints only if they cost the notice nothing. Standing
            // metadata never competes — it stood down before we got here —
            // and the hints are the one group that is optional twice over.
            let alone = available.saturating_sub(phase_width + GROUP_GAP_WIDTH);
            let text = fit_notice(&toast.text, alone)?;
            let hints_survive = hint_cost > 0 && text.width() + hint_cost <= alone;
            Some((text, status_toast_ink(toast.level), hints_survive))
        });

    let mut keep_hints = true;
    if let Some((text, ink, hints_survive)) = notice {
        keep_hints = hints_survive || hint_cost == 0;
        left.push(Span::raw(GROUP_GAP));
        left.push(Span::styled(
            text,
            Style::default().fg(ink.color(&app.ui_theme)),
        ));
    } else {
        // No notice: the standing facts own the row. Identity first, then the
        // ledger, then the session metrics strip on whatever is genuinely
        // left. Each group fits whole or is not painted.
        let mut used = phase_width + hint_cost;
        let identity_budget = available.saturating_sub(used + GROUP_GAP_WIDTH);
        if let Some(fields) = route_identity_fields(app, tier, identity_budget) {
            let model_index = usize::from(fields.len() == 3);
            let spans = identity_spans(&fields, model_index, &app.ui_theme);
            used += GROUP_GAP_WIDTH + span_width(&spans);
            left.push(Span::raw(GROUP_GAP));
            left.extend(spans);
        }

        let mut ledger: Vec<Span<'static>> = Vec::new();
        let chip = app.cumulative_usage_chip();
        if tier != ShellTier::Compact
            && let Some(amount) = match &chip {
                crate::route_billing::UsageChip::Money(amount) => Some(amount.clone()),
                crate::route_billing::UsageChip::PricedSubtotal { .. } => {
                    crate::route_billing::format_usage_chip(&chip)
                }
                _ => None,
            }
            && amount.width() + GROUP_GAP_WIDTH <= available.saturating_sub(used)
        {
            used += GROUP_GAP_WIDTH + amount.width();
            ledger.push(Span::raw(GROUP_GAP));
            ledger.push(Span::styled(
                amount,
                Style::default().fg(ChromeInk::Metadata.color(&app.ui_theme)),
            ));
        }

        // The session metrics strip owns the cache cell when it is on; the
        // standalone `cache N%` chip stays for users who turned the strip off.
        let metrics_enabled = app
            .status_items
            .contains(&crate::config::StatusItem::SessionMetrics);
        if !metrics_enabled
            && tier != ShellTier::Compact
            && app.status_items.contains(&crate::config::StatusItem::Cache)
            && let Some(pct) = session_cache_hit_percentage(app)
        {
            let chip = format!("cache {pct}%");
            let separator_width = if ledger.is_empty() {
                GROUP_GAP_WIDTH
            } else {
                ITEM_SEPARATOR_WIDTH
            };
            if chip.width() + separator_width <= available.saturating_sub(used) {
                used += separator_width + chip.width();
                ledger.push(if ledger.is_empty() {
                    Span::raw(GROUP_GAP)
                } else {
                    Span::styled(
                        ITEM_SEPARATOR,
                        Style::default().fg(ChromeInk::MetadataDim.color(&app.ui_theme)),
                    )
                });
                ledger.push(Span::styled(
                    chip,
                    Style::default().fg(ChromeInk::Metadata.color(&app.ui_theme)),
                ));
            }
        }

        // Session metrics strip (`4 turns · 108 steps │ LLM 11m46s · tools
        // 1m52s │ TTFT 1.5s · 120 tok/s │ cache 99% │ in 9.3M`). It takes
        // whatever columns are genuinely free and sheds its lowest-value
        // groups to fit rather than truncating a number.
        if metrics_enabled && tier != ShellTier::Compact {
            let snapshot = crate::tui::session_metrics::snapshot_from_app(app);
            if !snapshot.is_empty() {
                let budget = available.saturating_sub(used + LEDGER_OPENER_WIDTH);
                let ascii = crate::tui::color_compat::ascii_safe_enabled();
                let strip = crate::tui::session_metrics::fit_to_width(
                    crate::tui::session_metrics::build_groups(snapshot, app.ui_locale),
                    budget,
                    crate::tui::session_metrics::Separators::for_ascii(ascii),
                );
                if !strip.is_empty() {
                    // The gutter opens the ledger group; the bar is the
                    // metrics strip's own internal divider, so it only shows
                    // up once the group is already open.
                    ledger.push(if ledger.is_empty() {
                        Span::raw(GROUP_GAP)
                    } else {
                        Span::styled(
                            if ascii { " | " } else { " │ " },
                            Style::default().fg(ChromeInk::MetadataDim.color(&app.ui_theme)),
                        )
                    });
                    ledger.extend(crate::tui::session_metrics::spans(&strip, &app.ui_theme));
                }
            }
        }
        left.extend(ledger);
    }

    let left_width = span_width(&left);
    if keep_hints && hint_width > 0 && left_width + hint_width < available {
        left.push(Span::raw(" ".repeat(available - left_width - hint_width)));
        left.push(Span::styled(
            right_text.into_owned(),
            Style::default().fg(ChromeInk::MetadataHint.color(&app.ui_theme)),
        ));
    }
    Paragraph::new(Line::from(left)).render(area, buf);
}

/// Peers inside one group — provider and model, a count and its verb — keep
/// the middle dot.
const ITEM_SEPARATOR: &str = " · ";
const ITEM_SEPARATOR_WIDTH: usize = 3;
/// Blank gutter between the rail's semantic groups.
///
/// The rail used to join every group with that same ` · ` in that same ink,
/// so live state, route identity, a privacy notice, and keyboard hints all
/// read as one run-on list of peers. A dot separates peers inside a group; a
/// gutter separates groups. Same three columns, no extra ink, and the eye
/// finally has something to skim by.
const GROUP_GAP: &str = "   ";
const GROUP_GAP_WIDTH: usize = 3;
/// Blank columns kept between the left run and the right-aligned key hints.
const HINT_GAP: usize = 2;
/// Width of whichever mark opens the session metrics strip — the group
/// gutter when the ledger starts with it, the ` │ ` bar when chips came
/// first. Both are three columns.
const LEDGER_OPENER_WIDTH: usize = 3;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::Config,
        tui::active_cell::ActiveCell,
        tui::app::TuiOptions,
        tui::history::{ExecCell, ExecSource, HistoryCell, ToolCell, ToolStatus},
    };
    use ratatui::{Terminal, backend::TestBackend};
    use std::{
        path::PathBuf,
        time::{Duration, Instant},
    };

    fn test_app() -> App {
        App::new(
            TuiOptions {
                model: "deepseek-v4-flash".to_string(),
                ..crate::test_support::test_tui_options(PathBuf::from("."))
            },
            &Config::default(),
        )
    }

    #[test]
    fn live_phases_sit_above_composer_idle_stays_below() {
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Working),
            PhaseStripPlacement::AboveComposer
        );
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Waiting),
            PhaseStripPlacement::AboveComposer
        );
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Approval),
            PhaseStripPlacement::AboveComposer
        );
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Failed),
            PhaseStripPlacement::AboveComposer
        );
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Done),
            PhaseStripPlacement::AboveComposer
        );
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Idle),
            PhaseStripPlacement::BelowComposer
        );
        assert_eq!(
            PhaseStripPlacement::for_phase(ShellPhase::Typing),
            PhaseStripPlacement::BelowComposer
        );
    }

    #[test]
    fn footer_toasts_stay_inside_the_closed_color_grammar() {
        use crate::tui::app::StatusToastLevel;

        for (level, expected) in [
            (StatusToastLevel::Info, ChromeInk::Info),
            (StatusToastLevel::Success, ChromeInk::Outcome),
            (StatusToastLevel::Warning, ChromeInk::Attention),
            (StatusToastLevel::Error, ChromeInk::Failure),
        ] {
            assert_eq!(status_toast_ink(level), expected, "{level:?}");

            let mut app = test_app();
            app.ui_theme = crate::palette::ThemeId::Dracula.ui_theme();
            app.push_status_toast("toast proof", level, None);
            let area = Rect::new(0, 0, 160, 1);
            let mut buf = Buffer::empty(area);
            render(area, &mut buf, &mut app);
            let rendered = (0..area.width)
                .map(|x| buf[(x, 0)].symbol())
                .collect::<String>();
            let byte = rendered
                .find("toast proof")
                .unwrap_or_else(|| panic!("{level:?} toast should render: {rendered:?}"));
            let x = rendered[..byte].width() as u16;
            assert_eq!(
                buf[(x, 0)].fg,
                expected.color(&app.ui_theme),
                "{level:?} must use the active theme's grammar slot"
            );
        }
    }

    #[test]
    fn working_marker_uses_the_live_work_status_role() {
        let app = test_app();
        assert_eq!(ShellPhase::Working.color(&app), app.ui_theme.status_working);
        assert_ne!(ShellPhase::Working.color(&app), app.ui_theme.info);
        assert_eq!(
            crate::tui::underwater::phase_ink(ShellPhase::Working),
            ChromeInk::Active
        );
        assert_eq!(
            crate::tui::underwater::phase_ink(ShellPhase::Failed),
            ChromeInk::Failure
        );
        assert_ne!(
            crate::tui::underwater::phase_ink(ShellPhase::Working).family(),
            crate::palette::SemanticFamily::Failure
        );
    }

    #[test]
    fn working_band_names_tool_use_and_bounded_count_without_key_chorus() {
        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        app.is_loading = true;
        app.turn_started_at = Some(Instant::now() - Duration::from_secs(12));
        let mut active = ActiveCell::new();
        active.push_tool(
            "exec-1",
            HistoryCell::Tool(ToolCell::Exec(ExecCell {
                // A build, not a test run — `cargo test` would truthfully
                // classify as the `verifying` phase (ShellPhase::Verifying).
                command: "cargo build -p tui".to_string(),
                status: ToolStatus::Running,
                output: None,
                live_output: None,
                shell_task_id: None,
                owner_agent_id: None,
                owner_agent_name: None,
                started_at: app.turn_started_at,
                duration_ms: None,
                stale_elapsed_since_output_ms: None,
                source: ExecSource::Assistant,
                interaction: None,
                output_summary: None,
            })),
        );
        app.active_cell = Some(active);

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("using tool"), "{text}");
        assert!(text.contains("×1"), "{text}");
        assert!(
            !text.contains("12s"),
            "tool elapsed time belongs to the live tool row: {text}"
        );
        assert!(
            !text.contains("run ×1"),
            "detail repeated the tool verb: {text}"
        );
        assert!(
            !text.contains("Alt+?") && !text.contains("F1:"),
            "live phase strip stays quiet: {text}"
        );
        assert!(text.contains("Esc to interrupt"), "{text}");
    }

    #[test]
    fn compact_activity_band_keeps_only_the_semantic_label() {
        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        app.turn_started_at = Some(Instant::now() - Duration::from_secs(12));
        let mut active = ActiveCell::new();
        active.push_tool(
            "exec-compact",
            HistoryCell::Tool(ToolCell::Exec(ExecCell {
                command: "cargo build -p tui".to_string(),
                status: ToolStatus::Running,
                output: None,
                live_output: None,
                shell_task_id: None,
                owner_agent_id: None,
                owner_agent_name: None,
                started_at: app.turn_started_at,
                duration_ms: None,
                stale_elapsed_since_output_ms: None,
                source: ExecSource::Assistant,
                interaction: None,
                output_summary: None,
            })),
        );
        app.active_cell = Some(active);

        let backend = TestBackend::new(50, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(text.contains("using tool"), "{text}");
        assert!(
            !text.contains('×'),
            "compact strip leaked count detail: {text}"
        );
        assert!(
            !text.contains("12s"),
            "compact strip leaked timing detail: {text}"
        );
    }

    fn strip_text(app: &mut App, width: u16) -> String {
        let backend = TestBackend::new(width, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), app))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn working_band_keeps_elapsed_time_when_model_is_thinking() {
        let mut app = test_app();
        app.is_loading = true;
        app.turn_started_at = Some(Instant::now() - Duration::from_secs(12));

        assert_eq!(
            working_detail(&app, LiveActivity::from_app(&app)).as_deref(),
            Some("12s")
        );

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Esc to interrupt"), "{text}");
    }

    #[test]
    fn completed_band_keeps_unresolved_warning_visible() {
        let mut app = test_app();
        app.runtime_turn_status = Some("completed".to_string());
        app.push_status_toast(
            "Auto-denied exec_shell: denied earlier; restart Codewhale",
            crate::tui::app::StatusToastLevel::Warning,
            Some(12_000),
        );

        let backend = TestBackend::new(100, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();

        assert!(text.contains("done"), "completion phase missing: {text}");
        assert!(
            text.contains("Auto-denied exec_shell"),
            "completion hid unresolved warning: {text}"
        );
    }

    #[test]
    fn cache_percentage_uses_wide_arithmetic_and_rounds() {
        let mut app = test_app();
        assert_eq!(session_cache_hit_percentage(&app), None);

        app.session.total_cache_hit_tokens = 2;
        app.session.total_cache_miss_tokens = 1;
        assert_eq!(session_cache_hit_percentage(&app), Some(67));

        app.session.total_cache_hit_tokens = u32::MAX;
        app.session.total_cache_miss_tokens = u32::MAX;
        assert_eq!(session_cache_hit_percentage(&app), Some(50));
    }

    #[test]
    fn cache_chip_is_labeled_configurable_and_hidden_when_compact() {
        let mut app = test_app();
        app.status_items = vec![crate::config::StatusItem::Cache];
        app.session.total_cache_hit_tokens = 7;
        app.session.total_cache_miss_tokens = 3;

        let backend = TestBackend::new(80, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("cache 70%"), "{text}");

        app.status_items.clear();
        terminal
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("draw without cache");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!text.contains("cache"), "{text}");

        app.status_items = vec![crate::config::StatusItem::Cache];
        let backend = TestBackend::new(50, 1);
        let mut compact = Terminal::new(backend).expect("compact terminal");
        compact
            .draw(|frame| render(frame.area(), frame.buffer_mut(), &mut app))
            .expect("compact draw");
        let text = compact
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(!text.contains("cache"), "compact strip: {text}");
    }

    fn app_with_session_metrics() -> App {
        let mut app = test_app();
        app.status_items = vec![crate::config::StatusItem::SessionMetrics];
        app.turn_counter = 4;
        // Two model calls: 100 tokens over a 2 s stream (TTFT 500 ms, whole
        // call 2.4 s) and 20 tokens over 1 s (TTFT 300 ms, 1.1 s).
        app.session_metrics
            .record_model_call(100, 2_000, Some(500), Some(2_400));
        app.session_metrics
            .record_model_call(20, 1_000, Some(300), Some(1_100));
        app.session_metrics.record_tool_started("t1");
        app.session_metrics.record_tool_completed("t1");
        app.session.total_input_tokens = 9_300_000;
        app.session.total_cache_hit_tokens = 99;
        app.session.total_cache_miss_tokens = 1;
        app
    }

    #[test]
    fn session_metrics_strip_paints_every_group_when_the_row_has_room() {
        let mut app = app_with_session_metrics();
        let text = strip_text(&mut app, 190);
        assert!(text.contains("4 turns · 3 steps"), "{text}");
        assert!(text.contains("LLM 3.5s · Tool call"), "{text}");
        assert!(text.contains("TTFT avg 400ms · 40 tok/s"), "{text}");
        assert!(text.contains("Cache hit 99%"), "{text}");
        assert!(text.contains("Input 9.3M"), "{text}");
        // The strip must not push the right-hand key hints off the row.
        assert!(text.contains("keys"), "{text}");

        // A 150-column idle row now owns the quiet route identity requested
        // for the footer. It keeps the highest-value session facts and sheds
        // lower-priority cells before it crowds out the key hints.
        let text = strip_text(&mut app, 150);
        assert!(
            text.contains("DeepSeek · deepseek-v4-flash · max"),
            "{text}"
        );
        assert!(text.contains("4 turns"), "{text}");
        assert!(text.contains("LLM 3.5s"), "{text}");
        assert!(text.contains("Cache hit 99%"), "{text}");
        assert!(text.contains("Input 9.3M"), "{text}");
        assert!(text.contains("keys"), "{text}");
    }

    #[test]
    fn session_metrics_strip_sheds_groups_on_narrow_rows_and_never_truncates() {
        let mut app = app_with_session_metrics();
        let normal = strip_text(&mut app, 100);
        assert!(normal.contains("Input 9.3M"), "{normal}");
        assert!(normal.contains("Cache hit 99%"), "{normal}");
        assert!(!normal.contains("tok/s"), "{normal}");
        assert!(normal.contains("keys"), "{normal}");

        let compact = strip_text(&mut app, 60);
        // Whatever survives at 60 columns is whole cells, never a cut number.
        for cell in ["9.3M", "99%", "3.5s", "4 turns"] {
            if compact.contains(cell) {
                assert!(
                    compact.contains(&format!("Input {}", "9.3M"))
                        || compact.contains(&format!("Cache hit {}", "99%"))
                        || compact.contains("LLM 3.5s")
                        || compact.contains("4 turns"),
                    "{compact}"
                );
            }
        }
        assert!(!compact.contains("Tool call"), "{compact}");
        assert!(!compact.contains("tok/s"), "{compact}");
    }

    #[test]
    fn session_metrics_strip_is_hidden_when_compact() {
        let mut app = app_with_session_metrics();
        // 59 columns is the widest Compact row. The working detail and the
        // cache chip already stand down here, so the metrics strip claiming
        // the leftovers is the one thing still crowding the label.
        let text = strip_text(&mut app, 59);
        assert!(!text.contains("turns"), "compact strip: {text}");
        assert!(!text.contains("LLM"), "compact strip: {text}");
        assert!(!text.contains('│'), "compact strip: {text}");
    }

    #[test]
    fn session_metrics_strip_is_hidden_when_the_status_item_is_off_or_nothing_happened() {
        let mut app = app_with_session_metrics();
        app.status_items = vec![crate::config::StatusItem::Cache];
        let text = strip_text(&mut app, 120);
        assert!(!text.contains("turns"), "{text}");
        // The legacy standalone cache chip still serves users who turned
        // the strip off.
        assert!(text.contains("cache 99%"), "{text}");

        let mut fresh = test_app();
        fresh.status_items = vec![crate::config::StatusItem::SessionMetrics];
        let text = strip_text(&mut fresh, 120);
        assert!(!text.contains("turns"), "{text}");
        assert!(!text.contains("│"), "{text}");
    }

    /// The whole point of the gutter: four kinds of fact used to be strung
    /// together with one separator at one weight, so nothing was grouped and
    /// the eye had nothing to skim by.
    #[test]
    fn live_state_and_route_identity_are_divided_by_a_gutter_not_a_dot() {
        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        let text = strip_text(&mut app, 120);
        let label = text.find("idle").expect(&text);
        assert!(
            text[label..].starts_with("idle   DeepSeek"),
            "phase and identity must be divided by the group gutter: {text}"
        );
        // Peers inside the identity group keep the middle dot.
        assert!(
            text.contains("DeepSeek · deepseek-v4-flash · max"),
            "{text}"
        );
    }

    /// `/context:context` cost eighteen columns to advertise something the
    /// composer announces the moment you type `/`. The rail now reads the
    /// same at 80 columns as at 200.
    #[test]
    fn the_rail_advertises_chords_not_slash_commands() {
        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        for width in [80, 120, 200] {
            let text = strip_text(&mut app, width);
            assert!(text.contains("keys"), "{width}: {text}");
            assert!(
                !text.contains("/context"),
                "{width} advertised a slash command: {text}"
            );
        }
    }

    #[test]
    fn notice_clauses_split_on_sentences_and_keep_versions_and_paths_whole() {
        assert_eq!(
            notice_clauses(
                "Counts are on. Code is never collected. See docs/T.md",
                &SENTENCE_MARKS
            ),
            vec![
                "Counts are on.",
                "Code is never collected.",
                "See docs/T.md"
            ]
        );
        assert_eq!(
            notice_clauses("Updated to 0.9.11 from 0.9.10", &SENTENCE_MARKS),
            vec!["Updated to 0.9.11 from 0.9.10"]
        );
        // Full-width stops carry no trailing space, so they break on sight.
        assert_eq!(
            notice_clauses(
                "匿名の利用回数は有効です。会話とコードは収集されません。",
                &SENTENCE_MARKS
            ),
            vec![
                "匿名の利用回数は有効です。",
                "会話とコードは収集されません。"
            ]
        );
        // A colon inside a URL is not a joint.
        assert_eq!(
            notice_clauses("Docs: https://example.test/x", &CLAUSE_MARKS),
            vec!["Docs:", "https://example.test/x"]
        );
    }

    /// `。` already carries its own breathing room; a Latin space after it is
    /// a typographic accent in the wrong language.
    #[test]
    fn shed_clauses_rejoin_without_a_latin_space_after_a_full_width_stop() {
        const JA: &str = "匿名の利用状況集計はオンです。会話やコードは一切収集しません。/settings で変更できます。スキーマ: docs/TELEMETRY.md";
        assert_eq!(
            fit_notice(JA, 62).as_deref(),
            Some("匿名の利用状況集計はオンです。会話やコードは一切収集しません。")
        );
        assert_eq!(
            fit_notice(JA, 40).as_deref(),
            Some("匿名の利用状況集計はオンです。")
        );
    }

    #[test]
    fn a_notice_sheds_whole_clauses_and_never_dangles() {
        const NOTICE: &str = "Anonymous usage counts are on. Conversations and code are never collected. Change this in /settings; schema: docs/TELEMETRY.md";
        assert_eq!(fit_notice(NOTICE, 200).as_deref(), Some(NOTICE));
        assert_eq!(
            fit_notice(NOTICE, 80).as_deref(),
            Some("Anonymous usage counts are on. Conversations and code are never collected.")
        );
        assert_eq!(
            fit_notice(NOTICE, 40).as_deref(),
            Some("Anonymous usage counts are on.")
        );
        assert_eq!(fit_notice("   ", 40), None);
    }

    /// The failure this caught: a one-sentence warning longer than the row
    /// used to have no sentence joint to shed at, so the rail dropped the
    /// whole warning. Inner joints are the fallback, and the phrase that
    /// survives never ends on a `:` or `;` — that mark says "more is coming"
    /// as loudly as an ellipsis does.
    #[test]
    fn a_clause_less_warning_sheds_at_inner_joints_rather_than_vanishing() {
        const WARNING: &str =
            "Auto-denied exec_shell: denied earlier; restart Codewhale to re-enable it.";
        assert_eq!(fit_notice(WARNING, 120).as_deref(), Some(WARNING));
        assert_eq!(
            fit_notice(WARNING, 60).as_deref(),
            Some("Auto-denied exec_shell: denied earlier")
        );
        assert_eq!(
            fit_notice(WARNING, 30).as_deref(),
            Some("Auto-denied exec_shell")
        );

        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        app.push_status_toast(WARNING, crate::tui::app::StatusToastLevel::Warning, None);
        for width in [60u16, 72, 80, 100, 120] {
            let text = strip_text(&mut app, width);
            assert!(
                text.contains("Auto-denied exec_shell"),
                "{width} dropped the warning: {text}"
            );
            assert!(!text.contains('…'), "{width} dangled: {text}");
        }
    }

    /// A notice is a transient thing the session is telling you; identity and
    /// the ledger are standing facts that will still be there in ten seconds.
    /// While a notice is live the standing facts stand down, so the notice
    /// gets a whole sentence instead of a stump.
    #[test]
    fn a_live_notice_stands_the_standing_facts_down() {
        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        app.status_items = vec![crate::config::StatusItem::SessionMetrics];
        app.turn_counter = 4;
        app.push_status_toast(
            "Anonymous usage counts are on. Conversations and code are never collected. Change this in /settings; schema: docs/TELEMETRY.md",
            crate::tui::app::StatusToastLevel::Info,
            Some(12_000),
        );

        let text = strip_text(&mut app, 120);
        assert!(
            text.contains(
                "Anonymous usage counts are on. Conversations and code are never collected."
            ),
            "{text}"
        );
        assert!(
            !text.contains("deepseek-v4-flash"),
            "identity should stand down for a live notice: {text}"
        );
        assert!(!text.contains("turns"), "ledger stood down too: {text}");
        assert!(text.contains("keys"), "hints still fit at 120: {text}");

        // 80 columns holds one clause beside the hints.
        let text = strip_text(&mut app, 80);
        assert!(text.contains("Anonymous usage counts are on."), "{text}");
        assert!(!text.contains("Conversations"), "{text}");
        assert!(text.contains("keys"), "{text}");

        // 60 columns cannot hold a clause and the hints, so the hints — the
        // one group that is optional twice over — yield last and yield whole.
        let text = strip_text(&mut app, 60);
        assert!(text.contains("Anonymous usage counts are on."), "{text}");
        assert!(!text.contains("keys"), "{text}");
    }

    #[test]
    fn nothing_on_the_rail_advertises_truncation() {
        let mut app = test_app();
        app.ui_locale = crate::localization::Locale::En;
        app.status_items = vec![crate::config::StatusItem::SessionMetrics];
        app.turn_counter = 4;
        for width in [40u16, 50, 59, 60, 72, 80, 100, 120, 160] {
            let clean = strip_text(&mut app, width);
            assert!(!clean.contains('…'), "{width} dangled: {clean}");
        }
        app.push_status_toast(
            "Anonymous usage counts are on. Conversations and code are never collected. Change this in /settings; schema: docs/TELEMETRY.md",
            crate::tui::app::StatusToastLevel::Info,
            Some(12_000),
        );
        for width in [40u16, 50, 59, 60, 72, 80, 100, 120, 160] {
            let noticed = strip_text(&mut app, width);
            assert!(!noticed.contains('…'), "{width} dangled: {noticed}");
        }
    }

    /// `deepseek-v4-flash-prev…` could be any of several routes. A clipped
    /// model name is worse than no model name, so the qualifiers go first.
    #[test]
    fn identity_sheds_qualifiers_before_it_would_clip_a_model_name() {
        let mut app = App::new(
            TuiOptions {
                model: "deepseek-v4-flash-preview-2026-05-01".to_string(),
                ..crate::test_support::test_tui_options(PathBuf::from("."))
            },
            &Config::default(),
        );
        app.ui_locale = crate::localization::Locale::En;

        let wide = strip_text(&mut app, 140);
        assert!(
            wide.contains("DeepSeek · deepseek-v4-flash-preview-2026-05-01 · max"),
            "{wide}"
        );

        let mid = strip_text(&mut app, 80);
        assert!(
            mid.contains("deepseek-v4-flash-preview-2026-05-01"),
            "{mid}"
        );
        assert!(!mid.contains("DeepSeek ·"), "provider sheds first: {mid}");

        // Narrow enough that only the bare model name survives; it survives
        // whole or not at all.
        for width in [50u16, 60, 70] {
            let narrow = strip_text(&mut app, width);
            assert!(
                !narrow.contains("deepseek-v4-flash-p")
                    || narrow.contains("deepseek-v4-flash-preview-2026-05-01"),
                "{width} clipped the model name: {narrow}"
            );
        }
    }

    #[test]
    fn session_metrics_strip_is_on_by_default() {
        assert!(
            crate::config::StatusItem::default_footer()
                .contains(&crate::config::StatusItem::SessionMetrics)
        );
        assert_eq!(
            crate::config::StatusItem::from_key("session_metrics"),
            Some(crate::config::StatusItem::SessionMetrics)
        );
    }
}
