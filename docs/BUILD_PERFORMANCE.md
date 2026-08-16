# Build and test performance

Measured facts about how long Codewhale takes to build and test, what was
changed to make the contributor loop faster, and what is deferred. Numbers
are from one machine (Apple Silicon, 14 cores, rustc 1.97.0, Xcode 26.2
`ld-1230`) taken while four other cargo jobs were running (1-minute load
average 10–27, recorded next to each number), so treat them as relative
before/after evidence, not benchmarks.

## Where the time goes (baseline, commit 533c530b)

| Step | Wall | Notes |
| --- | --- | --- |
| Cold `cargo build -p codewhale-tui` (empty target) | 94 s (user 270 s) | 543 units; `codewhale-tui` alone is 70 s and is the critical path; next longest units are `codewhale-config` 7.5 s, `jsonschema` 6.3 s, `codewhale-workflow` 5.6 s, `tokio` 5.4 s. Load 13. |
| Cold `cargo test -p codewhale-tui --lib --no-run` (empty target) | 148 s (user 347 s) | The 10.5k-test unit binary is 357 MB and trips the macOS linker's `__eh_frame > 16MB` compact-unwind warning (harmless). Load 20. |
| Incremental `cargo build -p codewhale-tui` after a one-line edit | 12.5 s | Load 11. |
| Incremental `cargo test -p codewhale-tui --lib --no-run` after a one-line edit | 19 s | Load 11. |
| `cargo test --workspace --all-features --locked --no-run` with deps warm | 155 s (user 366 s) | 61 test binaries. Load 6→11. |
| Cold-ish `cargo check --workspace --all-targets --locked` (deps built) | 82 s | Load 19. |
| Running the tui unit suite with libtest (`cargo test -p codewhale-tui --lib`) | 268 s | From the release gate log; 10,531 tests. Load ~10. |
| Running the same suite with `cargo nextest run -p codewhale-tui --lib` | 96–108 s | Same tests, one process per test, all cores busy. Load 12–20. |
| Running the whole workspace under `cargo nextest run --workspace --all-features` | 353 s | 12,744 tests, PTY suite serialized by the nextest config. Load 17. |

Structural facts behind those numbers:

- `crates/tui` is ~746k lines of Rust (609k non-test, 137k inline tests in
  488 `#[cfg(test)]` modules, 10.6k `#[test]`/`#[tokio::test]` functions).
  It compiles as one crate, so the frontend of that crate is the critical
  path of every build and every unit-test run recompiles it with
  `cfg(test)`.
- Dependencies are already trimmed (`reqwest` rustls-no-provider, `image`
  png only, `syntect` default-fancy, `rmcp` no default features, `mimalloc`
  no default features). `cargo tree -d` shows only routine duplicates
  (`toml` 0.8/1.1, `thiserror` 1/2, `strum` 0.27/0.28, `syn` 2/3,
  `sha2` 0.10/0.11) that come from third-party crates, not from workspace
  choices.
- `[profile.dev] debug = "line-tables-only"` is already set (#5246) and
  Cargo already uses `split-debuginfo = unpacked` on macOS.
- `target/debug` grows past 50 GB only through accumulation across
  feature sets and worktrees; a fresh test build is ~7 GB.

## What changed (this lane)

1. **`cargo nextest` is supported and documented** (`.config/nextest.toml`).
   Same test binaries, one process per test, so the tui unit suite runs in
   ~100 s instead of ~270 s here and slow or hanging tests are named instead
   of stalling the binary. The PTY binary is pinned to one test at a time
   (it drives pseudo-terminals and shared mock servers; today it serializes
   on an in-process mutex, which nextest's process-per-test model would
   otherwise bypass), and the integration binary that spawns the real
   `codewhale` executable is capped at four concurrent tests so its 30 s
   start-up budgets survive a fully loaded machine.
   `cargo test --workspace --all-features --locked` remains the
   authoritative gate; nextest is the local loop.
2. **Three tests depended on test order** and only passed because another
   test in the same process had installed the rustls crypto provider first:
   `codewhale-tui mcp::sse::endpoint_tests::message_before_endpoint_is_rejected_instead_of_buffered`,
   `codewhale-app-server tests::failed_config_set_keeps_the_stdio_bridge`,
   and `tests::successful_config_set_still_invalidates_the_stdio_bridge`.
   Each now installs the provider itself, exactly as production does at
   startup. No runtime code changed.
3. **CONTRIBUTING.md has a "Fast local loop" section**: `cargo check`
   first, targeted `-p codewhale-tui --lib <filter>` runs, nextest, sharing
   one `CARGO_TARGET_DIR` across worktrees, and the optional accelerators
   below.

## Measured and deliberately not adopted

- `[profile.dev.package."*"] opt-level = 1` (dependencies optimized once,
  workspace crates untouched). Paired measurement, back to back, same
  target layout: cold `cargo test -p codewhale-tui --lib --no-run` from an
  empty target went 148 s → 193 s (user 347 s → 778 s); the tui unit suite
  under nextest went 96 s → 81 s; incremental rebuilds are unchanged. A
  ~15 % faster test run is not worth a 2.2× more expensive cold build for
  people trying to build Codewhale for the first time. Contributors who
  mostly re-run tests can opt in locally by adding that table to a
  user-level `~/.cargo/config.toml` `[profile.dev.package."*"]` section.
- Extra `RUSTFLAGS`/linker flags in a repo `.cargo/config.toml`
  (`-no_deduplicate`, alternative linkers). Rustflags apply to every
  profile and would change shipped binaries; the macOS system linker is
  already `ld-prime`, and the measured incremental link cost is inside the
  12–19 s incremental numbers above. Documented as optional local
  accelerators instead.

## Deferred: split `codewhale-tui`

The single lever left that changes the shape of the numbers is splitting
the crate so a change to a leaf module does not re-typecheck 600k lines
and re-link a 357 MB test binary. Mechanical candidates, in dependency
order (each already only depends on `codewhale-config`/`codewhale-paths`
plus third-party crates, and each is consumed through a single module
path today):

| Candidate crate | From | Why it is a clean cut | Consumers to re-export from |
| --- | --- | --- | --- |
| `codewhale-glyphs` | `crates/tui/src/tui/glyphs.rs` | Constant tables + pure fns; no crate-internal deps. | `crate::tui::glyphs` |
| `codewhale-palette` | `crates/tui/src/palette/{tokens,themes,adapt,contrast,detect,osc11,user_theme}.rs` + `assets/user-theme.schema.json` | Pure color math and theme tables; depends on ratatui `Color` and `codewhale_config::codewhale_home` only. Also unblocks the web/CWC token drift noted in the tokens survey. | `crate::palette` |
| `codewhale-i18n` | `crates/tui/src/localization.rs` + `crates/tui/locales/*.json` | The `rust_i18n::i18n!` macro compiles all 15 packs into whichever crate hosts it; moving it out means locale-only edits no longer rebuild the TUI. `MessageId` is a plain enum. | `crate::localization` |
| `codewhale-mcp-transport` | `crates/tui/src/mcp/{sse,stdio,external_import}.rs` | Already talks to `codewhale-mcp`; the reviewed-launch binding is the only tui coupling. | `crate::mcp` |

Rules for the split: pure moves plus `pub use` re-exports at the old
paths, no behavior change, one crate per PR, each PR measured with the
table above (cold build, incremental build, incremental test build,
`cargo test -p codewhale-tui --lib --no-run`). Expected win: the tui
frontend time drops with lines removed; the test-binary link is unchanged
until the tests that live with those modules move with them.

## Optional accelerators (not required)

- `cargo install cargo-nextest` — see above.
- One `CARGO_TARGET_DIR` for all worktrees (e.g. `export
  CARGO_TARGET_DIR=$HOME/.cache/codewhale-target`) so dependency
  artifacts are compiled once; run `cargo clean -p codewhale-tui` rather
  than deleting the directory when it grows.
- `sccache` as `RUSTC_WRAPPER` caches dependency compilation across clean
  checkouts and matches what CI does (`.github/workflows/ci.yml` uses
  `mozilla-actions/sccache-action` plus `Swatinem/rust-cache`).
