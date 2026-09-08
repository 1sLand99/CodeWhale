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
      en: "Run the command below on macOS or Linux, or open the install guide for Windows, package managers, and instructions for building from source.",
      zh: "在 macOS 或 Linux 上运行下方命令即可安装；如果使用 Windows、包管理器，或想从源码编译，可以查看完整安装指南。",
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
      en: "Connect a local model or use your own provider key, as shown here for DeepSeek; the provider guide walks through the other options.",
      zh: "你可以连接本地模型，也可以使用自己的提供商密钥；下面以 DeepSeek 为例，其他连接方式见提供商指南。",
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
      en: "Open Codewhale in your project folder and ask it to explain the code, build a feature, or automate a task. You can change providers with /provider, select a model with /model, and choose how it works with /mode.",
      zh: "在项目文件夹中打开 Codewhale，就可以让它解释代码、开发功能或自动完成任务。工作过程中可以用 /provider 切换提供商、用 /model 选择模型，并用 /mode 调整工作模式。",
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
      en: "When a task would benefit from several models and roles, run /fleet setup inside Codewhale to put a team together. You can see the saved Fleet from your shell with codewhale fleet status.",
      zh: "当任务需要多个模型和角色配合时，可以在 Codewhale 中运行 /fleet setup 来配置团队，然后在 shell 中用 codewhale fleet status 查看已保存的 Fleet。",
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
      en: "Browse and search the documentation for help on a specific topic, with a link from each page to its source in the repository.",
      zh: "你可以浏览或搜索文档来查找需要的帮助，并通过每页的链接查看仓库中的源文档。",
    },
  },
];
