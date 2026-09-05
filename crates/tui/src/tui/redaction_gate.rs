//! Startup gate for the `[redaction] model_bound = "disabled"` opt-out.
//!
//! Setting `[redaction] model_bound = "disabled"` in `config.toml` only
//! records a request. Lowering the model-bound masking boundary is a security
//! decision, so the interactive TUI shows this full-screen gate on the next
//! launch and only applies the opt-out after the user confirms it here (see
//! [`codewhale_config::redaction`] for the effective-mode contract).
//!
//! The gate follows the onboarding visual grammar — one Underwater surface,
//! one bottom action rail — but it is **not** an onboarding step: returning
//! users see it, and answering it never touches the `.onboarded` marker. The
//! three actions mirror the workspace-trust screen's explicit-key discipline:
//! Enter never confirms by reflex, and each choice advertises its own key.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::localization::MessageId;
use crate::palette;
use crate::tui::app::App;
use crate::tui::views::{ActionHint, render_modal_footer, render_underwater_surface};

/// Whether the startup gate must ask before the current config's
/// `[redaction] model_bound` request can take effect.
pub fn confirmation_required(config: &crate::config::Config) -> bool {
    codewhale_config::redaction::confirmation_required(config.model_bound_redaction())
}

/// Render the gate. Callers (the frame compositor) invoke this only while
/// `app.redaction_gate` is set. The gate has two stages: the first stage
/// explains the opt-out and its risk; pressing the confirm key moves to the
/// second, final-confirmation stage (`app.redaction_gate_confirming`), which
/// repeats the red warning and requires a second explicit confirm before the
/// opt-out is recorded.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    let title = if app.redaction_gate_confirming {
        app.tr(MessageId::RedactionGateConfirmTitle).into_owned()
    } else {
        app.tr(MessageId::RedactionGateTitle).into_owned()
    };
    let hints = action_hints(app);
    let buf = f.buffer_mut();
    let inner = render_underwater_surface(area, buf, &title);
    let content = render_modal_footer(inner, buf, &hints);
    let lines = screen_lines(app, usize::from(content.width), usize::from(content.height));
    if lines.is_empty() {
        return;
    }
    let body = center_vertically(content, lines.len());
    f.render_widget(Paragraph::new(lines), body);
}

fn center_vertically(area: Rect, rows: usize) -> Rect {
    let pad = (area
        .height
        .saturating_sub(u16::try_from(rows).unwrap_or(area.height)))
        / 2;
    Rect {
        y: area.y.saturating_add(pad),
        height: area.height.saturating_sub(pad),
        ..area
    }
}

fn action_hints(app: &App) -> Vec<ActionHint> {
    if app.redaction_gate_confirming {
        vec![
            ActionHint::new(
                "1/Y",
                app.tr(MessageId::RedactionGateActionConfirm).to_string(),
            ),
            ActionHint::new(
                "2/U",
                app.tr(MessageId::RedactionGateActionBack).to_string(),
            ),
            ActionHint::new(
                "3/N",
                app.tr(MessageId::RedactionGateActionQuit).to_string(),
            ),
        ]
    } else {
        vec![
            ActionHint::new(
                "1/Y",
                app.tr(MessageId::RedactionGateActionConfirm).to_string(),
            ),
            ActionHint::new(
                "2/U",
                app.tr(MessageId::RedactionGateActionKeep).to_string(),
            ),
            ActionHint::new(
                "3/N",
                app.tr(MessageId::RedactionGateActionQuit).to_string(),
            ),
        ]
    }
}

fn screen_lines(app: &App, width: usize, _height: usize) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    // The surface title (rendered by `render`) already names the screen, so
    // the body opens with the question itself — no duplicated heading.
    if app.redaction_gate_confirming {
        wrap_body(
            &mut out,
            app,
            MessageId::RedactionGateConfirmQuestion,
            width,
        );
        out.push(Line::from(""));
        wrap_body_danger(&mut out, app, MessageId::RedactionGateDangerNotice, width);
    } else {
        wrap_body(&mut out, app, MessageId::RedactionGateQuestion, width);
        out.push(Line::from(""));
        // The red warning is part of both stages: disabling masking sends
        // credential text to the model, and the user must see that stated in
        // bold red before either confirm.
        wrap_body_danger(&mut out, app, MessageId::RedactionGateDangerNotice, width);
        out.push(Line::from(""));
        wrap_body_muted(&mut out, app, MessageId::RedactionGateRisk, width);
        wrap_body_muted(&mut out, app, MessageId::RedactionGateEffect, width);
        wrap_body_muted(&mut out, app, MessageId::RedactionGateRollbackHint, width);
    }
    if let Some(message) = app.status_message.as_deref() {
        out.push(Line::from(""));
        out.push(Line::from(Span::styled(
            message.to_string(),
            Style::default().fg(palette::STATUS_WARNING),
        )));
    }
    out
}

/// Body sentence in the primary lane.
fn wrap_body(lines: &mut Vec<Line<'static>>, app: &App, id: MessageId, width: usize) {
    let text = app.tr(id);
    for segment in wrap_words(&text, width) {
        lines.push(Line::from(Span::styled(
            segment,
            Style::default().fg(palette::TEXT_PRIMARY),
        )));
    }
}

/// The red, bold warning shown on both gate stages. Wrap on display width
/// exactly like the other lanes so no locale clips mid-word.
fn wrap_body_danger(lines: &mut Vec<Line<'static>>, app: &App, id: MessageId, width: usize) {
    let text = app.tr(id);
    for segment in wrap_words(&text, width) {
        lines.push(Line::from(Span::styled(
            segment,
            Style::default()
                .fg(palette::STATUS_ERROR)
                .add_modifier(ratatui::style::Modifier::BOLD),
        )));
    }
}

/// Supporting hint in the muted lane.
fn wrap_body_muted(lines: &mut Vec<Line<'static>>, app: &App, id: MessageId, width: usize) {
    let text = app.tr(id);
    for segment in wrap_words(&text, width) {
        lines.push(Line::from(Span::styled(
            segment,
            Style::default().fg(palette::TEXT_MUTED),
        )));
    }
}

/// Characters that may not begin a line in Japanese and Chinese typography
/// (a small, uncontroversial kinsoku set). Kept in sync with the onboarding
/// screens' wrapper: a gate question cut mid-word is not answerable.
const NO_LINE_START: &[char] = &[
    '。', '、', '．', '，', '」', '』', '）', '］', '｝', '〕', '〉', '》', '”', '’', '！', '？',
    '：', '；', 'ー', '々', '·', '…', '!', '?', ',', '.', ':', ';', ')', ']', '}',
];

/// Break one unbreakable token into lines of at most `width` display columns.
fn break_by_display_width(text: &str, width: usize) -> Vec<String> {
    use unicode_segmentation::UnicodeSegmentation;
    use unicode_width::UnicodeWidthStr;

    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;

    for cluster in text.graphemes(true) {
        let cluster_width = UnicodeWidthStr::width(cluster);
        if current_width + cluster_width > width && !current.is_empty() {
            let starts_forbidden = cluster
                .chars()
                .next()
                .is_some_and(|c| NO_LINE_START.contains(&c));
            if starts_forbidden {
                current.push_str(cluster);
                out.push(std::mem::take(&mut current));
                current_width = 0;
                continue;
            }
            out.push(std::mem::take(&mut current));
            current_width = 0;
        }
        current.push_str(cluster);
        current_width += cluster_width;
    }

    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// Word wrap by display width so the composed row count is exact and no
/// paragraph re-wrap can clip a locale with longer sentences.
fn wrap_words(text: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;
    let width = width.max(8);
    let mut out = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    for word in text.split_whitespace() {
        let word_width = UnicodeWidthStr::width(word);

        if word_width > width {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
                current_width = 0;
            }
            let mut chunks = break_by_display_width(word, width);
            if let Some(last) = chunks.pop() {
                out.extend(chunks);
                current_width = UnicodeWidthStr::width(last.as_str());
                current = last;
            }
            continue;
        }

        let needed = if current.is_empty() {
            word_width
        } else {
            current_width + 1 + word_width
        };
        if !current.is_empty() && needed > width {
            out.push(std::mem::take(&mut current));
            current_width = 0;
        }
        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(word);
        current_width += word_width;
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

/// Persist the confirmation and return the written receipt path. Called after
/// the user picks the explicit "confirm" action.
pub fn record_confirmation() -> anyhow::Result<std::path::PathBuf> {
    codewhale_config::redaction::record_model_bound_disabled_confirmation()
        .map_err(anyhow::Error::from)
}

// The "keep masking" answer persists nothing and rewrites no file: the
// current launch stays on the safe default, and because the config field
// still requests `"disabled"`, the gate asks again on the next launch until
// the user confirms or edits the field back to `"enabled"`. The event loop
// implements this inline (it only clears the gate flag); this module-level
// contract comment is where the semantics live.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::tui::app::TuiOptions;
    use crate::tui::views::action_footer_lines;
    use std::path::PathBuf;

    fn app_fixture() -> App {
        let options = TuiOptions {
            model: "test-model".to_string(),
            ..crate::test_support::test_tui_options(PathBuf::from("workspace-fixture"))
        };
        let mut app = App::new(options, &Config::default());
        app.ui_locale = crate::localization::Locale::En;
        app.redaction_gate = true;
        app
    }

    #[test]
    fn gate_names_the_boundary_and_the_three_explicit_actions() {
        let app = app_fixture();
        let body = screen_lines(&app, 70, 24)
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|span| span.content.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(flat.contains("model-bound"), "{body}");
        assert!(flat.contains("API keys"), "{body}");

        let rail = action_hints(&app)
            .iter()
            .flat_map(|hint| action_footer_lines(std::slice::from_ref(hint), 60))
            .flat_map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        for expected in ["1/Y", "2/U", "3/N"] {
            assert!(
                rail.contains(expected),
                "missing {expected} in rail: {rail}"
            );
        }
        assert!(rail.contains("confirm"), "{rail}");
        assert!(rail.contains("keep"), "{rail}");
        assert!(rail.contains("quit"), "{rail}");
    }

    #[test]
    fn gate_renders_without_panicking_on_short_screens() {
        // The gate must survive very narrow terminals without clipping the
        // question (see the trust screen's narrow-terminal discipline).
        for width in [40usize, 60, 80, 120] {
            for locale in [
                crate::localization::Locale::En,
                crate::localization::Locale::ZhHans,
            ] {
                let mut app = app_fixture();
                app.ui_locale = locale;
                let _ = screen_lines(&app, width, 24);
                // Both stages must survive the same narrow lanes.
                app.redaction_gate_confirming = true;
                let _ = screen_lines(&app, width, 24);
            }
        }
    }

    /// The red warning is part of both stages, and the second stage swaps the
    /// "keep" action for a "back" action: you can only move forward with an
    /// explicit second confirm.
    #[test]
    fn both_stages_show_the_danger_warning_and_second_stage_offers_back() {
        let first = app_fixture();
        let first_body = screen_lines(&first, 70, 24)
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|span| span.content.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(first_body.contains("Caution"), "{first_body}");

        let mut confirming = app_fixture();
        confirming.redaction_gate_confirming = true;
        let confirm_body = screen_lines(&confirming, 70, 24)
            .into_iter()
            .flat_map(|line| line.spans.into_iter().map(|span| span.content.to_string()))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(confirm_body.contains("really sure"), "{confirm_body}");
        assert!(confirm_body.contains("Caution"), "{confirm_body}");

        let rail = action_hints(&confirming)
            .iter()
            .flat_map(|hint| action_footer_lines(std::slice::from_ref(hint), 60))
            .flat_map(|line| {
                line.spans
                    .into_iter()
                    .map(|span| span.content.to_string())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(rail.contains("back"), "{rail}");
        assert!(
            !rail.contains("keep"),
            "second stage must not offer keep: {rail}"
        );
        assert!(rail.contains("quit"), "{rail}");
    }
}
