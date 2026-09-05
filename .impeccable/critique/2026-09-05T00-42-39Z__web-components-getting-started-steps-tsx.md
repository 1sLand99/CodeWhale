---
target: Website install step
total_score: 21
max_score: 28
na_heuristics: 1,3,9
p0_count: 0
p1_count: 0
target_identity: "file:/Volumes/VIXinSSD/CW/worktrees/cw-site-github-install-guide-20260904/web/components/getting-started-steps.tsx"
target_fingerprint: "sha256:f5180f0101eebebe501c90cb467dccca62da8c3ae726a8f1ca59bc2ab5cbe50f"
target_path: /Volumes/VIXinSSD/CW/worktrees/cw-site-github-install-guide-20260904/web/components/getting-started-steps.tsx
timestamp: 2026-09-05T00-42-39Z
slug: web-components-getting-started-steps-tsx
closed: true
---
Method: dual-agent (A: /root/critique_design · B: /root/critique_detector)

# Website install step — independent Assessment A

Scope: only the changed install step in `web/lib/content/getting-started.ts`, rendered by `web/components/getting-started-steps.tsx`. Read the workspace, repository, and `web/AGENTS.md` guidance. No context script, detector, installation, or external request was run. No detector findings were seen. No source files were edited.

## Verdict

The copy achieves the intended hierarchy: published GitHub release on macOS/Linux first; `codewhale update` for later releases; npm/Cargo available through the full guide; local development builds explicitly separate. English and Chinese convey the same four facts. No critical copy defect found. This is a successful narrow correction, not an assessment of the complete guide or website.

The wording is concrete and restrained. It belongs to a technical product and does not need a broader visual redesign. The shared content source preserves consistency between its consumers and avoids a page-specific localization fork.

## Actual observations

- Opened the built `/en/docs/guide` and `/zh/docs/guide` in a fresh native CUA Chrome Guest tab. Both displayed the changed paragraph, installer command, `codewhale doctor`, and descriptive localized Full install guide link.
- The complete installer command is present in the accessibility text in both locales.
- In the inspected desktop layout, the new long command extends beyond the narrow first-column code box and requires horizontal scrolling. The visible first line stops partway through the URL; the full release-installer destination and pipe are not visible together. This is contained code-block overflow, not established page-level overflow or lost text.
- Source inspection confirms `<pre><code>` semantics, `overflow-x: auto` and `white-space: pre`. The renderer offers no copy control. Full keyboard scrolling, text selection, screen-reader announcements, and contrast were not tested.
- Local installer source selects GitHub release assets and handles Darwin/Linux; local docs show this same installer and the later updater. These are source receipts, not proof of current public downloads or successful installation.

## Priority issue

**P2 — The longer primary install command is awkward to inspect and copy at the rendered desktop width.** The new default command makes the existing horizontal-scroll presentation more consequential: a first-time reader cannot see the complete command in one glance. Make a narrowly scoped improvement for this install command: permit soft wrapping without changing its text, or reuse an existing accessible copy control. If keeping horizontal scrolling, verify keyboard reachability and full-command selection. Do not expand this into a redesign of the four-step grid or the untouched steps.

No P0/P1 issue found within this copy slice. The release/update/alternative/local-build wording needs no additional prerequisite text or warning banner.

## Applicable heuristic scores

| Heuristic | Score | Scope-specific basis |
|---|---:|---|
| System status | n/a | Static instruction; no installation was performed. |
| Match to real world | 4 | Clear platform, release provenance, later-update instruction. |
| User control/freedom | n/a | No stateful operation in this slice. |
| Consistency | 4 | Shared EN/ZH content, matching local installer/docs. |
| Error prevention | 3 | Published release and local development build are distinguished. |
| Recognition over recall | 2 | Complete command requires horizontal inspection. |
| Flexibility/efficiency | 2 | Long command has no direct copy action; keyboard scrolling unverified. |
| Aesthetic/minimalism | 3 | Short, focused paragraph; one primary route with alternatives deferred. |
| Error recovery | n/a | Installer outcome/recovery is outside the reviewed copy. |
| Help/documentation | 3 | Clear localized link to deeper installation guidance. |
| **Total** | **21/28** | **Good; narrow command usability improvement remains.** |

Cognitive load is low for the changed decision: one recommended installation path, one validation command, and one deeper-reading link. Alternatives are named without presenting competing command menus. The long command adds interaction effort rather than conceptual ambiguity.

## Evidence limits

The CUA browser provider was unavailable; native CUA Chrome supplied a fresh Guest tab. EN/ZH desktop screenshots were observed without changing viewport/window size; output image size was 768×930, exact CSS viewport unmeasured. The Guest tab was closed, then browser ownership was returned to Assessment B. No mobile inspection, homepage rendering, external GitHub/install URL validation, install/update execution, broad website audit, or test suite was performed by Assessment A. Untouched steps and full-guide content were not assessed.

Questions skipped: the user already authorized implementation.

## Independent responsive evidence

B ran the narrow markup detector once: exit 0, zero findings. Native Chrome at 390px confirmed EN/ZH command-container overflow beyond the grid, and keyboard Tab reached the localized Full install guide link with a visible outline. Root independently measured the same intrinsic-width problem in CUA: a 343px grid allocated a 382.6px track, extending the command box beyond the viewport. This strengthens A’s scoped P2 without changing its provisional 21/28 score. No P0/P1 established. Both assessments were isolated, and A completed before B findings reached synthesis. No mutable overlay was available; native screenshots and read-only DOM geometry supplied the evidence. No installer or updater was executed.
