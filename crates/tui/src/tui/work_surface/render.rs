use std::collections::HashMap;

use ratatui::{
    Frame,
    layout::Rect,
    prelude::Widget,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use unicode_width::UnicodeWidthStr;

use crate::localization::MessageId;
use crate::tui::app::{App, SidebarHoverRow, SidebarHoverSection};
use crate::tui::ui_text::truncate_line_to_width;

use super::model::{
    RailPanel, WorkHitbox, WorkRow, WorkSurfacePlacement, WorkTone, project_visible,
};

const SIDE_RAIL_MIN_HOST_WIDTH: u16 = 72;
const SIDE_RAIL_MIN_CHAT_WIDTH: u16 = 40;

fn effective_placement(configured: WorkSurfacePlacement, host_width: u16) -> WorkSurfacePlacement {
    if configured == WorkSurfacePlacement::Off {
        return WorkSurfacePlacement::Off;
    }
    if host_width < SIDE_RAIL_MIN_HOST_WIDTH {
        WorkSurfacePlacement::Top
    } else {
        configured
    }
}

/// Responsive work-surface height.
///
/// `rail_budget` is the caller's answer to "how many rows can the transcript
/// actually spare this frame" — terminal height minus fixed chrome minus the
/// transcript's own floor. See [`crate::tui::ui::rail_row_budget`]. The rail
/// takes spare rows; it never takes rows the transcript needs.
///
/// Every Top panel auto-fits its content the same way: content rows + optional
/// goal title + the divider, capped by `top_height` and ambient room. A
/// two-item checklist is two rows; eight agents grow to show eight. The only
/// Top title is an active goal — never panel chrome ("Pinned"). Side rails
/// keep a muted panel name because a full-height column needs naming.
pub fn height(app: &mut App, width: u16, terminal_height: u16, rail_budget: u16) -> u16 {
    app.work_surface.effective_placement = effective_placement(app.work_surface.placement, width);
    // Off hides the rail outright: no strip, no side reservation, no stale
    // interaction state.
    if app.work_surface.effective_placement == WorkSurfacePlacement::Off {
        collapse_strip(app);
        return 0;
    }
    // Non-Tasks panels on Top auto-fit like Tasks. Empty projections collapse
    // to zero — an empty panel is not a panel. Side placements reserve via
    // `split_chat` and take no top strip.
    if app.work_surface.panel != RailPanel::Tasks {
        if app.work_surface.effective_placement != WorkSurfacePlacement::Top {
            return 0;
        }
        if !super::panels::panel_has_useful_content(app, app.work_surface.panel) {
            collapse_strip(app);
            return 0;
        }
        let cap = top_cap(app, terminal_height, rail_budget);
        if cap < super::model::TOP_HEIGHT_MIN {
            collapse_strip(app);
            return 0;
        }
        let goal_rows = u16::from(top_goal_title(app).is_some());
        let content_width = usize::from(width.saturating_sub(2).max(1));
        // When the goal is the strip title, omit it from Pinned body rows so
        // height and paint agree.
        let content_rows = super::panels::panel_content_row_count(
            app,
            app.work_surface.panel,
            content_width,
            goal_rows > 0,
        );
        if content_rows == 0 && goal_rows == 0 {
            collapse_strip(app);
            return 0;
        }
        let desired = u16::try_from(content_rows)
            .unwrap_or(u16::MAX)
            .saturating_add(goal_rows)
            .saturating_add(1); // divider
        return desired.clamp(super::model::TOP_HEIGHT_MIN, cap);
    }

    let rows = project_visible(app);
    let goal_rows = u16::from(
        app.work_surface.effective_placement == WorkSurfacePlacement::Top
            && top_goal_title(app).is_some(),
    );
    if rows.is_empty() {
        // A live goal alone still deserves a strip: title + divider.
        if goal_rows == 0 {
            collapse_strip(app);
            app.work_surface.latest_rows.clear();
            app.work_surface.visible_rows = 0;
            app.work_surface.total_rows = 0;
            app.work_surface.scroll_offset = 0;
            return 0;
        }
        if app.work_surface.effective_placement != WorkSurfacePlacement::Top {
            return 0;
        }
        let cap = top_cap(app, terminal_height, rail_budget);
        if cap < super::model::TOP_HEIGHT_MIN {
            collapse_strip(app);
            return 0;
        }
        return (goal_rows.saturating_add(1)).clamp(super::model::TOP_HEIGHT_MIN, cap);
    }
    if app.work_surface.effective_placement != WorkSurfacePlacement::Top {
        return 0;
    }
    // The strip auto-fits its content: the literal selectable list plus the
    // optional goal title, the pinned progress receipt, and the divider row,
    // bounded by `top_cap`.
    let cap = top_cap(app, terminal_height, rail_budget);
    if cap < super::model::TOP_HEIGHT_MIN {
        collapse_strip(app);
        return 0;
    }
    let selectable = rows.iter().filter(|row| row.selectable).count();
    let progress = u16::from(top_todo_progress(app, &rows).is_some());
    let desired = u16::try_from(selectable)
        .unwrap_or(u16::MAX)
        .saturating_add(progress)
        .saturating_add(goal_rows)
        .saturating_add(1);
    desired.clamp(super::model::TOP_HEIGHT_MIN, cap)
}

/// The ceilings the *terminal* imposes, independent of anything the user
/// asked for, smallest wins:
///
/// - half the terminal: proportional restraint, so a tall rail on a short
///   terminal still reads as a strip over a transcript.
/// - `rail_budget`: the rows the transcript can actually spare. This is the
///   only one that knows the transcript has a floor, and it is the one that
///   lets decorative water outrank a panel nobody is watching.
///
/// Kept separate from [`top_cap`] because the collapse cliff must be charged
/// against ambient room alone. Both are monotone non-decreasing in terminal
/// height, which is what keeps the strip from blinking across a resize.
fn ambient_cap(terminal_height: u16, rail_budget: u16) -> u16 {
    terminal_height
        .saturating_div(2)
        .clamp(super::model::TOP_HEIGHT_MIN, super::model::TOP_HEIGHT_MAX)
        .min(rail_budget)
}

/// [`ambient_cap`] plus `top_height` — what the user asked for via
/// drag-resize / settings. This is the ceiling on how *tall* a strip may
/// grow; it is deliberately not the quantity a collapse threshold is
/// compared against.
fn top_cap(app: &App, terminal_height: u16, rail_budget: u16) -> u16 {
    app.work_surface
        .top_height
        .min(ambient_cap(terminal_height, rail_budget))
}

/// Drop the interaction state that only means anything while a strip is on
/// screen. Every path reporting "no strip this frame" must run this: hitboxes
/// outlive the rows they described, so a strip that yielded its rows would
/// still swallow clicks landing on the transcript that replaced it.
fn collapse_strip(app: &mut App) {
    app.work_surface.last_area = None;
    app.work_surface.hitboxes.clear();
    app.work_surface.focused = false;
    app.work_surface.selected = None;
    app.work_surface.opened = None;
    app.work_surface.hovered = None;
    app.work_surface.resizing = false;
    app.work_surface.divider_hovered = false;
}

/// Split the transcript slot for a side rail. Top placement consumes its own
/// vertical row before this point, so it returns the chat area unchanged.
///
/// Placement and auto-fit are orthogonal but share one rule: **empty work is
/// not a rail**. Top expresses that as `height() == 0`. Left/Right express it
/// here — no column is reserved when the selected panel has nothing to say.
/// When there *is* content, the rail takes the full chat height at the
/// configured `side_width` (width is the ceiling, the way `top_height` is the
/// ceiling on Top). Narrow terminals that cannot fit the rail fall back to
/// Top, where height auto-fit takes over.
///
/// `min_chat_width` is the column-axis twin of `height`'s `rail_budget`: the
/// columns the transcript must keep. When the idle ocean is on screen that is
/// the ambient floor, and a rail that cannot fit beside it hides rather than
/// squeezing the water into a strip too narrow to draw.
pub fn split_chat(app: &mut App, area: Rect, min_chat_width: u16) -> (Rect, Option<Rect>) {
    let placement = effective_placement(app.work_surface.placement, area.width);
    app.work_surface.effective_placement = placement;
    if placement == WorkSurfacePlacement::Top || placement == WorkSurfacePlacement::Off {
        return (area, None);
    }
    // Same empty-collapse rule as Top: a panel with nothing to show does not
    // spend columns on a blank (or "No agents") column.
    if !side_rail_has_content(app) {
        return (area, None);
    }

    let min_chat_width = min_chat_width.max(SIDE_RAIL_MIN_CHAT_WIDTH);
    let rail_width = app
        .work_surface
        .side_width
        .clamp(super::model::SIDE_WIDTH_MIN, super::model::SIDE_WIDTH_MAX)
        .min(area.width.saturating_sub(min_chat_width));
    if rail_width < super::model::SIDE_WIDTH_MIN {
        // Too narrow for a side column — fall back to Top. The caller will
        // re-ask height() with effective_placement Top so content auto-fits
        // as a strip instead of vanishing.
        app.work_surface.effective_placement = WorkSurfacePlacement::Top;
        return (area, None);
    }

    let chat_width = area.width.saturating_sub(rail_width);
    match placement {
        WorkSurfacePlacement::Left => (
            Rect {
                x: area.x.saturating_add(rail_width),
                width: chat_width,
                ..area
            },
            Some(Rect {
                width: rail_width,
                ..area
            }),
        ),
        WorkSurfacePlacement::Right => (
            Rect {
                width: chat_width,
                ..area
            },
            Some(Rect {
                x: area.x.saturating_add(chat_width),
                width: rail_width,
                ..area
            }),
        ),
        WorkSurfacePlacement::Top | WorkSurfacePlacement::Off => (area, None),
    }
}

/// Whether a Left/Right rail should reserve columns this frame.
fn side_rail_has_content(app: &mut App) -> bool {
    match app.work_surface.panel {
        RailPanel::Tasks => !project_visible(app).is_empty(),
        panel => super::panels::panel_has_useful_content(app, panel),
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    if area.width == 0 || area.height == 0 {
        app.work_surface.last_area = None;
        return;
    }

    if let Some(previous) = app.work_surface.last_area {
        app.sidebar_hover
            .sections
            .retain(|section| section.content_area != previous);
    }

    let placement = app.work_surface.effective_placement;
    // Off renders no rail; height()/split_chat() never hand us an area for it.
    if placement == WorkSurfacePlacement::Off {
        app.work_surface.last_area = None;
        return;
    }
    let body_area = match placement {
        WorkSurfacePlacement::Top => Rect {
            height: area.height.saturating_sub(1),
            ..area
        },
        WorkSurfacePlacement::Left => Rect {
            width: area.width.saturating_sub(1),
            ..area
        },
        WorkSurfacePlacement::Right => Rect {
            x: area.x.saturating_add(1),
            width: area.width.saturating_sub(1),
            ..area
        },
        WorkSurfacePlacement::Off => unreachable!("off placement returned above"),
    };

    // Non-Tasks panels render as a titled line list and skip the row
    // machinery (hitboxes, selection, todo ordinals) entirely.
    if app.work_surface.panel != RailPanel::Tasks {
        render_panel(frame, area, body_area, app);
        return;
    }

    let mut rows = project_visible(app);
    if placement == WorkSurfacePlacement::Top {
        // The top bar is the literal list: to-dos first, then sub-agents.
        // Section summaries belong to the optional side/detail surface and
        // must not spend scarce transcript rows on generic chrome.
        rows.retain(|row| row.selectable);
    }
    let todo_ordinals = if placement == WorkSurfacePlacement::Top {
        todo_ordinals(&rows)
    } else {
        HashMap::new()
    };
    let ordinal_width = todo_ordinals.len().max(1).to_string().len();
    let goal_title = (placement == WorkSurfacePlacement::Top)
        .then(|| top_goal_title(app))
        .flatten();
    let todo_progress = (placement == WorkSurfacePlacement::Top)
        .then(|| top_todo_progress(app, &rows))
        .flatten();
    // Pin goal title, then progress receipt, above the scrollable rows.
    // At the minimum two-row surface keep one usable content row + divider.
    let goal_height = u16::from(goal_title.is_some() && body_area.height >= 1);
    let progress_height = u16::from(
        todo_progress.is_some() && body_area.height.saturating_sub(goal_height) >= 2,
    );
    let header_height = goal_height.saturating_add(progress_height);
    let list_height = body_area.height.saturating_sub(header_height);
    let body_height = usize::from(list_height);
    let overflow = rows.len() > body_height;
    let inset = u16::from(body_area.width >= 60);
    let rail_width = u16::from(overflow);
    let content_area = Rect {
        x: body_area.x.saturating_add(inset),
        y: body_area.y.saturating_add(header_height),
        width: body_area
            .width
            .saturating_sub(inset.saturating_mul(2))
            .saturating_sub(rail_width),
        height: list_height,
    };

    app.work_surface.visible_rows = body_height;
    app.work_surface.total_rows = rows.len();
    // A redraw may clamp an obsolete offset, but it must not reveal the
    // remembered keyboard selection: doing so undoes mouse-wheel scrolling
    // whenever that selection is above the viewport (#4594).
    app.work_surface.clamp_viewport(&rows);
    let max_offset = rows.len().saturating_sub(body_height.max(1));
    app.work_surface.scroll_offset = app.work_surface.scroll_offset.min(max_offset);

    Block::default()
        .style(Style::default().bg(app.ui_theme.surface_bg))
        .render(area, frame.buffer_mut());

    if let Some((goal_text, goal_style)) = goal_title.filter(|_| goal_height > 0) {
        let goal_text = truncate_line_to_width(&goal_text, usize::from(content_area.width));
        Paragraph::new(Line::from(Span::styled(
            goal_text,
            goal_style.bg(app.ui_theme.surface_bg),
        )))
        .render(
            Rect {
                y: body_area.y,
                height: 1,
                ..content_area
            },
            frame.buffer_mut(),
        );
    }

    if let Some(progress) = todo_progress.filter(|_| progress_height > 0) {
        let progress = truncate_line_to_width(&progress, usize::from(content_area.width));
        Paragraph::new(Line::from(Span::styled(
            progress,
            Style::default()
                .fg(app.ui_theme.accent_primary)
                .bg(app.ui_theme.surface_bg)
                .add_modifier(Modifier::BOLD),
        )))
        .render(
            Rect {
                y: body_area.y.saturating_add(goal_height),
                height: 1,
                ..content_area
            },
            frame.buffer_mut(),
        );
    }

    let start = app.work_surface.scroll_offset;
    let visible = rows
        .iter()
        .skip(start)
        .take(body_height)
        .collect::<Vec<_>>();
    let mut lines = Vec::with_capacity(visible.len());
    let mut hover_rows = Vec::new();
    let mut hitboxes = Vec::new();
    for (visible_index, row) in visible.iter().enumerate() {
        let row_y = content_area.y.saturating_add(visible_index as u16);
        let selected =
            app.work_surface.focused && app.work_surface.selected.as_ref() == Some(&row.id);
        let hovered = app.work_surface.hovered.as_ref() == Some(&row.id);
        let opened = app.work_surface.opened.as_ref() == Some(&row.id);
        let style = row_style(app, row, selected, hovered, opened);
        let compact_owner = if placement == WorkSurfacePlacement::Top {
            todo_ordinals
                .get(&row.id.0)
                .map(|ordinal| format!("{ordinal:>ordinal_width$} · "))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let mark = if opened && row.selectable {
            "▾"
        } else {
            row.mark
        };
        let prefix = if row.tone == WorkTone::Heading {
            format!("{} ", mark)
        } else {
            format!("{compact_owner}{mark} ")
        };
        let detail_candidate = if row.tone != WorkTone::Heading && content_area.width >= 44 {
            format!("  {}", row.detail)
        } else {
            String::new()
        };
        let prefix_width = UnicodeWidthStr::width(prefix.as_str());
        let row_width = usize::from(content_area.width);
        let label_budget = row_width.saturating_sub(prefix_width).max(1);
        let label = truncate_line_to_width(&row.label, label_budget);
        let detail_budget =
            row_width.saturating_sub(prefix_width + UnicodeWidthStr::width(label.as_str()));
        let detail = if detail_budget >= 4 {
            truncate_line_to_width(&detail_candidate, detail_budget)
        } else {
            String::new()
        };
        let detail_width = UnicodeWidthStr::width(detail.as_str());
        let gap = usize::from(content_area.width)
            .saturating_sub(prefix_width + UnicodeWidthStr::width(label.as_str()) + detail_width);
        let display = format!("{prefix}{label}{}{detail}", " ".repeat(gap));
        lines.push(Line::from(Span::styled(display.clone(), style)));

        hitboxes.push(WorkHitbox {
            id: row.id.clone(),
            row_y,
        });

        if row.selectable {
            hover_rows.push(SidebarHoverRow {
                row_y,
                display_text: display,
                full_text: format!("{} · {}", row.label, row.detail),
                detail: Some(row.detail.clone()),
                is_truncated: label != row.label || detail != detail_candidate,
                click_action: row.primary_action.clone(),
                stop_action: None,
                stop_zone_start_col: None,
                stop_zone_end_col: None,
            });
        }
    }

    Paragraph::new(lines).render(content_area, frame.buffer_mut());
    render_divider(frame, area, placement, app);
    if overflow {
        render_scrollbar(
            frame,
            Rect {
                x: body_area.right().saturating_sub(1),
                y: content_area.y,
                width: 1,
                height: content_area.height,
            },
            app.work_surface.scroll_offset,
            body_height,
            rows.len(),
            app,
        );
    }

    app.work_surface.last_area = Some(area);
    app.work_surface.hitboxes = hitboxes;
    app.sidebar_hover.sections.push(SidebarHoverSection {
        content_area,
        lines: visible.iter().map(|row| row.label.clone()).collect(),
        rows: hover_rows,
    });
}

/// Render a non-Tasks rail panel (Agents / Context / Pinned) as a line list
/// in the same body area and with the same divider and scrollbar the Tasks
/// list would use. Row interactivity (hitboxes, selection, click actions)
/// is Tasks-only for now; panels scroll via the shared `scroll_offset`.
fn render_panel(frame: &mut Frame, area: Rect, body_area: Rect, app: &mut App) {
    let panel = app.work_surface.panel;
    let placement = app.work_surface.effective_placement;

    Block::default()
        .style(Style::default().bg(app.ui_theme.surface_bg))
        .render(area, frame.buffer_mut());

    // Title row policy:
    // - Top: only an active goal (`Goal: …`). Never panel chrome ("Pinned").
    // - Left/Right: muted panel name — a full-height column needs naming.
    let goal = (placement == WorkSurfacePlacement::Top)
        .then(|| top_goal_title(app))
        .flatten();
    let side_panel_title = matches!(
        placement,
        WorkSurfacePlacement::Left | WorkSurfacePlacement::Right
    );

    let title_rows = if let Some((goal_text, goal_style)) = goal.as_ref() {
        let goal_text =
            truncate_line_to_width(goal_text, usize::from(body_area.width).max(1));
        Paragraph::new(Line::from(Span::styled(
            goal_text,
            goal_style.bg(app.ui_theme.surface_bg),
        )))
        .render(
            Rect {
                height: 1,
                ..body_area
            },
            frame.buffer_mut(),
        );
        1_u16
    } else if side_panel_title {
        Paragraph::new(Line::from(Span::styled(
            truncate_line_to_width(panel.title(), usize::from(body_area.width).max(1)),
            Style::default()
                .fg(app.ui_theme.text_muted)
                .bg(app.ui_theme.surface_bg),
        )))
        .render(
            Rect {
                height: 1,
                ..body_area
            },
            frame.buffer_mut(),
        );
        1_u16
    } else {
        0
    };

    let content_area = Rect {
        y: body_area.y.saturating_add(title_rows),
        height: body_area.height.saturating_sub(title_rows),
        ..body_area
    };
    let body_height = usize::from(content_area.height);
    let lines = super::panels::panel_lines(
        app,
        panel,
        usize::from(content_area.width),
        body_height.max(1),
        goal.is_some(),
    )
    .unwrap_or_default();

    let max_offset = lines.len().saturating_sub(body_height.max(1));
    app.work_surface.scroll_offset = app.work_surface.scroll_offset.min(max_offset);
    let overflow = lines.len() > body_height;
    let visible: Vec<Line> = lines
        .iter()
        .skip(app.work_surface.scroll_offset)
        .take(body_height)
        .cloned()
        .collect();
    Paragraph::new(visible).render(content_area, frame.buffer_mut());

    render_divider(frame, area, placement, app);
    if overflow {
        render_scrollbar(
            frame,
            Rect {
                x: body_area.right().saturating_sub(1),
                y: content_area.y,
                width: 1,
                height: content_area.height,
            },
            app.work_surface.scroll_offset,
            body_height,
            lines.len(),
            app,
        );
    }

    app.work_surface.last_area = Some(area);
    app.work_surface.visible_rows = body_height;
    app.work_surface.total_rows = lines.len();
    app.work_surface.hitboxes.clear();
    app.work_surface.selected = None;
    app.work_surface.opened = None;
    app.work_surface.hovered = None;
}

/// Active goal as the Top strip's only title. Uses the same
/// paused/active/terminal resolution as the ocean header chip so a goal set
/// via `create_goal` is either visible everywhere or nowhere. Returns
/// `None` when no live goal exists — Top then paints no title row at all.
fn top_goal_title(app: &App) -> Option<(String, Style)> {
    let (objective, paused) = crate::tui::footer_ui::active_goal_chip_state(app)?;
    let flat = objective.trim().replace(['\n', '\r'], " ");
    if flat.is_empty() {
        return None;
    }
    let text = if paused {
        format!("Goal (paused): {flat}")
    } else {
        format!("Goal: {flat}")
    };
    let style = if paused {
        Style::default()
            .fg(app.ui_theme.warning)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(app.ui_theme.status_working)
            .add_modifier(Modifier::BOLD)
    };
    Some((text, style))
}

fn todo_ordinals(rows: &[WorkRow]) -> HashMap<String, usize> {
    rows.iter()
        .filter(|row| row.id.0.starts_with("graph:"))
        .enumerate()
        .map(|(index, row)| (row.id.0.clone(), index.saturating_add(1)))
        .collect()
}

fn top_todo_progress(app: &App, rows: &[WorkRow]) -> Option<String> {
    let todos = rows
        .iter()
        .filter(|row| row.id.0.starts_with("graph:"))
        .collect::<Vec<_>>();
    let total = todos.len();
    if total == 0 {
        return None;
    }
    let completed = todos
        .iter()
        .filter(|row| row.tone == WorkTone::Success)
        .count();
    let remaining = total.saturating_sub(completed);
    let label = format!("{} ·", app.tr(MessageId::SidebarTodoLabel));
    Some(
        app.tr(MessageId::WorkSurfaceTodoProgress)
            .replace("{label}", &label)
            .replace("{completed}", &completed.to_string())
            .replace("{total}", &total.to_string())
            .replace("{remaining}", &remaining.to_string()),
    )
}

fn row_style(app: &App, row: &WorkRow, selected: bool, hovered: bool, opened: bool) -> Style {
    let fg = match row.tone {
        WorkTone::Heading => app.ui_theme.accent_primary,
        WorkTone::Live => app.ui_theme.status_working,
        WorkTone::Attention => app.ui_theme.error_fg,
        WorkTone::Success => app.ui_theme.success,
        WorkTone::Muted => app.ui_theme.text_muted,
    };
    let mut style = Style::default().fg(fg).bg(app.ui_theme.surface_bg);
    if row.tone == WorkTone::Heading {
        style = style.add_modifier(Modifier::BOLD);
    }
    if !row.selectable {
        return style;
    }
    if opened {
        style = style
            .fg(app.ui_theme.accent_primary)
            .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
    }
    if selected {
        style = style
            .bg(app.ui_theme.selection_bg)
            .add_modifier(Modifier::BOLD);
    } else if hovered {
        style = style.bg(app.ui_theme.elevated_bg);
    }
    style
}

fn render_divider(frame: &mut Frame, area: Rect, placement: WorkSurfacePlacement, app: &App) {
    let active = app.work_surface.resizing || app.work_surface.divider_hovered;
    let color = if active {
        app.ui_theme.accent_primary
    } else {
        app.ui_theme.border
    };
    match placement {
        WorkSurfacePlacement::Off => {}
        WorkSurfacePlacement::Top => {
            let y = area.bottom().saturating_sub(1);
            for x in area.left()..area.right() {
                frame.buffer_mut()[(x, y)]
                    .set_symbol(if active { "━" } else { "─" })
                    .set_fg(color)
                    .set_bg(app.ui_theme.surface_bg);
            }
        }
        WorkSurfacePlacement::Left | WorkSurfacePlacement::Right => {
            let x = if placement == WorkSurfacePlacement::Left {
                area.right().saturating_sub(1)
            } else {
                area.left()
            };
            for y in area.top()..area.bottom() {
                frame.buffer_mut()[(x, y)]
                    .set_symbol(if active { "┃" } else { "│" })
                    .set_fg(color)
                    .set_bg(app.ui_theme.surface_bg);
            }
        }
    }
}

fn render_scrollbar(
    frame: &mut Frame,
    area: Rect,
    offset: usize,
    visible: usize,
    total: usize,
    app: &App,
) {
    let rail_height = area.height;
    if rail_height == 0 || total == 0 {
        return;
    }
    let thumb_height = ((usize::from(rail_height) * visible) / total)
        .max(1)
        .min(usize::from(rail_height));
    let max_offset = total.saturating_sub(visible).max(1);
    let max_start = usize::from(rail_height).saturating_sub(thumb_height);
    let thumb_start = offset.saturating_mul(max_start) / max_offset;
    let x = area.right().saturating_sub(1);
    for row in 0..usize::from(rail_height) {
        let in_thumb = row >= thumb_start && row < thumb_start.saturating_add(thumb_height);
        frame.buffer_mut()[(x, area.y.saturating_add(row as u16))]
            // Match the transcript rail exactly: a fine border track with a
            // brighter, narrow thumb. The old solid block looked like a
            // separate native scrollbar bolted onto the work surface.
            .set_symbol(if in_thumb { "┃" } else { "│" })
            .set_fg(if in_thumb {
                app.ui_theme.status_working
            } else {
                app.ui_theme.border
            })
            .set_bg(app.ui_theme.surface_bg);
    }
}
