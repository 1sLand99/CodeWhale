# Localization Matrix

Canonical tracking document for every locale Codewhale ships, is actively
building, is planning, or has explicitly deferred.

> **Scope note (2026-07-12):** this matrix covers three surfaces — the TUI
> locale packs (`crates/tui/locales/`), the translated READMEs (repo root),
> and the website (`web/`). The three ship on different cadences, so a
> locale can be **shipped** on one surface and **planned** on another; the
> per-surface tables below are the per-surface truth. The website registry
> is `web/lib/i18n/config.ts` (`ALL_LOCALES`): the locale switcher and route
> generation both derive from it.

Customer-visible copy also follows the [Codewhale voice and terminal
charter](VOICE.md); commands, key names, and glyphs remain code-owned around
localized prose.

Last updated: 2026-07-29 (v0.9.2 wave).
Source-of-truth README: `README.md` (English, post-#3087).

## Status legend

| Status | Meaning |
|--------|---------|
| **shipped** | Live on codewhale.net and/or published as a standalone README, or a TUI pack at exact `en.json` parity |
| **partial** | Shipped but intentionally incomplete; missing scope falls back to English and the partial status is visible |
| **planned** | Explicitly prioritized for the next wave |
| **deferred** | Acknowledged as wanted but not yet scheduled; needs layout QA, bridge support, or community champion |

---

## TUI locale packs

The TUI packs under `crates/tui/locales/` are the largest translation
surface in the repo. `en.json` is the reference; a pack is **complete**
only at exact raw key parity with it, enforced by
`scripts/check-tui-locale-parity.py` (CI) and the parity tests in
`crates/tui/src/localization.rs`. See `crates/tui/locales/AGENTS.md` for the
authoring contract.

| Locale | File | Keys vs `en.json` (1248) | Status | Notes |
|--------|------|--------------------------|--------|-------|
| English | `en.json` | 1248/1248 | **shipped** | Reference pack. |
| Japanese | `ja.json` | 1248/1248 | **shipped** | Complete. |
| Simplified Chinese | `zh-Hans.json` | 1248/1248 | **shipped** | Complete. |
| Traditional Chinese | `zh-Hant.json` | 499/1248 | **partial** | Setup core only; missing keys fall back to English at runtime. Deliberate scope per #4057. |
| Brazilian Portuguese | `pt-BR.json` | 1248/1248 | **shipped** | Complete. |
| Latin American Spanish | `es-419.json` | 1248/1248 | **shipped** | Complete. Note the website tracks `es` — the shipped TUI pack is Latin American Spanish, not `es-ES`. |
| Vietnamese | `vi.json` | 1248/1248 | **shipped** | Complete. |
| Korean | `ko.json` | 1248/1248 | **shipped** | Complete. |
| Catalan | `ca.json` | 1248/1248 | **shipped** | Complete (#4749/#4788). Awaiting native-speaker review. |
| German | `de.json` | 1248/1248 | **shipped** | Complete (#4788). Awaiting native-speaker review. |
| French | `fr.json` | 1248/1248 | **shipped** | Complete (#4788). Awaiting native-speaker review. |
| Indonesian | `id.json` | 1248/1248 | **shipped** | Complete (#4789). Awaiting native-speaker review. |
| Hindi | `hi.json` | 1248/1248 | **shipped** | Complete (#4790). Devanagari shaping spike: `docs/evidence/v092-devanagari-terminal-shaping.md` — code-level guarantees only; terminal visual QA and native review still open. |
| Russian | `ru.json` | 1248/1248 | **shipped** | Complete (#3092). Cyrillic script fixtures guard against mixed-language copy. Awaiting native-speaker review. |
| Ukrainian | `uk.json` | 1248/1248 | **shipped** | Complete (#4791). Cyrillic script fixtures keep it distinct from Russian (no ы/э/ъ; і/ї/є/ґ present). Awaiting native-speaker review. |

## Website locales

The website derives routing, the switcher, sitemap, and hreflang from
`ALL_LOCALES` in `web/lib/i18n/config.ts` — one canonical registry, no
second taxonomy. **partial** locales route and are selectable with a
visible `(partial)` badge in the switcher; their dictionaries
(`web/lib/i18n/dictionaries/<code>/`) cover shared chrome (nav, footer,
switcher) and the home page, held to exact key parity with the English
reference by `npm run check:locales` and `web/lib/i18n/dictionaries.test.ts`.
Everything outside that scope renders the English page copy — a deliberate
fallback, never a dictionary key on screen.

| Locale | Code | Status | Notes |
|--------|------|--------|-------|
| English | `en` | **shipped** | Source text. Every page has an EN route. |
| Simplified Chinese | `zh` | **shipped** | Full parity with EN on all first-class pages (inline en/zh copy). |
| Japanese | `ja` | **partial** | #3091. Chrome + home page localized via dictionary; other page bodies/metadata fall back to English. |
| Vietnamese | `vi` | **partial** | #3091. Same scope as Japanese. |
| Korean | `ko` | **partial** | #3093. Same scope as Japanese. |
| Russian | `ru` | **partial** | #3092. Same scope as Japanese. |
| Ukrainian | `uk` | **partial** | #4791 — shipped alongside Russian, same scope. |
| Spanish | `es` | **partial** | #3093. Same scope as Japanese. |
| Brazilian Portuguese | `pt-BR` | **partial** | #3093. Same scope as Japanese. |
| French | `fr` | **planned** | #4788 — TUI pack shipped in v0.9.2; website next wave. |
| German | `de` | **planned** | #4788 — TUI pack shipped in v0.9.2; website next wave. |
| Catalan | `ca` | **planned** | #4749/#4788 — TUI pack shipped in v0.9.2; website next wave. |
| Indonesian | `id` | **planned** | #4789 — TUI pack shipped in v0.9.2; website next wave. |
| Hindi | `hi` | **planned** | #4790 — TUI pack shipped in v0.9.2; website next wave. |
| Arabic | `ar` | **deferred** | RTL candidate. Deferred until layout/typography QA exists (bidirectional text, mirrored chrome, number formatting). |

Remaining website scope for the partial locales (next wave): per-page body
copy and `generateMetadata` titles/descriptions beyond the home page. The
dictionary layer, routing, hreflang, and switcher already cover them, so
filling in a page is a dictionary edit, not plumbing.

## README locales

| Locale | File | Status | Parity check |
|--------|------|--------|-------------|
| English | `README.md` | **shipped** | Canonical source |
| Simplified Chinese | `README.zh-CN.md` | **shipped** | `scripts/check-readme-translations.py` (stamp + fences + URLs + sections) |
| Japanese | `README.ja-JP.md` | **shipped** | Same |
| Vietnamese | `README.vi.md` | **shipped** | Same |
| Korean | `README.ko-KR.md` | **shipped** | Same |
| Latin American Spanish | `README.es-419.md` | **shipped** | Same |
| Brazilian Portuguese | `README.pt-BR.md` | **shipped** | Same |
| Russian | `README.ru.md` | **shipped** | Same (#3092). Awaiting native-speaker review. |
| Ukrainian | `README.uk.md` | **shipped** | Same (#4791). Awaiting native-speaker review. |
| Indonesian | `README.id.md` | **shipped** | Same (#4789). Awaiting native-speaker review. |

## Drift checks

| Check | Tool | Status |
|-------|------|--------|
| TUI pack key parity with `en.json` (complete packs) | `scripts/check-tui-locale-parity.py` + parity tests in `crates/tui/src/localization.rs` | **Shipped** (CI Lint job) |
| README translations stay in sync with `README.md` | `scripts/check-readme-translations.py` | **Shipped** (CI Lint job) |
| README locale links symmetric | `scripts/check-readme-locales.sh` | **Shipped** (CI Lint job) |
| Website dictionaries cover all routed partial locales | `npm run check:locales` + `web/lib/i18n/dictionaries.test.ts` | **Shipped** (#3091) |
| Accept-Language routes deterministically to all routed locales | `web/lib/i18n/detect.test.ts` (middleware delegates to `lib/i18n/detect.ts`) | **Shipped** (#3091) |
| Locale selector lists all routed locales with partial badges | `web/lib/i18n/config.test.ts` (switcher + router derive from one registry) | **Shipped** (#3091) |
| hreflang alternates cover every routed locale | `web/lib/page-meta.test.ts` | **Shipped** (#3091) |
| Cyrillic packs stay script-pure (no mixed-language copy, ru≠uk) | `cyrillic_packs_have_script_purity_and_no_mixed_language_fixtures` in `crates/tui/src/localization.rs` + `dictionaries.test.ts` | **Shipped** (#3092/#4791) |
| Devanagari grapheme-safe clip/wrap at 40/60/80 columns | `truncate_to_width_never_splits_devanagari_clusters` + width fixtures in `crates/tui/src/localization.rs` | **Shipped** (#4790) |
| Adding a UI locale never changes model-visible prompt bytes | `v092_locales_add_no_prompt_bookends_so_prompt_bytes_stay_stable` in `crates/tui/src/prompts.rs` | **Shipped** (cache-stability contract) |
| No shipped locale renders a missing-message marker | `no_shipped_locale_renders_a_missing_message_marker` in `crates/tui/src/localization.rs` | **Shipped** |

## How to add a locale

A locale is not "added" until all three surfaces below either ship it or
carry an explicit `planned`/`partial`/`deferred` row in this matrix.

### 1. TUI pack

1. Create `crates/tui/locales/<tag>.json` with every key in `en.json`,
   following `crates/tui/locales/AGENTS.md` (placeholders stay literal;
   product terms stay English per pack convention; preserve intentional
   leading/trailing spaces).
2. Add the `Locale` variant plus its `tag`/`translation_target_name`/
   `parse_locale`/`shipped`/`shipped_complete` arms in
   `crates/tui/src/localization.rs`, and the `include_str!` arm in the
   test module.
3. Wire the typed settings schema (`UiLocale` in
   `crates/tui/src/config_ui.rs`) plus the pickers and displays that enumerate
   locales: onboarding language picker
   (`crates/tui/src/tui/onboarding/language.rs` — a test forces every shipped
   locale to be offered), setup-wizard match arms, and the locale display arms
   in the `/config` and changelog commands. Keep the schema/round-trip invariant
   tied to `Locale::shipped()` so these surfaces cannot silently drift.
4. Run `python3 scripts/check-tui-locale-parity.py` and
   `cargo test -p codewhale-tui localization`.
5. If the pack must ship incomplete, declare it partial (see `zh-Hant` /
   #4057): keep it out of `shipped_complete()`, mark it in
   `is_partial_pack()`, and add it to `PARTIAL_PACKS` in
   `scripts/check-tui-locale-parity.py` with a tracking issue.

### 2. README

1. Translate `README.md` into `README.<tag>.md`, preserving structure,
   commands, and the #3087 factual history.
2. Cross-link it from the language line in `README.md` and from the other
   translated READMEs.
3. Restamp per `scripts/check-readme-translations.py`, then run
   `python3 scripts/check-readme-translations.py` and
   `bash scripts/check-readme-locales.sh`.

### 3. Website

1. Add/flip the locale entry in `ALL_LOCALES` in `web/lib/i18n/config.ts` —
   the switcher, routes, middleware, sitemap, and hreflang derive from it,
   so no per-locale switcher edit is needed. Use the `partial` status for
   locales that ship the chrome+home dictionary scope before full page
   parity.
2. Create `web/lib/i18n/dictionaries/<code>/chrome.ts` and `home.ts`
   following the English reference shape (`dictionaries/en/`).
3. Middleware detection needs no change for base tags; region variants and
   base→variant mappings live in `web/lib/i18n/detect.ts`.
4. Run `cd web && npm run check:locales && npm test && npm run build`.

### 4. Matrix

Update the TUI, README, and Website tables above — one row per surface,
with per-surface status.

## Assessments

### Galician (`gl`) and Basque (`eu`) — 2026-07-25, per #4749

Assessed alongside the Catalan pack (#4749 / #4788), which asked whether
Galician and Basque are "similar-value European additions" worth shipping
in the same wave.

**Decision: defer both.** Rationale:

- The case #4788 makes for Catalan is specifically that it "has an
  unusually strong software-localization tradition and an active volunteer
  community" — a review-capacity argument, not a market-size one. That
  argument does not transfer: Galician and Basque have materially smaller
  localization communities, so a pack for either would ship with no
  realistic path to native-speaker review.
- Galician speakers have a workable fallback already: the shipped
  `es-419` pack (and `pt-BR` is lexically close). Basque is a language
  isolate with no fallback proximity — its per-string review cost is the
  highest of the three, and machine-translated Basque is the least
  trustworthy of the three.
- There is no natural "ship together" grouping: the v0.9.2 wave already
  bundles the locales that share acceptance criteria (Latin-script
  fr/de/ca/id, Cyrillic uk, Devanagari hi). gl/eu share only the
  review-capacity constraint, which neither clears.

**Cost/demand evidence behind the decision:** a complete TUI pack is
1,153 keys (~8–12k words) plus an ongoing obligation to retranslate every
changed English string in lockstep — the parity gate makes silent drift a
CI failure, so an unmaintained pack is worse than none. No community
member has requested gl or eu (no issues, no PRs, no translations offered),
while the gl/eu base tags already route cleanly through
`web/middleware.ts` the day a champion appears. We do not ship packs we
cannot get natively reviewed, and we do not advertise unshipped packs.

Revisit when a native-speaker champion appears for either language, or if
Catalan uptake after v0.9.2 suggests demand. Both base tags (`gl`, `eu`)
route through `web/middleware.ts` with no middleware change when that
happens.

## Related issues

- #3091 — Website parity with JA + VI README locales
- #3092 — Russian README + website localization
- #3093 — Korean, Spanish, Brazilian Portuguese next-wave locales
- #3087 — Post-rebrand README source text refresh
- #4057 — `zh-Hant` scoped as a partial TUI pack with English fallback
- #4787 — This matrix's TUI table + the locale-drift CI gates
- #4788 — French, German, Catalan TUI localization
- #4789 — Indonesian localization
- #4790 — Hindi localization + Devanagari terminal-shaping spike
- #4791 — Ukrainian localization alongside Russian
- #4749 — Catalan UI language + Galician/Basque assessment
