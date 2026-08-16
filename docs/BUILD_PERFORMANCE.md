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

## A0 receipts (commit 533c530b + hermeticity fixes; empty target dir)

`CARGO_TARGET_DIR=/Volumes/VIXinSSD/CW/.tmp/compile-speed-baseline`, HTML
timing reports archived under
`backups/compile-speed-evidence-20260815/` (a0-cold-lib-test-timing.html,
a0-incremental-lib-test-timing.html, a0-llvm-lines-top40.txt).

| Receipt | Wall | Load (1 min) |
| --- | --- | --- |
| Cold `cargo test -p codewhale-tui --lib --locked --no-run --timings` | 127 s (user 329 s) | 8.9 |
| `touch crates/tui/src/elapsed.rs` + same command | 21 s | 12.3 |
| `touch` + `cargo test -p codewhale-tui --lib --locked elapsed::` (the everyday loop) | 20 s (4 tests run) | 11.2 |
| Lib-test binary size | 357 MB (`codewhale_tui-<hash>`); links with the `__eh_frame section too large (max 16MB)` compact-unwind warning | — |
| `cargo check -p codewhale-tui --lib --tests` incremental after `touch` (frontend only) | 14 s | 6.2 |
| Incremental full lib-test build after `touch`, same conditions | 28 s | 6.2 |

Cold timing report, top units (605 units): `codewhale-tui` lib test
**106.0 s**, `codewhale-config` 7.9 s, `jsonschema` 5.8 s, `moxcms` 4.7 s,
`codewhale-protocol` 4.2 s, `tokio` 4.0 s, `rustls` 3.8 s, `schemaui`
3.4 s, `rmcp` 3.4 s, `h2` 3.3 s, `jsonschema` (second copy) 3.2 s,
`codewhale-workflow` 3.2 s, `syn` 3.1 s, `rio-vt` 3.1 s, `regex-automata`
3.0 s. The incremental report has exactly one non-zero unit: `codewhale-tui`
lib test 20.8 s. So the everyday tax is the tui crate itself, split
roughly half frontend (check --tests 14 s) and half codegen + link (28 s
total); dependencies and the linker are not where the time is.

`cargo llvm-lines -p codewhale-tui --lib`: **8,138,810 lines in 223,052
copies**. Largest single function is the `rust_i18n` backend closure
(`_RUST_I18N_BACKEND::{closure#0}`, 311,782 lines, 3.8 % of the crate on
its own — the 15 locale packs are compiled into a match by the `i18n!`
macro), then `run_event_loop` 27 k, `Engine::handle_deepseek_turn` 26 k,
`RuntimeThreadManager::monitor_turn` 16 k, then serde `Deserialize`
expansions for `Config`/`ProvidersConfig`/`Settings` (5–6 k each, several
copies per toml deserializer).

### A0.1 dependency ratchet

`cargo metadata --locked` counted **690** packages and `cargo deny check
bans` warned on duplicate `fancy-regex`, `jsonschema`, `jsonschema-regex`,
`referencing` plus stale `jni`/`jni-sys`/`redox_syscall` skips. Cause: the
workspace `jsonschema` pin had been bumped to 0.49 while `schemaui` 0.12
(latest 0.12.4 included) still requires `^0.46`. Pinning the workspace
back to the 0.46 line removes the second jsonschema stack (**685**
packages; deny bans and advisories clean; `--locked` resolves;
codewhale-workflow-js 61 tests and the tui schema tests pass). Cold saving
is the two duplicated units (~9 s of unit time, ~3 s of wall).

### A1 cache topology (desk-local, not committed)

New-worktree cold `cargo test -p codewhale-tui --lib --locked --no-run`,
same machine, back to back:

| Topology | Wall | CPU (user) | Notes |
| --- | --- | --- | --- |
| Per-worktree fresh target (control) | 127 s | 329 s | A0 |
| One shared `CARGO_TARGET_DIR` (warm from another worktree) | 121 s | 188 s | deps reused; every workspace crate recompiles (path-keyed); no lock waits observed; target 14 GB |
| `build.build-dir = ".../{workspace-path-hash}"` per workspace + warm shared `sccache` (`CARGO_INCREMENTAL=0`) | 107 s | 161 s | 73.6 % sccache hit rate (all 337 Rust dep units hit; the 125 misses are workspace crates); 2.7 GB build dir per workspace + 483 MB cache; the same command that *populated* the cache took 108 s / 157 s CPU |

Wall time is the tui crate in every topology; the topologies buy CPU
(~50 %), which is what matters when several checkouts build at once.
Recommended user-level `~/.cargo/config.toml` (adjust the two roots):

```toml
[build]
# One build root for every checkout; each workspace gets its own subdir,
# so worktrees never wait on each other's target lock.
build-dir = "/path/to/cache/codewhale/build/{workspace-path-hash}"
# Optional: reuse dependency compilation across checkouts.
# rustc-wrapper = "sccache"
```

`sccache` was installed with `brew install sccache` on this machine for the
measurement.

### A2 nextest in CI

`cargo test --workspace --all-features --locked --doc` inventories
**3 passing / 8 ignored doctests across 21 crates**; CI keeps them as a
separate step next to `cargo nextest run --workspace --all-features
--locked --profile ci`.

### A3/A4 (not adopted, measured)

Frontend and codegen split the tui unit roughly evenly (14 s / 14 s
incremental); the linker is a small part of that and dependencies are
already warm after the first build, so `[profile.dev.package."*"]
opt-level = 1` (paired result above), `-Wl,-dead_strip`, and other
`RUSTFLAGS` stay out of the repo (they would apply to shipped profiles);
`split-debuginfo` is already `unpacked` on macOS.

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
