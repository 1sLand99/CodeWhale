# Codewhale product bugs found during install testing

This file lists **product** defects: the binary, the installer scripts, or the
npm wrapper doing the wrong thing. Documentation problems are in
`DOC_DEFECTS.md`. Each entry was reproduced on Ubuntu 24.04 x86_64 with
**codewhale 0.10.0 (1be1a703b975)** unless stated otherwise. Evidence is in
`RECEIPTS.md`, `logs/` and `screenshots/`.

Severity: **High** = a new user gets stuck or loses data or privacy.
**Medium** = misleading, but there's a workaround. **Low** = cosmetic or
polish.

---

## B1 (High). With no API key, the TUI accepts a message and then does nothing

**Repro**
1. Fresh user, no `DEEPSEEK_API_KEY`, no `~/.codewhale`.
2. `mkdir ~/tui && cd ~/tui && git init -q && codewhale`
3. Type `hello` and press Enter.

**Expected:** an inline error such as "No DeepSeek key. Press F3 or run
`codewhale auth set --provider deepseek`". The README (line 47–48) says the
launch screen shows "no model connected".

**Actual:** the launch screen looks exactly like a configured one ("New
session", footer `DeepSeek · deepseek-flash · max`). After Enter the message
appears in the transcript and **nothing else happens**: no spinner, no error,
no reply. I waited more than 16 s (screenshots `30-`, `31-`, `32-cw2-*`). Only
opening F3 reveals `DeepSeek * missing key`. After I set a key through F3, the
original `hello` was never answered or retried.

The localization string `LaunchNoModelConnected` ("no model connected") is
present in v0.10.0 (`crates/localization/locales/en.json:1342`), but nothing in
`crates/tui/src` references it.

Contrast: `codewhale exec` with no key prints an excellent multi-step "DeepSeek
API key not found" message. A *wrong* key in the TUI sends you to a "Getting
started → Choose your model provider" flow that marks DeepSeek `last check
failed (authentication)` (screenshots `43-`, `44-`). Only the no-key case is
silent.

---

## B2 (High). v0.10.0 writes workspace snapshots to the legacy `~/.deepseek/snapshots`, not `~/.codewhale/snapshots`

**Repro:** fresh user, run any tool-using turn (TUI or `exec --auto`) in a git
repo, then `ls ~/.deepseek`.

**Expected:** `~/.codewhale/snapshots/<project_hash>/<worktree_hash>/.git`,
per `docs/ARCHITECTURE.md:271` and `:383`. `codewhale doctor` also reports
`State Root: active: ~/.codewhale` and "no known .deepseek entries need
migration".

**Actual:** a fresh install creates `~/.deepseek/snapshots/…`, with side-git
repos holding every pre- and post-turn copy of the workspace (636 KB after a
few turns; this grows with repo size). It happened for 4 of 5 test users (the
fifth never ran a tool turn).

**Root cause:** in the v0.10.0 tag, `crates/tui/src/snapshot/paths.rs`
`snapshot_base_with_home()` returns `~/.codewhale/snapshots` only **if it
already exists**, and otherwise falls back to `~/.deepseek/snapshots`. On a
fresh install it never exists, so it always takes the fallback. Current
`main` routes through `codewhale_config::resolve_state_dir("snapshots")`,
which returns the primary path when neither exists, so this looks **fixed on
main but shipped in v0.10.0**.

**Impact:** users who uninstall by deleting `~/.codewhale` leave behind full
copies of every repo they used, in a directory named after a different product.

---

## B3 (Medium). `codewhale doctor` exits 0 and says "All checks complete!" with no key, and with a key that fails the live probe

**Repro:**
* No key: `codewhale doctor; echo $?` prints `0`, and the summary line is
  `All checks complete!`. The API Keys section says
  `deepseek: env_source=not inspected, config_source=not declared` even when
  `DEEPSEEK_API_KEY` **is** set.
* Wrong key: `DEEPSEEK_API_KEY=sk-000…dead codewhale doctor --probe-api` prints
  `✗ API connection failed` and still exits 0.

**Expected** (per `docs/INSTALL.md:1174-1179`): doctor "checks API key" and
"exits non-zero if it finds a problem".

**Actual:** doctor never reports which credential source is active. Only
`codewhale auth status` does. `doctor --json` gives `"api_key": {"source":
"secret_store_unprobed", "availability": "not_probed"}` in every state.

---

## B4 (Medium). The offline error blames HTTP/2, Windows and proxies instead of saying "no network"

**Repro:** run `codewhale exec "say hi"` with no network (empty netns, no DNS).
The TUI behaves the same way (screenshot `47-tui-no-network.png`).

**Actual:**
```
error: Network error: SSE stream request failed after HTTP/1.1 fallback: Responses API request failed. `codewhale doctor` can still pass when non-streaming requests work; on Windows or proxy networks, try `CODEWHALE_FORCE_HTTP1=1` and rerun `codewhale`.
```
It never names the host (`api.deepseek.com`) or the underlying cause (DNS
failure or connection refused). `doctor --probe-api` gives the same opaque `✗ API
connection failed / Error details omitted because provider failures can contain
credential material.` for a bad key and for no network.

**Expected:** "Could not reach api.deepseek.com (DNS lookup failed). Check your
network or proxy." The credential-leak concern doesn't apply to a DNS or
connect error, so the cause could be shown.

---

## B5 (Medium). `codewhale auth set` silently changes the default model from `deepseek-flash` to `deepseek-v4-pro`

**Repro:** fresh user with the key in env: `codewhale doctor | grep model:`
gives `deepseek-flash`. Then `codewhale auth set --provider deepseek`; now
`doctor` shows `model: deepseek-v4-pro (resolved)` and `config.toml` contains
`default_text_model = "deepseek-v4-pro"`. `auth clear` removes the key but
leaves that line behind.

**Expected:** saving a credential doesn't change which model runs. The pricing
table in the F3 picker shows Pro at about 3× Flash's input price ($0.43 vs
$0.15 per Mtok). `docs/PROVIDERS.md:72-74` says a fresh install runs
`deepseek-flash`. Setting the key through the TUI's F3 flow keeps
`deepseek-flash`, so the two key-setup paths disagree. (The behaviour is
asserted by a unit test in `crates/cli/src/lib.rs` around line 7753, so it may
be intentional. If so, it needs to be visible to the user.)

---

## B6 (Medium). zsh completion loses sub-command context

**Repro:** follow docs §8 for zsh, then in `zsh -i` type `codewhale auth ` +
Tab, or `codew completion ` + Tab.

**Expected:** `auth` sub-commands (`set status clear …`), or the shell names.

**Actual:** zsh lists the **top-level** command list (`account app-server apply
auth cloud-agent …`) again, at any depth. Bash is correct. Fish is partly
correct: sub-commands complete, but `codewhale completion <Tab>` lists files
instead of `bash zsh fish …`.

---

## B7 (Medium). Text-mode `exec` doesn't save a session, so `exec --continue` can't find it

**Repro:** in a repo, `codewhale exec --auto "create primes.py …"` (succeeds),
then `codewhale sessions` gives `No sessions found.`, then `codewhale exec
--continue "…"` gives
`error: No saved sessions found for workspace …`.
Repeat the first step with `--output-format stream-json`: a `session_capture`
event is emitted, the session appears in `codewhale sessions`, and `exec
--continue` / `exec --resume <prefix>` both work.

**Expected** (`docs/MODES.md:333-334`, `codewhale exec --help`): `--continue`
continues "the most recent saved session for this workspace", with no mention
that only stream-json runs are saved. Either save text-mode sessions too, or
say so in `exec --help`.

---

## B8 (Low). `codewhale auth status` reports the config file as "(missing)" when it exists

After `auth set` (or the TUI F3 flow), `auth status --provider deepseek` prints
`config file: "/home/cw1/.codewhale/config.toml" (missing)`, although the file
exists and holds provider metadata. "missing" refers to the `api_key` slot.
Suggest: `config file: … (exists; no api_key)`.

## B9 (Low). The generated `config.toml` comment describes an old keybinding

The first TUI launch writes `~/.codewhale/config.toml` containing
```
# Thinking mode (DeepSeek V4 reasoning effort):
# Shift+Tab in the TUI cycles between off / high / max.
reasoning_effort = "auto"
```
In 0.10.0, Shift+Tab cycles the permission posture (Ask → Auto-Review → Full
Access), and **Ctrl-T** cycles reasoning effort (`docs/KEYBINDINGS.md`, and
observed in screenshot `09-`).

## B10 (Low). The provider picker and `doctor` disagree on the DeepSeek endpoint

`codewhale doctor` gives `base_url: https://api.deepseek.com`. The TUI F3
provider picker gives `Endpoint: https://api.deepseek.com/beta` (screenshot
`12-`). After saving a key, the TUI note says `Endpoint: api.deepseek.com`.
`docs/PROVIDERS.md:674` says the default is `/beta`. At least one of these
displays is wrong.

## B11 (Low). TUI error text wraps mid-word

The offline error in the TUI wraps as `…when non-st` / `reaming requests work…`
(screenshot `47-`). Word-wrapping should break at spaces.

## B12 (Low). The website installer downloads 160 MB before refusing an existing different install

`CODEWHALE_VERSION=v0.9.13 sh install.sh` over an existing 0.10.0 downloads
both binaries and verifies them, and only then refuses ("refusing to replace
existing …"). `check_destination` could run before the download.

## B13 (Low, security hardening). The installer accepts a plain-`http://` release base and takes the checksum manifest from the same origin

`CODEWHALE_RELEASE_BASE_URL=http://…` is accepted without warning. The
manifest comes from the same base, so the checksum only detects corruption.
It can't detect a malicious or MITM'd mirror. Suggest refusing non-`https`
bases unless an explicit `CODEWHALE_ALLOW_INSECURE_MIRROR=1` is set, or
printing a warning.

## B14 (Low). `CODEWHALE_VERSION=0.9.13 codewhale update` ignores the pin without saying so

It prints `Latest stable release: v0.10.0 … Already up to date; no download
needed.` and exits 0. Refusing to downgrade is documented and reasonable, but
the output should say "CODEWHALE_VERSION=0.9.13 is older than the installed
0.10.0; refusing to downgrade. See docs/INSTALL.md#roll-back".

## B15 (Low). The installer creates the target directory before refusing it as "managed"

`curl -fsSL https://codewhale.net/install.sh | CODEWHALE_INSTALL_DIR=$HOME/.cargo/bin sh`
prints `codewhale install: refusing managed/system directory /home/cw6/.cargo/bin`,
but by then `mkdir -p` has already created `~/.cargo/bin` (and `~/.cargo`),
left empty. That empty directory can later confuse rustup or Cargo detection.
The managed-prefix check should run on the path before `mkdir -p`.

## B16 (Medium). The declared minimum Rust version (1.88) is wrong: v0.10.0 needs **1.89**

**Repro:** with `rustup toolchain install 1.88.0`, run either
`cargo +1.88.0 install codewhale-cli --locked` or, in a `v0.10.0` checkout,
`cargo +1.88.0 install --path crates/cli --locked`.

**Actual** (8 s, before any compilation):
```
error: failed to compile `codewhale-cli v0.10.0`, …
Caused by:
  rustc 1.88.0 is not supported by the following package:
    serde-saphyr@1.3.0 requires rustc 1.89
```
The workspace declares `rust-version = "1.88"` (`Cargo.toml:40`), and
`docs/INSTALL.md:377` and `:728` say "Rust 1.88+". But `Cargo.lock` pins a
dependency that needs 1.89, and `--locked` makes that unfixable for the user.
With `+1.89.0` the build gets past the version gate, but a **source**
build (`cargo +1.89.0 install --path crates/cli --locked` in a v0.10.0
checkout) then fails after 11 min 30 s:
```
error: this lint expectation is unfulfilled
   --> crates/tui/src/prompt_zones.rs:308:30
308 | #[cfg_attr(not(test), expect(dead_code))]
   = note: `-D unfulfilled-lint-expectations` implied by `-D warnings`
error: could not compile `codewhale-tui` (lib) due to 8 previous errors
```
The eight sites are `prompt_zones.rs:308`, `tools/file_tool.rs:230`,
`tui/menu_style.rs:95/97/99/104`, `core/turn.rs:73` and `runtime_log.rs:68`.
They're all `expect(dead_code)`, which rustc 1.89's dead-code analysis
doesn't trigger. The cause is `[workspace.lints.rust] warnings = "deny"`
(`Cargo.toml:48-49`), which turns every compiler-version-specific warning into
a hard error for **anyone building from a checkout**. crates.io installs are
unaffected, because Cargo applies `--cap-lints allow` to registry
dependencies. **Fix:** CI should build with the declared MSRV. Bump `rust-version` and the
docs to whatever actually builds, or pin `serde-saphyr`. Keep
`warnings = "deny"` in CI (`RUSTFLAGS=-Dwarnings`) rather than in
`Cargo.toml`, so end-user source builds don't break on compiler drift.

Related: the repo's `rust-toolchain.toml` (`channel = "stable"`) makes rustup
silently download the latest stable (1.98.1 here) inside a checkout, even when
the user's default toolchain is older. A source build therefore never uses the
MSRV unless you pass `+<version>`, so the mismatch is invisible to maintainers.
