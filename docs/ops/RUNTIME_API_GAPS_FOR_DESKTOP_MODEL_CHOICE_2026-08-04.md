# Runtime API gaps found while building desktop model choice (2026-08-04)

Notes for the Codewhale runtime, written while building a real model picker in
the managed desktop app (`cwc/apps/web` + `clients/desktop-shell`). Everything
below was measured against a live `0.9.4 (a241772af6f9)` `app-server --http`,
read-only. **No runtime code was changed.**

## What already works well

`GET /v1/providers` is the right endpoint and is well shaped:

```json
{
  "current": "modelstudio-token-plan",
  "providers": [
    {
      "id": "modelstudio-token-plan",
      "display_name": "Alibaba Cloud Model Studio",
      "default_base_url": "https://…/compatible-mode/v1",
      "default_model": "qwen3.8-max",
      "has_model_catalog": true,
      "env_vars": ["MODELSTUDIO_API_KEY", "DASHSCOPE_API_KEY"]
    }
  ]
}
```

`current` plus a stable `id`/`display_name`/`default_model` per provider is
exactly enough to render a picker whose contents are true for *this* runtime,
rather than guessed from a version snapshot. `GET /v1/config` (effective
`model`, `provider`, `approval_mode`, `reasoning_effort`, `sandbox_mode`) is
likewise useful and already consumed.

The desktop projects only `id`, `display_name`, `default_model`, and
`has_model_catalog` across the bridge. `default_base_url` and `env_vars` are
deliberately dropped — endpoints and credential *names* should not reach a
browser layer.

## Gap 1 — no signal for "this route has a usable credential here"

**This is the highest-value addition.** `/v1/providers` lists what the runtime
can *represent*, not what it can actually serve. A picker therefore has to offer
every route and let the operator discover at turn time that a provider has no
key on this machine. The failure lands after the user has committed to a run.

`env_vars` is not a substitute: the app cannot read the runtime's environment or
keyring, and should not try.

Suggested shape — a boolean the runtime can answer locally:

```json
{ "id": "anthropic", "…": "…", "credential_present": true }
```

or an opt-in filter, `GET /v1/providers?usable=1`.

Either lets a client grey out or hide unusable routes and fail closed *before*
a run instead of during one. (openwork solves the same problem by filtering to
connected providers before rendering its picker.)

## Gap 2 — `has_model_catalog: true` with no way to read the catalog

Providers advertise `has_model_catalog: true`, but there is no route that
returns a provider's models. Measured 404: `GET /v1/models`,
`GET /v1/runtime/models`, `GET /v1/runtime/providers`.

Consequence: a client can only ever offer each provider's `default_model`, so
"choose a model" is really "choose a provider". Two routes in the current
catalog already share `deepseek-v4-pro` as their default, which makes the
distinction visible to users.

Suggested: `GET /v1/providers/{id}/models` returning at least
`{ id, display_name? }[]`, ideally with the provider's default flagged.

## Gap 3 — thread/turn `model` is accepted but not discoverable

`POST /v1/threads` accepts `model`, `model_provider`, and `model_provider_id`,
and the turn envelope reports `effective_model` / `effective_provider`. That
pairing works well and the desktop relies on it.

What is missing is a documented statement of **which** `model` values are valid
for a given provider — the same gap as #2, seen from the write side. Today a
client can only safely submit a provider's `default_model` or the exact value
`/v1/config` reports.

## Gap 4 — commit drift under a stable version string

Observed twice in one working day: `codewhale --version` reported `0.9.4` with
commit `8df9707b680b`, then `a241772af6f9`, with the CLI and TUI binaries
replaced together.

The desktop bridge already handles this correctly — it requires the CLI and TUI
to report the *same* version **and** commit, and fails closed otherwise. Noting
it here because anything that pins a `0.9.4` commit will rot quickly, and
because "same version, different commit" is a real state that consumers must
tolerate.

## Non-issues, recorded so they are not re-investigated

- `auth_required: true` with a bearer token on loopback works as documented.
- `/v1/runtime/info` `capabilities` is sufficient for the desktop's readiness
  probe; no additions needed for this work.
- Provider ids are stable enough to key UI state against (the desktop remembers
  a chosen route per folder using the provider id).
