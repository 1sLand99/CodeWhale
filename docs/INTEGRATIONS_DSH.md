# DeepSeek Harness connected through Codewhale

`codewhale integrations dsh …` connects a user's **existing** official DeepSeek
Harness installation (`dsh`, npm `@deepseek-ai/dsh`) to their Codewhale setup.
DSH stays an integrated harness surface. Codewhale remains the owner of Fleet
configuration, provider/model selection, permissions, credentials, and
lifecycle authority; DSH is not a second Fleet scheduler and never an
authority bypass.

Verified against `dsh 0.1.0-rc.6` (the latest published release at the time
of writing). DSH is a developer preview that warns of compatibility-breaking
changes; a newer `dsh` is reported as `stale-version` (launchable, unverified),
an older one or one without `--patch` as `incompatible`.

## What is (and is not) connected

Codewhale uses only DSH's documented seams:

| Seam | How Codewhale uses it |
| --- | --- |
| `dsh --version` / `dsh --help` | read-only detection (never initializes a profile) |
| `$DSH_HOME` (or `~/.dsh`) | read-only inventory: profile names, `settings.yaml` top-level namespaces, whether `.credentials.yaml` exists and is `0600`. Values are never read. |
| `--patch <file>` overlay | Codewhale writes **one** overlay under its own home and passes it at launch |
| `DSH_PERMISSION_MODE` env | mirrors the Codewhale permission posture |
| `--profile web` / `--profile headless` | the two shipped DSH profiles; DSH initializes them itself on first launch (its own documented behavior) |

Codewhale writes **only** under `$CODEWHALE_HOME/integrations/dsh/`:

- `codewhale.patch.yml` — the overlay. Identity only: provider route, model,
  base URL, and (native DeepSeek route) `reasoningEffort`. For
  OpenAI-compatible providers it declares a `codewhale-<provider>` route on
  DSH's `llm-pi-ai` adapter with `apiKeyEnv` naming the provider's canonical
  environment variable — the *name*, never the value. Keyless local routes
  (loopback Ollama / LM Studio / vLLM / SGLang) carry no credential reference.
- `receipt.json` — the current connection record plus an append-only history
  of `connect` / `update` / `disable` / `enable` / `remove` events with the
  overlay SHA-256, dsh version, `$DSH_HOME`, mapped identity, permission mode,
  and timestamps (see `docs/RECEIPTS.md`). Every event is also appended to
  `$CODEWHALE_HOME/audit.log`.
- `codewhale-dsh-skin.css` and `codewhale-dsh-skin-preview.html` — only with
  `--skin`; see below.

Codewhale **never**:

- copies, prints, or embeds API keys, OAuth documents, environment secrets,
  prompts, or filesystem contents (a `--api-key`/keyring credential Codewhale
  itself materialized into the process is stripped from the launched child;
  a key the user exported in their own shell is left alone);
- writes to `$DSH_HOME` (settings, credentials, profiles, sessions);
- edits installed `@deepseek-ai/dsh` package files;
- switches to a cloud model or broadens permissions silently. Codewhale
  `read-only` → DSH `read-only`; anything else → `workspace-write`;
  `danger-full-access` only with `--allow-full-access` **and** a Codewhale
  full-access posture (`sandbox_mode = "danger-full-access"` / yolo).

## States

| State | Meaning | Launch |
| --- | --- | --- |
| `not-installed` | `dsh` not on `PATH` | refused |
| `offline` | `dsh` exists but `--version` failed | refused |
| `incompatible` | older than 0.1.0-rc.6 or no `--patch` | refused |
| `detected` | usable dsh, no Codewhale overlay | refused (`connect` first) |
| `connected` | overlay matches the current Codewhale route | allowed |
| `stale-config` | route changed, overlay edited outside Codewhale, or missing | refused (`update`) |
| `stale-version` | connected, but dsh is newer than verified | allowed, unverified |
| `disabled` | overlay kept, launches refused | refused (`enable`) |

`status`, `plan`, `/setup tools` (Tools and MCP step) and `codewhale doctor`
are side-effect free.

## Commands

```bash
codewhale integrations dsh status [--json]
codewhale integrations dsh plan [--profile web|headless] [--allow-full-access] [--skin] [--json]
codewhale integrations dsh connect [--profile web|headless] [--allow-full-access] [--skin] [--yes]
codewhale integrations dsh update  [--profile …] [--allow-full-access] [--skin true|false] [--yes]
codewhale integrations dsh launch  [--profile web|headless] [--dry-run] [-- <dsh app args>]
codewhale integrations dsh disable
codewhale integrations dsh enable
codewhale integrations dsh remove [--yes]
```

`connect`, `update`, and `remove` print the exact plan (files, identity,
permission mode, disclosures, and the overlay text) and require confirmation
(`--yes` when stdin is not a terminal). `launch` runs
`DSH_PERMISSION_MODE=<mode> dsh --profile <p> --patch <overlay> …` in the
Codewhale workspace with the user's own `$DSH_HOME`, so their credentials,
sessions, and profiles remain theirs.

### Disclosures the plan makes

- DSH layers the user's `settings.yaml` sections (`agent-default-model`,
  `llm-deepseek`, `llm-pi-ai`) over the overlay per field. If those sections
  exist, DSH's saved selection can shadow the pinned identity until it is
  cleared in DSH; `status`/`plan` list them.
- Reasoning tiers are mapped only for the native DeepSeek route
  (`off|high|max`); hand-declared routes send no effort parameter.
- Anthropic Messages and OpenAI Responses routes cannot be carried and are
  refused rather than approximated.

## Skin (unsupported overlay)

DSH 0.1.0-rc.6 has no supported custom-theme API — only the built-in
`ui-theme.preference` (`light|dark|system`). Its theme package documents
third-party themes as "an extension point, not a product": overriding
same-named `--dsw-alias-*` CSS variables, with no validation.

`--skin` therefore exports `codewhale-dsh-skin.css`, generated from the TUI's
real palette (`crates/tui/src/palette`, Blue Stage dark and light), including
the ombre water column, semantic role/permission/mode colors, focus,
selection, error, waiting, and working states, the crown-fluke mark as an
inline SVG, typography sizing, and `prefers-reduced-motion` fallbacks, and
maps a bounded set of `--dsw-alias-*` variables onto them. **It is never
injected**: applying it to a running DSH page (browser user stylesheet or a
future DSH plugin) is an unsupported overlay you enable yourself. The
attribution chip in the sheet reads "DeepSeek Harness connected through
Codewhale". `codewhale-dsh-skin-preview.html` renders the tokens for
inspection and is explicitly not the DSH UI.

## Removal

`remove` deletes only the overlay, skin, and preview under
`$CODEWHALE_HOME/integrations/dsh/`, appends a `remove` receipt, and never
touches `$DSH_HOME` or the installed package. DSH keeps working exactly as
before the connection.

## Attribution

DeepSeek Harness is © 2026 DeepSeek, MIT licensed; the integration invokes the
installed launcher and does not redistribute it. This is not native Codewhale
functionality: every surface labels it "DeepSeek Harness connected through
Codewhale".
