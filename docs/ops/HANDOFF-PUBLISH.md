# Handoff: publishing CodeWhale

You are taking over a release. Everything below is verified state, not guesswork —
trust it and don't redo the forensics. Repo: `/Volumes/VIXinSSD/CW/codewhale`,
branch `main`, origin `github.com/Hmbown/CodeWhale`.

## READ THIS FIRST — the decision that gates everything

**`main` is 22 commits ahead of `origin/main`, and it is no longer v0.9.4.**

- `6b7eb20ef` is the commit the v0.9.4 CHANGELOG section describes and the only
  commit anyone attempted to validate in CI.
- `main` is now `6a38edca7`, carrying 22 further commits recorded under
  `## [Unreleased]` — Agent Plugins, sub-agent billing attribution, a VS Code
  fix, SSE retry hardening, website accessibility and motion, and two harvested
  community PRs.
- The `## [0.9.4] - 2026-08-05` CHANGELOG section is **dated and closed**.

So `git tag v0.9.4 main` would ship a binary containing features its own release
notes never mention. Pick one deliberately before doing anything else:

1. **Tag v0.9.4 at `6b7eb20ef`.** The release matches its notes and its (attempted)
   validation. The 22 commits become 0.9.5. Tags name commits, not branches:
   `git tag v0.9.4 6b7eb20ef`.
2. **Ship everything as 0.9.5.** Bump the workspace + npm version, move the
   `[Unreleased]` block into a dated `[0.9.5]` section, re-run
   `./scripts/release/check-versions.sh --require-dated-release`, and validate the
   whole thing in CI.
3. **Fold the 22 commits into 0.9.4.** Requires rewriting a dated section and
   re-validating. Least honest of the three; listed for completeness.

**Ask the maintainer (Hunter) which one.** Do not choose for him. He has said
explicitly, more than once, *do not release 0.9.4* and *keep everything offline* —
those instructions stand until he lifts them.

## Current state

- Local `main` = `6a38edca7`, **not pushed**. `origin/main` = `4d89fdac4`.
- No `v0.9.4` tag exists locally or on origin. Nothing published to crates.io or npm.
- Release binaries built from this tree: `target/release/codewhale-tui`,
  `target/release/codewhale`.

### Verified locally at this HEAD
`cargo test --workspace --all-features` (9,941 pass) · `cargo fmt --all --check` ·
release-parity clippy invocation · `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace
--no-deps` · `scripts/check-source-structure-budget.py` (673375) ·
`check-versions.sh --require-dated-release` · `publish-crates.sh dry-run` ·
web gates (`prebuild`, `check:facts`, `check:docs`, 250 tests, eslint, build).

### NOT verified anywhere
**No CI run has ever seen any of these 22 commits.** GitHub Actions was in a
major outage for the entire session; every attempted run died with ~34-line logs
reading `Failed to resolve action download info`. Actions recovered near the end
of the session but nothing was dispatched.

In particular, **`RUST_MIN_STACK=16777216` is unproven**. It is a well-reasoned fix
for a confirmed root cause (production runs the engine at 16 MiB via
`CODEWHALE_MAIN_STACK_BYTES`; `#[tokio::test]` built its own runtime at ~2 MiB,
~1 MiB on Windows, which aborted the whole Windows test binary with
`STATUS_STACK_OVERFLOW` in
`runtime_api::tests::start_turn_accepts_dynamic_tools_and_environment_id` and
masked every other Windows result). It has never had a Windows lane complete
against it. Treat it as very likely correct, unproven.

## Blockers you must clear before publishing

1. **`npm audit` in `web/` reports 1 high-severity `js-yaml` advisory.** The
   release contract requires 0 across root/web/telemetry-ingest/extensions/vscode.
   Not introduced by this work — a new advisory since the baseline. Bump via npm;
   no Dependabot dismissals.
2. **Contributor credit is CI-enforced and currently owed.** Two community PRs
   were harvested: #5254 (@mky, FreeBSD build fix) and #5252 (@cacdcaecawae,
   embedder-owned sub-agent state roots). Both need `.github/AUTHOR_MAP` entries
   and CHANGELOG credit. `scripts/check-coauthor-trailers.py` rejects bot/tool
   trailers — human contributors only.
3. **npm auth is dead.** `npm whoami` returns 401. `npm login` is interactive, so
   the maintainer must do it. `cargo owner --list codewhale-tui` authenticates
   fine as `Hmbown`.

## The publish sequence (only after the above)

1. Dispatch CI and get a genuinely green run on the exact commit you intend to tag:
   `gh workflow run ci.yml --repo Hmbown/CodeWhale --ref main -f expected_sha=<sha>`
   Dispatch (not push) also forces the Ubuntu Test lane to actually run tests —
   on push it skips them, which is how seven failures hid for a whole release train.
2. **Verify the Windows Test lane's "Run tests" STEP conclusion is `success`, not
   just the job's.** A skipped step inside a green job burned an entire prior
   session. `gh api repos/Hmbown/CodeWhale/commits/<sha>/check-runs` for the job
   list; read step conclusions individually.
3. Tag and push. `release.yml` runs on `v*`. Its `parity` job runs
   `cargo test --workspace --all-features` on **ubuntu-latest** — that gate is why
   the no-sandbox test failures had to be fixed before any tag could succeed.
4. Verify `gh release view <tag>` and the CNB mirror:
   `git ls-remote https://cnb.cool/codewhale.net/codewhale.git refs/tags/<tag>`.
   CNB sync runs inside Actions using a secret token; you cannot push it locally.
5. From a **clean detached checkout of the tag**: `./scripts/release/publish-crates.sh
   dry-run`, then `publish` (19 crates, dependency order). It gates on
   `require-release-tag-checkout.sh` and `verify-release-assets.sh` — the latter
   requires a real GitHub Release and a successful Release workflow run for that
   SHA, so crates cannot be published ahead of the release. Do not bypass it.
6. `npm publish --workspace npm/codewhale --access public`; verify
   `npm view codewhale@<version> version`.
7. Cleanup: delete throwaway branches `bisect/premerge-clippy`,
   `bisect/win-after-5077`, `bisect/win-after-5240`, `bisect/win-debug` (local +
   origin), the merged `codex/memory-refine`, `codex/visibility-tips`,
   `codex/rlm-bindings`, `codex/agent-plugins`, `codex/website-polish`, and the
   worktrees at `/Volumes/VIXinSSD/CW/wt-agent-plugins` and `wt-website`.

## Approval-gated — never do these unprompted

Production credentials, billing, DNS, deploys, customer data or comms, publishing
(tags, releases, crates, npm), and pushes to origin on the release lane. The
maintainer's standing instruction at handoff time is **do not release**; treat
that as binding until he says otherwise.

## Things that cost real time this session — don't repeat them

- **A green job can contain a skipped step.** Check step conclusions.
- **Push runs skip the Ubuntu test lane.** Only `workflow_dispatch` and `schedule`
  run it. Seven test failures hid there for an entire release train.
- **`Failed to resolve action download info` in a ~34-line log is GitHub, not you.**
  Re-run those; never debug them. When `Change detection` is cancelled, downstream
  jobs show `skipped`, and `gh run rerun --failed` will NOT restore them — dispatch
  a fresh run instead.
- **The repo has more built than is findable.** Three times this session a
  capability was assumed missing and already existed: the Continual Harness
  (`continual_harness.rs`), `muse-spark-1.2` (shipped in the catalog and the
  configured default), and model-assisted constitution drafting
  (`tui/setup/model_draft.rs`). **Grep before you build.**
- **Delegated agents are working sets, not facts.** A DeepSeek sweep this session
  made a 401 authentication failure retryable — caught only because its brief
  forbade it and the repo already had `anthropic_stream_open_error_is_not_retried`
  defending that boundary. Read what agents produce; run the gates yourself.

## What landed in these 22 commits, briefly

Memory gained `revise`/`retire` with required evidence and a journal; the
continual harness gained an audit trail (removal previously left no record at
all); a durable-state tip in all 15 complete locale packs; `muse`/`muse-spark`
registry drift to 1.1 fixed; a configured `memory_path` no longer nests a second
store inside itself; the `Fast` loadout no longer silently re-prices child agents
onto a cheaper sibling; sub-agent model attribution now appears in
`exec --output-format stream-json`; SSE header stalls are typed retryable instead
of killing the turn; Agent Plugins v1.0.0 consume/publish/slugify; a VS Code
extension that no longer reports "Connected" to a runtime that will 401 every
call; website accessibility (a real WCAG AA contrast failure), structured data,
and motion with reduced-motion paths throughout.
