# Handoff — recover `/fleet` UI/UX, then finish the v0.9.4 release

**Written:** 2026-08-04 · **For:** a cloud agent with a fresh checkout
**Repo:** `https://github.com/Hmbown/CodeWhale`
**Read first:** `AGENTS.md` (the contract), then `docs/ops/CURRENT.md` (lane state).

Everything referenced here is pushed to GitHub. No local-only state is required.

---

## 0. Orientation — where the code actually is

| Branch | Tip | What it holds |
|---|---|---|
| `main` | `b63e48331` | Last shipped line. v0.9.3 is the last published release (2026-07-31). |
| `v094-integration` | `abcd7e27b` | **The release train.** ~463 commits ahead of `main`. Includes the new website wave. Mirrored to `agent/v094-release-train-20260802`, which is the head of open PR **#5135** (train → `main`). |
| `codex/v094-fleet-rebuild` | `dfaa6bb20` | **The Fleet rebuild — 16 commits, NOT merged into the train.** Contains both the work we want and the UI we are rejecting. |

Two facts that matter before you touch anything:

1. **The bad `/fleet` UI is quarantined.** It exists only on `codex/v094-fleet-rebuild`. The release train does **not** contain it. Your job is not to revert a merge — it is to harvest the good parts of that branch without bringing its `/fleet` surface along.
2. **Workspace version is already `0.9.4`** (Cargo/npm/runtime-sdk/vscode/facts all pinned). Nothing is published yet. `web/data/latest-published-release.json` still says `0.9.3` — that is **correct** until real packages exist.

---

## 1. TASK A (primary) — recover the old `/fleet` UI/UX

### What went wrong

The 0.9.4 Fleet rebuild was driven by a design contract at
`docs/decisions/2026-08-04-fleet-rebuild.md`. The implementing agents followed it
**too literally**: they turned the primary `/fleet` surface into a
file-manager-shaped list-and-detail editor. The owner's verdict on seeing it:
*"the nuked new fleet thing is awful."*

The offending change is commit `371846f5c9`
(*feat(tui): Fleet manager — list/detail/select/rename/delete, no shadow pile*),
which:

- Repointed the bare `/fleet` command from `AppAction::OpenFleetRoster` to
  `AppAction::OpenFleetList`
  (`crates/tui/src/commands/groups/core/fleet.rs`).
- Added `crates/tui/src/tui/views/fleet_list.rs` (~811 lines) — one row per saved
  Fleet showing display name, `[user]`/`[folder]` scope badge, and **source file
  path**, with keys `u`/`w` select, `d` delete, `m` migrate.
- Added `crates/tui/src/tui/views/fleet_detail.rs` (~1056 lines) — operator/member
  row editor with keys `o`/`e` route picker, `t` reasoning cycle, `v` vision
  toggle, `a` add role, `d` remove, `r` rename, `s` save, `c` copy scope, `u`/`w`
  select.
- Demoted the previous surface to `/fleet roster`.

The result reads as a filesystem browser with a keybinding legend, not as a way to
set up a fleet. **The old surface — the `/fleet` setup wizard
(`crates/tui/src/tui/views/fleet_setup.rs`) plus the roster
(`crates/tui/src/tui/views/fleet_roster.rs`) — is the UX the owner wants back.**
Both files still exist unchanged on `v094-integration`; you do not have to
reconstruct them from history.

### What to do

**Keep the engine, restore the face.** The rebuild's *data model and routing
logic* are genuinely good work and should survive. Its *presentation layer* should
not become the primary surface.

Harvest from `codex/v094-fleet-rebuild` (cherry-pick or re-implement on top of
`v094-integration`):

| Keep | Commit | Why |
|---|---|---|
| Named Fleet store v2 (`crates/tui/src/fleet/store.rs`) | `9782105bc6` | Self-contained TOML Fleets, scope-explicit selection, migration receipts, atomic saves, real validation. Solid. |
| Scout replaces "faster" (`crates/tui/src/fleet/scout.rs`) | `76abfdaf15` | Catalog-**verified** fast sibling, never a guessed model name. Correctness win. |
| Truthful model-picker rows | `cb0c70c04` | Family grouping + chips that state only what the catalog knows. |
| Config credential scope fix | `adfecad97a` | Fixes real "authorized here, locked there" bug. Independent of Fleet UI. |
| `workflow run` no longer requires `--fleet` | `35a2d3c75e` | Independent CLI ergonomics win. |
| Clippy `-D warnings` clean | `98e8476bd5` | Needed for release gates anyway. |
| Composer scroll fix | `5d9848da2` | Independent. |
| Running sub-agent reaches work bar under both rail panels | `dfaa6bb20` | Independent; relates to Task C. |
| Release evidence + Model Studio proof | `44fa4be80`, `ec82bc674` | Docs/evidence. |

Reject as the primary surface:

- `crates/tui/src/tui/views/fleet_list.rs` and
  `crates/tui/src/tui/views/fleet_detail.rs` **as what `/fleet` opens**.
- The `/fleet` → `OpenFleetList` repoint.

**`/fleet` (bare) must open the setup/roster experience again.**

### Judgment call left to you

Do not simply delete the list/detail code and walk away — think about it:

- The v2 store genuinely supports *multiple named Fleets*, which the old wizard
  predates. If a person has more than one saved Fleet, they need some way to pick
  between them. A reasonable shape is: `/fleet` opens the familiar setup/roster
  surface for the **selected** Fleet, and switching between saved Fleets is a
  small inline affordance (a header selector, or `/fleet list` as a subcommand) —
  **not** a filesystem list standing between the user and their fleet.
- Do not show raw source file paths as primary row content. Paths belong in
  receipts and detail/debug views, not as the thing a person reads first.
- Do not present a wall of single-letter keybindings as the interface.
- Receipts naming the exact file written are **good** — keep that honesty.

Commit `f9ba6b2063` (*session route changes are temporary until explicitly saved*)
plus the `/fleet save` / `/fleet save-as` verbs are a mixed case: the underlying
principle (nothing writes to disk without an explicit command) is right and worth
keeping. Evaluate whether the command surface is discoverable enough, and note
that a blocking modal was already tried and rejected because it broke scripted
PTY terminals — see the rejection rationale in
`docs/ops/RELEASE_EVIDENCE_2026-08-04.md` before proposing one again.

**Before rebuilding anything, `rg` for the symbols to check what `v094-integration`
already does.** Re-landing landed work is the failure mode this repo's ethos calls
out, and it is the one you own.

---

## 2. TASK B — the sub-agent runtime is too restrictive

The owner's read: *"that runtime seems too restrictive? kinda weird."* They are
right, and this handoff exists partly because of it.

**Where it lives:** `crates/tui/src/runtime_handoff.rs`, `WAITING_EVENT_SUFFIX`
(~line 29). Current injected text, verbatim:

```
 sub-agent(s) are still running. Do NOT poll them with agent(action="peek") or
 agent(action="status"). Do NOT use sleep or any shell blocking primitive as a
 waiting strategy. The runtime will deliver <codewhale:subagent.done> sentinels
 automatically when each child finishes — polling will never make that happen
 sooner. Stop immediately: emit zero tool calls and end the turn.
```

Emitted from `crates/tui/src/core/engine/turn_loop.rs:1732`; the same condition
feeds `StepFingerprint::waiting_for_subagents` in
`crates/tui/src/core/engine/stuck_guard.rs:48` for stuck detection.

**The problem:** the first three sentences are correct and worth keeping — polling
and sleeping genuinely accomplish nothing. But `emit zero tool calls and end the
turn` is far broader than the harm it prevents. It forbids the parent from doing
*any* independent work while children run: no reading files, no unrelated edits,
no independent shell work, no answering the user's question. In this very session
it repeatedly stalled the parent mid-release with useful, non-conflicting work
available, and turned "children are running" into "the session is frozen."

**What to do:** design and implement a more prudent policy. The obvious shape is to
forbid the specific harms rather than all action:

- **Still forbidden:** polling children (`peek`/`status`), `sleep` or any blocking
  primitive as a waiting strategy, and starting work that *depends on* a running
  child's result.
- **Should be allowed:** independent read-only investigation, unrelated edits that
  cannot conflict with a child's worktree, and continuing to talk to the user.

Verify the constraints rather than assuming them:

- Confirm whether the stuck-guard still behaves correctly if the parent is
  permitted to act — `StepFingerprint` dedup exists to catch a spinning parent, and
  a policy change must not blind it.
- Check whether any test pins the current wording
  (`runtime_handoff.rs` has tests around line 710).
- Consider whether "independent" can be stated crisply enough for a model to apply,
  or whether it needs a concrete rule (e.g. worktree-isolated children make file
  conflicts structurally impossible, so the real constraint is only about
  *result* dependencies).

Land it as its own reviewable commit with a real body explaining the reasoning.

---

## 3. TASK C — the work bar grows without bound

**This is not the same complaint as Task A.** The owner explicitly confirmed:
*"the work rails thing is fine, it's `/fleet`."* The rails/panel work is good. The
remaining issue is narrower: **settled sub-agents never leave the top strip.**

Commit `ff97641b7` (*release-quality transcript pass + persistent sub-agent
visibility*) made finished workers durable for the whole session. After a few
fan-outs the strip becomes a permanent stack of `scout · completed · …` rows that
eats the transcript. That was a fix for "finished agents vanish before I can open
them" and it overcorrected.

**Relevant code:** `crates/tui/src/tui/work_surface/model.rs` (`project_visible`,
`catalog_rows`), the height/cap arithmetic in the `layout` module, and the PTY test
`crates/tui/tests/work_bar_subagents_pty.rs`, which currently asserts a finished
agent **stays** in the top bar — that assertion is the contract you are changing.

**Target behavior:**

- Top strip = actionable work only: running / queued / needs-input agents, active
  goal and to-dos.
- Settled workers collapse out of the strip — either after a short TTL / at the
  next user turn, or immediately into a single summary row
  (e.g. `▾ Subagents 2 running · 6 completed`).
- Completed agents stay **reachable**, never deleted: the Agents panel / catalog
  and the existing transcript pager remain the door to their receipts.
- **Do not bind `Ctrl+S`** — it is already the composer stash. The owner floated it
  as an idea; it conflicts. Use the existing Agents panel or a `/agents`-style
  command; if a dedicated chord is truly needed, audit the keymap first.

---

## 4. TASK D — sub-agent elapsed time and token counts are wrong

Owner report: *"the time update for how long sub agents are working + how many
tokens they're using is pretty weird and not really working properly."*

Not yet diagnosed — treat this as a real investigation, not a known fix.

Starting points:
- `crates/tui/src/tools/subagent/mailbox.rs` — `token_usage()` (~line 135) is the
  incremental usage path from a child's API calls.
- `crates/tui/src/tui/app.rs` — `received_tokens` (~line 448), `started_at`
  (~line 738); note the existing comment about freezing a timer at
  `finished_at - started_at` so completed goals stop ticking.
- The rows themselves come from `7b20ef513`
  (*show sub-agents as type, objective, elapsed and tokens*) and `172bf65ce`.

Likely suspects worth confirming before fixing: elapsed time computed from a stale
snapshot rather than ticking; a timer that never freezes on settle (or freezes too
early); token totals that never accumulate across a child's turns, or that
double-count cache hits/misses. Reproduce first, then fix — see
`docs/AGENT_ETHOS.md` and commit as **WIP** if you cannot demonstrate the fix.

---

## 5. TASK E — website copy is too text-heavy

Owner: *"i'm not a huge fan of all of the text that was on the previous one —
granted we'd have to also make sure it's able to be translated."*

A survey already ran; its findings, to be re-verified:

- Home copy is dictionary-driven from `web/lib/i18n/dictionaries/*/home.ts`
  (EN reference + 9 locales). Chrome is separate and mostly short labels.
- `page.tsx` is dictionary-driven; the hardcoded leftovers are intentional
  code-owned voice (Plan · Act · Operate, the install command, GitHub, receipt
  verbs, package-manager names) — leave those alone.
- Extra bulk that is **not** in the dictionaries:
  `web/lib/content/getting-started.ts` (EN/ZH step bodies) and
  `web/components/thinking-trace.tsx` (EN/ZH terminal traces).
- Locales: `en` + `zh` are fully shipped; `ja vi ko ru uk es pt-BR id` are partial
  (chrome + home only).
- Densest prose, worst first: getting-started step bodies → `heroIntro` →
  `proofBody` (which repeats the hero) → `decidesLede` → `metaDescription` /
  `startLede`.

**Hard constraint:** every locale dictionary must keep exact **key and token
parity**. Shortening EN without updating all 9 locales breaks the parity tests. If
you cannot translate faithfully, cut the *key* everywhere rather than leaving
locales stranded on stale long copy. Run the web tests (`cd web && npm test`) plus
the locale/facts parity checks.

---

## 6. TASK F — finish the v0.9.4 release

This is the original job. Do not start it until Tasks A–E are landed or explicitly
deferred by the owner, because the whole point is that these fixes ship *in* 0.9.4.

### Release-blockers (GitHub issues, all still open)

| Issue | Status from prior recon — **re-verify, do not trust** |
|---|---|
| **#5123** agent spawn surface / builder runs read-only and self-BLOCKED | **PARTIAL.** `deliberate=true` requires type/profile + workspace_policy + expected_artifact + write_authority (`crates/tui/src/tools/subagent/mod.rs:10630-10664`); some contradictions fail closed (`:10709-10714`, `:10770-10788`). But `type=builder` + `write_authority=read_only` still **parses and runs**, then silently clamps write/shell off (`apply_spawn_write_authority`, `:7905-7936`). It should fail closed. |
| **#5034** provider switch default | Looks fixed in code (`crates/config/src/lib.rs:3264-3271`, tests `tui/ui/tests.rs:6285-6388`). Verify, then close with evidence. |
| **#5035** workflow authoring / fan-out | Looks fixed (`crates/tui/src/tools/workflow.rs:300-312`, `1662-1675`, `2463-2491`, tests `7520-7619`). Verify, then close with evidence. |

### Known release-machinery gap

`crates/telemetry` (`codewhale-telemetry`) is a **path + version dependency** of
both `crates/cli` and `crates/tui` (`version = "0.9.4"`), but it is **absent from
the publish list** in `scripts/release/crates.sh:4-24`. A `cargo publish` of the
CLI/TUI will fail on a missing crates.io dependency. Add it in correct dependency
order and re-run the dry-run.

### CHANGELOG

`CHANGELOG.md` currently has **both** a large `## [Unreleased]` block (line 8) and
`## [0.9.4] - Unreleased candidate` (line 153). Before tagging:

1. Triage the `[Unreleased]` block — fold what ships into the 0.9.4 notes.
2. Date the 0.9.4 heading. `auto-tag.yml` / `release.yml` validate with
   `--require-dated-release` and **will fail** on "Unreleased candidate."
3. The GitHub Release body is generated from the CHANGELOG section by
   `scripts/release/generate-release-body.sh` — there is no separate notes
   template. The CHANGELOG *is* the release notes.

Also stale: `integrations/verifiers-codewhale/README.md` still pins harness `0.9.1`.

### Sequence

Authoritative procedure is `docs/RELEASE_RUNBOOK.md` + `docs/RELEASE_CHECKLIST.md`.
Summary:

1. **Land Tasks A–E** on `v094-integration`, pushing as you go.
2. **Pre-merge prep:** CHANGELOG disposition; `./scripts/release/prepare-release.sh 0.9.4`;
   fix the `crates.sh` telemetry gap; verify/close #5034, #5035, #5123.
3. **Gates:** `cargo fmt`, `cargo check`, `cargo clippy -D warnings`,
   `cargo test --workspace`, `./scripts/release/check-versions.sh`,
   `publish-crates.sh` **dry-run**, npm smoke, and `cd web && npm test`.
4. **Merge PR #5135** (train → `main`). Keep the train branch
   `agent/v094-release-train-20260802` in sync with `v094-integration` — that is
   the PR's head. Do **not** tag the integration branch.
5. **Exact-head proof on `main`:** dispatch `ci.yml` and `release-candidate.yml`
   with `expected_sha=$(git rev-parse origin/main)`.
6. **Tag `v0.9.4`** → `release.yml` builds the matrix, pushes GHCR, and creates the
   GitHub Release with assets. Then
   `./scripts/release/verify-release-assets.sh 0.9.4`.
7. **Publish registries manually** from a detached `v0.9.4` worktree:
   `publish-crates.sh publish` (19 crates, dependency order), then
   `npm publish --access public` in `npm/codewhale` (OTP required; `version` and
   `codewhaleBinaryVersion` must both be `0.9.4`).
8. **Website cutover:** bump `web/data/latest-published-release.json` to `0.9.4`,
   regenerate facts, merge, then `gh workflow run web.yml --ref main` and confirm
   `/api/facts` matches that SHA. The site does **not** auto-deploy on merge — it
   is maintainer `workflow_dispatch` only.
9. **Close out:** `./scripts/release/check-published.sh 0.9.4`, CNB mirror tag
   check, close the milestone issues.

---

## 7. Hard rails

- **Steps 6–8 are owner-gated.** Tagging, GitHub Release, `cargo publish`,
  `npm publish`, and the website deploy are irreversible and public. Get explicit
  approval from Hunter before each. Everything up to and including step 5 is
  ordinary work.
- Never rewrite published history, retag a shipped release, or force-push a shared
  ref. `main` stays protected.
- Respect the do-not-delete guardrail in `AGENTS.md`
  (`tui/src/context_budget.rs`, `tui/src/model_registry.rs`,
  `tui/src/prompt_zones.rs`, `tui/src/tools/remember.rs`, `config/src/route/`) —
  these are repeatedly misflagged as dead code and deleting them breaks the build.
- Credit is CI-enforced: `Co-authored-by` trailers are for **human** contributors
  only. Note agent assistance in a plain commit body.
- Commit as **WIP** unless you actually verified the behavior — built the binary,
  ran the test, reproduced the fix. Report failures with their output.
- Leave unrelated edits by other people or agents intact.

## 8. Loose ends in the workspace

Four `codex/agent-*` branches (`fix-5123-spawn-4e323c47`,
`fix-work-bar-archive-d5ee25f9`, `spawn-contract-fix-fa1ef4d7`,
`workbar-collapse-88efb53d`) all sit at `abcd7e27b` — the train tip — with **zero
commits and zero diffs**. Four sub-agents were dispatched against Tasks A/C and all
four died on provider quota exhaustion before producing anything. They are safe to
delete; there is no work to salvage.
