# TUI agent guidance

Scope: the terminal UI, its embedded runtime engine, and user-visible behavior.
Read the repository guidance first.

## UI contracts

- One owner per fact: route/mode/permission/context in the header; work in the
  top strip; receipts and the active row in the transcript; phase/cost/detail
  controls in the footer.
- Derive state from typed enums such as `ShellPhase` and `OceanTreatment`.
  Renderers must not infer state from English strings or invent lifecycle state.
- Keep settled output still. Motion is semantic, bounded, and fully disabled by
  reduced-motion settings.
- Route notices through the toast system, with typed level and lifetime; do not
  add new writes to the legacy `status_message` sink.
- Compact layouts remove chrome before content. Selectable rows need recorded
  hitboxes, visible focus, keyboard/mouse parity, and confirmation for
  destructive actions.
- User-visible prose uses `tr(locale, MessageId::...)`. Commands, key names, and
  glyphs are composed in code. Follow `locales/AGENTS.md` for string changes.

## Verification

```sh
cargo test -p codewhale-tui --lib --locked
cargo test -p codewhale-tui --tests --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

`--lib` and `--tests` are disjoint; use both or the workspace gate. For PTY
failures, rerun the exact test before changing behavior. Script one input at a
time and capture after the UI settles. Validate representative layouts at
40x12, 60x16, 80x24, 100x32, and 140x40; judge motion from repeated frames, not
a single screenshot. Remove inherited `NO_COLOR`, `TERM=dumb`, and tmux motion
overrides when they would invalidate the observation.
