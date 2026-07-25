# Tool surface

This document describes the current model-facing tool contract in the v0.9.1
source candidate. The registry remains larger than the first-turn catalog so
saved transcripts can replay and uncommon capabilities can be loaded on demand.
The model should learn one canonical name for each common operation.

Implementation sources:

- `crates/tui/src/core/engine/tool_catalog.rs` owns the eager/deferred catalog.
- `crates/tui/src/tools/registry.rs` registers canonical tools and hidden aliases.
- `crates/tui/src/tools/{file_tool,git_tool,run_tool,web_tool,shell}.rs` own the
  canonical action schemas.
- `docs/RUNTIME_SIMPLIFICATION_DESIGN.md` records the v0.9.1 cutover and receipt.

## Default-active contract

The default-active policy contains exactly these ten names:

1. `Bash`
2. `File`
3. `Git`
4. `Run`
5. `agent`
6. `remember`
7. `tasks`
8. `update_plan`
9. `work_update`
10. `tool_search`

`remember` is registered only when the user enables the built-in memory path;
once present, it stays eager so a model can capture a durable preference without
first discovering the tool. A memory-disabled or Moraine-fallback runtime omits
that registration and therefore exposes nine of the ten policy names.
`tool_search` is synthetic rather than registry-backed and is always active.

The surface is action-based. A model calls one stable tool name and selects the
operation through its `action` field instead of choosing among many synonymous
single-purpose tools.

### Core action tools

| Tool | Actions | Purpose |
|---|---|---|
| `Bash` | `run`, `wait`, `interact`, `cancel` | Run bounded commands, continue background work, send input, and cancel processes. |
| `File` | `read`, `list`, `search_name`, `search_content`, `write`, `edit`, `patch` | Read, find, and modify workspace files with structured, workspace-aware results. |
| `Git` | `status`, `diff`, `log`, `show`, `blame` | Inspect repository state and history without parsing shell output. |
| `Run` | `tests`, `verifiers` | Run project tests or independent verifier gates with structured results. |

`Bash` appears only when the active session/profile permits shell use. Plan
keeps it unavailable. In Act and Operate, the active permission posture,
sandbox, command policy, trusted paths, repository law, and managed policy still
apply. Full Access removes ordinary approval prompts; it does not bypass hard
safety or repository-policy holds.

`File` is capability-filtered by mode. Plan advertises its read-only actions;
write/edit actions require Act or Operate, and `patch` also requires the
apply-patch feature. The same read-before-edit, workspace, and policy checks used
by the former spellings remain in force.

### Coordination tools

| Tool | Purpose |
|---|---|
| `agent` | Dispatch one focused sub-agent run and return an id, compact receipt, and transcript handle. |
| `remember` | Append one terse durable preference or convention when the user has enabled built-in memory. |
| `tasks` | Create, list, read, cancel, gate, and inspect durable task work through one action family. |
| `update_plan` | Publish optional high-level strategy, phases, constraints, verification, and handoff context. |
| `work_update` | Replace the concrete To-do / Work progress projection for the active thread or durable task. |
| `tool_search` | Discover and load a deferred tool only when the current turn needs it. |

`update_plan` and `work_update` are complementary, not competing checklists.
The former carries strategy; the latter is the concrete progress ledger shown
to the user.

## Deferred and dynamic tools

`Web` is a conditional, deferred action tool with `search`, `fetch`, and `wait`
actions. It is discoverable through `tool_search` only when the active network
policy and runtime backend permit it; it is not one of the ten default-active
names.

The durable `github`, `automation`, and `rlm` action families are also deferred
by default. `rlm` owns `open`, `eval`, `configure`, and `close` actions for a
persistent sandboxed Python session. Feature-gated native tools may be added to
the active or deferred catalog only when their implementation and host
dependencies are available.

MCP tools are dynamic. Successfully connected servers register names such as
`mcp_<server>_<tool>` from `~/.codewhale/mcp.json`; a failed or disabled server
must not be presented as an available model tool.

## Modes and permission postures

Modes and permission postures are separate controls:

- **Plan** is read-only. It exposes the read-only `File` projection and other
  safe inspection capabilities, but no shell or file mutation.
- **Act** is ordinary interactive execution.
- **Operate** uses the same direct-tool authority as Act while preferring Fleet
  workers for independent, parallel, isolated, background, or long-running work.
- **Ask**, **Auto-Review**, and **Full Access** control approval behavior within
  an action-capable mode. They never widen a Plan turn into write access.

See `docs/MODES.md` for the full mode and posture contract.

## Replay-only aliases

Legacy single-purpose names stay registered so saved transcripts, sessions, and
recorded automation replay without migration. They are hidden from the model
catalog and from `tool_search`; new prompts and docs must use the canonical
action tools.

| Replay-only spellings | Canonical action |
|---|---|
| `exec_shell`, `exec_shell_wait`, `exec_wait`, `exec_shell_interact`, `exec_interact`, `exec_shell_cancel` | `Bash`: `run`, `wait`, `interact`, `cancel` |
| `read_file`, `list_dir`, `grep_files`, `file_search`, `write_file`, `edit_file`, `apply_patch` | `File`: `read`, `list`, `search_content`, `search_name`, `write`, `edit`, `patch` |
| `git_status`, `git_diff`, `git_log`, `git_show`, `git_blame` | `Git`: matching action |
| `run_tests`, `run_verifiers` | `Run`: `tests`, `verifiers` |
| `web_search`, `fetch_url`, `wait_for_dev_server` | `Web`: `search`, `fetch`, `wait` |
| `task_*` | `tasks`: matching action |
| `github_*` | `github`: matching action |
| `automation_*` | `automation`: matching action |
| `rlm_open`, `rlm_eval`, `rlm_configure`, `rlm_close` | `rlm`: `open`, `eval`, `configure`, `close` |
| `checklist_*`, `todo_*` | `work_update` |

Replay compatibility does not make an alias a supported spelling for new model
calls. Alias execution must stay behaviorally equivalent to its canonical
action and must not add the alias back to the advertised catalog.

## Long-running work

Use `Bash` with `action: "run"` for bounded commands. Set its background option
for work that may outlive a normal foreground wait, then use `wait`, `interact`,
or `cancel` against the returned process id. Live shell jobs are also visible in
`/jobs`; process-local jobs must be marked stale after restart rather than shown
as reattached processes.

Use `tasks` when the work itself needs a durable lifecycle, structured gates,
artifacts, replayable timelines, or a stable task id. Large tool results should
remain behind bounded handles or artifacts instead of being copied wholesale
into the parent transcript.

## Parallel fan-out

The sub-agent capacity source of truth is
`crates/tui/src/config/subagent_limits.rs`:

- default configured concurrency: **64**;
- maximum configured concurrency: **128**;
- maximum admitted running-plus-queued work: **1024**.

These are capacity ceilings, not advice to dispatch every available slot. A
manager should use the smallest useful fan-out, preserve a single owner for
fan-in, and verify worker receipts before reporting combined completion.

RLM child-query batching is a different, cheaper cost class. Its
`sub_query_batch` helper accepts 1–16 one-shot children inside a live `rlm`
session; it is not a substitute for tool-carrying `agent` workers.

## Release verification

Do not infer the public surface from handler function names. Verify the model
catalog and alias visibility at the exact candidate SHA:

```bash
python3 scripts/measure-runtime-contract.py
cargo test -p codewhale-tui --bin codewhale-tui --locked canonical_runtime_tools_hide_legacy_aliases
cargo test -p codewhale-tui --bin codewhale-tui --locked shell_alias_tools_hidden_from_model_catalog
cargo test -p codewhale-tui --bin codewhale-tui --locked runtime_task_families_expose_canonical_tools_with_hidden_aliases
```

The provider-free full-policy receipt enables built-in memory and must report the
ten default-active names listed above. A memory-disabled receipt truthfully omits
`remember`. A separate repository-wide tool count may include deferred, dynamic,
feature-gated, and replay-only registrations; it is not the number of tools
placed in the first-turn model catalog.
