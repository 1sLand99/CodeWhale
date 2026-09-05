# Website installation guide — local receipt, 2026-09-04

Branch `fix/site-github-install-guide-20260904` is based on the installer owner's
public candidate `cdad345275809aabe38561ee2b2b17bd4a5806b8`. It preserves that
candidate and the separately owned website redesign branch. No installer,
release fact matrix, credentials, hosted service or release artifact was changed.

The shared EN/ZH getting-started content now leads with the direct installer
for a published GitHub release on macOS/Linux, mentions `codewhale update` for
later updates and retains npm/Cargo as alternatives in the full guide. It
explicitly distinguishes local development builds. The current published-release
record remains v0.9.11; the separately installed local v0.9.12 build is not a
publication receipt and is not substituted into public release facts.

The longer command exposed an intrinsic-width defect in the existing step
grid. Scoped `pre-wrap` and `overflow-wrap: anywhere` keep the complete command
visible without inserting actual line breaks into it or adding another control.
The existing content assertion now compares the first command with the canonical
public install recommendation instead of pinning npm-first copy.

## Verification

- Production Next.js **16.3.3** build with webpack passed.
- Website suite: **46 files, 386 tests passed, 0 failed**.
- Facts check and all **23** documentation topics passed.
- Lint: **0 errors, 2 existing image warnings** in nav/footer.
- Regression negative control: the previous npm-first content fails the exact
  recommendation assertion (**1 failed, 10 skipped**); restored content passes
  (**1 passed, 10 skipped**). No installer/update command was executed.
- Dual-agent Impeccable review: **21/28 applicable points**, no P0/P1, detector
  **0** findings. A and B independently found the narrow command-width issue.
  After polish, CUA measured EN/ZH at **390×844**: grid and pre both **343px**,
  pre scroll/client widths both **341px**, right edge **359px** within the
  **375px** content viewport (15px desktop scrollbar). Command text remained
  exactly `curl -fsSL https://codewhale.net/install.sh | sh` followed by
  `codewhale doctor`. All four command boxes also had equal scroll/client widths
  at the default **1280px** desktop viewport. Localized guide links were keyboard
  reachable with visible focus. The baseline snapshot was read and closed after
  the built polish check; no new score or repeated detector run is claimed.

Runnable preview: `http://127.0.0.1:3135/en/docs/guide` and
`http://127.0.0.1:3135/zh/docs/guide`. Next is served from this worktree with
`node node_modules/next/dist/bin/next start --hostname 127.0.0.1 --port 3135`
inside `web/`. Installed dependencies were reused locally; none were downloaded.

Logs: `/tmp/cwa-app-parity-20260904/site-guide-final-tests.log`,
`site-guide-final-build.log`, `site-guide-final-lint.log`, `site-guide-facts.log`,
`site-guide-docs.log`, `site-guide-negative-control.log`, and
`site-guide-regression.log`. Screenshots: `screenshots/site-guide-en-390.png`,
`site-guide-zh-390.png`, and `site-guide-final-desktop.png` under that directory.
Temporary viewport overrides were reset; reviewers closed their own Guest tabs.

This is a narrow source/copy and local browser receipt. Hosted CI, deployment,
actual public download/update, and customer installation remain unproven. The
complete website and unchanged guide steps were not re-audited in this slice.
