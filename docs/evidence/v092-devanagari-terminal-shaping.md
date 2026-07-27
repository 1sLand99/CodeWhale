# Devanagari terminal-shaping spike (#4790, v0.9.2)

Status: **spike complete — code-level guarantees only. No native-speaker or
per-terminal visual signoff has been performed or is claimed here.**

## Question

Can the TUI render Hindi (Devanagari) UI copy without corrupting conjunct
clusters when strings are clipped or wrapped at narrow terminal widths
(40/60/80 columns)?

## What Devanagari needs

Devanagari renders through complex text shaping: a consonant + virama
(U+094D) + consonant forms a conjunct glyph (क + ् + ष → क्ष), vowel signs
reorder around their base (क + ि → कि, the ि renders *before* the consonant),
and nukta forms (क़) combine a base with U+093C. Cutting a string between a
base and its virama, or between a base and a combining sign, leaves a
dangling halant or an orphaned mark — visibly broken copy.

## Findings

1. **Truncation was char-based and could split clusters.** The old
   `truncate_to_width` iterated `chars()`; a width budget landing between
   क and ् emitted a trailing virama. It now iterates extended grapheme
   clusters (`unicode-segmentation`, already a workspace dependency) and
   measures each cluster with `unicode-width`, so a cluster is kept whole
   or dropped whole. Covered by
   `truncate_to_width_never_splits_devanagari_clusters` (budgets
   1/2/3/5/7/40/60/80: no U+FFFD, no trailing virama/ZWJ/combining mark)
   and `cyrillic_latin_extended_and_devanagari_rows_wrap_within_terminal_columns`
   in `crates/tui/src/localization.rs`.
2. **Wrapping is word-based and safe.** ratatui's `Paragraph::wrap` breaks
   on whitespace/punctuation, not inside words, so conjuncts inside a word
   are never split by wrapping. Verified at 40/60/80 columns with a Hindi
   fixture row (same test module).
3. **Width is the honest weak point.** `unicode-width` reports Devanagari
   base letters as width 1 and combining marks as width 0, which matches
   what a correctly shaping terminal displays *most of the time*. Some
   conjuncts render narrower than their cluster sum on shaping terminals
   and wider on non-shaping ones; the budget logic errs on the side of
   clipping early, never overdrawing the row.
4. **The test suite cannot see pixels.** These tests assert codepoint- and
   cell-level invariants in ratatui's buffer model. They do not prove any
   real terminal shaped the text correctly.

## Terminal support matrix (informed assessment, not tested on hardware)

| Terminal | Devanagari shaping expectation |
|----------|-------------------------------|
| WezTerm, Kitty (recent), foot | HarfBuzz/pango-class shaping; conjuncts render correctly |
| Windows Terminal (recent) | Shaping via DirectWrite; generally correct |
| GNOME Terminal / VTE, Konsole | Correct via Pango/Qt |
| macOS Terminal.app, iTerm2 | CoreText shaping; generally correct |
| Alacritty | **No complex text shaping** — conjuncts render as base+visible halant; readable but wrong |
| tmux/screen | Pass-through cells; inherits the outer terminal's behavior, but cluster-aware cursor math is limited |
| Linux VT (fbcon), older conhost | No shaping; expect broken conjuncts |
| SSH into any of the above | Inherits the *local* terminal's shaping |

**Recommendation:** the Hindi pack ships with this caveat documented;
users on Alacritty or the Linux console will see un-shaped conjuncts.
This is a terminal capability limit, not something the TUI can fix from
the cell grid. Native-speaker review of the pack copy and visual QA on at
least one shaping terminal (VTE-class or WezTerm) remain open follow-ups
before the pack should be called fully signed off.

## What was NOT verified

- No native Hindi speaker has reviewed the pack.
- No physical terminal rendering was inspected (screenshot QA).
- `unicode-width` conjunct widths vs. real glyph advances on shaping
  terminals (known approximation, see finding 3).
