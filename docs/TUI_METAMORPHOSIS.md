# TUI metamorphosis boundary

The underwater TUI is being replaced as a staged molt, not rewritten inside
the legacy god files.

## New owners

- `crates/tui/src/tui/work_surface/` owns transcript-top Tasks, To-do, active
  workers, stable row IDs, focus, scrolling, hitboxes, and row actions.
- `crates/tui/src/route_billing.rs` owns whether a route presents money,
  subscription/quota usage, or local usage. Model IDs never decide this alone.
- `crates/tui/src/tui/underwater.rs` remains the Ocean shell composition owner.

`ui.rs` and `mouse_ui.rs` are adapters: they forward terminal events and apply
typed actions. They must not regain per-surface state or rendering rules.

## Rollback contract (historical — Classic is gone)

Classic and its sidebar were deleted in `1b07e2cbc` (0.9.4 rail
unification), after the gates below were satisfied. This section is kept as
a record of that decision, not as live guidance: there is no Classic shell
to roll back to, and "restore old behavior" must never mean resurrecting
`sidebar.rs`. Panel logic lives in `work_surface/panels.rs` and the row
machinery in `work_surface/render/`; the remaining `tui::sidebar` line
builders are `pub(crate)` helpers being wound down.

The original gates, for provenance:

1. `40x12`, `60x16`, `80x24`, `100x32`, and `140x40` pass keyboard and mouse
   interaction checks.
2. Full/reduced motion and Ombre/Flat/Terminal treatments pass live PTY checks.
3. The hermetic TUI suite passes twice.
4. A release build is installed through `scripts/release/install-dogfood.sh`,
   and its commit/SHA receipt matches the running binary.
5. Hunter accepts the live candidate.
