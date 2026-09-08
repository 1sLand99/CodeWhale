/**
 * getting-started.ts — the canonical new-user path for codewhale.net.
 *
 * Four steps, in order: install → provider connection → first task
 * → optional fleet setup. Both the homepage band and the /docs/guide page
 * render from this module, so the path reads identically everywhere.
 *
 * TRUTH CONTRACT:
 *   - Step copy must match documented behavior in docs/GUIDE.md, docs/MODES.md,
 *     docs/PROVIDERS.md, and docs/FLEET.md. The runtime launches without any
 *     API key (recommended working-agreement setup); model replies require a provider —
 *     hosted key or a keyless loopback route. Do not imply otherwise.
 *   - `href` values are locale-relative (no locale prefix); consumers render
 *     `/${locale}${href}` and the tests assert every target route exists.
 *
 * EXTENSION PATH FOR NEW LOCALES: add the locale key to each `{ en, zh }`
 * pair; commands stay locale-agnostic shell.
 */

import type { LocalizedText } from "./vocabulary";

export interface GuideStep {
  id: "install" | "first-session" | "connect-provider" | "fleet-workflow";
  title: LocalizedText;
  body: LocalizedText;
  /** Locale-agnostic shell commands shown for the step (may be empty). */
  commands: string[];
  /** Deeper-reading link; href is locale-relative. */
  link: { href: string; label: LocalizedText };
}

export const GETTING_STARTED_STEPS: GuideStep[] = [
  {
    id: "install",
    title: { en: "Install Codewhale", zh: "安装 Codewhale" },
    body: {
      en: "On macOS or Linux, run the command below. The install guide also covers Windows, package managers, and building from source.",
      zh: "在 macOS 或 Linux 上运行下方命令。安装指南也介绍 Windows、包管理器和源码编译方式。",
    },
    commands: ["curl -fsSL https://codewhale.net/install.sh | sh"],
    link: {
      href: "/install",
      label: { en: "Full install guide", zh: "完整安装指南" },
    },
  },
  {
    id: "connect-provider",
    title: { en: "Connect your model", zh: "连接你的模型" },
    body: {
      en: "Use your own provider key or connect a local model. This example saves a DeepSeek key; the provider guide covers the other options.",
      zh: "使用你自己的提供商密钥，或连接本地模型。此示例保存 DeepSeek 密钥；其他选项见提供商指南。",
    },
    commands: ["codewhale auth set --provider deepseek"],
    link: {
      href: "/models",
      label: { en: "Choose a provider", zh: "选择提供商" },
    },
  },
  {
    id: "first-session",
    title: { en: "Give it a task", zh: "交给它一项任务" },
    body: {
      en: "Open Codewhale in your project folder. Ask it to explain the code, build a feature, or automate a task. Use /provider and /model to change your selection, and /mode to choose how it works.",
      zh: "在项目文件夹中打开 Codewhale。让它解释代码、开发功能或自动完成任务。用 /provider 和 /model 切换选择，用 /mode 选择工作模式。",
    },
    commands: ["codewhale"],
    link: {
      href: "/docs/modes",
      label: { en: "Modes and permissions", zh: "模式与权限" },
    },
  },
  {
    id: "fleet-workflow",
    title: { en: "Add a Fleet when you need one", zh: "需要时配置 Fleet" },
    body: {
      en: "Start with one agent. When you want a roster of models and roles, run /fleet setup inside Codewhale. From your shell, codewhale fleet status shows the saved Fleet.",
      zh: "先从一个智能体开始。需要配置模型与角色时，在 Codewhale 中运行 /fleet setup。在 shell 中运行 codewhale fleet status 可查看已保存的 Fleet。",
    },
    commands: ["/fleet setup", "codewhale fleet status"],
    link: {
      href: "/docs/fleet",
      label: { en: "Fleet and Workflow docs", zh: "Fleet 与 Workflow 文档" },
    },
  },
];

/**
 * Where to go after the path — discovery links rendered at the end of the
 * /docs/guide page. Hooks are first-class here on purpose: they are the
 * supported extension point a new user should find without digging.
 */
export const GUIDE_NEXT_LINKS: { href: string; label: LocalizedText; note: LocalizedText }[] = [
  {
    href: "/docs/hooks",
    label: { en: "Hooks", zh: "钩子" },
    note: {
      en: "Run your own commands before and after tool calls, at turn end, and on session events, with per-project trust rules.",
      zh: "借助项目级信任规则，响应生命周期事件——工具调用前后、回合结束、会话事件。",
    },
  },
  {
    href: "/docs/modes",
    label: { en: "Modes and permissions", zh: "模式与权限" },
    note: {
      en: "Plan / Work / Operate and Ask / Auto-Review / Full Access: what each one allows.",
      zh: "Plan / Work / Operate 与 Ask / Auto-Review / Full Access：各自允许做什么。",
    },
  },
  {
    href: "/docs",
    label: { en: "Documentation hub", zh: "文档中心" },
    note: {
      en: "Every topic, searchable. Each page links to its source document in the repository.",
      zh: "所有主题均可搜索。每页都链接到仓库中的源文档。",
    },
  },
];
