import type { DocsGuideDict } from "../types";

/**
 * English reference dictionary for the docs "Getting started" page.
 * Copy moved verbatim from `app/[locale]/docs/guide/page.tsx` — any wording
 * change belongs in its own commit, never mixed into a structural move.
 */
export const docsGuide: DocsGuideDict = {
  metaTitle: "Getting started · Codewhale Docs",
  metaDescription:
    "The full path from install to your ideal fleet: install, a first keyless session, provider connection, and fleet setup.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Getting started",
  overviewLead:
    "Four steps from one install command to a fleet set up for your work. Every step states only what the current candidate actually does; anything unreleased or unrecorded is labeled as such.",
  sessionTitle: "Watch a real session",
  sessionLead:
    "Below is the real-session media slot. It is deliberately in the pending state: until the v0.9.2 candidate dogfood recording exists, this site shows no placeholder or staged footage.",
  nextTitle: "Where next",
  sourceNote:
    "Source documents: docs/GUIDE.md, docs/KEYBINDINGS.md · Step copy lives in web/lib/content/getting-started.ts; update docs-map.ts when changing.",
};
