# Post-0.9.1: thin TUI over core + stream consolidation

**Status:** seams landed in v0.9.1; full split deferred.

## What shipped in 0.9.1 (seams only)

New visual / capability systems stay in focused modules (do not grow the
monoliths without necessity):

| System | Module |
|--------|--------|
| Ambient ocean life | `tui/ambient_life.rs` |
| Hot tail | `tui/hot_tail.rs` |
| Hover aura | `tui/hover_hit.rs` + `tui/hover_layer.rs` |
| Git status cache | `tui/git_status.rs` |
| Worktree manager UI | `tui/worktree_manager.rs` |
| Phase rail | `tui/phase_strip.rs` |
| Stream entry seam | `client/stream_entry.rs` |

Business logic must not land in `ui.rs` / `app.rs` / `widgets/mod.rs` unless it
is pure view wiring.

## StreamFn consolidation (partial)

`client/stream_entry.rs` is the shared open-path seam:

- HTTP policy (`DualWithH1Fallback` / `Http1Only`)
- H1 retry classification
- idle-timeout message format

Wire-protocol adapters remain at the edge (`chat.rs`, `anthropic.rs`,
`responses.rs`). Chat Completions now opens through `StreamOpenRequest`;
**follow-up:** route the Anthropic Messages and Responses adapters through the
same policy before collapsing further toward a piagent-style single StreamFn.

## Thin TUI over core (north star)

`ui.rs` / `app.rs` / `widgets/mod.rs` remain large. Post-0.9.1 priority:

1. Extract tool / git / github / session / workflow / MCP routing out of the TUI
   crate into a core/data layer (kimi-code `agent-core` / piagent package shape).
2. Keep the TUI a projection of state + input routing.
3. Prefer new modules over adding to the three monoliths.

## Optional deferred

- Full live global model subscriptions (refresh on every open + `r` / Ctrl+R is
  the practical path; continuous live feed if unstable stays deferred).
- YOLO mode is gone from product UI; `mode_yolo` remains only as legacy theme
  palette data.
