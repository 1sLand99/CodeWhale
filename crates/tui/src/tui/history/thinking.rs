//! Rendering for reasoning/thinking transcript cells.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::palette;
use crate::tui::markdown_render;

/// Reasoning header opener. Replaces the spinner glyph on thinking cells —
/// reasoning is a slow exhale, not a tool spin.
pub(super) const REASONING_OPENER: &str = "\u{2026}"; // …
/// Reasoning body left rail. Dashed (`╎`) instead of the solid `▏` block to
/// visually separate reasoning from message body and tool output.
pub(super) const REASONING_RAIL: &str = "\u{254E} "; // ╎ + space
/// Trailing-line cursor on streaming reasoning. Anchored to the live colour
/// so the user sees where new tokens land.
pub(super) const REASONING_CURSOR: &str = "\u{258E}"; // ▎

const THINKING_SUMMARY_LINE_LIMIT: usize = 4;
const THINKING_COMPLETED_PREVIEW_LINE_LIMIT: usize = 10;
const THINKING_STREAMING_PREVIEW_LINE_LIMIT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThinkingVisualState {
    Live,
    Done,
    Idle,
}

#[allow(dead_code)] // Kept for compatibility/tests; live view uses explicit summaries only.
#[must_use]
pub fn extract_reasoning_summary(text: &str) -> Option<String> {
    extract_explicit_reasoning_summary(text).or_else(|| {
        let fallback = text.trim();
        if fallback.is_empty() {
            None
        } else {
            Some(fallback.to_string())
        }
    })
}

fn extract_explicit_reasoning_summary(text: &str) -> Option<String> {
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim();
        if trimmed.to_lowercase().starts_with("summary") {
            let mut summary = String::new();
            if let Some((_, rest)) = trimmed.split_once(':')
                && !rest.trim().is_empty()
            {
                summary.push_str(rest.trim());
                summary.push('\n');
            }
            while let Some(next) = lines.peek() {
                let next_trimmed = next.trim();
                if next_trimmed.is_empty() {
                    break;
                }
                if next_trimmed.starts_with('#') || next_trimmed.starts_with("**") {
                    break;
                }
                summary.push_str(next_trimmed);
                summary.push('\n');
                lines.next();
            }
            let summary = summary.trim().to_string();
            return if summary.is_empty() {
                None
            } else {
                Some(summary)
            };
        }
    }
    None
}

pub(super) fn render_thinking(
    content: &str,
    width: u16,
    streaming: bool,
    duration_secs: Option<f32>,
    collapsed: bool,
    low_motion: bool,
) -> Vec<Line<'static>> {
    render_thinking_with_highlight(
        content,
        width,
        streaming,
        duration_secs,
        collapsed,
        low_motion,
        true,
    )
}

pub(crate) fn render_thinking_with_highlight(
    content: &str,
    width: u16,
    streaming: bool,
    duration_secs: Option<f32>,
    collapsed: bool,
    low_motion: bool,
    highlight: bool,
) -> Vec<Line<'static>> {
    let state = thinking_visual_state(streaming, duration_secs);
    let style = thinking_style();
    // 12% reasoning surface tint over the app ink — the only deliberately
    // warm element in the transcript. Dropped on Ansi-16 terminals where the
    // tint would distort the named palette.
    let depth = cached_color_depth();
    let body_bg = palette::reasoning_surface_tint(depth);
    let body_style = match (highlight, body_bg) {
        (true, Some(bg)) => style.italic().bg(bg),
        (_, None) | (false, Some(_)) => style.italic(),
    };
    let mut lines = Vec::new();

    // Header: `…` opener (replaces the spinner; reasoning isn't a tool, it's
    // a slow exhale) followed by the reasoning label and live status.
    let mut header_spans = vec![
        Span::styled(
            format!("{REASONING_OPENER} "),
            Style::default().fg(thinking_state_accent(state)),
        ),
        Span::styled("reasoning", thinking_title_style()),
    ];
    header_spans.push(Span::styled(" ", Style::default()));
    header_spans.push(Span::styled(
        thinking_status_label(state),
        thinking_status_style(state),
    ));
    if let Some(dur) = duration_secs {
        header_spans.push(Span::styled(" · ", Style::default().fg(palette::TEXT_DIM)));
        header_spans.push(Span::styled(
            crate::elapsed::format_elapsed_ms((dur * 1000.0) as u64),
            thinking_meta_style(),
        ));
    }
    lines.push(Line::from(header_spans));

    let content_width = width.saturating_sub(3).max(1);
    let mut collapsed_without_explicit_summary = false;
    let body_text = if collapsed {
        if streaming {
            // #861 RC4 / #1324: during streaming we don't yet have a
            // completed reasoning block, so `extract_reasoning_summary`
            // is meaningless. Show the raw content and let the
            // truncation logic below keep the *last* `LIMIT` lines so
            // the user sees the model's most recent thinking instead of
            // staring at an empty placeholder.
            content.to_string()
        } else {
            match extract_explicit_reasoning_summary(content) {
                Some(summary) => summary,
                None => {
                    collapsed_without_explicit_summary = true;
                    content.to_string()
                }
            }
        }
    } else {
        content.to_string()
    };
    // #4146/#4148 used to scrub snake_case tokens out of the collapsed
    // reasoning here, to keep CodeWhale's own internals out of the transcript.
    // Removed: the rule could not tell our identifiers from the user's, and in
    // a coding harness the user's dominate. It rendered `short_dated_radar.py`
    // as `….py`, `data/market_data/` as `data/…/`, and every env var and
    // module name as a bare `…`, which made the default reasoning view
    // unreadable. It also protected nothing — the full body was always one
    // keypress away on Space/Ctrl+O — so the only thing it reliably did was
    // damage the surface people actually read.
    let mut rendered = if body_text.trim().is_empty() {
        Vec::new()
    } else {
        markdown_render::render_markdown(&body_text, content_width, body_style)
    };
    let mut truncated = false;
    let line_limit = if streaming {
        THINKING_STREAMING_PREVIEW_LINE_LIMIT
    } else if collapsed_without_explicit_summary {
        THINKING_COMPLETED_PREVIEW_LINE_LIMIT
    } else {
        THINKING_SUMMARY_LINE_LIMIT
    };
    if collapsed && rendered.len() > line_limit {
        if streaming {
            // Drop the *head* during streaming so the visible window
            // tracks the live cursor at the bottom.
            let drop = rendered.len() - line_limit;
            rendered.drain(0..drop);
        } else {
            rendered.truncate(line_limit);
        }
        truncated = true;
    }

    let rail_style = Style::default().fg(thinking_state_accent(state));
    let cursor_style = Style::default().fg(palette::ACCENT_REASONING_LIVE);

    if rendered.is_empty() && streaming {
        let mut spans = vec![Span::styled(REASONING_RAIL.to_string(), rail_style)];
        spans.push(Span::styled("reasoning...", body_style.italic()));
        if !low_motion {
            spans.push(Span::styled(format!(" {REASONING_CURSOR}"), cursor_style));
        }
        lines.push(Line::from(spans));
    }

    let last_idx = rendered.len().saturating_sub(1);
    for (idx, line) in rendered.into_iter().enumerate() {
        let mut spans = vec![Span::styled(REASONING_RAIL.to_string(), rail_style)];
        spans.extend(line.spans);
        // Trailing cursor on the very last body line while streaming —
        // signals "still generating" without churning every line.
        if streaming && !low_motion && idx == last_idx {
            spans.push(Span::styled(format!(" {REASONING_CURSOR}"), cursor_style));
        }
        lines.push(Line::from(spans));
    }

    let needs_affordance = collapsed
        && if streaming {
            // #861 RC4 / #1324: during streaming, surface the affordance
            // whenever any head lines have been clipped so the user
            // knows there's more above and how to reach it.
            truncated
        } else {
            truncated || body_text.trim() != content.trim()
        };
    if needs_affordance {
        // One notation with the footer: `cap:verb`, middle-dot separator.
        let label = if streaming {
            "Ctrl+O:more"
        } else {
            "Space:expand · Ctrl+O:detail"
        };
        lines.push(Line::from(vec![
            Span::styled(REASONING_RAIL.to_string(), rail_style),
            Span::styled(label, Style::default().fg(palette::TEXT_MUTED).italic()),
        ]));
    }

    lines
}

pub(super) fn render_hidden_thinking_activity(
    _width: u16,
    duration_secs: Option<f32>,
    low_motion: bool,
) -> Vec<Line<'static>> {
    let state = ThinkingVisualState::Live;
    let mut header_spans = vec![
        Span::styled(
            format!("{REASONING_OPENER} "),
            Style::default().fg(thinking_state_accent(state)),
        ),
        // A hidden live block needs one receipt, not stacked variants of the
        // same state ("reasoning live" plus "reasoning hidden; working").
        Span::styled("reasoning hidden", thinking_title_style()),
    ];
    if let Some(dur) = duration_secs {
        header_spans.push(Span::styled(" · ", Style::default().fg(palette::TEXT_DIM)));
        header_spans.push(Span::styled(
            crate::elapsed::format_elapsed_ms((dur * 1000.0) as u64),
            thinking_meta_style(),
        ));
    }
    if !low_motion {
        header_spans.push(Span::styled(
            format!(" {REASONING_CURSOR}"),
            Style::default().fg(palette::ACCENT_REASONING_LIVE),
        ));
    }
    vec![Line::from(header_spans)]
}

fn thinking_style() -> Style {
    Style::default().fg(palette::TEXT_REASONING)
}

fn thinking_visual_state(streaming: bool, duration_secs: Option<f32>) -> ThinkingVisualState {
    if streaming {
        ThinkingVisualState::Live
    } else if duration_secs.is_some() {
        ThinkingVisualState::Done
    } else {
        ThinkingVisualState::Idle
    }
}

fn thinking_status_label(state: ThinkingVisualState) -> &'static str {
    match state {
        ThinkingVisualState::Live => "live",
        ThinkingVisualState::Done => "done",
        ThinkingVisualState::Idle => "idle",
    }
}

fn thinking_title_style() -> Style {
    Style::default()
        .fg(palette::TEXT_SOFT)
        .add_modifier(Modifier::BOLD)
}

fn thinking_status_style(state: ThinkingVisualState) -> Style {
    Style::default().fg(match state {
        ThinkingVisualState::Live => palette::ACCENT_REASONING_LIVE,
        ThinkingVisualState::Done => palette::TEXT_DIM,
        ThinkingVisualState::Idle => palette::TEXT_DIM,
    })
}

fn thinking_meta_style() -> Style {
    Style::default().fg(palette::TEXT_DIM)
}

fn thinking_state_accent(state: ThinkingVisualState) -> Color {
    match state {
        ThinkingVisualState::Live => palette::ACCENT_REASONING_LIVE,
        ThinkingVisualState::Done => palette::TEXT_DIM,
        ThinkingVisualState::Idle => palette::TEXT_DIM,
    }
}

/// Once-initialised colour depth for the terminal session. Avoids re-reading
/// `COLORTERM` / `TERM` env vars on every frame.
static COLOR_DEPTH: std::sync::OnceLock<palette::ColorDepth> = std::sync::OnceLock::new();

fn cached_color_depth() -> palette::ColorDepth {
    *COLOR_DEPTH.get_or_init(palette::ColorDepth::detect)
}
