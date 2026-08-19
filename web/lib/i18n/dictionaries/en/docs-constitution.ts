import type { DocsConstitutionDict } from "../types";

export const docsConstitution: DocsConstitutionDict = {
  metaTitle: "Constitution and /constitution · Codewhale Docs",
  metaDescription:
    "User-global constitution, repo-local law, project instructions, and runtime boundaries.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Constitution and /constitution",
  overviewCompanion: "宪章与 /constitution",
  overviewLead:
    "Codewhale gives the agent an accountable address, then a legal system for context conflicts. {constitutionCommand} is the primary personal constitution surface: guided setup stores structured user-global data in {globalPath} and renders it as model-facing prose. Repos can still add local law via {repoPath}; runtime policy separately encodes modes, approval, sandbox, cost, and tool boundaries.",
  scopes: [
    [
      "User-global",
      "用户全局",
      "Use /constitution for standing personal law across projects. It is structured data rendered to prose, not a raw prompt editor.",
    ],
    [
      "Repo-local",
      "仓库本地",
      ".codewhale/constitution.json is optional project policy for protected invariants, branch rules, verification, and escalation.",
    ],
    [
      "Runtime",
      "运行时",
      "Constitution text may express preferences, but approval, sandbox, shell, network, trust, and MCP permissions remain enforced config.",
    ],
  ],
  authorityNote:
    "Standard project instructions still live in AGENTS.md; memory and handoffs rank below constitutions and project instructions; the full base-prompt Markdown override is an expert escape hatch, not the normal setup path. See {configurationDocs}.",
  configurationLink: "configuration docs",
  sourceNote: "Source document: docs/ARCHITECTURE.md · Update docs-map.ts when changing.",
};
