//! Tideline topbar — the one-row status surface from the approved screens.
//!
//! This is the translation seam between the approved Tideline reference
//! screens and the live shell. It is a pure, deterministic widget: no `App`,
//! no wall-clock read, no ambient motion. The caller owns facts; this module
//! owns cells.
//!
//! The brand lockup is the `codewhale` wordmark alone, in the sanctioned
//! whale-mark gold. There is no glyph before it by founder decree: the
//! canonical mark is a raster asset with no approved ASCII or block-glyph
//! substitute, and a one-row topbar cannot carry it faithfully. The retired
//! hand-drawn crown glyph is absent from this module.
//!
//! Segment grammar (left → right): brand lockup, then contextual segments as
//! `label value` pairs joined by `│`, then the pinned right side — the
//! context reading and one help hint:
//!
//! ```text
//! codewhale │ mcp-gateway │ ⑂ main │ model deepseek-v4   context 61% ▰▰▰▰▰▰▱▱▱▱  Ctrl+/ help
//! ```
//!
//! There is no clock. A date stamp is not a fact anyone runs an agent to
//! read, and it outranked `model not connected` in the row it shared (the
//! accepted mockups carry none). The context reading is painted here and
//! only here — the merged footer used to print the same percentage a second
//! time from the same snapshot.
//!
//! Shed order as width drops (spec §5b): the meter's bar glyphs first, then
//! the help hint, then contextual segments by
//! [`TopbarSegmentId::shed_priority`] (folder, branch, then the work facts).
//! The brand, the route identity, and the `context NN%` text are the floor
//! and never shed — and the route identity is never truncated to keep a
//! decorative gauge, because the bar only re-states the number beside it.
//! The full working line is 83 cells, so at 80 columns the bar is what
//! yields; it reappears from roughly 90 columns up.
//!
//! Interaction: segment geometry is recorded for parity tests, but only the
//! effective model/route segment and the pinned context meter advertise an
//! action in the live shell. Status-only facts do not brighten on hover or
//! pretend to be controls.
//!
//! Color: semantic ink only ([`ChromeInk`]); no hex, per the status-bar color
//! grammar. ASCII-safe mode substitutes every glyph through
//! [`glyphs::ascii_fallback`].

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::Widget,
};
use unicode_width::UnicodeWidthStr;

use crate::palette::{ChromeInk, UiTheme};
use crate::tui::glyphs;

/// Separator between segments — one cell, dim. Also joins the brand lockup
/// to the first segment.
const SEGMENT_JOIN: &str = " │ ";
/// Gap between the context meter and the help hint.
const HELP_GAP: &str = "  ";
/// Width of the context meter bar (cells of ▰/▱).
const METER_CELLS: usize = 10;
/// The brand lockup: wordmark only, no glyph (founder decree — see the
/// module docs). Pure ASCII, so it never widens under ascii-safe mode.
const WORDMARK: &str = "codewhale";

/// Identity of a topbar segment. Most variants are status facts; the live
/// shell currently registers an action only for [`Self::Model`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopbarSegmentId {
    /// The brand lockup — status-only until a product menu exists.
    Brand,
    /// Workspace folder name.
    Workspace,
    /// Checked-out git branch (`⑂ main`), from the cached status probe.
    Branch,
    /// Current run (work screen).
    Run,
    /// Active pod (work screen).
    Pod,
    /// Whale capacity `n/m` (work screen).
    Whales,
    /// Scheduled automation work `⏱ N scheduled · M running` — the top
    /// strip's work fact. The `AutomationPanelState` projection owns the
    /// count; the topbar only reads it.
    Automation,
    /// Effective model / route — click opens the provider inspector.
    Model,
    /// Settings breadcrumb (settings screen) — click walks up one category.
    /// Not constructed by the main shell yet: the settings screen is a
    /// later Tideline slice (spec §5a).
    #[allow(dead_code)]
    SettingsPath,
}

impl TopbarSegmentId {
    /// Shed priority: higher sheds first as width drops. `0` never sheds.
    /// Segments only start shedding after the meter bar and the help hint
    /// have already gone. The floor is brand + route identity + the
    /// `context NN%` text; among segments the declared order is folder,
    /// then branch, then the work facts, because route identity is the one
    /// fact the user must always be able to read (spec §5b).
    #[must_use]
    pub fn shed_priority(self) -> u8 {
        match self {
            Self::Workspace => 5,
            Self::Branch => 4,
            Self::Whales | Self::Automation => 3,
            Self::Pod => 2,
            Self::Run | Self::SettingsPath => 1,
            Self::Model | Self::Brand => 0,
        }
    }
}

/// One contextual topbar segment.
#[derive(Debug, Clone)]
pub struct TopbarSegment {
    pub id: TopbarSegmentId,
    pub label: String,
    pub value: String,
    pub ink: ChromeInk,
}

impl TopbarSegment {
    #[must_use]
    pub fn new(id: TopbarSegmentId, label: &str, value: impl Into<String>, ink: ChromeInk) -> Self {
        Self {
            id,
            label: label.to_string(),
            value: value.into(),
            ink,
        }
    }

    fn rendered_width(&self) -> usize {
        segment_text(self).width()
    }
}

fn segment_text(segment: &TopbarSegment) -> String {
    if segment.label.is_empty() {
        segment.value.clone()
    } else {
        format!("{} {}", segment.label, segment.value)
    }
}

/// What the caller owes the topbar. Everything is injected so renders are
/// deterministic (golden buffers) and wall-clock keyed by the owner, never
/// frame-count keyed (spec §5e).
pub struct Topbar<'a> {
    pub theme: &'a UiTheme,
    /// The single right-hand key hint, e.g. `Ctrl+/ help`. Empty means the
    /// caller has no hint to advertise. It is the first thing to shed.
    pub help_hint: &'a str,
    /// Context window percentage, 0–100.
    pub context_percent: u8,
    /// Contextual segments in display order.
    pub segments: &'a [TopbarSegment],
    /// Actionable segment under the mouse. Only [`TopbarSegmentId::Model`]
    /// currently advertises hover feedback in the live shell.
    pub hovered: Option<TopbarSegmentId>,
    /// ASCII-safe / NO_COLOR mode: every glyph goes through
    /// [`glyphs::ascii_fallback`].
    pub ascii_safe: bool,
}

impl<'a> Topbar<'a> {
    #[must_use]
    pub fn new(
        theme: &'a UiTheme,
        help_hint: &'a str,
        context_percent: u8,
        segments: &'a [TopbarSegment],
    ) -> Self {
        Self {
            theme,
            help_hint,
            context_percent,
            segments,
            hovered: None,
            ascii_safe: false,
        }
    }

    #[must_use]
    pub fn ascii_safe(mut self, ascii_safe: bool) -> Self {
        self.ascii_safe = ascii_safe;
        self
    }

    #[must_use]
    pub fn hovered(mut self, hovered: Option<TopbarSegmentId>) -> Self {
        self.hovered = hovered;
        self
    }
}

fn ascii_of(glyph: &str) -> String {
    if let Some(fb) = glyphs::ascii_fallback(glyph) {
        return fb.to_string();
    }
    glyph
        .chars()
        .map(|c| {
            glyphs::ascii_fallback(&c.to_string())
                .map(str::to_string)
                .unwrap_or_else(|| c.to_string())
        })
        .collect()
}

fn sym(glyph: &str, ascii_safe: bool) -> String {
    if ascii_safe {
        ascii_of(glyph)
    } else {
        glyph.to_string()
    }
}

fn brand_width() -> usize {
    WORDMARK.width()
}

/// Ink for the meter bar and the percentage. At the 80% cap the whole
/// context reading turns to the error token — it is the one topbar fact
/// that becomes a problem rather than a status.
fn meter_ink_for(pct: u8) -> ChromeInk {
    if pct >= 80 {
        ChromeInk::Failure
    } else {
        ChromeInk::Info
    }
}

/// Ink for the `context ` label. Follows the value into the error token at
/// the cap so the reading reads as one warning, not a gray word beside a
/// red number.
fn context_label_ink_for(pct: u8) -> ChromeInk {
    if pct >= 80 {
        ChromeInk::Failure
    } else {
        ChromeInk::Metadata
    }
}

/// The right block's text at one shed state, used for width arithmetic and
/// mirrored span-for-span by the render.
fn right_text(pct: u8, meter: &str, help: &str, show_bar: bool, show_help: bool) -> String {
    let mut text = format!("context {pct}%");
    if show_bar {
        text.push(' ');
        text.push_str(meter);
    }
    if show_help && !help.is_empty() {
        text.push_str(HELP_GAP);
        text.push_str(help);
    }
    text
}

/// The shed pass's answer: which segments survive at this row width, whether
/// the help hint and meter bar survived, the effective right-block width, and
/// the context reading's own span. Shared by the render and the hitbox
/// computation so the two can never disagree about where the meter
/// painted — the same single-arithmetic discipline the startup stage's
/// `startup_layout` follows.
struct ShedRow<'t> {
    kept: Vec<&'t TopbarSegment>,
    show_bar: bool,
    show_help: bool,
    right_width: usize,
    /// Width of the `context NN% ▰▰▱▱▱` span alone (no help hint).
    context_width: usize,
    meter: String,
}

fn shed_pass<'t>(topbar: &'t Topbar<'_>, area: Rect) -> ShedRow<'t> {
    let ascii = topbar.ascii_safe;
    // Right-side pinned block: `context NN% ▰▰▰▰▰▰▱▱▱▱  Ctrl+/ help`.
    let pct = topbar.context_percent.clamp(0, 100);
    let meter: String = (0..METER_CELLS)
        .map(|i| {
            let filled = (i + 1) * 100 / METER_CELLS <= usize::from(pct);
            sym(if filled { "▰" } else { "▱" }, ascii)
        })
        .collect();
    let help = sym(topbar.help_hint, ascii);

    let brand_w = brand_width();
    let join_w = SEGMENT_JOIN.width();
    let mut kept: Vec<&TopbarSegment> = topbar.segments.iter().collect();
    let total_needed = |segs: &[&TopbarSegment], right: usize| -> usize {
        brand_w
            + if segs.is_empty() { 0 } else { join_w }
            + segs.iter().map(|s| s.rendered_width()).sum::<usize>()
            + if segs.is_empty() {
                0
            } else {
                join_w * (segs.len() - 1)
            }
            + 2
            + right
    };

    // Shed pass, in the declared order: the meter's bar glyphs, then the help
    // hint, then segments by priority. `context NN%`, the brand, and the
    // route identity are the floor; below that the render truncates. The bar
    // goes first on purpose — it encodes the same number printed beside it,
    // so it is the cheapest thing on the row to lose, and no folder, branch,
    // or model name should be cut to keep ten decorative cells.
    let mut show_help = !help.is_empty();
    let mut show_bar = true;
    let mut right_width = right_text(pct, &meter, &help, show_bar, show_help).width();
    while total_needed(&kept, right_width) > area.width as usize {
        if show_bar {
            show_bar = false;
        } else if show_help {
            show_help = false;
        } else if let Some(pos) = kept
            .iter()
            .enumerate()
            .filter(|(_, s)| s.id.shed_priority() > 0)
            .max_by_key(|(_, s)| s.id.shed_priority())
            .map(|(i, _)| i)
        {
            kept.remove(pos);
        } else {
            break;
        }
        right_width = right_text(pct, &meter, &help, show_bar, show_help).width();
    }

    ShedRow {
        kept,
        show_bar,
        show_help,
        right_width,
        context_width: right_text(pct, &meter, &help, show_bar, false).width(),
        meter,
    }
}

/// The pinned context meter's hitbox (spec §6: the meter is the chrome
/// row's one always-present inspector target — `Alt+C`'s mouse route).
/// Covers exactly the painted `context NN% ▰▰▱▱▱` span. `None` when the
/// row is too narrow for that span to have painted whole and clear of the
/// brand lockup: a hitbox never claims cells another element paints (the
/// posture-floor discipline the classic header's meter hitbox carried).
#[must_use]
pub fn context_meter_hitbox(topbar: &Topbar<'_>, area: Rect) -> Option<Rect> {
    if area.width < 1 || area.height < 1 {
        return None;
    }
    let shed = shed_pass(topbar, area);
    let start = usize::from(area.width).saturating_sub(shed.right_width);
    if start <= brand_width() + SEGMENT_JOIN.width()
        || shed.context_width >= usize::from(area.width)
    {
        return None;
    }
    Some(Rect {
        x: area.x + u16::try_from(start).unwrap_or(u16::MAX),
        y: area.y,
        width: u16::try_from(shed.context_width).unwrap_or(area.width),
        height: 1,
    })
}

impl Widget for Topbar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height < 1 || area.width < 1 {
            return;
        }
        let theme = self.theme;
        let ascii = self.ascii_safe;
        let pct = self.context_percent.clamp(0, 100);
        let meter_ink = meter_ink_for(pct);
        let label_ink = context_label_ink_for(pct);
        let ShedRow {
            kept,
            show_bar,
            show_help,
            right_width,
            meter,
            ..
        } = shed_pass(&self, area);

        let mut x = area.x as usize;
        let y = area.y;
        // All positions below are usize and cast at the `set_span` boundary;
        // every write is clamped inside `area` by construction.
        let set = |buf: &mut Buffer, cx: usize, span: &Span<'_>| {
            buf.set_span(cx as u16, y, span, span.content.width() as u16);
        };

        // Brand lockup: the wordmark alone in Attention gold, bold (the
        // whale-mark gold is the one gold that is not chrome, per the token
        // table). No glyph precedes it — founder decree, see module docs.
        set(
            buf,
            x,
            &Span::styled(
                WORDMARK,
                chrome(theme, ChromeInk::Attention).add_modifier(Modifier::BOLD),
            ),
        );
        x += WORDMARK.width();

        // Contextual segments with recorded hitboxes, joined to the brand by
        // the same separator they use between themselves.
        for segment in kept.iter() {
            set(
                buf,
                x,
                &Span::styled(SEGMENT_JOIN, chrome(theme, ChromeInk::MetadataDim)),
            );
            x += SEGMENT_JOIN.width();
            let hovered = segment.id == TopbarSegmentId::Model
                && self.hovered == Some(TopbarSegmentId::Model);
            let mut style = chrome(theme, segment.ink);
            if hovered {
                style = style
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED);
            }
            // label dim, value in the segment's ink (two spans, one hitbox)
            if segment.label.is_empty() {
                set(buf, x, &Span::styled(&segment.value, style));
                x += segment.value.width();
            } else {
                // The label may be a glyph (`⑂`); ascii-safe projects it, and
                // every projection is single-width so the shed arithmetic
                // above stays exact.
                let label = sym(&segment.label, ascii);
                set(
                    buf,
                    x,
                    &Span::styled(label.clone(), chrome(theme, ChromeInk::Metadata)),
                );
                x += label.width() + 1;
                set(buf, x, &Span::styled(&segment.value, style));
                x += segment.value.width();
            }
        }

        // Right pinned block, right-aligned to the area edge.
        let mut sx = (area.x as usize + area.width as usize).saturating_sub(right_width);
        let mut spans: Vec<Span> = Vec::with_capacity(6);
        spans.push(Span::styled("context ", chrome(theme, label_ink)));
        spans.push(Span::styled(format!("{pct}%"), chrome(theme, meter_ink)));
        if show_bar {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(meter.clone(), chrome(theme, meter_ink)));
        }
        if show_help {
            spans.push(Span::raw(HELP_GAP));
            spans.push(Span::styled(
                sym(self.help_hint, ascii),
                chrome(theme, ChromeInk::MetadataHint),
            ));
        }
        for span in &spans {
            set(buf, sx, span);
            sx += span.content.width();
        }
    }
}

fn chrome(theme: &UiTheme, ink: ChromeInk) -> Style {
    crate::palette::grammar::chrome_style(theme, ink)
}

/// Recorded hitboxes for one rendered topbar row. Mirrors the
/// `viewport.last_workflow_cancel_area` storage pattern: render computes the
/// rects, the caller stores them, `mouse_ui` hit-tests against them.
#[derive(Debug, Clone)]
pub struct TopbarHitbox {
    pub id: TopbarSegmentId,
    pub area: Rect,
}

/// Compute the hitbox `Rect` for each kept segment. Must be called with the
/// same inputs as the render so the rects match the painted cells exactly.
/// The brand lockup is included as recorded geometry, though it is status-only
/// in the live shell. The context meter has its own exact hitbox helper.
#[must_use]
pub fn topbar_hitboxes(topbar: &Topbar<'_>, area: Rect) -> Vec<TopbarHitbox> {
    let mut out = Vec::new();
    if area.height < 1 || area.width < 1 {
        return out;
    }
    let brand_w = brand_width();
    if brand_w <= usize::from(area.width) {
        out.push(TopbarHitbox {
            id: TopbarSegmentId::Brand,
            area: Rect {
                x: area.x,
                y: area.y,
                width: brand_w as u16,
                height: 1,
            },
        });
    }
    let mut x = area.x as usize + brand_w;
    let join_w = SEGMENT_JOIN.width();
    let shed = shed_pass(topbar, area);
    let right_start = usize::from(area.x + area.width).saturating_sub(shed.right_width);
    for segment in shed.kept.iter() {
        x += join_w;
        let w = segment.rendered_width();
        if x + w <= right_start && x + w <= usize::from(area.x + area.width) {
            out.push(TopbarHitbox {
                id: segment.id,
                area: Rect {
                    x: x as u16,
                    y: area.y,
                    width: w as u16,
                    height: 1,
                },
            });
        }
        x += w;
    }
    out
}

#[cfg(test)]
mod tests;
