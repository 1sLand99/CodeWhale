import type { DocsFleetDict } from "../types";

export const docsFleet: DocsFleetDict = {
  metaTitle: "Fleet & Workflow · Codewhale Docs",
  metaDescription:
    "The control plane for durable multi-worker runs, plus the optional Workflow orchestration overlay.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Fleet & Workflow",
  overviewLead:
    "Fleet is the control plane for durable multi-worker runs. It is not a separate execution engine: a fleet worker is a headless codewhale exec run that the fleet launches and tracks durably. Reach for Fleet instead of short-lived agent fan-out whenever the work needs retry, sleep/restart survival, remote execution, receipts, or a ledgered audit trail.",
  runTitle: "Run a fleet",
  runLead:
    "Fleet state lives in the workspace's .codewhale/fleet.jsonl ledger, with worker logs under .codewhale/fleet/. codewhale fleet resume <run-id> is the restart-recovery verb: it replays the ledger, reconciles in-flight leases whose workers stopped heartbeating, and is idempotent — safe after a manager exit, laptop sleep, or runtime restart.",
  statusLead:
    "Two similarly named status surfaces exist: in the TUI, {fleetStatusTui} (or {subagents}) shows the sub-agents attached to the current interactive session; in a shell, {fleetStatusShell} reads the durable Fleet ledger.",
  profilesTitle: "Roles and /fleet setup",
  profilesLead:
    "/fleet setup opens a progressive wizard for authoring a reusable agent-team profile: one focused choice at a time — a role, then a model (inherit, or a concrete model from any configured provider), then a thinking tier (inherit, off, low, medium, high, max, or auto) — and a review of the full posture (route, thinking, permissions, tools, scope, and review policy) before anything is saved. Profiles live in project scope (.codewhale/agents/<role>.toml, travels with the repo) or personal scope ($CODEWHALE_HOME/agents/<role>.toml, available in every repo on the machine); a same-id project profile wins. Profile storage scope never widens the authority of a running operation.",
  workflowTitle: "Workflow orchestration",
  workflowLead:
    "Ordinary multi-agent work does not need Workflow: send normal messages in Operate and let Codewhale prefer background workers when parallelism, isolation, or duration makes delegation useful. Use Workflow when ordered phases, gates, shared budgets, replay, or deterministic fan-in matter. A Workflow script is a coordinator only: it has no filesystem or shell of its own; real work happens in the sub-agents it launches. Scripts are written in a declarative compile-only JS subset that lowers to a typed WorkflowSpec validated and executed by Rust; effectful constructs such as import, fetch, process, eval, and async/await are rejected by the compiler.",
  workflowLimits:
    "Default validation bounds: up to 100 worker agents per workflow run, up to 5 recursive Fleet rings, loops must declare max_iterations, and dynamic expand nodes must declare max_children plus a template. These are population limits, not launch concurrency — a valid 100-agent workflow still drains through the configured Fleet worker pool. Inside the Workflow JS sandbox, one run keeps at most 16 concurrent live agents and at most 1,000 spawns over the VM lifetime.",
  sourceNote:
    "Source documents: docs/FLEET.md, docs/WORKFLOW_AUTHORING.md · Update docs-map.ts when changing.",
};
