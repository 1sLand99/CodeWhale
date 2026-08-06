# Agent Plugins support — design

Date: 2026-08-06
Branch: `codex/agent-plugins` (merges after v0.9.4 ships)
Status: approved, not yet implemented

## Why

[Agent Plugins](https://agent-plugins.org/) v1.0.0 is a vendor-neutral standard
for packaging agent extensions, with a steering committee of Amazon, Cursor,
Microsoft, OpenAI, and Vercel. Compatible clients at launch: Codex, ChatGPT,
Cursor, GitHub Copilot, Kiro, VS Code.

Codewhale already has every concept the standard names — skills, MCP servers,
a plugin manifest, discovery, install, and a registry. What it does not have is
the shared *format*, so none of that surface is reachable from the ecosystem in
either direction. This is an interop change, not a capability change.

## The standard, precisely

`plugin.json` — required `$schema` and `name`; optional `version`,
`description`, `author{name,email,url}`, `homepage`, `repository`, `license`,
`keywords`. The root is `additionalProperties: false`. Client-specific data
belongs in `extensions`, an object keyed by reverse-domain namespace.

`name` must match `^(?!.*(?:--|\.\.))[a-z0-9](?:[a-z0-9.-]*[a-z0-9])?$`,
1–64 chars: lowercase, no `--`, no `..`.

`mcp.json` — a *separate* file, because `plugin.json`'s root is closed.
`mcpServers` keyed by id; transports `stdio`, `streamable-http`, `sse`.
`env` may not define `PLUGIN_ROOT` or `PLUGIN_DATA` (reserved).

`skills/{skill-name}/SKILL.md` — markdown with YAML frontmatter requiring
`name` and `description`. This already matches Codewhale's skill layout.

## Decisions

**`plugin.json` becomes the native format.** `plugin.toml` drops to
legacy-readable, the same status pre-versioned manifests already hold.

**Codewhale-specific fields live under `extensions["net.codewhale"]`:**
`commands`, `agents`, `hooks`, `lsp`, `native`, `capabilities`, `when`.
The standard reserves this namespace shape for exactly this purpose, so a
Codewhale plugin stays spec-valid and other clients ignore what they don't
know. `mcp_servers` moves out to `mcp.json`.

**Migration writes files, but only where we own them.** Loading a
`plugin.toml` inside Codewhale's managed plugin root rewrites it to
`plugin.json` + `mcp.json`, preserving the original as `plugin.toml.bak`.
Outside that root — a plugin living in someone else's repository — the
manifest stays read-only legacy and `/plugin validate` reports the migration.
Rewriting files under another project's version control is not ours to do.

**Name slugification is the real migration hazard.** Existing names with
uppercase, underscores, spaces, `--`, or `..` are invalid under the standard.
Migration slugifies to a conforming `name` and preserves the original as the
display name. A slugified name that collides with an existing plugin is an
error, not a silent rename.

## Consume

Discovery prefers `plugin.json`, falls back to `plugin.toml`. Both parse into
the existing `PluginManifest`, so nothing downstream of discovery changes.
Unknown `extensions` namespaces are ignored — that is the standard's whole
point — so a Cursor or Copilot plugin contributes its skills and MCP servers
and its client-specific data is skipped.

## Publish

`/plugin export <name>` writes a spec-valid bundle: `plugin.json`,
`mcp.json` when servers exist, and the `skills/` tree. Tests validate output
against the published JSON Schemas so upstream drift fails CI rather than
shipping.

## First plugin: Kitesurf

Cloudflare's Kitesurf is a CDP-compatible remote browser for agents, reached
through `chrome-devtools-mcp` pointed at a Cloudflare websocket endpoint. It
is therefore an `mcp.json` and nothing else — a natural first bundled Agent
Plugin, and a live test that the format round-trips. It needs a Cloudflare
account id and API token, so the bundle ships with those as required config
rather than baked values.

## Testing

- round-trip: `plugin.toml` -> internal -> `plugin.json` -> internal is stable
- schema conformance for every emitted `plugin.json` and `mcp.json`
- name slugification, including collision-is-an-error
- migration preserves `.bak` and refuses to write outside the managed root
- a third-party plugin with an unknown `extensions` namespace loads cleanly

## Out of scope

Distribution, installation UX, permissions, and client-specific capabilities
stay Codewhale's own. The standard deliberately does not define them.
