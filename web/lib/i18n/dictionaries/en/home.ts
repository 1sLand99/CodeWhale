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
    "{brand} brings the models you already use into one terminal and lets them work as a crew — reading your code, editing, running the checks — while you decide what each one may do. Open source, on your machine.",
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

  gainHeading: "What you get is not a chatbot. It is leverage over the models you already pay for.",
  gainLede:
    "One session can hold several models at once, each in a role you gave it, all working in the same repository under the same rules.",
  gain: [
    [
      "Your models",
      "Hosted keys, a gateway, or a local runtime with no key at all. Pin a different model to each role and keep the provider you chose — a model name never switches it for you.",
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
  modelsHeading: "Bring what you have. Change nothing you did not choose.",
  modelsBody:
    "Codewhale ships with {count} providers and treats them as peers. Save a key once, name a model, and the route stays exactly as you set it. Local models over vLLM, SGLang, or Ollama need no key.",
  modelsFacts: [
    ["Hosted", "Your own API key, saved with codewhale auth set"],
    ["Gateway", "One endpoint for many models, provider still chosen by you"],
    ["Local", "vLLM, SGLang, Ollama on localhost — usually no key"],
  ],
  modelsLink: "See every provider",

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
      "npm, Cargo, and prebuilt binaries for Linux, macOS, and Windows. Android on Termux is a preview.",
    ],
    [
      "Web app",
      "Account sign-in available",
      "Sign in or create an account at app.codewhale.net. The browser workbench itself is a development preview.",
    ],
    [
      "Desktop",
      "Development build",
      "Alpha builds exist for macOS, Linux, and Windows. There is no released desktop app yet.",
    ],
    [
      "Cloud computers",
      "Not available yet",
      "Running work on a hosted computer is in development. This page will say so when it works.",
    ],
  ],
  availabilityNote:
    "No account is needed for the terminal. An account is never a paid plan by itself, and nothing on this site can charge you.",
  accountLink: "Create an account",

  surfacesHeading: "Use it where the work happens.",
  surfaces: [
    ["TUI", "Interactive terminal work"],
    ["codewhale exec", "Scripts and CI"],
    ["Web client", "Browser client, localhost only"],
    ["Runtime API + MCP", "Local integrations"],
    ["Fleet", "Several agents on one job"],
  ],
  runtimeLink: "Runtime surfaces and what is stable",

  installBandHeading: "Start with one command.",
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
