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
  metaTitle: "Codewhale — Create what you want.",
  metaDescription:
    "Build software, work with files, and automate tasks with Codewhale. Choose hosted or local models and switch providers as your work changes.",

  kicker: "Open-source AI agents",
  heroTitleA: "Create what you want.",
  heroTitleB: "Choose your models.",
  heroIntro:
    "{brand} gives you agents that can build software, work with files, and automate tasks. Use the models you choose, and switch providers as your work changes.",
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
  chapterTerminalTitle: "Start with something you want to make.",

  gainHeading: "Put your ideas to work.",
  gainLede:
    "Build a project, research a question, or automate a task. Start with one agent and share larger jobs across several.",
  gain: [
    [
      "Build something",
      "Turn an idea into working software. Your agents can edit files, run commands, and check the result.",
    ],
    [
      "Automate the repeat work",
      "Create scripts and workflows for tasks you repeat, then run them from the terminal.",
    ],
    [
      "Choose your models",
      "Connect hosted or local models. Give parts of a larger job to agents with different models and roles.",
    ],
  ],

  chapterModels: "Your models",
  modelsHeading: "Find a model that fits the task.",
  modelsBody:
    "Use a hosted provider, connect through a gateway, or run a model locally. Choose a provider and model for each session, and change them as you work.",
  modelsFacts: [
    ["Hosted", "Your own API key, saved with codewhale auth set --provider <id>"],
    ["Gateway", "One endpoint for many models, provider still chosen by you"],
    ["Local", "vLLM, SGLang, Ollama on localhost — usually no key"],
  ],
  modelsLink: "Explore models and providers",

  startHeading: "Start your first task.",
  startLede:
    "Install Codewhale, connect a model, and tell it what you want to do. Add a Fleet when you want several agents to share the work.",
  startGuideLink: "Read the getting-started guide",
  startVocabularyLink: "Look up a term",

  chapterAccount: "Get Codewhale",
  availabilityHeading: "Where to use Codewhale.",
  availabilityLede:
    "Start in the terminal. The app and cloud computers are in development.",
  availability: [
    [
      "Terminal",
      "Released",
      "GitHub release binaries for Linux, macOS, and Windows; npm and Cargo are alternatives. Android on Termux is a preview.",
    ],
    [
      "Web app",
      "Development preview",
      "Account access and browser pairing in the development preview.",
    ],
    [
      "Desktop",
      "Development build",
      "The macOS app is in development; a public download is coming later.",
    ],
    [
      "Cloud computers",
      "In development",
      "Hosted computers for running your tasks.",
    ],
  ],
  availabilityNote:
    "The terminal works without a Codewhale account. Hosted model usage is billed by your provider.",
  accountLink: "Create an account",

  surfacesHeading: "Use it where the work happens.",
  surfaces: [
    ["TUI", "Interactive terminal work"],
    ["codewhale exec", "Scripts and CI"],
    ["Local web client", "Localhost interface; hosted browser workbench in development"],
    ["Runtime API + MCP", "Local integrations"],
    ["Fleet", "Several agents on one job"],
  ],
  runtimeLink: "Explore integrations",

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
