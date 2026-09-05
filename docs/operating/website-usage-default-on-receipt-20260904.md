# Website usage counting: default-on, control on the privacy page — 2026-09-04

Founder direction: usage analytics on by default unless disabled; controls and
explanations live primarily in the app and in Codewhale itself; the website
keeps privacy details and its own opt-out on the privacy page and does not
make usage statistics a marketing topic.

What changed on the website:

- The foot-of-viewport consent sheet is gone. Counting (page views, docs
  views, install copies, downloads as plain totals) is on by default; the
  only stored state is the person's own choice under the historical
  `cw-usage-consent` key, so an opt-out recorded under the earlier opt-in
  policy still counts as off. Unreadable stored state fails closed.
- The privacy page (`/legal/privacy#usage-counting`) hosts the status line
  and the Turn off / Turn on control; the footer "Usage data" link points
  there. Nothing records an acceptance on the visitor's behalf.
- The envelope is schema 3 / notice 5 (`notice_version` replaces
  `consent_version`), matching the runtime and the ingest's v3 contract.
- Trust page, FAQ, roadmap, and `docs/public-surface-facts.json` now say
  that 0.9.12 counts by default and discloses it, and that the published
  0.9.11 release asked first. The privacy policy gains an "Anonymous usage
  counting" section and its effective date moves to September 4, 2026.
- Screenshot provenance: the hero capture is the founder's PNG
  (2760x1494, SHA-256 `5a762fce…ecd0`) whose header reads
  `v0.9.12 (15fe6983bfa5)`; facts now record that version and commit. The
  README image `assets/screenshot.webp` is a lossless 1136x615 downscale of
  the same capture. Both show a local dogfood build, not a published
  release; Full Access is the capture's posture, not a default.

Verification on this tree (local, no deployment, no provider call):

- `vitest run`: 407 passed / 0 failed across 47 files.
- `tsc --noEmit`: clean. `eslint .`: 0 errors, 2 pre-existing `<img>` warnings.
- `check:locales` (incl. GT catalog check), `check:facts`, `check:docs`: PASS.
- `next build --webpack` after a clean `.next`: success. A first build in a
  `.next` shared with the running dev server produced an `en.html` that
  referenced chunks from another build; the clean rebuild removed that and
  is what was served for the browser check.
- Browser check against the production build on 127.0.0.1:3137, Chromium,
  EN and ZH at 390x844 and 1280x900: no consent sheet on the home page; the
  home visit counts one `page_view` with no preference record written; the
  privacy control shows "on (default)", Turn off writes
  `{version:5,granted:false}` and clears counters and install id, Turn on
  writes `granted:true`; footer link present; no horizontal overflow
  (scrollWidth == clientWidth); no console errors. The page's one
  `POST /api/product-telemetry` reached only the same-origin route, which
  answers `disabled` without `CODEWHALE_TELEMETRY_INGEST_URL`.
  Receipt: `scratch/takeover-20260904/website-default-on/privacy-browser-qa.json`
  and screenshots beside it.

Not proven here: hosted deployment, live ingest or PostHog delivery, and
non-EN/ZH claim review of the remaining README translations.
