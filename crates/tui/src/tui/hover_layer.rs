//! Frame-scoped hover registry for transcript / diff / tool surfaces.
//!
//! Collects hit targets during render, resolves the pointer once, and applies
//! restrained aura / copy / link glow. Reuses [`super::hover_hit`] primitives
//! and context-menu hover-follow patterns without growing `ui.rs`.

use std::cell::RefCell;
use std::sync::Mutex;
use std::time::Instant;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use unicode_width::UnicodeWidthStr;

use crate::palette;
use crate::tui::hover_hit::{
    HoverHit, HoverTargetKind, copy_affordance, hit_test, link_hover_style,
};

/// Pointer position from the last mouse move (column, row).
static POINTER: Mutex<Option<(u16, u16)>> = Mutex::new(None);

#[cfg(test)]
pub static HOVER_TEST_LOCK: Mutex<()> = Mutex::new(());

// Targets registered for the current frame (thread-local for render path).
thread_local! {
    static FRAME_TARGETS: RefCell<Vec<HoverHit>> = const { RefCell::new(Vec::new()) };
    static FRAME_HOVER: RefCell<Option<HoverHit>> = const { RefCell::new(None) };
    static FRAME_START: RefCell<Option<Instant>> = const { RefCell::new(None) };
}

/// Clear targets at the start of a draw.
pub fn begin_frame() {
    FRAME_TARGETS.with(|t| t.borrow_mut().clear());
    FRAME_HOVER.with(|h| *h.borrow_mut() = None);
    FRAME_START.with(|s| *s.borrow_mut() = Some(Instant::now()));
}

/// Record an interactive region for hit-testing this frame.
pub fn register(hit: HoverHit) {
    FRAME_TARGETS.with(|t| t.borrow_mut().push(hit));
}

#[cfg(test)]
pub fn registered_targets() -> Vec<HoverHit> {
    FRAME_TARGETS.with(|targets| targets.borrow().clone())
}

/// Convenience: register a rectangular target.
pub fn register_rect(kind: HoverTargetKind, area: Rect, label: impl Into<String>, copyable: bool) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    register(HoverHit {
        kind,
        area,
        label: label.into(),
        copyable,
    });
}

/// Update the shared pointer from mouse motion (call from mouse_ui).
pub fn set_pointer(column: u16, row: u16) {
    if let Ok(mut guard) = POINTER.lock() {
        *guard = Some((column, row));
    }
}

/// Clear the process-wide pointer between tests.
#[cfg(test)]
pub fn clear_pointer() {
    if let Ok(mut guard) = POINTER.lock() {
        *guard = None;
    }
}

/// Resolve hover after targets are registered; call once near end of draw.
pub fn resolve_hover() {
    let pointer = POINTER.lock().ok().and_then(|g| *g);
    let Some((col, row)) = pointer else {
        FRAME_HOVER.with(|h| *h.borrow_mut() = None);
        return;
    };
    FRAME_TARGETS.with(|t| {
        let targets = t.borrow();
        let hit = hit_test(col, row, &targets).cloned();
        FRAME_HOVER.with(|h| *h.borrow_mut() = hit);
    });
}

/// Current hover hit, if any.
#[must_use]
pub fn current_hover() -> Option<HoverHit> {
    FRAME_HOVER.with(|h| h.borrow().clone())
}

/// Elapsed ms since frame begin for pulse math.
fn elapsed_ms() -> u128 {
    FRAME_START
        .with(|s| s.borrow().map(|t| t.elapsed().as_millis()))
        .unwrap_or(0)
}

/// Paint OSC-8 / file-ref underline glow on a hovered link span row.
pub fn paint_link_glow(
    buf: &mut Buffer,
    area: Rect,
    fg: ratatui::style::Color,
    reduced_motion: bool,
) {
    let ms = elapsed_ms();
    let style = link_hover_style(fg, reduced_motion, ms);
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            if x >= buf.area.x.saturating_add(buf.area.width)
                || y >= buf.area.y.saturating_add(buf.area.height)
            {
                continue;
            }
            let cell = &mut buf[(x, y)];
            if let Some(color) = style.fg {
                cell.set_fg(color);
            }
            cell.modifier.insert(Modifier::UNDERLINED);
        }
    }
}

/// Modifier-only hover mark for clickable controls and rows (Slice G:
/// buttons, rows, tabs, chips, hotbar slots, toggles). Adds underline +
/// bold to every cell in `area` while preserving each cell's fg/bg, so the
/// hovered control keeps its own treatment (primary, danger, tinted row)
/// and never masquerades as keyboard selection. Bounds-checked against
/// `buf` like [`paint_link_glow`].
pub fn paint_control_hover(buf: &mut Buffer, area: Rect) {
    for y in area.y..area.y.saturating_add(area.height) {
        for x in area.x..area.x.saturating_add(area.width) {
            if x >= buf.area.x.saturating_add(buf.area.width)
                || y >= buf.area.y.saturating_add(buf.area.height)
            {
                continue;
            }
            let cell = &mut buf[(x, y)];
            cell.modifier.insert(Modifier::UNDERLINED | Modifier::BOLD);
        }
    }
}

/// Apply all hover effects for the resolved target onto `buf`.
pub fn apply_resolved_effects(buf: &mut Buffer, reduced_motion: bool, theme: &palette::UiTheme) {
    resolve_hover();
    let Some(hit) = current_hover() else {
        return;
    };
    match hit.kind {
        HoverTargetKind::Link => {
            paint_link_glow(buf, hit.area, theme.accent_primary, reduced_motion);
            // Hover-only copy chip on the trailing edge of copyable targets.
            if hit.copyable && hit.area.width > 8 {
                let chip = copy_affordance();
                let chip_w = UnicodeWidthStr::width(chip) as u16;
                if chip_w < hit.area.width {
                    let x = hit
                        .area
                        .x
                        .saturating_add(hit.area.width.saturating_sub(chip_w + 1));
                    let y = hit.area.y;
                    for (i, ch) in chip.chars().enumerate() {
                        let cx = x.saturating_add(i as u16);
                        if cx >= buf.area.x.saturating_add(buf.area.width) {
                            break;
                        }
                        let cell = &mut buf[(cx, y)];
                        cell.set_symbol(&ch.to_string());
                        cell.set_fg(theme.text_hint);
                        cell.modifier.insert(Modifier::DIM);
                    }
                }
            }
        }
        HoverTargetKind::TruncatedText => {
            paint_link_glow(buf, hit.area, theme.accent_primary, true);
            paint_full_text_popover(buf, &hit, theme);
        }
        HoverTargetKind::Button
        | HoverTargetKind::Row
        | HoverTargetKind::Tab
        | HoverTargetKind::Chip
        | HoverTargetKind::HotbarSlot
        | HoverTargetKind::Toggle => {
            paint_control_hover(buf, hit.area);
        }
    }
}

fn full_text_popover_area(hit: &HoverHit, bounds: Rect) -> Option<Rect> {
    if bounds.width < 4 || bounds.height < 3 || hit.label.is_empty() {
        return None;
    }

    let widest_line = hit
        .label
        .lines()
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(1);
    let max_width = bounds.width.min(72);
    let width = u16::try_from(widest_line.saturating_add(2))
        .unwrap_or(u16::MAX)
        .clamp(4, max_width);
    let content_width = width.saturating_sub(2).max(1);
    let paragraph = Paragraph::new(hit.label.as_str()).wrap(Wrap { trim: false });
    let content_height = u16::try_from(paragraph.line_count(content_width))
        .unwrap_or(u16::MAX)
        .max(1);
    let height = content_height.saturating_add(2).min(bounds.height);

    let rightmost_x = bounds.x.saturating_add(bounds.width).saturating_sub(width);
    let x = hit.area.x.clamp(bounds.x, rightmost_x);
    let bounds_bottom = bounds.y.saturating_add(bounds.height);
    let below_y = hit.area.y.saturating_add(hit.area.height);
    let y = if below_y.saturating_add(height) <= bounds_bottom {
        below_y
    } else {
        hit.area.y.saturating_sub(height).max(bounds.y)
    };
    Some(Rect::new(x, y, width, height))
}

fn paint_full_text_popover(buf: &mut Buffer, hit: &HoverHit, theme: &palette::UiTheme) {
    let Some(area) = full_text_popover_area(hit, buf.area) else {
        return;
    };
    Clear.render(area, buf);
    Paragraph::new(hit.label.as_str())
        .style(Style::default().fg(theme.text_body).bg(theme.elevated_bg))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.accent_primary))
                .style(Style::default().bg(theme.elevated_bg)),
        )
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_resolve_hit() {
        let _guard = HOVER_TEST_LOCK.lock().unwrap();
        clear_pointer();
        begin_frame();
        set_pointer(5, 2);
        register_rect(
            HoverTargetKind::Link,
            Rect::new(0, 2, 20, 1),
            "fn main",
            true,
        );
        resolve_hover();
        let hit = current_hover().expect("hover");
        assert_eq!(hit.kind, HoverTargetKind::Link);
        assert!(hit.copyable);
        clear_pointer();
    }

    #[test]
    fn control_kinds_mark_hovered_cells_and_keep_unhovered_clean() {
        use ratatui::style::{Color, Modifier};
        let _guard = HOVER_TEST_LOCK.lock().unwrap();
        for kind in [
            HoverTargetKind::Button,
            HoverTargetKind::Row,
            HoverTargetKind::Tab,
            HoverTargetKind::Chip,
            HoverTargetKind::HotbarSlot,
            HoverTargetKind::Toggle,
        ] {
            let area = Rect::new(2, 1, 10, 1);
            let mut plain = Buffer::empty(Rect::new(0, 0, 20, 4));
            for x in 2..12 {
                plain[(x, 1)].set_fg(Color::Yellow);
            }
            let mut hovered = plain.clone();
            clear_pointer();
            begin_frame();
            set_pointer(5, 1);
            register_rect(kind, area, "control", false);
            apply_resolved_effects(&mut hovered, true, &palette::UI_THEME);
            for x in 2..12 {
                assert!(
                    hovered[(x, 1)].modifier.contains(Modifier::UNDERLINED),
                    "{kind:?} cell {x} needs underline feedback"
                );
                assert!(
                    hovered[(x, 1)].modifier.contains(Modifier::BOLD),
                    "{kind:?} cell {x} needs bold feedback"
                );
                assert_eq!(
                    hovered[(x, 1)].fg,
                    plain[(x, 1)].fg,
                    "{kind:?} must preserve the control's own ink"
                );
                assert_eq!(
                    plain[(x, 1)].modifier & (Modifier::UNDERLINED | Modifier::BOLD),
                    Modifier::empty(),
                    "{kind:?} unhovered baseline must stay clean"
                );
            }
            // Cells outside the target stay untouched.
            assert_eq!(hovered[(0, 0)].symbol(), plain[(0, 0)].symbol());
            assert_eq!(hovered[(0, 0)].modifier, plain[(0, 0)].modifier);
            assert_eq!(hovered[(0, 0)].fg, plain[(0, 0)].fg);
            clear_pointer();
        }
    }

    #[test]
    fn truncated_text_popover_wraps_and_stays_inside_bottom_edge() {
        let hit = HoverHit {
            kind: HoverTargetKind::TruncatedText,
            area: Rect::new(8, 8, 22, 1),
            label: "完整的中文说明 keeps the full underlying copy".into(),
            copyable: false,
        };
        let bounds = Rect::new(0, 0, 32, 10);
        let area = full_text_popover_area(&hit, bounds).expect("popover");
        assert!(
            area.y < hit.area.y,
            "bottom row should place above: {area:?}"
        );
        assert!(area.right() <= bounds.right());
        assert!(area.bottom() <= bounds.bottom());

        let mut buf = Buffer::empty(bounds);
        paint_full_text_popover(&mut buf, &hit, &palette::UI_THEME);
        let rendered = buf
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        for glyph in ['完', '整', '中', '文', '说', '明'] {
            assert!(rendered.contains(glyph), "missing {glyph:?}: {rendered:?}");
        }
        assert!(
            rendered.contains("underlying copy"),
            "rendered: {rendered:?}"
        );
    }
}
