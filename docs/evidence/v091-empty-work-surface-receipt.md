# v0.9.1 empty Work-surface receipt

This receipt verifies the empty-surface behavior from the real TUI binary. It
is not based on a product mockup.

## Source

- Version: `codewhale-tui 0.9.1 (fa46105a7183)`
- Commit: `fa46105a7183ce961f503996a7e746f763ffb29c`
- Checkout at capture: clean detached exact-source worktree for
  `codex/v091-final-integration-20260721`
- Terminal: `120x32`
- Route: isolated local Ollama configuration using `qwen3-coder`; no API key
- Workspace: clean public fixture shown as `~/codewhale-demo`
- Theme: Blue Stage dark

## Capture method

The binary was built with:

```bash
cargo build --release --locked -p codewhale-tui --bin codewhale-tui
```

It was launched in a 120-column by 32-row tmux PTY with isolated state. VHS
0.10.0 rasterized the real terminal cells at 1280x720; it did not generate or
reconstruct product UI:

```bash
NO_ANIMATIONS=1 \
CODEWHALE_HOME=/path/to/sealed-home/.codewhale \
CODEWHALE_CONFIG_PATH=/path/to/sealed-home/.codewhale/config.toml \
CODEWHALE_MCP_CONFIG=/path/to/sealed-home/.codewhale/mcp.json \
target/release/codewhale-tui \
  --skip-onboarding \
  --fresh \
  --no-project-config \
  --no-mouse-capture \
  --workspace ~/codewhale-demo
```

The resulting 1280x720 capture has SHA-256
`b6d869b74985e8c1c89288076185ce8de5f951e190f0a745e5949c5b60cc666f`.
It contains no username, credential, account identifier, private repository
path, error state, or unsupported product claim. The idle capture process had
no open TCP or UDP socket. `NO_ANIMATIONS=1` makes this one canonical still
stable; the separate real-PTY suite proves full, reduced, and still motion.
The captured header visibly identifies `v0.9.1 (fa46105a7183)`, and the context
line is `~/codewhale-demo · main · mcp 0`.

## Acceptance

The fresh active session contains the header, idle canvas, composer, and
footer. It does not contain `Work · empty`, a Work heading, or reserved Work
rows. Active, error, and disconnected Work projections remain covered by the
TUI unit suite.

```text
cargo test -p codewhale-tui --bins --all-features --locked
test result: ok. 8062 passed; 0 failed; 4 ignored

cargo test -p codewhale-tui --test qa_pty --locked
test result: ok. 25 passed; 0 failed; 1 ignored
```
