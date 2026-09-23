# Codewhale install receipts

Every command below was run on this VM. Output is trimmed (`…`) but not edited.
The DeepSeek key is shown as `sk-…REDACTED`. Codewhale's own "last4" key
hints are also replaced with `...XXXX`. Full raw logs are in `logs/`.

## Results at a glance

| # | Path | Result | Time |
|---|---|---|---|
| 1 | `curl -fsSL https://codewhale.net/install.sh \| sh` | **PASS**. Checksums verified, idempotent re-run, refuses over a different version, tamper detected | 5.9 s |
| 2a | Manual bare binaries + `codewhale-artifacts-sha256.txt` | **PASS**. The docs lack the install/rename step | 4 s |
| 2b | Manual archive + `codewhale-bundles-sha256.txt` + `./install.sh` | **PASS** | 7 s |
| 3 | `npm install -g codewhale` | **FAIL as documented** (EACCES with a root-owned Node). **PASS** with a user npm prefix | 6 s |
| 4a | `cargo install codewhale-cli --locked` | **PASS** on stable 1.98.1 with `libdbus-1-dev`. **FAIL** on the documented minimum Rust 1.88 | 25.5 min |
| 4b | `git clone` + `cargo install --path crates/cli --locked` | **PASS** on repo-selected stable. **FAIL** on 1.88 and on 1.89 | 83 min |
| 5a | Homebrew (Linux) | **FAIL as documented** (untrusted tap). **PASS** with `brew install Hmbown/deepseek-tui/codewhale` | 73 s |
| 5b | Nix `nix run github:Hmbown/CodeWhale` | **PARTIAL / not finished**. The build ran more than 105 min (source + tests), plus a proxy workaround | – |
| 6 | `codewhale update --check` / `update` (0.9.13 → 0.10.0), rollback to 0.9.13 | **PASS**. The update pin is silently ignored; rollback works via the installer | 10 s |
| 7 | Completions bash / zsh / fish | bash **PASS**, fish **PARTIAL**, zsh **FAIL** below the first word | – |
| 8 | Uninstall | **PASS** for the programs. Data is left in `~/.codewhale`, `~/.deepseek` and `<repo>/.codewhale`, and none of it is documented | – |
| – | TUI in Ghostty 1.3.1 (paths 1, 2, 3, 4a) | **PASS** with a key. **FAIL** (silent) with no key | – |
| – | Headless `exec`, resume | **PASS**. Text-mode `exec` isn't saved, so `--continue` needs stream-json | – |
| – | Failure paths: no key / wrong key / no network | exec: clear / OK / misleading. TUI: silent / onboarding / misleading | – |

Estimated total DeepSeek spend: under $0.05 (the TUI footer
showed $0.0017–$0.0040 per session).

## Test machine

| Item | Value |
|---|---|
| OS | Ubuntu 24.04.4 LTS (Noble), kernel 6.18.44, x86_64 |
| CPU / RAM / disk | 4 vCPU, 15 GiB RAM, ~30 GiB free |
| glibc | 2.39 |
| Preinstalled | git 2.43, curl, Node 22.22.2 / npm 10.9.7 (`/opt/node22`), rustup Rust 1.94.1 (root only), python3 |
| Network | All HTTPS goes through an egress proxy (`HTTPS_PROXY`) with its own CA. GitHub is reachable only for `Hmbown/CodeWhale`. `ghostty.org` fails TLS. |
| Date | 2026-09-23 |
| Release under test | v0.10.0 (`codewhale 0.10.0 (1be1a703b975)`), published 2026-09-22T17:28:34Z |

### How "clean state" was achieved

Each install path runs as a **brand-new Linux user** (`cw1`, `cw2`, …) created
with `useradd -m`. The account gets a pristine `$HOME` from `/etc/skel` and the
stock Ubuntu `~/.profile`, and I use a real login shell (`bash -l`). The wrapper
(`/usr/local/bin/cwrun`) starts from `env -i` and passes only these variables:

* `HOME USER LOGNAME SHELL TERM LANG PATH`: the system default PATH only.
  `~/.local/bin` is added by Ubuntu's stock `~/.profile`, not by me.
* `HTTPS_PROXY`/`NO_PROXY`, plus `SSL_CERT_FILE`/`NODE_EXTRA_CA_CERTS`
  pointing at a world-readable copy of the proxy CA. **This is a VM artefact**,
  and a normal user does not need it.
* `CODEWHALE_TELEMETRY=0`, so the test VM does not pollute product analytics.
  Telemetry is **on by default** (`codewhale --help`: "default on; Codewhale +
  PostHog").
* `DEEPSEEK_API_KEY` **only** when a scenario needs the env-var key
  (`WITHKEY=1`).
* "No network" scenarios run inside an empty network namespace
  (`unshare -n`), so there is no DNS and no route.

All output is piped through a redaction filter before it is saved.

---

## Path 1: recommended installer (`curl -fsSL https://codewhale.net/install.sh | sh`)

**Result: PASS.** Install took 5.9 s (160 MB download).

User `cw1`, fresh home, nothing on PATH.

```console
$ command -v codewhale codew          # (nothing, exit 1)
$ curl -fsSL https://codewhale.net/install.sh | sh
Installing Codewhale for linux-x64
Release assets: https://github.com/Hmbown/CodeWhale/releases/latest/download
Install dir: /home/cw1/.local/bin
Checksums verified
Installed checksummed release commands:
  /home/cw1/.local/bin/codewhale
  /home/cw1/.local/bin/codew

Use this installation: "/home/cw1/.local/bin/codewhale"
Future updates: "/home/cw1/.local/bin/codewhale" update
PATH selects no codewhale command; this install is /home/cw1/.local/bin/codewhale
PATH selects no codew command; this install is /home/cw1/.local/bin/codew
To use this directory in the current shell, then verify the commands:
  export PATH="/home/cw1/.local/bin:$PATH"
  hash -r
  command -v codewhale codew
Keep the directory first in your shell profile after verifying it.
exit=0     real 0m5.868s
```

The installer script is 345 lines, sha256 `57db92eb…6042f` on 2026-09-23. It
**never edits shell profiles**. In a *new* login shell, Ubuntu's stock
`~/.profile` picks up `~/.local/bin` because the directory now exists:

```console
$ command -v codewhale codew
/home/cw1/.local/bin/codewhale
/home/cw1/.local/bin/codew
$ ls -la ~/.local/bin
-rwxr-xr-x 1 cw1 cw1 81753984 codew
-rwxr-xr-x 1 cw1 cw1 81753984 codewhale
$ cmp ~/.local/bin/codewhale ~/.local/bin/codew && echo IDENTICAL
IDENTICAL                                  # two full 78 MiB copies, not a link
$ codewhale --version ; codew --version
codewhale 0.10.0 (1be1a703b975)
codewhale 0.10.0 (1be1a703b975)
```

`$HOME` after the install contained only `~/.local/bin/{codewhale,codew}`.

**Re-run over the same version: PASS (idempotent).** It downloads again, then:
```
Checksums verified
Already installed: /home/cw1/.local/bin/codewhale
Already installed: /home/cw1/.local/bin/codew
… exit=0
```

**Re-run with a different version (`CODEWHALE_VERSION=v0.9.13`) over 0.10.0:
PASS (refuses and leaves files alone).** It downloads 160 MB first, then:
```
codewhale install: refusing to replace existing /home/cw1/.local/bin/codewhale.
It may be a newer build, another installation, or a symlink. No existing file was changed.
For an existing direct Codewhale install, run its full path with 'update'.
To migrate safely from a package manager or mixed installation, create a fresh directory:
  …
exit=1          # codewhale --version still 0.10.0
```

**Checksum verification: PASS.** I served the real v0.10.0 manifest plus a
`codew-linux-x64` with one appended byte from a local HTTP mirror:
```console
$ curl -fsSL https://codewhale.net/install.sh | CODEWHALE_RELEASE_BASE_URL=http://127.0.0.1:8765 CODEWHALE_INSTALL_DIR=$HOME/tamper-test sh
Installing Codewhale for linux-x64
Release assets: http://127.0.0.1:8765
Install dir: /home/cw1/tamper-test
codewhale install: checksum mismatch for codew-linux-x64
exit=1          # ~/tamper-test was never created
```
(The installer accepts a plain-`http://` base URL. The checksum manifest comes
from that same URL, so the check only protects against corruption, not against
a hostile mirror. See BUGS.md.)

### Path 1 smoke test

#### `codewhale doctor` with **no key**: runs, exit 0, does **not** flag the missing key
```
codewhale Doctor
==================
Version Information:
  codewhale-tui: 0.10.0 (1be1a703b975)
  rust: unknown
Updates:
  · latest: unknown (not checked; offline default)
Configuration:
  ! config.toml not found at ~/.codewhale/config.toml (using defaults/env)
…
Setup State:
  · credential: source=secret_store_unprobed, availability=not_probed
  ! first-run: needs action
…
API Keys:
  · deepseek: env_source=not inspected, config_source=not declared
  … (50 more provider rows)
API Connectivity:
  · provider: deepseek
  · base_url: https://api.deepseek.com
  · model: deepseek-flash (resolved)
  · Live hosted connectivity not checked (offline default)
…
Platform:
  ! sandbox not available (commands run best-effort)

All checks complete!
exit=0     real 0.76s
```
`codewhale doctor --json` also exits 0 and gives
`"api_key": {"source": "secret_store_unprobed", "availability": "not_probed"}`
and `"api_connectivity": {"checked": false, "status": "not_probed", …}`.

#### `codewhale auth status --provider deepseek` (no key): clear
```
provider: deepseek (active provider)
auth mode: api_key
active source: missing
lookup order: config -> secret store -> env
config file: "/home/cw1/.codewhale/config.toml" (missing)
secret store: file-based (~/.codewhale/secrets/) (missing)
env var: DEEPSEEK_API_KEY (unset)
…
exit=0
```

#### `codewhale exec "say hi"` with no key: good message, exit 1
```
error: DeepSeek API key not found.

1. Get a key:  https://platform.deepseek.com/api_keys
2. Save it (works in every folder, no OS prompts):
codewhale auth set --provider deepseek

Alternatives:
• export DEEPSEEK_API_KEY=<your-key>      (current shell only;
also note: zsh users — exports in ~/.zshrc only reach interactive
shells, prefer ~/.zshenv for everything)
• api_key = "<your-key>"  in ~/.codewhale/config.toml
• already configured DeepSeek Harness? grant read-only access:
codewhale auth external-consent --provider deepseek --mode read-only
exit=1
```

#### Key in the environment (`DEEPSEEK_API_KEY=sk-…REDACTED`)
```
$ codewhale auth status --provider deepseek
active source: env (last4: ...XXXX)
env var: DEEPSEEK_API_KEY (set, last4: ...XXXX)
$ codewhale doctor | grep -A1 'API Keys'
  · deepseek: env_source=not inspected, config_source=not declared   # doctor still doesn't say
$ codewhale doctor --probe-api
  · Testing connection...  ✓ API connection successful          exit=0
```

#### Headless: `codewhale exec`, in a scratch git repo (`~/primes`, `git init`): PASS
```console
$ codewhale exec "Reply with exactly: pong"
pong                                   exit=0  real 1.5s
$ codewhale exec --auto "create a Python file primes.py that prints the first 10 primes, run it, and show me the output"
tool: bash (command: ls -la /home/cw1/primes)
tool bash completed: …
tool: write (path: /home/cw1/primes/primes.py, content: <286 chars>)
tool write completed: Successfully wrote 287 bytes to /home/cw1/primes/primes.py
tool: bash (command: cd /home/cw1/primes && python3 primes.py)
tool bash completed: 2 3 5 7 11 13 17 19 23 29

`primes.py` created at `/home/cw1/primes/primes.py`, run with `python3 primes.py`.
Output:
2 3 5 7 11 13 17 19 23 29
exit=0  real 6.3s
$ git status --short
?? .codewhale/          # Codewhale creates this in the workspace (state/subagents.v1.lock)
?? primes.py
$ codewhale exec --json "Reply with exactly: pong"
{ "mode": "one-shot", "provider": "deepseek", "model": "deepseek-flash", "success": true, "output": "pong", … }
```

#### Resume: **text-mode `exec` does not save a session**, `stream-json` does
```console
$ codewhale sessions                          # after the two text-mode exec runs above
No sessions found.
$ codewhale exec --continue "What file did you create in the previous turn?"
error: No saved sessions found for workspace /home/cw1/primes. Use `codewhale sessions` to list sessions, or pass `codewhale exec --resume <SESSION_ID> ...`.
exit=1
$ codewhale exec --auto --output-format stream-json "Run: python3 primes.py and report the output."
… {"type":"session_capture", "saved_session_id":"e8ff834b-daa7-4bad-ba38-d771c14c11d1", …}
$ codewhale sessions
  * e8ff834b | Run: python3 primes.py and report the... | 4 msgs | 2026-09-23 15:39 UTC (just now)
$ codewhale exec --continue "What command did you just run? One short sentence."
resumed session: e8ff834b
`cd /home/cw1/primes && python3 primes.py`
session: e8ff834b                              exit=0
$ codewhale exec --resume e8ff834b "Reply with exactly: resumed-ok"
resumed session: e8ff834b
resumed-ok                                     exit=0
```

#### Failure paths
Wrong key (`DEEPSEEK_API_KEY=sk-000…dead`):
```
$ codewhale exec "say hi"
error: Responses API request failed
  caused by: Authentication failed: Authentication Fails, Your api key: ****dead is invalid
exit=1   real ~1s
$ codewhale doctor --probe-api
  · Testing connection...  ✗ API connection failed
    Error details omitted because provider failures can contain credential material.
exit=0          # a failed probe still exits 0
```
No network (empty netns, correct key):
```
$ codewhale exec "say hi"
error: Network error: SSE stream request failed after HTTP/1.1 fallback: Responses API request failed. `codewhale doctor` can still pass when non-streaming requests work; on Windows or proxy networks, try `CODEWHALE_FORCE_HTTP1=1` and rerun `codewhale`.
exit=1   (exec + doctor together took 28 s)
$ codewhale doctor --probe-api
  · Testing connection...  ✗ API connection failed
    Error details omitted because provider failures can contain credential material.
```
Verdict: the no-key message is excellent. The wrong-key message is
understandable ("api key … is invalid") but leads with "Responses API request
failed", and never suggests `codewhale auth status` or `auth set`. The offline
message never says "cannot reach api.deepseek.com / no network". It points at
HTTP/1.1, Windows and proxies instead. `doctor --probe-api` gives the same
opaque "API connection failed" for both a bad key and no network.

### Credential sources (tested on `cw1`)

| Method | Command | `auth status` "active source" | Real request | Result |
|---|---|---|---|---|
| env var | `export DEEPSEEK_API_KEY=…` | `env` | `exec` → `pong` | PASS |
| `auth set` (flag) | `printf '%s\n' "$KEY" \| codewhale auth set --provider deepseek --api-key-stdin` | `secret store` | yes | PASS |
| `auth set` (documented bare form, key piped) | `codewhale auth set --provider deepseek` → prints `Enter API key for deepseek:` | `secret store` | – | PASS |
| config file | `[providers.deepseek]` `api_key = "…"` in `~/.codewhale/config.toml` | `config` | `exec` → `pong` | PASS |
| CLI flag | `codewhale --api-key "$KEY" exec …` | – | `pong` (with a wrong key in config) | PASS: the flag beats config |
| precedence | wrong key in config **plus** correct `DEEPSEEK_API_KEY` | `config (last4: ...beef)` | `Authentication Fails, Your api key: ****beef is invalid` | Config wins over env, as documented. This is a trap for users. |
| `auth clear` | `codewhale auth clear --provider deepseek` | `missing` | – | PASS: `secrets.json` becomes `{"entries": {}}` |

`auth set` output and on-disk effect:
```
saved API key for deepseek to file-based (~/.codewhale/secrets/) (config contains metadata only)
$ stat -c '%a %n' ~/.codewhale/config.toml ~/.codewhale/secrets/secrets.json
600 /home/cw1/.codewhale/config.toml
600 /home/cw1/.codewhale/secrets/secrets.json
$ cat ~/.codewhale/config.toml
default_text_model = "deepseek-v4-pro"      # <- written by `auth set`; the default was deepseek-flash
provider = "deepseek"
auth_mode = "api_key"

[providers.deepseek]
auth_mode = "api_key"
$ cat ~/.codewhale/secrets/secrets.json
{"entries": {"deepseek": "sk-…REDACTED"}}     # plaintext, 0600; no keyring on a headless box
$ codewhale doctor | grep model:
  · model: deepseek-v4-pro (resolved)
```
After `auth set`, `auth status` still prints
`config file: "/home/cw1/.codewhale/config.toml" (missing)` even though the file
exists. "missing" refers to the key slot, not the file. `auth clear` removes the
key but leaves `default_text_model = "deepseek-v4-pro"` behind.

---

## Path 2: manual download from GitHub Releases

### 2a. Bare binaries + `codewhale-artifacts-sha256.txt`: **PASS** (download and verify took 4.1 s)

User `cw2`, fresh home. These are the docs §2 commands, preceded by the
download:
```console
$ mkdir -p ~/dl && cd ~/dl
$ curl -fsSL -O https://github.com/Hmbown/CodeWhale/releases/latest/download/codewhale-linux-x64 \
             -O https://github.com/Hmbown/CodeWhale/releases/latest/download/codew-linux-x64
$ curl -L -O https://github.com/Hmbown/CodeWhale/releases/latest/download/codewhale-artifacts-sha256.txt
$ sha256sum -c codewhale-artifacts-sha256.txt --ignore-missing
codew-linux-x64: OK
codewhale-linux-x64: OK                      exit=0
$ wc -l codewhale-artifacts-sha256.txt
33 codewhale-artifacts-sha256.txt
```
The docs never say how to *install* the bare binaries (rename, chmod, and
where to put them). I used:
```console
$ mkdir -p ~/.local/bin
$ install -m 755 codewhale-linux-x64 ~/.local/bin/codewhale
$ install -m 755 codew-linux-x64     ~/.local/bin/codew
# new login shell:
$ command -v codewhale codew
/home/cw2/.local/bin/codewhale
/home/cw2/.local/bin/codew
$ codewhale --version
codewhale 0.10.0 (1be1a703b975)
```

### 2b. Platform archive + `codewhale-bundles-sha256.txt`: **PASS** (7.3 s)

User `cw2b`. These are the docs §6 commands with `linux-arm64` swapped for `linux-x64`:
```console
$ codewhale_archive_dir="$(mktemp -d)"; cd "$codewhale_archive_dir"
$ curl -fsSLO https://github.com/Hmbown/CodeWhale/releases/latest/download/codewhale-linux-x64.tar.gz
$ curl -fsSLO https://github.com/Hmbown/CodeWhale/releases/latest/download/codewhale-bundles-sha256.txt
$ sha256sum -c codewhale-bundles-sha256.txt --ignore-missing
codewhale-linux-x64.tar.gz: OK
$ tar -xzf codewhale-linux-x64.tar.gz && ls codewhale-linux-x64
codew  codewhale  install.sh
$ cd codewhale-linux-x64 && ./install.sh
Installing codewhale to /home/cw2b/.local/bin ...
  /home/cw2b/.local/bin/codewhale
  /home/cw2b/.local/bin/codew

Done. Commands installed to /home/cw2b/.local/bin.
Future updates: "/home/cw2b/.local/bin/codewhale" update
PATH selects no codewhale command; this install is /home/cw2b/.local/bin/codewhale
…
install-exit=0
```

### Path 2 smoke test (both users): PASS
```
command -v → ~/.local/bin/codewhale, ~/.local/bin/codew
codewhale --version → codewhale 0.10.0 (1be1a703b975)
codewhale doctor --json → exit 0
codewhale exec --auto "create hello.py that prints hello from codewhale, run it, show output"
  tool: write (path: /home/cw2/p/hello.py …)  tool: bash (python3 hello.py) → hello from codewhale   exit=0
codewhale exec "Reply with exactly: pong" → pong
```

---

## Path 3: npm (`npm install -g codewhale`)

Registry metadata: `codewhale@0.10.0`, `dist-tags.latest = 0.10.0`,
`engines.node >= 18`, and bins `codewhale` and `codew`.

### 3a. As documented: **FAIL** on this machine (a common setup: Node in a root-owned prefix)
User `cw3`. Node 22.22.2 / npm 10.9.7 in `/opt/node22`, which root owns.
```console
$ npm prefix -g
/opt/node22
$ npm install -g codewhale
npm error code EACCES
npm error syscall mkdir
npm error path /opt/node22/lib/node_modules/codewhale
npm error Error: EACCES: permission denied, mkdir '/opt/node22/lib/node_modules/codewhale'
…
npm error the command again as root/Administrator.
exit=243
```
This is the same failure every distro-packaged or system-wide Node gives. I did
**not** use sudo. INSTALL.md §3 does not mention the problem.

### 3b. With a per-user npm prefix: **PASS** (5.9 s)
```console
$ npm config set prefix "$HOME/.npm-global"
$ echo 'export PATH="$HOME/.npm-global/bin:$PATH"' >> ~/.profile
$ export PATH="$HOME/.npm-global/bin:$PATH"
$ npm install -g codewhale
added 1 package in 5s
exit=0
# new login shell
$ command -v codewhale codew
/home/cw3/.npm-global/bin/codewhale
/home/cw3/.npm-global/bin/codew
$ ls -la ~/.npm-global/bin
codew -> ../lib/node_modules/codewhale/bin/codew.js
codewhale -> ../lib/node_modules/codewhale/bin/codewhale.js
$ du -sh ~/.npm-global/lib/node_modules/codewhale
157M
$ codewhale --version
codewhale 0.10.0 (1be1a703b975)
$ cat ~/.npm-global/lib/node_modules/codewhale/bin/downloads/codewhale.source
source=github
label=GitHub Releases
base=https://github.com/Hmbown/CodeWhale/releases/download/v0.10.0/
version=0.10.0
```
npm ≥ 7 hides lifecycle-script output, so the progress lines that INSTALL.md
says are printed only show up with `--foreground-scripts`:
```console
$ CODEWHALE_FORCE_DOWNLOAD=1 npm install -g codewhale --foreground-scripts
> codewhale@0.10.0 postinstall
> node scripts/install.js --optional
codewhale: probing GitHub Releases and CNB first-party mirror checksum manifests
codewhale: selected GitHub Releases for v0.10.0
codewhale: downloading codewhale-linux-x64 from GitHub Releases: 5 / 78 MB (6%)
…
codewhale: codew-linux-x64 from GitHub Releases ... done.
changed 1 package in 6s
```

### Path 3 smoke test: PASS
`doctor --json` exit 0; `exec "Reply with exactly: pong"` → `pong`.

`codewhale update --check` under npm exits 0. It prints the "managed by npm;
in-place self-update is disabled" message, the fresh-directory migration
recipe, and `Already up to date.` Plain `codewhale update` prints the same text
and then
`error: The package-managed executable was not changed.` with exit 1.

---

## Path 6: `codewhale update --check`, `codewhale update`, and rollback to v0.9.13

User `cw6`. **All PASS.**

```console
$ curl -fsSL https://codewhale.net/install.sh | CODEWHALE_VERSION=v0.9.13 sh
Installing Codewhale for linux-x64
Release assets: https://github.com/Hmbown/CodeWhale/releases/download/v0.9.13
… Checksums verified …
$ codewhale --version
codewhale 0.9.13 (a0b81f619b66)
$ codewhale update --check
Checking for stable updates...
Current binary: /home/cw6/.local/bin/codewhale
Current version: v0.9.13
Latest stable release: v0.10.0
Update available. Run `/home/cw6/.local/bin/codewhale update` to install v0.10.0.
Release source: GitHub Releases                 exit=0  real 1.8s
$ codewhale update
Probing codewhale-artifacts-sha256.txt for v0.10.0 from GitHub Releases and CNB mirror...
Release source: GitHub Releases
Downloading codewhale-linux-x64...
SHA256 checksum verified against codewhale-artifacts-sha256.txt from GitHub Releases.

✅ Successfully updated to v0.10.0!
Updated binaries:
  - /home/cw6/.local/bin/codewhale (codewhale-linux-x64)
  - /home/cw6/.local/bin/codew (codewhale-linux-x64)
Restart the application to use the new version.     exit=0  real 10.0s
$ codewhale --version; codew --version
codewhale 0.10.0 (1be1a703b975)
codewhale 0.10.0 (1be1a703b975)
$ codewhale update
Already up to date; no download needed.        exit=0
```

Rollback attempts:
```console
$ CODEWHALE_VERSION=0.9.13 codewhale update     # the pin is silently ignored
Current version: v0.10.0
Latest stable release: v0.10.0
Already up to date; no download needed.        exit=0   (still 0.10.0)

# Documented method: separate directory, first on PATH
$ curl -fsSL https://codewhale.net/install.sh | CODEWHALE_VERSION=v0.9.13 CODEWHALE_INSTALL_DIR=$HOME/.local/codewhale-0.9.13 sh
… Installed checksummed release commands:
  /home/cw6/.local/codewhale-0.9.13/codewhale
$ echo 'export PATH="$HOME/.local/codewhale-0.9.13:$PATH"' >> ~/.profile
# new shell
$ codewhale --version
codewhale 0.9.13 (a0b81f619b66)

# Simpler in-place method (also works):
$ rm ~/.local/bin/codewhale ~/.local/bin/codew
$ curl -fsSL https://codewhale.net/install.sh | CODEWHALE_VERSION=v0.9.13 sh     exit=0
$ codewhale --version
codewhale 0.9.13 (a0b81f619b66)
$ codewhale update && codewhale --version
codewhale 0.10.0 (1be1a703b975)
```
`codewhale update --help` lists only `--beta`, `--check`, and `--proxy`. There is
no `--version` or downgrade flag.

---

## Path 7: shell completions

All five generators work, and the `completions` alias gives byte-identical output:
```
bash exit=0 bytes=200349   zsh exit=0 bytes=154402   fish exit=0 bytes=112002
powershell exit=0 bytes=133405   elvish exit=0 bytes=71209
$ codewhale completions bash | cmp - <(codewhale completion bash) && echo alias-ok
alias-ok
```
I installed them with the exact docs §8 commands, then drove real interactive
shells in tmux (type a prefix, press Tab, capture the screen):

| Shell | `codewhale comp<Tab>` | `codewhale completion <Tab>` | `codew au<Tab>` then `<Tab>` | Verdict |
|---|---|---|---|---|
| bash 5.2 + bash-completion | → `completion` | `bash elvish fish powershell zsh -h --help` | → `auth`, then lists `xai-device chatgpt … set status get clear list migrate` | **PASS** |
| zsh 5.9 | → `completion` | lists **top-level commands** (`account app-server apply auth …`) | → `auth`, then lists **top-level commands** again | **FAIL**: sub-command context is lost |
| fish 3.7 | → `completion` | lists **files in cwd**, not shell names | → `auth`, then lists `list migrate print-api-key set status xai-device` with descriptions | **PARTIAL** |

zsh capture:
```
vm% codewhale auth
account         cloud       -- Sign in to your Codewhale account and manage account-scoped provider keys
app-server                  -- Run the canonical runtime API / control plane (HTTP/SSE, mobile, stdio)
apply                       -- Apply a patch file or stdin to the working tree
auth                        -- Manage authentication credentials and provider mode
…
```

---

## Ghostty setup (the terminal under test)

**Result: Ghostty 1.3.1 built from source and running on Xvfb + openbox.** This
is the real Ghostty, not a stand-in. It took about 12 minutes; details are in
`logs/ghostty-build.log`.

* There's no Ghostty package for Ubuntu 24.04. Ubuntu's own `ghostty_1.3.0` .deb
  (from a newer series) needs `libc6 >= 2.43`, and this VM has 2.39. github.com
  release downloads are blocked by the VM proxy, and `ghostty.org` fails TLS
  here.
* Build: the source tarball from `release.files.ghostty.org/1.3.1/`, Zig
  0.15.2 from ziglang.org, and apt `libgtk-4-dev libadwaita-1-dev gettext
  libxml2-utils blueprint-compiler libgl1-mesa-dri libonig-dev libbz2-dev`.
  `zig build -Doptimize=ReleaseFast -fno-sys=gtk4-layer-shell -Dgtk-x11=true -p /opt/ghostty`
  took 429 s. Three GitHub-only Zig dependencies came from Ubuntu's vendored
  tarball (proxy workaround), and `patchelf` fixed the RPATH of the bundled
  gtk4-layer-shell. It reports `Ghostty 1.3.1-dev+0000000` because the
  tarball build has no git metadata.
* Display: `Xvfb :1 -screen 0 1600x1000x24` plus `openbox`, rendering via Mesa
  llvmpipe ("loaded OpenGL 4.5"). Ghostty ran as each test user with a clean
  env (`/usr/local/bin/cwghostty`). Input came from `xdotool`, and screenshots
  from ImageMagick `import`.
* Inside Ghostty: `TERM=xterm-ghostty COLORTERM=truecolor`, 149×39 cells, with
  bash shell integration auto-injected.

## TUI smoke test in Ghostty

### Path 1 (installer, user `cw1`, key via env): **PASS**

| Step | Result | Screenshot |
|---|---|---|
| New Ghostty window: `command -v codewhale` | **`bash: codewhale: command not found`**. Ghostty's non-login bash doesn't read `~/.profile`. Fixed with `echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.bashrc` | `01-tui-first-launch.png` |
| `codewhale` in a scratch repo `~/tui-demo` (`git init`) | Launch screen: logo, `v0.10.0 (1be1a703b975)`, "New session", footer `ask · files: workspace (unenforced)` / `DeepSeek · deepseek-flash · max` | `02-tui-launch.png` |
| Prompt: "create a Python file primes.py that prints the first 10 primes, run it, and show me the output" | Wrote `primes.py` (+22 lines) and showed the diff with no approval (Ask posture allows workspace writes). Then **APPROVAL bash `python3 primes.py`** with `[1/y] Allow once [2/a] Allow for this session [3/d/n] Deny [Esc] Abort` | `03-`, `04-tui-after-send.png` |
| Press `y` | Ran it; printed `2 3 5 7 11 13 17 19 23 29`; final summary; cost `$0.0022`, ttft 966 ms | `05-tui-result.png` |
| F1 | Help overlay: "Help — Concepts, commands, and keybindings", 211 entries, filterable | `07-key-F1-help.png` |
| Ctrl-K | Command palette, 177 entries | `08-key-ctrlK-palette.png` |
| Shift+Tab | posture `ask` → `auto` | `09-key-shiftTab-posture.png` |
| Tab (empty composer) | mode `work` → `operate` | `10-key-tab-mode.png` |
| Ctrl-R | Session picker showing this session (`e2525dfb`, 6 msgs) | `11-key-ctrlR-resume.png` |
| F3 | Provider picker: `DeepSeek * ready`, `Credential: DEEPSEEK_API_KEY`, `Endpoint: https://api.deepseek.com/beta`, 4 models with prices | `12-key-F3-provider.png` |
| Resize window to 800×500 and back | Reflows cleanly, footer compacts, no artefacts | `13-`, `14-resize-*.png` |
| Mouse wheel up/down | Scrolls the transcript; a "jump to bottom" arrow appears | `15-`, `16-mouse-scroll-*.png` |
| Paste a 3-line block with Unicode/CJK via Ghostty `Ctrl+Shift+V` | Inserted into the composer and not sent; wide characters aligned; composer grows | `17-paste-multiline.png` |
| Ctrl-U, then Ctrl-Z | Draft cleared, then restored ("Restored cleared draft") | `18-`, `19-*.png` |
| Ctrl-D (empty composer) | Exits and prints `To resume this session, run codewhale resume e2525dfb-af50-4f66-92dd-e7aa1759d72b` | `20-exit-ctrlD.png` |
| After exit: `stty -a`, mouse click and wheel in the shell | Normal screen, cursor visible, sane tty (`rows 39; columns 149`), no escape-sequence garbage | `21-terminal-restored.png` |
| `codewhale resume e2525dfb` + "What was the 10th prime in the output?" | History restored; answered `29` | `22-`, `23-resume-*.png` |
| Ctrl-C, Ctrl-C | Quits (the first press shows no visible hint in my capture) | `24-`, `25-*.png` |
| `codewhale -c` | Reopens the most recent session in this folder ("Session context synced") | `26-continue-flag.png` |

Cosmetic: after each turn a faint label such as `other · drift` and a few `°`
dots appear in empty transcript space for a few seconds (`05-`, `40-`). That's
the intentional "ambient life" whale cameo
(`crates/tui/src/tui/ambient_life/`), not corruption. The effort indicator
read `max` in the first session and `auto` after `resume`.

### Path 2 (manual download, user `cw2`, **no key**): **FAIL (no-key UX)**, then **PASS** after setting the key in the TUI

| Step | Result | Screenshot |
|---|---|---|
| `codewhale` with no key or config | Launch screen identical to a configured one. **No** "no model connected" text | `30-cw2-tui-nokey-launch.png` |
| Send `hello` | Shown in the transcript; **nothing else, no error, still nothing after 16 s** | `31-`, `32-*.png` |
| F3 | `DeepSeek * missing key`, `! missing DEEPSEEK_API_KEY · checked env DEEPSEEK_API_KEY, DeepSeek Harness credentials…`, `Enter set key` | `33-cw2-nokey-F3.png` |
| Enter, type key | Input is masked (`Key: ***`) | `34-`, `35-*.png` |
| Enter | "Connection checked (/models returned 2xx). Pick a default model" | `36-cw2-after-setkey.png` |
| Enter (flash) | "Confirm provider setup". It shows the last 4 characters of the key, which are **redacted in the screenshot** | `38-cw2-confirm-provider-setup.png` |
| Enter | `Note Provider switched: deepseek → deepseek, Model: deepseek-flash → deepseek-flash, Endpoint: api.deepseek.com`. `~/.codewhale/secrets/secrets.json` created (0600). The earlier `hello` is not retried | `39-cw2-after-save.png` |
| "Reply with exactly: pong" | `pong` | `40-cw2-reply-after-setkey.png` |

### Path 3 (npm wrapper, user `cw3`): **PASS**
`readlink -f $(command -v codewhale)` →
`~/.npm-global/lib/node_modules/codewhale/bin/codewhale.js`. The TUI launched
through the Node wrapper, replied `pong`, and Ctrl-D exited cleanly with the
resume hint. Screenshots `41-`, `42-`.

### Wrong key in the TUI (user `cw2b`, `DEEPSEEK_API_KEY=sk-000…dead`): handled, but not explained
On launch the TUI goes straight to **Getting started → "Choose your model
provider"** and lists `DeepSeek * last check failed (authentication)` at the
top (`43-`, `44-`). Esc leads to "Codewhale works with you in this folder.
Let's get you ready." (`45-`), and Ctrl-C exits (`46-`). Nothing says in
words "your DEEPSEEK_API_KEY was rejected".

### No network in the TUI (user `cw1`, Ghostty started inside `unshare -n`)
After about 25 s, an inline red `Error Network error: SSE stream request failed
after HTTP/1.1 fallback: Responses API request failed. … on Windows or proxy
networks, try CODEWHALE_FORCE_HTTP1=1 …` appears. It wraps mid-word
("non-st/reaming"). Screenshot `47-tui-no-network.png`.

---

## Path 8: uninstall and leftovers

**Result: PASS for removing the programs. Nothing cleans up data, and the
docs are silent on it.**

What Codewhale created, from a full `find /` for files owned by the test
users, plus `~` inventories:

| Path | Created by | Notes |
|---|---|---|
| `~/.local/bin/codewhale`, `~/.local/bin/codew` | installer / archive / manual | 2 × 78 MiB |
| `~/.npm-global/lib/node_modules/codewhale` (+ 2 bin symlinks) | npm | 157 MiB |
| `~/.cargo/bin/codewhale` | cargo | see Path 4 |
| `~/.codewhale/` | first run of anything | `config.toml`, `secrets/secrets.json` (plaintext keys, 0600), `sessions/`, `logs/`, `catalog/` (5 MB), `skills/` (47 built-in), `builtin-plugins/`, `tasks/`, `automations/`, `crashes/`, `audit.log`, `composer_history.txt`, `settings.toml`, `update-check.json`, `last-launch.json`, `setup_state.json`, `*.lock`. 6–7 MB after a few turns |
| `~/.deepseek/snapshots/<hash>/<hash>/.git` | first tool turn in a repo | **legacy path** (BUGS B2). A side-git copy of the workspace per turn |
| `<repo>/.codewhale/state/subagents.v1.lock` | first tool turn in that repo | shows as `?? .codewhale/` in git |
| Nothing outside `$HOME` | – | The only files outside home belonged to Ghostty/GTK (`/tmp/dbus-*`, `~/.cache/mesa_shader_cache`, `~/.config/ghostty`) |

npm (user `cw3`):
```console
$ codewhale auth clear --provider deepseek
cleared API key for deepseek from config and secret store
$ npm uninstall -g codewhale
removed 1 package in 657ms
$ command -v codewhale codew || echo "codewhale/codew: not on PATH"
codewhale/codew: not on PATH
$ du -sh ~/.codewhale ~/.deepseek
6.3M	/home/cw3/.codewhale
220K	/home/cw3/.deepseek                        # left behind by npm uninstall
$ find ~ -maxdepth 3 -name .codewhale -type d
/home/cw3/tui/.codewhale
/home/cw3/.codewhale
```
Archive install (user `cw2b`):
```console
$ rm -v ~/.local/bin/codewhale ~/.local/bin/codew
removed '/home/cw2b/.local/bin/codewhale'
removed '/home/cw2b/.local/bin/codew'
$ rm -rf ~/.codewhale ~/.deepseek ~/tui/.codewhale ~/p/.codewhale
$ ls -la ~ | grep -E "codewhale|deepseek" || echo "no codewhale/deepseek dirs left"
no codewhale/deepseek dirs left
```

### Misc checks
```console
$ codewhale config set telemetry false
Anonymous usage counts are off. Local telemetry state was erased.
$ grep telemetry ~/.codewhale/config.toml
telemetry = false
$ curl -fsSL https://codewhale.net/install.sh | CODEWHALE_INSTALL_DIR=$HOME/.cargo/bin sh
Checksums verified
codewhale install: refusing managed/system directory /home/cw6/.cargo/bin; use a fresh user directory
# (but it had already created an empty ~/.cargo/bin; see BUGS B15)
$ file ~/.local/bin/codewhale
ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), static-pie linked, … stripped
$ ldd ~/.local/bin/codewhale
	statically linked
```

---

## Path 4: Cargo / build from source

Build machine: 4 vCPU, 15 GiB RAM. During most of these builds, a Nix build
of the same code was also running, so treat the times as upper bounds.

### 4a. `cargo install codewhale-cli --locked` (crates.io), user `cw4`, rustup stable 1.98.1

**Without `libdbus-1-dev`: FAIL (as the docs predict)**, after 75 s:
```
error: failed to run custom build command for `libdbus-sys v0.2.7`
  pkg-config exited with status code 1
  Package dbus-1 was not found in the pkg-config search path.
  The system library `dbus-1` required by crate `libdbus-sys` was not found.
exit=101
```
`build-essential` and `pkg-config` were already on the image.

**With `apt-get install libdbus-1-dev`: PASS.**
```
  Installing codewhale-cli v0.10.0
    Finished `release` profile [optimized] target(s) in 25m 31s
  Installing /home/cw4/.cargo/bin/codewhale
   Installed package `codewhale-cli v0.10.0` (executable `codewhale`)
exit=0   elapsed=1531s        ~/.cargo/registry: 474M
$ command -v codewhale codew
/home/cw4/.cargo/bin/codewhale                     # no codew (as documented)
$ codewhale --version
codewhale 0.10.0                                   # no commit hash
$ ldd ~/.cargo/bin/codewhale | grep dbus
	libdbus-1.so.3 => /lib/x86_64-linux-gnu/libdbus-1.so.3
$ codewhale exec --auto "create hello.py that prints hello, run it, show output"   → hello, exit 0
$ codewhale update --check
… To retain this secondary cargo installation, run `cargo install codewhale-cli --locked --force`.
Latest stable release: v0.10.0
Already up to date.
$ cargo uninstall codewhale-cli
    Removing /home/cw4/.cargo/bin/codewhale        # ~/.codewhale (6.3M) and ~/.deepseek (284K) remain
```
The TUI in Ghostty worked: `pong`, clean exit (screenshots `48-`, `49-`).

**Documented minimum Rust 1.88: FAIL** (`cargo +1.88.0 install codewhale-cli --locked`, 8 s):
```
Caused by:
  rustc 1.88.0 is not supported by the following package:
    serde-saphyr@1.3.0 requires rustc 1.89
```

### 4b. Git checkout: `git clone --depth 1 --branch v0.10.0 …` then `cargo install --path crates/cli --locked`, user `cw4b`

* The clone took 6.3 s.
* Inside the checkout, `rust-toolchain.toml` (`channel = "stable"`) makes
  rustup 1.29.1 **silently install stable 1.98.1**, although the user's
  default was 1.88.0 (`cargo --version` → `cargo 1.98.1`).
* `cargo +1.88.0 install --path crates/cli --locked` gives the same
  `serde-saphyr@1.3.0 requires rustc 1.89` error. **FAIL.**
* `cargo +1.89.0 install --path crates/cli --locked` **FAILS** after 690 s:
  ```
  error: this lint expectation is unfulfilled
     --> crates/tui/src/prompt_zones.rs:308:30
  308 | #[cfg_attr(not(test), expect(dead_code))]
     = note: `-D unfulfilled-lint-expectations` implied by `-D warnings`
  error: could not compile `codewhale-tui` (lib) due to 8 previous errors
  exit=101
  ```
  Root cause: `[workspace.lints.rust] warnings = "deny"` (Cargo.toml:48-49).
  `target/` was 1.2 GB at that point.
* The documented command on the repo-selected stable toolchain:
  **PASS**, slowly:
  ```
  rustc 1.98.1 (48a229cea 2026-09-01)
  warning: default toolchain implicitly overridden with `stable-x86_64-unknown-linux-gnu` by rustup toolchain file
      Finished `release` profile [optimized] target(s) in 83m 12s
    Installing /home/cw4b/.cargo/bin/codewhale
  exit=0   elapsed=4993s   target/ = 3.1G
  $ codewhale --version
  codewhale 0.10.0 (dev)
  $ codewhale exec "Reply with exactly: pong"
  pong
  ```
  The `codewhale_tui` crate alone compiled for more than 70 min, at up to
  8.7 GB RSS, under the workspace `[profile.release]` (`lto = "thin"`). A
  concurrent Nix build ran until about 17:38, so this time is inflated.

---

## Path 5: Homebrew on Linux and Nix

Run by a helper agent following the same clean-user rules. The full
transcripts are `logs/p5-homebrew.log` and `logs/p5-nix.log`.

### Homebrew (user `cw5`, Homebrew 7.0.6): **PASS, but the documented commands FAIL**
```
$ NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
==> Checking for `sudo` access (which may request your password)...
Insufficient permissions to install Homebrew to "/home/linuxbrew/.linuxbrew".      exit 1
# admin (root) stand-in for the installer's sudo step:
# mkdir -p /home/linuxbrew/.linuxbrew && chown cw5:cw5 /home/linuxbrew/.linuxbrew
$ (re-run installer)  → Homebrew 7.0.6 installed, 29 s
$ echo 'eval "$(/home/linuxbrew/.linuxbrew/bin/brew shellenv bash)"' >> ~/.bashrc
$ brew tap Hmbown/deepseek-tui                                   # 6 s
$ brew install codewhale                                         # as documented
Error: Refusing to load formula hmbown/deepseek-tui/codewhale from untrusted tap hmbown/deepseek-tui.
Run `brew trust --formula hmbown/deepseek-tui/codewhale` or `brew trust hmbown/deepseek-tui` to trust it.
$ brew install Hmbown/deepseek-tui/codewhale                     # full name: works, 73 s
==> Trusted formula hmbown/deepseek-tui/codewhale
# (alternative that also worked: brew trust hmbown/deepseek-tui && brew install codewhale, 59 s)
$ command -v codewhale codew
/home/linuxbrew/.linuxbrew/bin/codewhale
/home/linuxbrew/.linuxbrew/bin/codew
$ codewhale --version ; codew --version
codewhale 0.10.0 (1be1a703b975)      # both; byte-identical files
$ codewhale doctor; echo $?  → 0
$ codewhale update --check
This executable is managed by Homebrew; in-place self-update is disabled.
… (migration recipe) … To retain this secondary Homebrew installation, run `brew upgrade codewhale`.
Latest stable release: v0.10.0
Already up to date.
$ brew deps codewhale     → node (31 recursive deps, ~563 MB Cellar)
$ brew list codewhale     → bin/codewhale, bin/codew, INSTALL_RECEIPT.json, sbom.spdx.json, .brew/codewhale.rb (163.6 MB)
$ brew uninstall codewhale    # 5 s; also autoremoved node and the other 30 deps
$ brew untap Hmbown/deepseek-tui
```
The formula is version 0.10.0 (tap commit `7a6701d`), `depends_on "node"`,
and installs the release binaries (it doesn't build from source). The legacy
`deepseek-tui` formula is deprecated and points to `codewhale`.
`~/.cache/Homebrew` (326 MB) remains after uninstall. The tap printed "It
looks like you tapped a private repository…". That's likely a VM-proxy
artefact, because the proxy blocks the GitHub API.

### Nix (user `cw5n`, Nix 2.35.2 single-user): **PARTIAL, did not finish**
```
$ sh <(curl -L https://nixos.org/nix/install) --no-daemon
directory /nix does not exist; creating it by running 'mkdir -m 0755 /nix && chown cw5n /nix' using sudo
sudo: a password is required
# admin (root): mkdir -m 0755 /nix && chown cw5n /nix ; re-run → Nix 2.35.2 installed, 26 s
$ echo 'experimental-features = nix-command flakes' >> ~/.config/nix/nix.conf
$ nix flake metadata github:Hmbown/CodeWhale        # OK, 13 s, locks main @ 922679d
$ nix run github:Hmbown/CodeWhale -- --version
error: … unable to download 'https://github.com/nix-community/fenix/archive/164c596….tar.gz': HTTP error 403
# ^ VM proxy policy. Workaround used: --override-input fenix "git+https://github.com/nix-community/fenix?rev=164c596…&shallow=1"
```
* Build phase (`cargo build --release -p codewhale-cli`): **30 min 10 s**.
* Then `checkPhase` (`doCheck` in `nix/package.nix`) recompiled the workspace
  as test binaries. One rustc reached about 8 GB RSS, and it ran more than 70
  min. I killed it at about 105 min total, so nothing reached the store.
* The binary copied out of the sandbox printed **`codewhale 0.10.0 (dev)`**.
  The flake builds main-branch source, not the release tag.
* `nix/package.nix` builds only `codewhale-cli`, so there's **no `codew`**.
* Not tested: `nix run` to completion, `nix build`, `ls result/bin`, the
  store size, and `nix profile install/remove`.
