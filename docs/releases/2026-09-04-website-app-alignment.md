# Website/app alignment — local integration, 2026-09-04

This isolated branch combines the preserved website redesign at e6bfc48ed
(including 151d6dd9d) with the installer/guide integration at f78919e77.
The original website worktree and its preview processes remain untouched.
The merge preserves the original commit authorship; it is not a deployment.

A bounded ownership check found only the older preview processes with that
worktree as their working directory. Its recorded Claude session identifies
authorship, not an active writer. Work continues in
`/Volumes/VIXinSSD/CW/worktrees/cw-website-app-alignment-20260904`, branch
`feat/website-app-alignment-20260904`.

The merged homepage retains the illustrated paper-to-water treatment, serif
headline, and founder-supplied terminal capture. The capture is labeled as a
0.9.12 development build; its exact binary commit was not recorded. The
published-release record remains 0.9.11. Web sign-in and /rc are distinguished
from the development workbench; desktop is unreleased and cloud computers
remain unavailable. Those source claims do not establish deployed acceptance.

Two integration fixes accompany the merge:

- The final homepage install CTA now uses the shared getting-started command,
  avoiding an npm-first recommendation beside the GitHub-first guide.
- The header uses the exact supplied logo PNG, with the existing wordmark.
  SHA256: cb7e5ee63c22ca2aa7daffd10374f19ace838d8efe35a1bf5f28f750b38402aa.

Local verification uses the website's scoped gate; the runtime repository
has no root npm test/check:web scripts. The 47 website test files pass all
398 tests. Facts, documentation, and locale checks pass; lint reports zero
errors and two existing wordmark image warnings. No dependencies were
installed: this isolated tree links the existing website node_modules.

No auth callback, repository installation, model request, secret operation,
AWS operation, publication, billing, DNS, or external contact was performed.
The new app GitHub setup remains local-contract evidence, not a claim that a
public GitHub Agent trigger has been qualified.

Final polish also makes undecided consent dismissible without granting,
removes first-arrival autofocus, keeps the complete disclosure expandable,
preserves a light reading field behind hero copy, distinguishes the Local
web client in EN/ZH, and gives the docs guide sequential steps. Native
controls preserve keyboard access. Local GT catalog export used no API.

Final production build passed with Next16.3.3/webpack; 398/398 tests and lint
remain green (two existing wordmark warnings). Impeccable used isolated
A/B reviews, then polish; the closed snapshot records scores and limits.

Root inspected the final built EN/ZH home at390px: document client/scroll
width375px, with15px browser scrollbar and no horizontal overflow. Desktop
1280px client/scroll width1265px also matches. The Chinese guide uses one
343px column and preserves the complete GitHub command plus codewhale doctor.
Initial focus remains BODY; Escape removes the preference dialog; reopening
from the footer still reports undecided and focuses Close. No consent grant
or analytics transmission was attempted.

Preview: http://127.0.0.1:3136/en (own process, left running).
Screenshots under /tmp/cwa-app-parity-20260904/screenshots/:
site-aligned-en-390.png, site-aligned-zh-390.png,
site-aligned-guide-zh-390.png, site-aligned-en-desktop.png.
Native screenshot capture can crop the desktop panel; the DOM measurements
above, rather than screenshot edges, establish document containment.
