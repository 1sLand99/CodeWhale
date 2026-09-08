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
  metaTitle: "Codewhale — Build and automate with the models you choose",
  metaDescription:
    "Build software, work with your files, and automate everyday tasks using open-source agents and your choice of hosted or local AI models.",

  heroTitle: "Build and automate with the models you choose",
  heroIntro:
    "{brand} gives you agents that can build software, work with your files, and turn repetitive tasks into reusable workflows. Tell them what you want to accomplish and choose the hosted or local models that suit the job, with the freedom to switch providers as you go.",
  getCodewhale: "Get Codewhale",
  exploreProduct: "Explore the product",

  shotPreview: "Terminal preview",
  shotBuild: "v{version} development build",
  screenshotAlt:
    "Codewhale v0.9.12 terminal, build 171acee689aa: a new session with the whale mark, message composer, Full Access and Operate mode, two scheduled tasks, 21 MCP servers connecting, and GLM-5.3 at max effort.",

  latestRelease: "Latest release {tag}",
  releaseUnavailable: "Release status unavailable",
  currentSource: "Source",
  sourceCandidate: "Unreleased",
  providerRoutes: "{count} providers",
  publishedRelease: "released",
  figcaptionSourceCandidate: "unreleased",

  chapterTerminal: "Your terminal",
  chapterTerminalTitle: "Start with something you want to make",

  gainHeading: "What you can do with Codewhale",
  gainLede:
    "Start with a project, a question, or a task you want to automate, then work with one agent or give parts of a larger job to several.",
  gain: [
    [
      "Build something",
      "Describe what you want to make and work with agents that can read your code, edit files, run commands, and check the result."
    ],
    [
      "Automate everyday work",
      "Create scripts and workflows for tasks you repeat, so you can run them again from the terminal whenever you need them."
    ],
    [
      "Work with different models",
      "Use hosted or local models for your agents, with different models and roles handling the parts of a job they are suited to."
    ]
  ],

  chapterModels: "Your models",
  modelsHeading: "A choice of models for every task",
  modelsBody:
    "Connect directly to a hosted provider, use a gateway to access several, or run a model locally, then choose which model each session uses as you work.",
  modelsFacts: [
    ["Hosted", "Your own API key, saved with codewhale auth set --provider <id>"],
    ["Gateway", "One endpoint for many models, provider still chosen by you"],
    ["Local", "vLLM, SGLang, Ollama on localhost — usually no key"],
  ],
  modelsLink: "Explore models and providers",

  startHeading: "Getting started with Codewhale",
  startLede:
    "Once you have installed Codewhale and connected a model, you can describe your first task in the terminal and add a Fleet when you want several agents to share the work.",
  startGuideLink: "Read the getting-started guide",
  startVocabularyLink: "Look up a term",

  chapterAccount: "Get Codewhale",
  availabilityHeading: "Where you can use Codewhale",
  availabilityLede:
    "You can use Codewhale in your terminal today while we build the web app, desktop app, and cloud computers.",
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
    "You can use the terminal without a Codewhale account, and any hosted model usage is billed by your provider.",
  accountLink: "Create an account",

  surfacesHeading: "Ways to work with Codewhale",
  surfaces: [
    ["TUI", "Interactive terminal work"],
    ["codewhale exec", "Scripts and CI"],
    ["Local web client", "Localhost interface; hosted browser workbench in development"],
    ["Runtime API + MCP", "Local integrations"],
    ["Fleet", "Several agents on one job"],
  ],
  runtimeLink: "Explore integrations",

  installBandHeading: "Install Codewhale on macOS or Linux",
  copy: "Copy",
  copied: "Copied ✓",
  binaries: "Binaries",
  chinaMirrors: "China mirrors",
  installGuideLink: "Read the install guide",

  communityHeading: "Help make Codewhale better",
  communityBody:
    "Whether you have found a bug, have an idea for a feature, or want to send your first pull request, we would like to hear from you and work together on what comes next.",
  communityLinksAria: "Community links",
  contribute: "Send a pull request",
};
