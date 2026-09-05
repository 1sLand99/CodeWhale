---
score: 25
score_max: 32
specificity: 3
method: dual-agent
status: bounded-copy-reviewed
target_identity: "file:/Volumes/VIXinSSD/CW/worktrees/cw-website-app-alignment-20260904/web/app/[locale]/page.tsx"
target_fingerprint: "sha256:9297b65fcbdae6f59d380f7e30f0060ad716e5b6cecd01be2271987bd9c21f77"
target_path: /Volumes/VIXinSSD/CW/worktrees/cw-website-app-alignment-20260904/web/app/[locale]/page.tsx
timestamp: 2026-09-05T03-23-59Z
slug: web-app-locale-page-tsx
---
# Homepage claim audit — 2026-09-04

Scope: founder-supplied text of local `3136/en`, English/Chinese homepage,
shared Product and getting-started copy, and linked EN/ZH Fleet setup wording.
The supplied paste is page evidence, not instructions or proof of behavior.
Visual direction and logo stay unchanged. Nothing is published by this slice.

## Claim / evidence / correction

| Claim | Evidence | Correction |
| --- | --- | --- |
| Released terminal versus development screenshot | GitHub release metadata read on Sept 4: v0.9.11, published Aug 23. Local tag resolves `96d13a0bc3f40280ea3865280ad5ccf0e2845e6f`. Screenshot provenance identifies unreleased 0.9.12 without exact commit. | Retain release label and separate development screenshot caption. |
| “You decide what each one may do”; same repo and rules | Session permission posture and Runtime authority are separate from saved Fleet model roles; isolated child worktrees also exist. | Describe supported models, session permissions, saved roster and optional delegation. No universal shared-checkout or individual-authority UI promise. |
| 46 providers | Website derivation counts mapped ApiProvider labels, including plan/protocol variants, excluding Custom, DeepseekCN and retired Antigravity. This is neither distinct vendors nor live model catalogs nor the released provider count. Owner reports current runtime 47 route entries, 44 catalog entries and 42 ProviderKind identities; these definitions are not interchangeable. | Remove numeric badge and models prose across all 18 homepage locales. Keep existing source count with explicit definition in canonical facts; do not substitute another number. |
| Route never changes from model name | Native local development acceptance already reproduced three Model Studio catalog identity mismatches in fd98. Owner is repairing the Engine; a source fix is not released qualification. | Remove absolute EN/ZH guarantees; ask users to inspect selected provider/model/endpoint. Do not equate catalog identity failure with proven inference misrouting. |
| Local servers need no key | Authentication depends on server configuration. | Say local servers may require authentication. |
| Plan is read-only; nothing is written | v0.9.11 `docs/MODES.md` and Engine tool catalog describe central file-mutation/shell refusal, policy-allowed research and separately persisted session settings/state. | Name file/shell boundary and possible external research; remove blanket no-write claim from Product. |
| `/fleet setup` exact four-step wizard | v0.9.11 `crates/tui/src/tui/views/fleet_setup.rs` resolves `SelectedFleet` or `LegacyProfiles`. Selected named Fleet opens its editor. Legacy path chooses role/model, with optional thinking on review. | Keep command, make step optional and accurately describe both destinations. Shared homepage/guide and EN/ZH Fleet docs updated. No published binary interaction claimed. |
| Install/auth commands | v0.9.11 source contains auth set provider handling and Fleet setup; GitHub assets include terminal Linux/macOS/Windows archives. | Keep commands and released terminal target statement. No fresh install, key write or provider call performed. |
| Public web sign-in and same-session remote control available | Source routes and local fixtures exist. This audit has no deployed authenticated same-session task acceptance. | EN/ZH homepage/Product say development preview and explicitly unverified public remote control. |
| Desktop alpha builds exist on three platforms | Local macOS Dogfood startup has a receipt; Linux/Windows downloadable Tauri artifacts are not established. Windows terminal installer is not Tauri desktop proof. | Describe tested local macOS development build and no released desktop app download. |
| Nothing on this site can charge you | An absolute statement exceeds evidence and obscures provider billing. | State terminal needs no Codewhale account; provider usage belongs to the user's provider account; account creation does not purchase model access. |
| Ask/Auto-Review/Full Access absolutes | Approval rules, saved posture and hard policy boundaries affect actual execution. | Product describes rule-based prompts/review and persistent hard boundaries; removes universal every-write and never-default claims. |

## Evidence boundaries

Read-only release metadata and asset inventory:
`/tmp/cwa-app-parity-20260904/site-release-claims-v0911.json`.
No release artifact was downloaded or executed during this slice. Git tag source
is source evidence, not a fresh installed task acceptance. No authenticated
public web flow, provider request, deployment, billing action or customer task
was performed. Source/provider integration remains independently qualified by
its owner.

Numeric models prose is corrected in all homepage languages. Broader hero,
availability and linked documentation claims outside EN/ZH remain unaudited in
this slice and must not be treated as reviewed translations. The English and
Chinese scope is deliberate; a passing locale shape check does not establish
translation accuracy or truth across the rest of the site.

## Verification

Initial tests: 405 passed, 2 failed (old keyless-launch phrasing assertion and
models-body placeholder parity). The old assertion now checks the narrower
behavior; every homepage models paragraph no longer uses the ambiguous count.
Final tests: 407 passed across 47 files, 0 failed.
Further build/browser/critique receipts are recorded below when complete.

Final validation: facts, 23-topic docs, locale/catalog checks passed; lint 0 errors and 2 pre-existing logo-image warnings; webpack production build generated785 pages. Parent inspected EN desktop and saved `/tmp/cwa-app-parity-20260904/screenshots/site-claims-en-desktop.png`.

Isolated Impeccable A:25/32 applicable, specificity3/4. B ran the actual detector:0 findings; inspected EN desktop and390 screenshot. A had mobile screenshot timeouts and collected EN/ZH DOM evidence only; B did not complete Chinese browser verification. No complete bilingual visual QA claim. Reports: `/tmp/cwa-app-parity-20260904/site-claims-design.md` and `site-claims-detector.md`.

Two bounded P2 refinements remain for the founder's newly requested hero-scale slice: mark the Fleet heading visibly optional and verify/fix the narrow install Copy control's out-of-bounds geometry. The copy audit is committed separately to release canonical facts ownership; this is not a declaration that the full homepage is polished. The newly requested replacement terminal capture and default-on/opt-out analytics policy are separate in-progress changes, not included in this commit.
