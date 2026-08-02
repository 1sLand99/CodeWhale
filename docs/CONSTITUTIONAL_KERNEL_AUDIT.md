# Constitutional kernel audit

This audit records the first-turn boundary after the progressive-context work in
PR #5077. Counts use Codewhale's conservative characters / 3 estimator and the same
workspace used by the PR receipts: `/Volumes/VIXinSSD/CW/codewhale`.

## Measured result

| Receipt | Before kernel pass | After kernel pass | Change |
| --- | ---: | ---: | ---: |
| `doctor --context-json` active source total | 9,127 | 7,908 | -1,219 (-13.36%) |
| model-facing system blocks | 9,063 | 7,844 | -1,219 (-13.45%) |
| bundled constitution entry | 2,647 | 1,428 | -1,219 (-46.05%) |
| original installed-doctor baseline | 17,839 | 7,908 | -9,931 (-55.67%) |

The 64-token difference between the doctor total and model-facing blocks is
diagnostic accounting: project-context warnings (33) and provider facts (31).
It is not claimed as prompt savings.

## Pi comparison

Pi 0.80.3 was measured through its installed SDK in the same working directory,
with its actual default `read`, `bash`, `edit`, and `write` prompt snippets and
with extensions disabled. The table applies the same conservative characters / 3
estimator used by Codewhale's doctor:

| Pi fresh system variant | Characters | Conservative tokens |
| --- | ---: | ---: |
| Default context and 16 discovered skills | 27,671 | 9,224 |
| Context, skills disabled | 17,719 | 5,907 |
| Skills, context disabled | 12,485 | 4,162 |
| Base and actual tool snippets only | 2,533 | 845 |

The full configured Codewhale system receipt is 7,844 model-facing tokens, 1,380
below the full configured Pi receipt. With skill indexes removed from both, the
comparison is approximately 7,050 versus 5,907. That 1,143-token Codewhale
premium is the deliberate constitutional, mode, and runtime-special-feature
budget rather than undisclosed project text.

This is a system-message comparison, not a provider token-billing receipt. It
does not include Codewhale's separately transported tool schemas. Pi's one-line
tool snippets are part of its system message and are included.

Pi discovered and eagerly read `/Volumes/VIXinSSD/CW/AGENTS.md` (1,635
characters) and `/Volumes/VIXinSSD/CW/codewhale/AGENTS.md` (13,280 characters).
Like Codewhale, it keeps skill bodies lazy; unlike this branch, its discovered
skill metadata was not bounded and added 9,952 characters in this environment.

## First-turn constitutional kernel

The reduced constitution keeps these duties eager and test-locked:

- **Authority and user intent**: the current user request stays first in the
  exact `Whose word wins` chain; the agent must do that request and avoid silent
  scope expansion.
- **Safety and authorization**: irreversible actions, publication, spending,
  credentials, and material scope expansion require express authorization in
  the current request; otherwise the agent must name the decision and ask.
  Tool, approval, sandbox, skill, role, and project gates remain binding, and
  prose cannot manufacture authority the runtime withheld.
- **Truthfulness**: tool evidence remains ground truth; failures and uncertainty
  must be reported; facts may be set aside but never invented.
- **Completion**: work is not done until checked; external actions require tool
  confirmation; running work and partial results cannot be presented as whole.
- **Mechanism**: authorization, ordering, stopping, schemas, limits, and required
  checks belong in code, types, tests, tool gates, and runtime policy.
- **Identity**: the short `The A is already yours` idea remains, without carrying
  a second operational playbook inside the constitution.

`constitutional_kernel_keeps_first_turn_authority_safety_and_completion` proves
these clauses are present in the fresh static prefix, not merely available to a
later tool call. The ordering test separately locks the complete precedence
chain.

## Procedural detail outside the constitution

These are useful procedures, not constitutional authority. They do not need to
out-shout every user turn:

- execution cadence and delegation live in the active mode doctrine and the
  compact Core Execution block;
- causal debugging is discoverable through the bundled `debug` skill;
- candidate comparison is discoverable through `best-of-n`;
- minimal-change cleanup is discoverable through `simplify`;
- deep verification workflows are discoverable through `verify`, `test`, and
  `review`, while the obligation to verify remains in the eager kernel;
- continuity structure is action-local to `/relay`.

This is not a byte-for-byte relocation of the deleted prose. The first-turn
duties remain in the kernel; these pre-existing skills and action-local surfaces
carry deeper procedure only when it is useful.

Language mirroring and terminal output formatting remain eager in this pass.
They are distinctive product behavior, and removing them was not necessary to
separate constitutional law from operational procedure.

## Other eager entries

The remaining measured entries are intentionally distinct from the bundled
constitutional kernel:

| Entry | Tokens | Why eager |
| --- | ---: | --- |
| Repository constitution | 667 | Repository-scoped authority |
| Project instructions | 4,459 | Project law; eagerly discovered like Pi's `AGENTS.md` loader |
| Skills routing index | 794 | Bounded metadata only; bodies load on demand |
| Agent mode doctrine | 268 | Current execution mode |
| Core Execution | 119 | Compact execution/tool discipline |
| Authority recap | 88 | Recency-safe pointer to the one precedence chain |
| Environment | 21 | Exact workspace and locale facts |

The dominant remaining cost is project-owned instruction text, not hidden base
prompt policy. That boundary is deliberately preserved: Codewhale, like Pi,
discovers and loads applicable `AGENTS.md`/`CLAUDE.md` files itself rather than
asking the model to guess that they exist.
