import type { HomeDict } from "../types";

/**
 * English reference home dictionary — the copy contract for the Tidal Folio
 * landing page. Public-copy and public-surface tests assert against these
 * values, not against raw JSX strings.
 *
 * The page leads with what a person gains — their own models, capable
 * agents, and control on their own machine — and states availability per
 * surface as it is today. Nothing here claims cloud execution, and the
 * screenshot is described as the development build it is.
 */
export const home: HomeDict = {
  metaTitle: "Codewhale — Your models. More capable together.",
  metaDescription:
    "Codewhale is an open-source agentic computing system. Bring the models you already use — hosted, through a gateway, or local — and put them to work together in your terminal, on your machine, with you in control. Rust, MIT.",

  kicker: "Agentic computing, on your terms",
  heroTitleA: "Your models.",
  heroTitleB: "More capable together.",
  heroIntro:
    "{brand} puts coding agents in your terminal to read code, edit files, and run checks. Choose supported models and set the session’s permissions. Open source, on your machine.",
  getCodewhale: "Get Codewhale",
  exploreProduct: "Explore the product",

  shotPreview: "Terminal preview",
  shotBuild: "v{version} development build",
  screenshotAlt:
    "Codewhale v0.9.12 development build in a terminal: the braille whale mark, a new session with no recent sessions yet, the message composer, and a footer showing Full Access, Work mode, two scheduled tasks, MCP servers connecting, and the GLM-5.3 model at max effort",

  latestRelease: "Latest release {tag}",
  releaseUnavailable: "Release status unavailable",
  currentSource: "Source",
  sourceCandidate: "Unreleased",
  providerRoutes: "{count} providers",
  publishedRelease: "released",
  figcaptionSourceCandidate: "unreleased",

  chapterTerminal: "Your terminal",
  chapterTerminalTitle: "A familiar place to begin.",

  gainHeading: "Give your models a job to finish.",
  gainLede:
    "Start with one model. Use Fleet to save a roster, and delegate parts of a task when the work benefits from more than one agent.",
  gain: [
    [
      "Your models",
      "Use a supported hosted provider, gateway, or local model server. Fleet stores model choices for reusable agent roles.",
    ],
    [
      "Capable agents",
      "Plan, Work, and Operate modes; a fleet of sub-agents for one job; tools for files, shell, web, and MCP; sessions that save, resume, and roll back.",
    ],
    [
      "Control on your machine",
      "Ask, Auto-Review, or Full Access — you set how much it does before it asks. Runs locally, sandboxed where the OS allows, with an audit log you can read.",
    ],
  ],

  chapterModels: "Your models",
  modelsHeading: "A place for the models you choose.",
  modelsBody:
    "Connect a supported hosted provider, a gateway, or a local model server. Check the selected provider and model before starting work. Local servers may run without an API key, depending on their configuration.",
  modelsFacts: [
    ["Hosted", "Your own API key, saved with codewhale auth set"],
    ["Gateway", "One endpoint for many models, provider still chosen by you"],
    ["Local", "vLLM, SGLang, Ollama on localhost — usually no key"],
  ],
  modelsLink: "Explore provider options",

  startHeading: "Four steps to a first session.",
  startLede:
    "Install, open a session with no key, connect a provider, then set up a fleet when one model is not enough.",
  startGuideLink: "Read the getting-started guide",
  startVocabularyLink: "Look up a term",

  chapterAccount: "Where it runs today",
  availabilityHeading: "Available now, in development, and not yet — stated plainly.",
  availabilityLede:
    "The terminal is the released product. Everything else is listed with the state it is actually in.",
  availability: [
    [
      "Terminal",
      "Released",
      "GitHub release binaries for Linux, macOS, and Windows; npm and Cargo are alternatives. Android on Termux is a preview.",
    ],
    [
      "Web app",
      "Development preview",
      "Account pages and browser pairing are implemented in development. Public end-to-end remote control has not been verified; use the terminal for released task execution.",
    ],
    [
      "Desktop",
      "Development build",
      "A local macOS development build has been tested. There is no released desktop app to download.",
    ],
    [
      "Cloud computers",
      "Not available yet",
      "Running work on a hosted computer is in development. This page will say so when it works.",
    ],
  ],
  availabilityNote:
    "The terminal needs no Codewhale account. Hosted model providers bill under your own provider account; creating a Codewhale account does not purchase model access.",
  accountLink: "Create an account",

  surfacesHeading: "Use it where the work happens.",
  surfaces: [
    ["TUI", "Interactive terminal work"],
    ["codewhale exec", "Scripts and CI"],
    ["Local web client", "Localhost interface; hosted browser workbench in development"],
    ["Runtime API + MCP", "Local integrations"],
    ["Fleet", "Several agents on one job"],
  ],
  runtimeLink: "Runtime surfaces and what is stable",

  installBandHeading: "Install on macOS or Linux.",
  copy: "Copy",
  copied: "Copied ✓",
  binaries: "Binaries",
  chinaMirrors: "China mirrors",
  installGuideLink: "Read the install guide",

  communityHeading: "Built in public",
  communityBody:
    "MIT license. Contributors work on the runtime, providers, platforms, docs, and tests.",
  communityLinksAria: "Community links",
  contribute: "Contribute",
};
