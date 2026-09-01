//! Golden-buffer contract for the Tideline topbar (spec §5c/§6).
//!
//! Each golden is a cell-exact `.txt` dump of the rendered row at one of the
//! four canonical blocker sizes (`views/status_picker.rs::BLOCKER_SIZES`).
//! The goldens are the design contract: exact characters, exact columns.
//!
//! Re-bless after an intentional design change by DELETING the golden file
//! and running:
//!
//! ```sh
//! CODEWHALE_BLESS_GOLDENS=1 ./scripts/dev-test.sh tui topbar
//! ```

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::{Topbar, TopbarSegment, TopbarSegmentId};
use crate::palette::{ChromeInk, UI_THEME, UiTheme};

/// The hint the live shell advertises, from the one binding module that owns
/// it — a fixture string here would let chrome and routing drift apart.
fn help_hint() -> String {
    crate::tui::shell_key_routing::topbar_help_hint()
}

const BLOCKER_SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (120, 32), (160, 40)];

/// Approved startup screen: route identity is absent (not connected), the
/// workspace folder names what the session opened.
fn startup_segments() -> Vec<TopbarSegment> {
    vec![
        TopbarSegment::new(
            TopbarSegmentId::Workspace,
            "",
            "codewhale",
            ChromeInk::Metadata,
        ),
        TopbarSegment::new(
            TopbarSegmentId::Model,
            "model",
            "not connected",
            ChromeInk::Waiting,
        ),
    ]
}

/// Approved work screen: repository, branch, effective model. The repository
/// segment states the forge slug and keeps the folder basename as its shorter
/// form.
fn work_segments() -> Vec<TopbarSegment> {
    vec![
        TopbarSegment::new(
            TopbarSegmentId::Workspace,
            "",
            "Hmbown/CodeWhale",
            ChromeInk::Metadata,
        )
        .short("codewhale"),
        TopbarSegment::new(TopbarSegmentId::Branch, "⑂", "main", ChromeInk::Metadata),
        TopbarSegment::new(
            TopbarSegmentId::Model,
            "model",
            "deepseek-v4",
            ChromeInk::Identity,
        ),
    ]
}

/// Approved settings screen: breadcrumb, folder, effective model.
fn settings_segments() -> Vec<TopbarSegment> {
    vec![
        TopbarSegment::new(
            TopbarSegmentId::SettingsPath,
            "",
            "Settings / Appearance",
            ChromeInk::Identity,
        ),
        TopbarSegment::new(
            TopbarSegmentId::Workspace,
            "",
            "codewhale",
            ChromeInk::Metadata,
        ),
        TopbarSegment::new(
            TopbarSegmentId::Model,
            "",
            "claude-3.5-sonnet",
            ChromeInk::Identity,
        ),
    ]
}

/// The work fixture plus the conditional work facts the live shell adds when
/// a run, a pod, or scheduled automation is live. Used by the shed test: the
/// declared order has to hold with the whole ladder present.
fn crowded_segments() -> Vec<TopbarSegment> {
    let mut segments = work_segments();
    // A slug whose two forms share no substring, so the shed sweep can tell
    // "slug" from "basename" from "segment gone".
    segments[0] = TopbarSegment::new(
        TopbarSegmentId::Workspace,
        "",
        "acme/mcp-gateway",
        ChromeInk::Metadata,
    )
    .short("mcp-gateway");
    segments.insert(
        2,
        TopbarSegment::new(
            TopbarSegmentId::Run,
            "run",
            "release 0.9.12",
            ChromeInk::Info,
        ),
    );
    segments.insert(
        3,
        TopbarSegment::new(TopbarSegmentId::Pod, "pod", "launch pod", ChromeInk::Active),
    );
    segments.insert(
        4,
        TopbarSegment::new(TopbarSegmentId::Whales, "whales", "3/4", ChromeInk::Info),
    );
    segments
}

fn fixtures() -> Vec<(&'static str, Vec<TopbarSegment>, u8)> {
    vec![
        ("startup", startup_segments(), 0),
        ("work", work_segments(), 61),
        ("settings", settings_segments(), 61),
    ]
}

fn render_buffer(
    theme: &UiTheme,
    width: u16,
    segments: &[TopbarSegment],
    pct: u8,
) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(width, 1);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let hint = help_hint();
    terminal
        .draw(|frame| {
            let topbar = Topbar::new(theme, &hint, pct, segments);
            use ratatui::widgets::Widget;
            Widget::render(topbar, frame.area(), frame.buffer_mut());
        })
        .expect("draw");
    terminal.backend().buffer().clone()
}

fn render_row(theme: &UiTheme, width: u16, segments: &[TopbarSegment], pct: u8) -> String {
    render_cells(theme, width, segments, pct).concat()
}

/// Per-cell symbols of one rendered row (the golden dump, before joining).
fn render_cells(theme: &UiTheme, width: u16, segments: &[TopbarSegment], pct: u8) -> Vec<String> {
    render_buffer(theme, width, segments, pct)
        .content()
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect()
}

fn golden_path(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/tui/goldens")
        .join(format!("{name}.txt"))
}

fn bless(name: &str, text: &str) {
    let path = golden_path(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create goldens dir");
    }
    std::fs::write(path, text).expect("write golden");
}

fn golden_text(name: &str) -> Option<String> {
    // Normalize to LF; a Windows checkout can hand us CRLF while `render_row`
    // always terminates with LF. Cell symbols never contain CR.
    std::fs::read_to_string(golden_path(name))
        .ok()
        .map(|text| text.replace("\r\n", "\n"))
}

#[test]
fn topbar_matches_goldens_at_blocker_sizes() {
    for (screen, segments, pct) in fixtures() {
        for (w, h) in BLOCKER_SIZES {
            let name = format!("topbar_{screen}_{w}x{h}");
            let rendered = render_row(&UI_THEME, w, &segments, pct);
            let rendered = format!("{rendered}\n");
            match golden_text(&name) {
                Some(expected) => {
                    assert_eq!(
                        rendered, expected,
                        "topbar golden drift at {name}; re-bless only with an approved design change"
                    );
                }
                None => {
                    if std::env::var("CODEWHALE_BLESS_GOLDENS").is_ok() {
                        bless(&name, &rendered);
                    } else {
                        panic!(
                            "missing golden {name}; run with CODEWHALE_BLESS_GOLDENS=1 to write it"
                        );
                    }
                }
            }
        }
    }
}

/// The row states no time of day. The clock was the widest thing on the row
/// and the least load-bearing; nothing here may quietly bring it back.
#[test]
fn topbar_states_no_clock() {
    for (_, segments, pct) in fixtures() {
        for (w, _h) in BLOCKER_SIZES {
            let row = render_row(&UI_THEME, w, &segments, pct);
            assert!(
                !row.contains(':'),
                "{w}: the topbar carries no clock: {row:?}"
            );
        }
    }
}

/// Declared shed order (spec §5b): the bar glyphs, then the repository slug
/// down to the folder basename, then the help hint, then folder, then
/// branch. `codewhale`, the route identity, and the `context NN%` text are
/// the floor at every width.
#[test]
fn topbar_sheds_bar_then_slug_then_help_then_folder_then_branch() {
    let segments = crowded_segments();
    // The narrowest row that still shows a thing. A thing that sheds earlier
    // needs a wider row to survive, so these strictly decrease down the
    // declared order.
    let narrowest_showing = |needle: &str| -> u16 {
        (24..=180u16)
            .filter(|w| render_row(&UI_THEME, *w, &segments, 61).contains(needle))
            .min()
            .unwrap_or_else(|| panic!("{needle} never painted at any width"))
    };
    let bar = narrowest_showing("▱");
    let slug = narrowest_showing("acme/");
    let help = narrowest_showing("help");
    let folder = narrowest_showing("mcp-gateway");
    let branch = narrowest_showing("⑂ main");
    assert!(
        bar > slug && slug > help && help > folder && folder > branch,
        "shed order drifted: bar {bar}, slug {slug}, help {help}, \
         folder {folder}, branch {branch}"
    );
    // The slug degrades to the basename rather than costing the row a whole
    // segment: the repository is still named at every width the folder
    // survives.
    for width in folder..=180u16 {
        let row = render_row(&UI_THEME, width, &segments, 61);
        assert!(
            row.contains("mcp-gateway"),
            "{width}: the repository stays named: {row:?}"
        );
    }
    // The bar is the first thing to go, so the whole working line — the
    // repository, branch, an untruncated model name, and the help hint — is
    // what 80 columns spend their cells on.
    let row80 = render_row(&UI_THEME, 80, &work_segments(), 61);
    assert!(
        row80.contains("⑂ main") && row80.contains("model deepseek-v4"),
        "80: branch and model outrank the gauge: {row80:?}"
    );
    assert!(row80.contains("help"), "80: the hint survives: {row80:?}");
    assert!(
        !row80.contains('▱'),
        "80: the bar is what yields: {row80:?}"
    );

    for width in 24..=180u16 {
        let row = render_row(&UI_THEME, width, &segments, 61);
        assert!(row.contains("codewhale"), "{width}: brand is the floor");
        assert!(
            row.contains("context 61%"),
            "{width}: the context reading is the floor: {row:?}"
        );
        if width >= 60 {
            assert!(
                row.contains("deepseek-v4"),
                "{width}: route identity never sheds first: {row:?}"
            );
        }
    }
}

/// The bar is a solid 10-cell reading: filled cells are the tenths used.
#[test]
fn topbar_bar_fills_one_cell_per_tenth() {
    for (pct, filled) in [(0u8, 0usize), (61, 6), (80, 8), (100, 10)] {
        let row = render_row(&UI_THEME, 160, &work_segments(), pct);
        assert!(
            row.contains(&format!("context {pct}%")),
            "{pct}: the reading is the number: {row:?}"
        );
        assert_eq!(
            row.matches('▰').count(),
            filled,
            "{pct}% must fill {filled} of 10 cells: {row:?}"
        );
        assert_eq!(
            row.matches('▱').count(),
            10 - filled,
            "{pct}% must leave {} cells open: {row:?}",
            10 - filled
        );
    }
}

/// At the 80% cap the whole context reading turns to the error token — the
/// number, the percent sign, and the bar, so it reads as one warning.
#[test]
fn topbar_context_takes_the_error_token_at_eighty() {
    let segments = work_segments();
    let warn = render_buffer(&UI_THEME, 160, &segments, 83);
    let calm = render_buffer(&UI_THEME, 160, &segments, 61);
    let fg_of = |buf: &ratatui::buffer::Buffer, needle: char| {
        (0..160u16)
            .find(|x| buf[(*x, 0)].symbol() == needle.to_string())
            .map(|x| buf[(x, 0)].fg)
            .expect("row paints the glyph")
    };
    assert_eq!(fg_of(&warn, '%'), UI_THEME.error_fg, "83% is the error ink");
    assert_eq!(fg_of(&warn, '▰'), UI_THEME.error_fg, "the bar warns too");
    assert_ne!(
        fg_of(&calm, '%'),
        UI_THEME.error_fg,
        "61% is a status, not a failure"
    );
    assert_eq!(super::meter_ink_for(83), ChromeInk::Failure);
    assert_eq!(super::meter_ink_for(79), ChromeInk::Info);
    assert_eq!(super::context_label_ink_for(83), ChromeInk::Failure);
    assert_eq!(super::context_label_ink_for(79), ChromeInk::Metadata);
}

/// The repository segment states `owner/name` while the row can afford it,
/// and falls back to the folder basename — never to nothing — when it
/// cannot. A shorter form is taken before the help hint or any segment goes.
#[test]
fn topbar_repository_slug_falls_back_to_the_folder_basename() {
    let segments = work_segments();
    let wide = render_row(&UI_THEME, 120, &segments, 61);
    assert!(
        wide.contains("Hmbown/CodeWhale"),
        "the slug is the repository's name when it fits: {wide:?}"
    );
    let tight = render_row(&UI_THEME, 80, &segments, 61);
    assert!(
        !tight.contains("Hmbown/CodeWhale"),
        "80 cannot afford the slug: {tight:?}"
    );
    assert!(
        tight.matches("codewhale").count() == 2,
        "the basename keeps the slot beside the wordmark: {tight:?}"
    );
    // A "shorter" form that is not shorter is not adopted.
    let no_short = TopbarSegment::new(TopbarSegmentId::Workspace, "", "cw", ChromeInk::Metadata)
        .short("a-much-longer-name");
    assert!(no_short.short.is_none());
}

/// The hint must name a chord that actually opens help in this shell. `F1`
/// is advertised nowhere in chrome because terminals eat it, and bare `?` is
/// composer text; `Ctrl+/` is what `is_help_shortcut` accepts unconditionally.
#[test]
fn topbar_help_hint_names_a_chord_that_opens_help() {
    use crate::tui::shell_key_routing::{HELP_CHROME_CHORD, is_help_shortcut, topbar_help_hint};
    assert_eq!(HELP_CHROME_CHORD, "Ctrl+/");
    assert!(
        is_help_shortcut(&KeyEvent::new(KeyCode::Char('/'), KeyModifiers::CONTROL)),
        "the advertised chord must open help"
    );
    // The chords chrome deliberately does not advertise.
    assert!(
        !is_help_shortcut(&KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE)),
        "bare ? types text, so it must never be the printed hint"
    );
    let hint = topbar_help_hint();
    assert!(!hint.contains("F1"), "terminals eat F1: {hint}");
    let row = render_row(&UI_THEME, 160, &work_segments(), 61);
    assert!(row.ends_with(&hint), "the hint is pinned right: {row:?}");
}

#[test]
fn topbar_hitboxes_match_painted_cells() {
    use super::topbar_hitboxes;
    let segments = startup_segments();
    let hint = help_hint();
    let topbar = Topbar::new(&UI_THEME, &hint, 0, &segments);
    let area = ratatui::layout::Rect::new(0, 0, 160, 1);
    let hitboxes = topbar_hitboxes(&topbar, area);
    assert_eq!(hitboxes.len(), 3, "brand + two segments");
    // Every hitbox lies inside the row and is non-degenerate.
    for hb in &hitboxes {
        assert_eq!(hb.area.y, 0);
        assert_eq!(hb.area.height, 1);
        assert!(hb.area.width > 0);
        assert!(hb.area.x + hb.area.width <= 160);
    }
    // Hitboxes do not overlap.
    let mut sorted = hitboxes.clone();
    sorted.sort_by_key(|hb| hb.area.x);
    for pair in sorted.windows(2) {
        assert!(
            pair[0].area.x + pair[0].area.width <= pair[1].area.x,
            "hitboxes must not overlap"
        );
    }
    // The painted segment text sits inside its recorded hitbox. Slice by
    // cell, not by byte: the row contains multi-byte glyphs (`│`, meter).
    let cells = render_cells(&UI_THEME, 160, &segments, 0);
    for hb in &hitboxes {
        let text: String = (hb.area.x..hb.area.x + hb.area.width)
            .filter_map(|x| cells.get(usize::from(x)))
            .cloned()
            .collect();
        assert!(
            !text.trim().is_empty(),
            "hitbox {:?} covers empty cells",
            hb.id
        );
    }
}

#[test]
fn topbar_hitboxes_follow_the_same_shed_pass_as_paint() {
    let segments = crowded_segments();
    let hint = help_hint();
    for width in [120u16, 80, 60, 44, 30, 20] {
        let topbar = Topbar::new(&UI_THEME, &hint, 61, &segments);
        let hitboxes = super::topbar_hitboxes(&topbar, ratatui::layout::Rect::new(0, 0, width, 1));
        let cells = render_cells(&UI_THEME, width, &segments, 61);
        for hitbox in hitboxes {
            assert!(
                hitbox.area.right() <= width,
                "{width}: hitbox {:?} escapes the row: {:?}",
                hitbox.id,
                hitbox.area
            );
            let text: String = (hitbox.area.x..hitbox.area.right())
                .filter_map(|x| cells.get(usize::from(x)))
                .cloned()
                .collect();
            assert!(
                !text.trim().is_empty(),
                "{width}: hitbox {:?} covers unpainted cells",
                hitbox.id
            );
        }
    }
}

#[test]
fn topbar_ascii_safe_has_no_wide_or_unsupported_glyphs() {
    let segments = work_segments();
    let hint = help_hint();
    let row = {
        let backend = TestBackend::new(160, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let topbar = Topbar::new(&UI_THEME, &hint, 61, &segments).ascii_safe(true);
                use ratatui::widgets::Widget;
                Widget::render(topbar, frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    // The brand lockup is the wordmark alone (founder decree deleted the
    // crown glyph); it is pure ASCII, so ascii-safe mode changes nothing.
    assert!(row.starts_with("codewhale"), "wordmark is the brand: {row}");
    assert!(row.contains('#'), "meter projects to #");
    assert!(!row.contains('▰'), "no block glyphs survive ascii-safe");
    assert!(!row.contains('⑂'), "the branch glyph projects too: {row}");
    for ch in row.chars() {
        assert_eq!(ch.width(), Some(1), "ascii-safe row must be single-width");
    }
}

#[test]
fn topbar_hover_and_narrow_do_not_panic() {
    let segments = work_segments();
    let hint = help_hint();
    // Hover style change must not move cells.
    let plain = render_row(&UI_THEME, 160, &segments, 61);
    let _hovered = {
        let backend = TestBackend::new(160, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| {
                let topbar = Topbar::new(&UI_THEME, &hint, 61, &segments)
                    .hovered(Some(TopbarSegmentId::Model));
                use ratatui::widgets::Widget;
                Widget::render(topbar, frame.area(), frame.buffer_mut());
            })
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    };
    assert_eq!(plain, _hovered, "hover recolors, it does not relayout");
    // Degenerate sizes must not panic.
    for w in [1u16, 2, 10, 20, 40] {
        let _ = render_row(&UI_THEME, w, &segments, 61);
    }
    assert!(!plain.is_empty());
}

#[test]
fn context_meter_hitbox_covers_exactly_the_painted_meter_span() {
    // The meter's mouse route must land on the cells the meter painted —
    // the posture-floor discipline (a hitbox never claims cells another
    // element paints), proven against the buffer itself at row widths that
    // are roomy (nothing sheds), tight (help and segments shed), and too
    // narrow (no hitbox rather than an overlapping one).
    let segments = crowded_segments();
    let hint = help_hint();
    for width in [160u16, 80, 60, 44, 30, 20] {
        let topbar = Topbar::new(&UI_THEME, &hint, 61, &segments);
        let row = render_row(&UI_THEME, width, &segments, 61);
        let area = ratatui::layout::Rect::new(0, 0, width, 1);
        match super::context_meter_hitbox(&topbar, area) {
            Some(hitbox) => {
                let start = usize::from(hitbox.x);
                let covered = row
                    .chars()
                    .skip(start)
                    .take(usize::from(hitbox.width))
                    .collect::<String>();
                assert!(
                    covered.starts_with("context "),
                    "{width} wide: hitbox must start at the meter's first cell: {covered:?}"
                );
                assert!(
                    covered.contains('%'),
                    "{width} wide: hitbox must cover the percentage: {covered:?}"
                );
                assert!(
                    !covered.contains("help"),
                    "{width} wide: hitbox must not reach the help hint: {covered:?}"
                );
            }
            None => {
                // Refused only when even the shed floor cannot fit: brand +
                // join + the `context NN%` text + two cells of gap.
                let floor =
                    super::brand_width() + super::SEGMENT_JOIN.width() + 1 + "context 61%".width();
                assert!(
                    usize::from(width) < floor,
                    "{width} wide: a fittable meter (floor {floor}) must return a hitbox"
                );
            }
        }
    }
}
