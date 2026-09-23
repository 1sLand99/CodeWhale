# Documentation defects: `README.md` and `docs/INSTALL.md`

Checked against `main` @ `922679d` (2026-09-23) and the published **v0.10.0**
binaries, on Ubuntu 24.04 x86_64. Line numbers refer to that checkout.
"Observed" means I ran it (see `RECEIPTS.md`). Product bugs are in `BUGS.md`.

Rough priority: **P1** blocks or misleads a new user, **P2** is wrong but
survivable, **P3** is polish.

## Top five

| # | Where | Problem |
|---|---|---|
| D1 | README.md, INSTALL.md (whole files) | **No uninstall instructions** for any Linux or macOS path, and no list of files Codewhale creates. |
| D2 | README.md:46-50 | "the launch screen says 'no model connected'" is false in v0.10.0. The TUI looks fully configured and silently swallows the first message. |
| D3 | INSTALL.md:26-39, README.md:29-34 | Doesn't say that `~/.local/bin` may not be on PATH, especially in a **new terminal window** on a desktop session, and gives no profile line to add. |
| D4 | INSTALL.md:303-316 | `npm install -g codewhale` fails with **EACCES** for any system-owned Node (distro packages, `/usr/local`, `/opt`). No prefix or nvm advice, and no "don't use sudo". |
| D5 | INSTALL.md:1170-1179 | `doctor` "checks API key" and "exits non-zero if it finds a problem". Both are false in v0.10.0. |
| (runner-up) D13 | INSTALL.md:377, :728 | "Requires Rust 1.88+" is false. A locked dependency needs **1.89**, and the build fails in 8 s on 1.88. |

---

## Full list

### D1 (P1): uninstall is undocumented
*Where:* nowhere in README.md or INSTALL.md for Linux or macOS. INSTALL.md:682-684
covers only the Windows NSIS uninstaller.
*Observed:* removing Codewhale fully means deleting:

* the binaries: `~/.local/bin/{codewhale,codew}` (installer or archive),
  `npm uninstall -g codewhale`, or `cargo uninstall codewhale-cli`;
* `~/.codewhale/` (config, **plaintext `secrets/secrets.json`**, sessions,
  logs, a 5 MB model catalog, skills, plugins, tasks, audit log, crash dumps);
* `~/.deepseek/snapshots/` (side-git copies of every workspace; see BUGS B2);
* `<each workspace>/.codewhale/` (a lock/state dir created in every repo you
  use; it shows up as untracked in `git status`);
* completion files and any PATH lines you added.

`npm uninstall -g` leaves everything except the binaries (observed: 6.3 MB
`~/.codewhale` and 220 KB `~/.deepseek` remained).
*Fix:* add an "Uninstall" section per path, plus one "Remove all data" block.
Tell users to run `codewhale auth clear --provider <p>` first if they used
`auth set`.

### D2 (P1): the "no model connected" claim is false
*Where:* README.md:46-50: "until one is connected, the launch screen says 'no
model connected'. Run `/provider` (or press F3)…"
*Observed:* no such text. The launch screen is identical to a configured one
(`DeepSeek · deepseek-flash · max` in the footer). Sending a message does
nothing (BUGS B1). The string exists in the localization table but isn't
wired up. A *wrong* key does trigger a "Getting started" flow, which the README
also doesn't mention ("it does not walk you through setup").
*Fix:* fix the product. Until then, tell users: "If nothing happens after your
first message, press **F3**. If DeepSeek shows `missing key`, press Enter to
paste it."

### D3 (P1): PATH handling after the website installer
*Where:* INSTALL.md:34-39, README.md:31-34.
*Observed:* the installer never edits shell profiles (good), and prints
`PATH selects no codewhale command…` plus a temporary `export PATH=…`. On
Ubuntu, `~/.local/bin` joins PATH only through `~/.profile`, and only if the
directory existed **when you logged in**. A new login shell over SSH picks it
up. A **new terminal window on a desktop** (tested with Ghostty, which starts
a non-login bash) does not: `bash: codewhale: command not found` until you
log out and back in (screenshot `01-tui-first-launch.png`).
*Fix:* after the curl line, add:
```bash
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc   # or ~/.zshrc
exec "$SHELL" -l
```
Also, README.md:33 (`"$HOME/.local/bin/codewhale"`) works, but it hides the
PATH problem, so the next day `codewhale` is "not found".

### D4 (P1): npm global install fails with EACCES on a system-owned Node
*Where:* INSTALL.md:303-316, and INSTALL.md:1069-1072 ("Wrapper installs but
`codewhale` isn't found").
*Observed:* with Node in `/opt/node22` (root-owned), you get
`npm error code EACCES … mkdir '/opt/node22/lib/node_modules/codewhale'`,
exit 243. The same happens with Ubuntu's `apt install nodejs`.
*Fix:* add "If you see EACCES, don't use sudo. Either use nvm/fnm/volta, or run
`npm config set prefix ~/.npm-global` and add `~/.npm-global/bin` to PATH."
(I verified the prefix route.)

### D5 (P1): doctor's claimed behaviour
*Where:* INSTALL.md:1174 ("checks API key, provider, runtime, and PATH
integrity") and :1178 ("exits non-zero if it finds a problem").
*Observed:* with no key, and with a wrong key plus `--probe-api`, the exit
status is **0** and the output ends in "All checks complete!". The key section
reads `env_source=not inspected` even when the env var is set. Live checks
need the opt-in `--probe-api`, which isn't mentioned.
*Fix:* document `codewhale auth status` as **the** way to see the active key
source, and `codewhale doctor --probe-api` for a live check. Remove "exits
non-zero" until it's true.

### D6 (P2): "prebuilt binary needs the runtime library `libdbus-1`"
*Where:* INSTALL.md:395-398.
*Observed:* `file codewhale` reports `static-pie linked`, `ldd` says
`statically linked`, and there is no libdbus reference. §1 (INSTALL.md:149-154)
correctly says these are static musl builds. This also means prebuilt Linux
binaries **cannot use the Secret Service keyring**: `doctor` shows
`Secret Backend: backend: file`, and keys land in plaintext
`~/.codewhale/secrets/secrets.json` (mode 0600).
*Fix:* delete the runtime-libdbus sentence. State plainly that keys saved with
`auth set` are stored in a 0600 plaintext file on Linux.

### D7 (P2): no instructions for installing the bare binaries
*Where:* INSTALL.md:266-282 (§2) and :549-577 (§6).
*Observed:* §2 verifies `codewhale-linux-x64` and `codew-linux-x64`, but no
section says to rename them, `chmod +x`, and move them onto PATH. §6 shows
only the archive, and only for **Linux ARM64** (INSTALL.md:559-569), although
x86_64 is far more common. INSTALL.md:555 says "the archive is the easiest
manual install (see §6)" while already inside §6 (a self-reference).
*Fix:* give an x64 example and add the three `install -m 755` lines (see the
guide).

### D8 (P2): npm progress output "is printed"
*Where:* INSTALL.md:325-327.
*Observed:* npm 10 hides lifecycle-script output. A plain
`npm install -g codewhale` prints only `added 1 package in 5s`. The progress
lines appear only with `--foreground-scripts`.
*Fix:* "…printed when you pass `--foreground-scripts`, and always written to
`<binary>.source`".

### D9 (P2): rollback guidance is muddled
*Where:* INSTALL.md:54-59 ("`CODEWHALE_VERSION` pins the mirror version… An
explicit version or mirror cannot bypass the version check") and :579-606.
*Observed:* `CODEWHALE_VERSION=0.9.13 codewhale update` silently says "Already
up to date" (BUGS B14). §6's rollback block gives commands for npm and Cargo,
then only the *checksum-manifest* download URLs for manual installs, with no
actual install command. What works (tested):
`rm ~/.local/bin/codewhale ~/.local/bin/codew && curl -fsSL https://codewhale.net/install.sh | CODEWHALE_VERSION=v0.9.13 sh`,
or the same with `CODEWHALE_INSTALL_DIR=<fresh dir>`.
*Fix:* put the installer-based rollback first, and say that `CODEWHALE_VERSION`
on `update` doesn't downgrade.

### D10 (P2): no mention of files written inside your repo
*Where:* anywhere.
*Observed:* the first tool turn in a repo creates `<repo>/.codewhale/state/…`,
which shows as untracked (`?? .codewhale/`). The docs should suggest adding it
to `.gitignore` or a global gitignore.

### D11 (P2): `exec --continue` only works after a stream-json run
*Where:* README.md:67-71 introduces `codewhale exec`. `docs/MODES.md:333-334`
documents `exec --resume` and `--continue`.
*Observed:* text-mode `exec`/`exec --auto` doesn't save a session, so
`exec --continue` reports "No saved sessions found". See BUGS B7.

### D12 (P2): telemetry isn't mentioned in the install docs
*Where:* README.md and INSTALL.md.
*Observed:* `codewhale --help` says `--telemetry … (default on; Codewhale +
PostHog; durable off: config set telemetry false; CODEWHALE_TELEMETRY=0 always
wins)`. The TUI also checks for updates at startup (`[update]
check_for_updates = true` is written to config), while `doctor` says update
checks are an "offline default". A privacy-conscious installer wants a
one-line opt-out in the install docs.

### D13 (P2): Cargo section omits the rustup step and doesn't say a build takes ~X minutes
*Where:* INSTALL.md:365-398, :719-749.
*Observed:* see RECEIPTS Path 4 for the build time and disk usage on 4 vCPU.
* `libdbus-1-dev` **is** required. I confirmed `libdbus-sys` fails without it,
  so the docs are right about that.
* **"Requires Rust 1.88+" (INSTALL.md:377, :728) is false.** With 1.88.0,
  both `cargo install codewhale-cli --locked` and `cargo install --path
  crates/cli --locked` fail at once with
  `serde-saphyr@1.3.0 requires rustc 1.89` (BUGS B16).
* `cargo install` from crates.io took **25 min 31 s** on 4 vCPU (with another
  build competing for CPU). It pulled 474 MB into `~/.cargo/registry`, and the
  resulting `~/.cargo/bin/codewhale` is 122 MB and dynamically linked
  (`libdbus-1.so.3`).
* Cargo installs only `codewhale`, as the docs correctly say.
* The Cargo build's `--version` prints `codewhale 0.10.0` with **no commit
  hash**, unlike the release binaries.
* Inside a git checkout, `rust-toolchain.toml` (`channel = "stable"`) makes
  rustup download the latest stable automatically. That's worth one sentence,
  because it's a surprise ~250 MB download on first `cargo` use.
* A checkout build (`cargo install --path crates/cli --locked`) works only on
  the repo-selected latest stable. It took 83 min and left 3.1 GB of `target/`.
  On 1.89 it fails with `-D warnings` lint errors (BUGS B16).
*Fix:* say "current stable" (or fix the lock), give the expected build time and disk use,
and show `source "$HOME/.cargo/env"` after rustup.

### D14 (P1 for Homebrew, P2 for Nix): Homebrew and Nix sections
*Where:* INSTALL.md:532-545 (Homebrew), :451-505 (Nix).
* **Homebrew (P1):** `brew tap Hmbown/deepseek-tui` then `brew install
  codewhale` **fails on Homebrew 7.x** with `Error: Refusing to load formula
  hmbown/deepseek-tui/codewhale from untrusted tap hmbown/deepseek-tui.`
  *Fix:* document `brew install Hmbown/deepseek-tui/codewhale` (it
  auto-trusts; tested), or add `brew trust hmbown/deepseek-tui`. The formula
  name, the tap name, the node dependency, providing `codew`, and the version
  (0.10.0, no lag) are all correct. Worth adding: node pulls in ~31 bottles
  (~560 MB) on Linux, and `brew uninstall` autoremoves them.
  `codewhale update --check` on a Homebrew-only install calls it a
  "secondary Homebrew installation" and pushes a curl migration first.
* **Nix (P2):** INSTALL.md:461 says it "starts the dispatcher". There has been
  no separate dispatcher since v0.9.5. The docs don't say that:
  * there is **no binary cache**;
  * `nix run` builds **main-branch source** (the binary reports `0.10.0 (dev)`),
    not the release;
  * `doCheck` then runs the whole test suite. Build plus check took well over
    an hour here, with 8 GB+ for one rustc, and didn't finish;
  * there is **no `codew`**.
  *Fix:* say all four, or set `doCheck = false` for the user-facing package
  and pin the flake to release tags.

### D15 (P3): Windows docs vs. published assets (not executed, checked by inspecting the assets)
* INSTALL.md:639-640 points at `packaging/winget/Hmbown.CodeWhale.yaml`, which
  is **`PackageVersion: 0.9.6`** with v0.9.6 URLs, two releases behind v0.10.0.
* INSTALL.md:642 calls `codewhale-windows-x64.zip` the "portable ZIP". v0.10.0
  publishes both `codewhale-windows-x64.zip` (includes `install.bat`) **and**
  `codewhale-windows-x64-portable.zip` (no installer). The docs never mention
  the `-portable.zip` asset or `install.bat`.
* `install.bat` (inside the zip) copies to `%USERPROFILE%\bin`, which isn't
  documented. It also tells users to run "an admin PowerShell" to change the
  *user* PATH, which doesn't need admin.
* The stand-alone `codewhale.bat` release asset launches
  `%~dp0codewhale-windows-x64.exe`, so it only works next to the bare **x64**
  exe. That's consistent with INSTALL.md:330-333, but ARM64 users who download
  it get a broken launcher.

### D16 (P3): macOS and Android docs vs. assets (inspected, not executed)
* `codewhale-macos-arm64.tar.gz` contains `codewhale`, `codew` and `install.sh`,
  consistent with INSTALL.md:572-577.
* `codewhale-android-arm64.tar.gz` contains the same `install.sh` (6898 bytes,
  identical to the Linux one), which honours `PREFIX`. That's consistent with
  INSTALL.md:198-213. `codewhale-bundles-sha256.txt` covers it.
* I found no contradictions for these two, apart from the uninstall gap (D1).

### D17 (P3): stale or odd statements
* INSTALL.md:13-16: "This branch describes the **v0.10.0 source candidate**…
  A candidate is not a published install until…". v0.10.0 **is** published
  (2026-09-22), so this banner is stale.
* INSTALL.md:305: npm needs "Node 18+". The package's `engines` agrees
  (`>=18`), but INSTALL.md:19-21 says Computer Use needs Node 20+. Worth one
  sentence explaining that the two numbers differ.
* INSTALL.md:1015-1043 (edition2024 with Ubuntu's Cargo 1.75) is plausible, but
  I didn't reproduce it; this VM used rustup.
* README.md:55 and INSTALL.md:879-880: "each script completes **both**
  `codewhale` and the `codew` shorthand". True for command-name registration,
  but zsh completion is broken below the first word (BUGS B6).
* INSTALL.md:1174 comment `# checks API key, provider, runtime, and PATH
  integrity`: see D5.
