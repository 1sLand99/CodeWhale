//! Native first-run disclosure for anonymous usage counting.

use std::cell::RefCell;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Paragraph, Widget, Wrap},
};

use crate::localization::{Locale, MessageId, tr};
use crate::palette;
use crate::telemetry_notice::PendingTelemetryNotice;
use crate::tui::menu_style;
use crate::tui::views::{
    ActionHint, ModalKind, ModalView, ViewAction, ViewEvent, action_footer_lines,
    centered_modal_area, render_modal_footer, render_modal_surface,
};

/// The privacy notice is a normal Codewhale modal: one focus owner, two
/// explicit choices, and no second shell prompt after confirmation.
pub(crate) struct TelemetryNoticeView {
    pending: PendingTelemetryNotice,
    locale: Locale,
    enabled: bool,
    row_hitboxes: RefCell<Vec<(Rect, bool)>>,
}

impl TelemetryNoticeView {
    #[must_use]
    pub(crate) fn new(pending: PendingTelemetryNotice, locale: Locale) -> Self {
        Self {
            pending,
            locale,
            enabled: true,
            row_hitboxes: RefCell::new(Vec::new()),
        }
    }

    fn select_keep(&mut self) {
        self.enabled = true;
    }

    fn select_disable(&mut self) {
        self.enabled = false;
    }

    fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    fn commit(&self, enabled: bool) -> ViewAction {
        ViewAction::EmitAndClose(ViewEvent::TelemetryNoticeDecided {
            enabled,
            pending: self.pending.clone(),
        })
    }

    fn render_choice(&self, label: MessageId, enabled: bool) -> Line<'static> {
        let selected = self.enabled == enabled;
        let marker = crate::tui::glyphs::selection_marker(selected);
        let style = if selected {
            menu_style::selected_row_style().add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(palette::TEXT_PRIMARY)
        };
        Line::from(Span::styled(
            format!("{marker} {}", tr(self.locale, label)),
            style,
        ))
    }

    fn action_hints(&self) -> [ActionHint; 3] {
        [
            ActionHint::new(
                "↑↓",
                tr(self.locale, MessageId::TelemetryNoticeActionChoose),
            ),
            ActionHint::new(
                "Enter",
                tr(self.locale, MessageId::TelemetryNoticeActionConfirm),
            ),
            ActionHint::new("Esc", tr(self.locale, MessageId::TelemetryNoticeActionExit)),
        ]
    }

    fn compact_action_line(&self) -> Line<'static> {
        let key_style = Style::default()
            .fg(palette::WHALE_INFO)
            .add_modifier(Modifier::BOLD);
        let verb_style = Style::default().fg(palette::TEXT_MUTED);
        Line::from(vec![
            Span::styled(" ↑↓ ", key_style),
            Span::styled(
                tr(self.locale, MessageId::TelemetryNoticeActionChoose).into_owned(),
                verb_style,
            ),
            Span::styled(" · Enter ", key_style),
            Span::styled(
                tr(self.locale, MessageId::TelemetryNoticeActionConfirm).into_owned(),
                verb_style,
            ),
            Span::styled(" · Esc ", key_style),
            Span::styled(
                tr(self.locale, MessageId::TelemetryNoticeActionExit).into_owned(),
                verb_style,
            ),
            Span::raw(" "),
        ])
    }

    fn tiny_action_line() -> Line<'static> {
        Line::from(Span::styled(
            " ↑↓ · Enter · Esc ",
            Style::default()
                .fg(palette::WHALE_INFO)
                .add_modifier(Modifier::BOLD),
        ))
    }

    fn compact_notice_text(locale: Locale) -> String {
        // Locale packs author semantic clauses on separate lines for larger
        // compact modals. Reflow those clauses at the actual viewport width
        // so constrained terminals do not spend half-empty rows on source
        // newlines and clip the schema or persistent opt-out path.
        tr(locale, MessageId::TelemetryNoticeCompactBody)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn full_popup_area(area: Rect) -> Rect {
        centered_modal_area(area, 84, 22, 38, 18)
    }

    /// Use the full disclosure only when its exact localized wrapping leaves
    /// room for both choices and the action rail. Width/height thresholds alone
    /// selected the full copy at 56x18 while giving it only eleven prose rows.
    fn full_notice_fits(&self, area: Rect) -> bool {
        let popup = Self::full_popup_area(area);
        // Full mode has a one-cell border and two cells of horizontal padding
        // on both sides. Vertical padding is zero.
        let inner_width = popup.width.saturating_sub(6);
        let inner_height = popup.height.saturating_sub(2);
        if inner_width == 0 || inner_height < 3 {
            return false;
        }
        let footer_height =
            u16::try_from(action_footer_lines(&self.action_hints(), inner_width).len())
                .unwrap_or(u16::MAX);
        let prose_height = inner_height.saturating_sub(footer_height.saturating_add(2));
        let full_body = tr(self.locale, MessageId::TelemetryNoticeBody).into_owned();
        let wrapped_rows = Paragraph::new(full_body)
            .wrap(Wrap { trim: false })
            .line_count(inner_width);
        wrapped_rows <= usize::from(prose_height)
    }
}

impl ModalView for TelemetryNoticeView {
    fn kind(&self) -> ModalKind {
        ModalKind::TelemetryNotice
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_key(&mut self, key: KeyEvent) -> ViewAction {
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'C'))
        {
            return ViewAction::EmitAndClose(ViewEvent::TelemetryNoticeCancelled);
        }
        match key.code {
            KeyCode::Esc => ViewAction::EmitAndClose(ViewEvent::TelemetryNoticeCancelled),
            KeyCode::Enter => self.commit(self.enabled),
            KeyCode::Up | KeyCode::Left | KeyCode::BackTab => {
                self.select_keep();
                ViewAction::None
            }
            KeyCode::Down | KeyCode::Right => {
                self.select_disable();
                ViewAction::None
            }
            KeyCode::Tab => {
                self.toggle();
                ViewAction::None
            }
            KeyCode::Char('y' | 'Y') => self.commit(true),
            KeyCode::Char('n' | 'N') => self.commit(false),
            _ => ViewAction::None,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> ViewAction {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.select_keep();
                ViewAction::None
            }
            MouseEventKind::ScrollDown => {
                self.select_disable();
                ViewAction::None
            }
            MouseEventKind::Down(MouseButton::Left) => {
                let selected = self
                    .row_hitboxes
                    .borrow()
                    .iter()
                    .find_map(|(rect, enabled)| {
                        rect.contains(ratatui::layout::Position::new(mouse.column, mouse.row))
                            .then_some(*enabled)
                    });
                selected.map_or(ViewAction::None, |enabled| self.commit(enabled))
            }
            _ => ViewAction::None,
        }
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        let compact = !self.full_notice_fits(area);
        // At the smallest supported height, use the whole frame. The border,
        // localized action rail, both choices, and every privacy red line all
        // remain visible at 40x12; larger frames keep the calm outer gutter.
        let popup_area = if compact && area.height <= 12 {
            area
        } else if compact {
            centered_modal_area(area, 84, 16, 38, 12)
        } else {
            Self::full_popup_area(area)
        };
        render_modal_surface(area, popup_area, buf);

        let mut block = Block::default()
            .title(Line::from(Span::styled(
                format!(" {} ", tr(self.locale, MessageId::TelemetryNoticeHeadline)),
                Style::default()
                    .fg(palette::WHALE_INFO)
                    .add_modifier(Modifier::BOLD),
            )))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(palette::BORDER_COLOR))
            .style(Style::default().bg(palette::WHALE_BG))
            .padding(if compact {
                Padding::new(0, 0, 0, 0)
            } else {
                Padding::new(2, 2, 0, 0)
            });
        if compact {
            // The compact footer lives in the border instead of consuming one
            // of the eight disclosure rows available at 40x12. The release
            // floor uses key-only hints: localized verbs can exceed the whole
            // 38-column border, while the keys remain language-independent.
            block = block.title_bottom(if popup_area.width <= 40 {
                Self::tiny_action_line()
            } else {
                self.compact_action_line()
            });
        }
        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let content = if compact {
            inner
        } else {
            render_modal_footer(inner, buf, &self.action_hints())
        };
        let choice_height = 2.min(content.height);
        let choices = Rect {
            x: content.x,
            y: content.bottom().saturating_sub(choice_height),
            width: content.width,
            height: choice_height,
        };
        let prose = Rect {
            x: content.x,
            y: content.y,
            width: content.width,
            height: content.height.saturating_sub(choice_height),
        };

        let notice = if compact {
            Self::compact_notice_text(self.locale)
        } else {
            tr(self.locale, MessageId::TelemetryNoticeBody).into_owned()
        };
        Paragraph::new(notice)
            .style(Style::default().fg(palette::TEXT_MUTED))
            .wrap(Wrap { trim: false })
            .render(prose, buf);

        self.row_hitboxes.borrow_mut().clear();
        if choices.height > 0 {
            self.row_hitboxes
                .borrow_mut()
                .push((Rect::new(choices.x, choices.y, choices.width, 1), true));
        }
        if choices.height > 1 {
            self.row_hitboxes.borrow_mut().push((
                Rect::new(choices.x, choices.y.saturating_add(1), choices.width, 1),
                false,
            ));
        }
        Paragraph::new(vec![
            self.render_choice(MessageId::TelemetryNoticeChoiceKeep, true),
            self.render_choice(MessageId::TelemetryNoticeChoiceDisable, false),
        ])
        .render(choices, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codewhale_telemetry::SessionSource;
    use ratatui::{Terminal, backend::TestBackend};

    fn pending() -> PendingTelemetryNotice {
        PendingTelemetryNotice {
            config_path: Some("config.toml".into()),
            setup_state_path: "setup_state.json".into(),
            session_source: SessionSource::Interactive,
        }
    }

    fn decision(action: ViewAction) -> Option<bool> {
        match action {
            ViewAction::EmitAndClose(ViewEvent::TelemetryNoticeDecided { enabled, .. }) => {
                Some(enabled)
            }
            _ => None,
        }
    }

    #[test]
    fn enter_keeps_the_disclosed_default() {
        let mut view = TelemetryNoticeView::new(pending(), Locale::En);
        assert_eq!(
            decision(view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))),
            Some(true)
        );
    }

    #[test]
    fn keyboard_navigation_and_direct_shortcuts_are_explicit() {
        let mut view = TelemetryNoticeView::new(pending(), Locale::En);
        assert!(matches!(
            view.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            ViewAction::None
        ));
        assert_eq!(
            decision(view.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE))),
            Some(false)
        );

        let mut view = TelemetryNoticeView::new(pending(), Locale::En);
        assert_eq!(
            decision(view.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE))),
            Some(false)
        );
        assert_eq!(
            decision(view.handle_key(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::NONE))),
            Some(true)
        );
    }

    #[test]
    fn escape_exits_without_inventing_a_choice() {
        let mut view = TelemetryNoticeView::new(pending(), Locale::En);
        assert!(matches!(
            view.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            ViewAction::EmitAndClose(ViewEvent::TelemetryNoticeCancelled)
        ));
    }

    fn rendered_text(locale: Locale, width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        let view = TelemetryNoticeView::new(pending(), locale);
        terminal
            .draw(|frame| view.render(frame.area(), frame.buffer_mut()))
            .expect("render notice");
        let buf = terminal.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn disclosure_and_both_choices_survive_responsive_sizes() {
        for (width, height) in [(40, 12), (56, 18), (60, 18), (70, 20), (80, 24), (100, 32)] {
            let text = rendered_text(Locale::En, width, height);
            assert!(text.contains("Help improve Codewhale?"), "{width}x{height}");
            assert!(
                text.contains("Yes, keep anonymous counts"),
                "{width}x{height}"
            );
            assert!(text.contains("No, turn off tracking"), "{width}x{height}");
            assert!(text.contains("Enter"), "{width}x{height}");
            if width < 80 {
                for required in [
                    "chat/code/prompts/files/names",
                    "time/result",
                    "feature/error",
                    "90d",
                    "model",
                    "content/credentials",
                    "Schema:",
                    "docs/TELEMETRY.md",
                    "Off:",
                    "codewhale",
                    "config",
                    "set",
                    "telemetry",
                    "false",
                ] {
                    assert!(
                        text.contains(required),
                        "{width}x{height} lost {required}:\n{text}"
                    );
                }
            } else {
                assert!(text.contains("conversations"), "{width}x{height}");
                assert!(
                    text.contains("Full schema, field by field:  docs/TELEMETRY.md"),
                    "{width}x{height} clipped the full schema path:\n{text}"
                );
                assert!(
                    text.contains("codewhale config set telemetry false"),
                    "{width}x{height} clipped the full opt-out command:\n{text}"
                );
            }
        }
    }

    #[test]
    fn full_copy_is_selected_only_when_its_localized_wrapping_fits() {
        let view = TelemetryNoticeView::new(pending(), Locale::En);
        for (width, height) in [(40, 12), (56, 18), (60, 18), (70, 20)] {
            assert!(
                !view.full_notice_fits(Rect::new(0, 0, width, height)),
                "{width}x{height} must use compact disclosure"
            );
        }
        for (width, height) in [(80, 24), (100, 32)] {
            assert!(
                view.full_notice_fits(Rect::new(0, 0, width, height)),
                "{width}x{height} has room for the full disclosure"
            );
        }
    }

    #[test]
    fn every_locale_keeps_both_choices_visible_at_every_supported_size() {
        for locale in Locale::shipped_complete() {
            let keep = tr(*locale, MessageId::TelemetryNoticeChoiceKeep);
            let disable = tr(*locale, MessageId::TelemetryNoticeChoiceDisable);
            for (width, height) in [(40, 12), (56, 18), (60, 18), (70, 20), (80, 24), (100, 32)] {
                let text = rendered_text(*locale, width, height);
                // TestBackend stores the continuation cell of each wide CJK
                // glyph as a space. Remove whitespace from both sides so the
                // assertion detects clipping rather than that storage detail.
                let compact_text = text
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>();
                let compact_keep = keep
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>();
                let compact_disable = disable
                    .chars()
                    .filter(|ch| !ch.is_whitespace())
                    .collect::<String>();
                assert!(
                    compact_text.contains(&compact_keep),
                    "{} {width}x{height} clipped keep choice `{keep}`:\n{text}",
                    locale.tag()
                );
                assert!(
                    compact_text.contains(&compact_disable),
                    "{} {width}x{height} clipped disable choice `{disable}`:\n{text}",
                    locale.tag()
                );
                if (width, height) == (40, 12) || (width, height) == (56, 18) {
                    assert!(
                        compact_text.contains("docs/TELEMETRY.md"),
                        "{} {width}x{height} clipped compact schema path:\n{text}",
                        locale.tag(),
                    );
                    for required in ["codewhale", "config", "set", "telemetry", "false"] {
                        assert!(
                            compact_text.contains(required),
                            "{} {width}x{height} clipped `{required}` from compact opt-out path:\n{text}",
                            locale.tag(),
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_locale_fits_the_complete_compact_disclosure_at_40x12() {
        let mut overflow = Vec::new();
        for locale in Locale::shipped_complete() {
            let rows = Paragraph::new(TelemetryNoticeView::compact_notice_text(*locale))
                .wrap(Wrap { trim: false })
                .line_count(38);
            if rows > 8 {
                overflow.push((locale.tag(), rows));
            }
        }
        assert!(
            overflow.is_empty(),
            "compact disclosure exceeds eight prose rows at 40x12: {overflow:?}"
        );
    }

    #[test]
    fn clicking_a_rendered_choice_commits_that_choice() {
        let mut view = TelemetryNoticeView::new(pending(), Locale::En);
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
        terminal
            .draw(|frame| view.render(frame.area(), frame.buffer_mut()))
            .expect("render notice");
        let disable = view.row_hitboxes.borrow()[1].0;
        assert_eq!(
            decision(view.handle_mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: disable.x,
                row: disable.y,
                modifiers: KeyModifiers::NONE,
            })),
            Some(false)
        );
    }

    #[test]
    fn english_notice_is_the_schema_owned_copy_verbatim() {
        assert_eq!(
            tr(Locale::En, MessageId::TelemetryNoticeHeadline).as_ref(),
            codewhale_telemetry::notice::NOTICE_HEADLINE
        );
        assert_eq!(
            tr(Locale::En, MessageId::TelemetryNoticeBody).as_ref(),
            codewhale_telemetry::notice::NOTICE_BODY
        );
    }

    #[test]
    fn every_complete_locale_has_native_notice_copy() {
        let ids = [
            MessageId::TelemetryNoticeHeadline,
            MessageId::TelemetryNoticeBody,
            MessageId::TelemetryNoticeCompactBody,
            MessageId::TelemetryNoticeChoiceKeep,
            MessageId::TelemetryNoticeChoiceDisable,
            MessageId::TelemetryNoticeActionChoose,
            MessageId::TelemetryNoticeActionConfirm,
            MessageId::TelemetryNoticeActionExit,
            MessageId::TelemetryNoticeReceiptEnabled,
            MessageId::TelemetryNoticeReceiptDisabled,
            MessageId::TelemetryNoticeReceiptEnabledUnsaved,
            MessageId::TelemetryNoticeReceiptDisabledUnsaved,
        ];
        for locale in Locale::shipped_complete() {
            for id in ids {
                let copy = tr(*locale, id);
                assert!(!copy.trim().is_empty(), "{} has empty {id:?}", locale.tag());
                assert_ne!(
                    copy.as_ref(),
                    format!("{id:?}"),
                    "{} lacks {id:?}",
                    locale.tag()
                );
                if *locale != Locale::En {
                    assert_ne!(
                        copy,
                        tr(Locale::En, id),
                        "{} copied English {id:?}",
                        locale.tag()
                    );
                }
            }
        }
    }
}
