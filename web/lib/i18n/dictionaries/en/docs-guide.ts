import type { DocsGuideDict } from "../types";

/**
 * English reference dictionary for the docs "Getting started" page.
 * Copy moved verbatim from `app/[locale]/docs/guide/page.tsx` — any wording
 * change belongs in its own commit, never mixed into a structural move.
 */
export const docsGuide: DocsGuideDict = {
  metaTitle: "Getting started · Codewhale Docs",
  metaDescription:
    "Install Codewhale, connect a model, and start your first task. Add a Fleet when you want a roster of models and roles.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "Getting started",
  overviewLead:
    "Install Codewhale, connect your model, and give it a task. Fleet setup is optional.",
  sessionTitle: "Watch a real session",
  sessionLead:
    "Follow a task from the first request to the finished result.",
  nextTitle: "Where next",
  sourceNote:
    "For more detail, see the user guide and keyboard shortcuts in the documentation.",
};
