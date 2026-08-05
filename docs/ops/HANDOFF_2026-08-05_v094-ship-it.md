# Handoff — finish and ship Codewhale v0.9.4

**Written:** 2026-08-05 · **Repo:** `https://github.com/Hmbown/CodeWhale`
**Read first:** `AGENTS.md`, then this file. Verify every claim below with a
command before acting on it — some of it will be stale by the time you read it.

Your job is to get v0.9.4 **released**: green CI, merged to `main`, tagged,
GitHub Release, crates published. Stop before npm.

---

## 0. State (verify, do not trust)

```sh
git rev-parse origin/main origin/v094-integration origin/agent/v094-release-train-20260802
git rev-list origin/main..origin/v094-integration --count
gh pr checks 5135 --repo Hmbown/CodeWhale
```

At writing:

| Ref | SHA |
|---|---|
| `main` | `b63e48331` |
| `v094-integration` = `agent/v094-release-train-20260802` = PR #5135 head | `870106e00` |

505 commits ahead of `main`. Work on `v094-integration`; mirror every push to
`agent/v094-release-train-20260802` (it is the PR head):

```sh
git push origin v094-integration
git push origin v094-integration:agent/v094-release-train-20260802
```

**Required checks** (ruleset `protect-main`, 0 required approvals):
`Lint`, `Test (ubuntu-latest)`, `Test (macos-latest)`, `Test (windows-latest)`,
`Version drift`, `npm wrapper smoke (ubuntu-latest)`.
**GitGuardian is NOT required** and does not block the merge.

---

## 1. THE BLOCKERS — three real cross-platform regressions

These pass on `main` and fail on the train. They are the only thing standing
between here and a release. `Test (macos-latest)` and `Test (windows-latest)`
are required checks, so this is a factual blocker, not a policy one.

### 1a. macOS — `paste_matrix_lands_in_the_composer_without_autosubmitting`

`crates/tui/tests/terminal_matrix_qa.rs`. Fails with:

```
Error: large bracketed payload: pasted tail never reached the composer: wait_for timed out after 6s
```

**Reproduces locally on macOS, deterministically (3/3 on a quiet machine).**
Proof it is a regression, run on one machine:

```sh
git worktree add --detach /tmp/cw-main origin/main
cd /tmp/cw-main && cargo test -p codewhale-tui --test terminal_matrix_qa paste_matrix_lands
#   -> ok
cd <train> && cargo test -p codewhale-tui --test terminal_matrix_qa paste_matrix_lands
#   -> FAILED
```

This is the cheapest one to fix because you can reproduce it locally. Bisect it
across the 505 commits (`git bisect` with that single test as the predicate).
Suspects worth checking first, both touch composer/terminal-mode handling:
`30f409596` (composer scroll at draft boundary) and `d794cd122` (keep alternate
scroll off while mouse capture is active, #5223). The failure dump shows
`?1007 off (alternate scroll)`, which is why those two are suspects — but
**bisect, do not guess.**

### 1b. Windows — `commands::groups::config::status::tests::status_report_surfaces_effective_safety_policy`

Fails on `Test (windows-latest)`. Not yet diagnosed.

### 1c. Windows — `runtime_api::tests::start_turn_accepts_dynamic_tools_and_environment_id`

```
has overflowed its stack
process didn't exit successfully: ... (exit code: 0xc00000fd, STATUS_STACK_OVERFLOW)
```

**Fix this one first.** A stack overflow aborts the whole test binary, so it may
be masking further Windows failures behind it — you will not know Windows is
clean until it stops aborting. Windows gives spawned threads a smaller default
stack than Linux/macOS; look for a large future or a big value held across an
`.await` in that runtime-API path.

### The trap that hid all of this

`docs/ops/RELEASE_EVIDENCE_2026-08-04.md` calls the paste failure "proven
pre-existing on base." That is true of the **train's** base and false of
`main` — so it would ship as a regression against the last released line.
**Correct that document as part of this work.** Do not let "pre-existing"
stand without naming which base.

Second trap, which cost real time:

```sh
cargo test --workspace 2>&1 | grep -E 'FAILED'   # exit code is GREP's, always 0
```

A piped `cargo test` reports success even when the suite fails. Read the
`test result:` lines; never trust the exit code through a pipe.

---

## 2. Credit gate — owner has approved a force-push

`Lint` also runs `scripts/check-coauthor-trailers.py` over `origin/main..HEAD`.
It fails on 14 commits. This was invisible until 2026-08-05 because `cargo fmt`
failed earlier in the same job and short-circuited it.

- **13 commits** carry `Co-authored-by: Cursor <cursoragent@cursor.com>`, which
  `AGENTS.md` forbids. Each carries *only* that trailer, so removing it loses no
  human credit. Oldest is `f831f7a46`, newest `b17748137`. List them with:
  ```sh
  git log origin/main..HEAD --format='%H %s' --grep='cursoragent@cursor.com'
  ```
- **`ed00d735fc`** needs `Co-authored-by: Inference1 <68734681+Inference1@users.noreply.github.com>`
  (verified author of harvested PR #5236), and carries
  `Co-authored-by: GuohuanFeng0 <fengfrank0329@gmail.com>` whose GitHub account
  **404s** — `gh api users/GuohuanFeng0` returns Not Found.

**Hunter has explicitly approved force-pushing the train** to fix these, and
wants GuohuanFeng0 credited too via an `.github/AUTHOR_MAP` alias. **Ask Hunter
for GuohuanFeng0's current GitHub login** — do not invent an identity, and do
not drop a real contributor silently.

Blast radius is small: the earliest offender (`ed00d735fc`) is ~32 commits from
HEAD, so ~33 SHAs change. Message-only; trees, authors and dates unchanged.
A `git filter-branch --msg-filter` or `git rebase --exec` over that short range
is enough. Verify before pushing:

```sh
python3 scripts/check-coauthor-trailers.py --range origin/main..HEAD
```

Do **not** add these to `LEGACY_AUTOMATION_TRAILER_EXCEPTIONS`. That list is
documented for commits already immutable on `origin/main`; these are not, and
using it would permanently record bot credit in the release.

---

## 3. GitGuardian — false positive, needs Hunter

`GitGuardian Security Checks` fails with "1 secret uncovered" across the PR's
250 scanned commits. Independently verified as a false positive twice: every
match in the train diff is a synthetic redaction-test fixture
(`sk-live-abcdef0123456789abcdef`, `sk-test0000000000000000`, a fake
`eyJ0000...` JWT). `RELEASE_EVIDENCE_2026-08-04.md` reaches the same conclusion
for commit `918aa8c` — the JWT's signature literally decodes to the word
"signature". No credential files are added anywhere in the train.

GitGuardian scans commit *history*, so a new commit cannot clear a finding in an
earlier one. It must be dismissed in the GitGuardian dashboard, which needs
Hunter's login. **It is not a required check, so it does not block the merge.**

---

## 4. Already done — do not redo

All pushed and verified on `870106e00`:

- `cargo fmt` fixes; **source-structure ratchet** re-baselined and tightened
  (168 large modules, 668406 lines, no "can tighten" remainder). This gate is
  what buildkite runs — buildkite went green at build #1041. **Re-measure it
  last**, after all code changes, or it will catch your own growth.
- **Windows CRLF fix.** `crates/telemetry/tests/golden/v1.json` is
  `include_str!`'d and compared byte-for-byte against
  `serde_json::to_string_pretty` (always LF), but `* text=auto` made it CRLF on
  Windows. `.gitattributes` now covers the whole `include_str!` asset class.
  Confirmed: `golden_payload_v1 ... ok` on Windows. Zero content churn
  (`git add --renormalize .`).
- **#5123 over-reach fixed.** The fail-closed guard keyed on
  `agent_type_explicit`, which `role` also sets, so it rejected every read-only
  Workflow leaf *and* the canonical read-only worker. Final rule: caller wrote
  `type`, it resolves to `Builder` (either spelling), authority is `read_only`.
  Deciding evidence is in the issue transcript — the builder self-BLOCKED, the
  worker beside it ran fine. `type=worker` + read_only and any `role` +
  read_only stay legal.
- Test-isolation fix: `refresh_system_prompt_is_noop_when_unchanged` was the
  only one of 39 env-sensitive tests in its file without `lock_test_env()`.
- `Version drift` fix — `scripts/sync-changelog.sh` regenerates
  `crates/tui/CHANGELOG.md` from the root; run it after **every** CHANGELOG edit.
- CHANGELOG dated `## [0.9.4] - 2026-08-05`; `integrations/verifiers-codewhale/README.md`
  un-pinned from 0.9.1.

Green as of writing: `cargo clippy --workspace --all-targets -- -D warnings`,
`cd web && npm test` (250/250), all 8 budget/policy gates,
`scripts/release/check-versions.sh`, `publish-crates.sh dry-run` (20 crates,
`codewhale-telemetry` correctly ordered before cli/tui), and
`scripts/release/prepare-release.sh 0.9.4` with a **zero-byte diff**.

Fleet/sub-agent product intents are all verified in code: `/fleet` opens
roster/setup, operator pinned as row 0 as the Fleet leader, header shows
`Fleet <name> · <user-global|this folder>`, `fleet_list` demoted to
`/fleet fleets` with no source paths, waiting policy allows independent work,
Ctrl+S still the composer stash.

---

## 5. Release sequence (owner-authorized through cargo; STOP before npm)

1. Fix §1. Re-run `cargo test --workspace` and read the `test result:` lines.
2. Fix §2, force-push both refs.
3. Confirm all six required checks green on PR #5135.
4. Merge PR #5135 with a **merge commit** — not squash. Squashing destroys the
   contributor credit that `check-coauthor-trailers.py` exists to protect.
5. Exact-head proof on `main`:
   ```sh
   git fetch origin main && candidate_sha="$(git rev-parse origin/main)"
   gh workflow run ci.yml --ref main -f expected_sha="$candidate_sha"
   gh workflow run release-candidate.yml --ref main -f expected_sha="$candidate_sha"
   ```
6. Tag `v0.9.4` and push it. `release.yml` fires on `push: tags: ['v*']`, builds
   the matrix, pushes GHCR, creates the GitHub Release from the CHANGELOG via
   `scripts/release/generate-release-body.sh`. Then
   `./scripts/release/verify-release-assets.sh 0.9.4`.
7. Crates, from a clean detached tag worktree:
   ```sh
   git worktree add --detach ../codewhale-release-v0.9.4 v0.9.4
   cd ../codewhale-release-v0.9.4
   ./scripts/release/require-release-tag-checkout.sh 0.9.4
   ./scripts/release/publish-crates.sh publish
   ```
8. **STOP.** npm is Hunter's — it needs an OTP. Hand him exactly:
   ```sh
   cd ../codewhale-release-v0.9.4/npm/codewhale && npm publish --access public
   ```
   `version` and `codewhaleBinaryVersion` are both already `0.9.4`.
9. Website cutover only after packages exist: bump
   `web/data/latest-published-release.json` to 0.9.4, regenerate facts, merge,
   then `gh workflow run web.yml --ref main`. The site does not auto-deploy.

---

## 6. Hard rails

- Force-push is approved **only** for the credit fix on the train branch in §2.
  Never force-push `main`, never rewrite published history, never retag a
  shipped release.
- Do not delete the guardrail files repeatedly misflagged as dead code:
  `tui/src/context_budget.rs`, `tui/src/model_registry.rs`,
  `tui/src/prompt_zones.rs`, `tui/src/tools/remember.rs`, `config/src/route/`.
- `Co-authored-by` trailers are for humans only. Note agent assistance in a
  plain commit body.
- Commit as **WIP** unless you actually reproduced the behavior. Report failures
  with their output. Do not publish without verification.
- Leave `fleet_list.rs` / `fleet_detail.rs` alone — they are deliberate
  secondary surfaces behind `/fleet fleets`, not the primary face.

## 7. Known non-blockers

- 16 open Dependabot alerts, all npm (undici, fast-uri) in the web / vscode /
  telemetry-ingest build trees. The shipped npm wrapper has **zero
  dependencies**, so none reach users. `cargo-audit` and `cargo-deny` pass.
- `main`'s own `Lint` is currently red on a README stamp check — unrelated to
  the credit gate, and it does not affect the PR.
- The v0.9.4 milestone has ~20 open issues. A milestone is not a merge gate.
  Verify and close only #5123, #5034, #5035 (all three confirmed fixed in code
  with passing regression tests); do not mass-close the rest.
