---
target_identity: "file:/Volumes/VIXinSSD/CW/worktrees/cw-website-app-alignment-20260904/web/app/[locale]/page.tsx"
target_fingerprint: "sha256:f6b255122ad152248255f391aaa3034bb30f620c279bec26d3d88ef465a51c2f"
target_path: /Volumes/VIXinSSD/CW/worktrees/cw-website-app-alignment-20260904/web/app/[locale]/page.tsx
timestamp: 2026-09-05T03-51-38Z
slug: web-app-locale-page-tsx
closed: true
---
Method: dual-agent (A: critique_design; B: critique_detector).

# Terminal hero scale — 2026-09-04

The founder-provided terminal capture now spans the hero width beneath the
headline and actions. Mobile stacks the same content. Fleet setup is explicitly
optional in EN/ZH. Install wrappers allow shrinking so the command scrolls
inside its own box and the Copy button stays within the page.

Source asset: codewhale-ops/design/website-20260904/founder-tui-15fe-20260904.png,
2760x1494, SHA256 5a762fcee58428b9745710a459b3f0ad2ccadf37406d271473701b5f065de762.
It is a development capture, not evidence of a new published release.

Verification:407 tests passed across47 files; production webpack build785
pages. Final lint0 errors,2 existing logo img warnings; locale/catalog checks
passed. Logs: /tmp/cwa-app-parity-20260904/site-scale-{tests,build,final-lint,final-locales}.log.

Direct in-app browser inspection: EN/ZH at390x844 and1280x900. At390, both
pages report innerWidth390, clientWidth375 and scrollWidth375 (scrollbar uses
15px). Copy bounds EN290.53–347, ZH296.31–347: both contained. Keyboard
focus on EN Copy is visible; command scrolling is independent of page width.
Clipboard submission itself was not exercised. Screenshots under
/tmp/cwa-app-parity-20260904/screenshots/:
site-scale-en-390.png, site-scale-zh-390.png,
site-scale-install-en-390.png, site-scale-en-desktop.png,
site-scale-zh-desktop.png. Temporary viewport override reset and QA tab closed.

Independent review: A27/32, specificity3/4; B zero findings across its scoped
TSX scans. Reports /tmp/cwa-app-parity-20260904/scale-voice-design.md and
scale-voice-detector.md. Reviewers could not complete narrow browser checks;
parent verification above supplies that missing evidence. Optional Fleet
framing and narrow install containment findings are resolved for this slice.

No hosted CI, deployment, provider call, or customer task proof. Default-on
usage migration and moving website usage controls to Privacy remain pending
and are deliberately not claimed by this visual slice.
