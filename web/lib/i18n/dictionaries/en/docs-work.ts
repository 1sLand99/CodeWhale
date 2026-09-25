import type { DocsWorkDict } from "../types";

/**
 * English reference dictionary for `app/[locale]/docs/work/page.tsx`.
 * Copy moved verbatim from the page's `isZh` ternaries — any wording change
 * belongs in its own commit, never mixed into a structural move.
 */
export const docsWork: DocsWorkDict = {
  metaTitle: "Work Surface · Codewhale Docs",
  metaDescription:
    "The single To-do list, how the model sees it, and how one work state stays continuous.",
  bodyClassName: "text-ink-soft leading-relaxed",
  overviewTitle: "The Work surface",
  overviewLead:
    "The TUI sidebar has a Work area that shows live state for the current job. It is more than a visual to-do list: the same work state is maintained by model-visible tools, session relay, and sub-agent handoff. Codewhale has exactly one Work surface — the counted To-do execution ledger. update_plan is conversational reasoning, not a second progress surface.",
  checklistTitle: "To-do: the sole canonical ledger",
  checklistBody:
    "The To-do is the progress ledger for concrete work: a list of items with status (pending / in_progress / completed / cancelled), a completion percentage, and the item currently in progress. The model replaces this projection for the active thread or durable task through the canonical {todoWrite} tool — the model-visible progress surface. The legacy {checklistAlias} and {todoAlias} names remain hidden compatibility aliases: they stay dispatchable against the same To-do state so old transcripts replay, but they are not advertised to the model catalog.",
  strategyTitle: "Strategy is conversational reasoning: update_plan",
  strategyLead:
    "update_plan carries optional high-level strategy — it is not a second list. Its fields serve phase-level understanding: title, objective, context summary, explanation, sources, critical files, constraints, recommended approach, verification plan, risks and unknowns, a handoff packet, and a list of steps. It helps a parent session or a later worker understand the approach; concrete execution progress always belongs to the To-do list. The sidebar deliberately does not render strategy state as a second progress list, and neither does any To-do snapshot surface — plan state with an empty To-do produces no snapshot at all.",
  continuityTitle: "Continuity: one state, many surfaces",
  continuityLead:
    "The model learns the To-do from its own tool results: what todo_write returned is ordinary conversation history, so nothing re-states the list on each step. Only at a seam a person asked for does one renderer show the current To-do once: a forked (fork_context) sub-agent receives that body inside its structured state block, and /relay writes the same body into the handoff instruction. The To-do body is byte-identical in both, so a child agent and the next thread continue from the parent's real progress position instead of a paraphrased summary. The sidebar renders that same state live, in full.",
  captureTitle: "Terminal capture (faithful text)",
  captureLead:
    "This text block reproduces the sidebar Work area line-for-line from the rendering logic in crates/tui/src/tui/sidebar.rs: the goal row with its ◆ icon, elapsed time, and token budget bar, then the settled counter and the numbered status items.",
  captureLegend:
    "The item prefixes map to the four statuses: {pending} pending, {inProgress} in progress, {completed} completed, {cancelled} cancelled. When space runs out, the sidebar windows around the in-progress item and marks the omission with “+N more To-do items”.",
  modelFacingTitle: "What is model-facing vs. visual-only",
  modelFacingLead:
    "Three model-facing paths are implemented and covered by tests: the todo_write tool itself, which is active in the model catalog and whose tool result is how the model sees the list; the forked sub-agent's structured state block (the To-do section inside <codewhale:fork_state>, resolved at the moment of the fork); and /relay output. No request re-states the To-do on any step — a structural test asserts the real outbound request body does not contain the list. The sidebar rendering is a visual presentation — it informs the operator and is not injected into model context.",
  modelFacingBoundaries:
    "The boundaries are worth stating: because nothing is injected per step, the stable system-and-tool prefix is untouched by To-do changes and prefix caching is never invalidated by them. The one snapshot taken at a fork reads the authoritative state (the work graph's staged projection where one exists, not the not-yet-published legacy view), so a todo_write made earlier in the same turn is included. Item count and character count are both hard-bounded, the in-progress item is preserved preferentially, and elided content is marked. An empty To-do emits nothing at all. The renderer guarantees exactly three things — wrapper framing cannot be closed early, control characters cannot forge the line format, and the bounds hold. It does not vet what item text says, so arbitrary To-do content is not thereby made safe to follow as instructions.",
  sourceNote:
    "Source documents: docs/TOOL_SURFACE.md, docs/TOOL_LIFECYCLE.md · Update docs-map.ts when changing.",
};
