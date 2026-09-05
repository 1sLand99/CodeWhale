/**
 * getting-started.ts — the canonical new-user path for codewhale.net.
 *
 * Four steps, in order: install → first offline session → provider connection
 * → fleet setup. Both the homepage band and the /docs/guide page
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
      en: "Install a published GitHub release on macOS or Linux. Run codewhale update for later releases. npm and Cargo remain alternatives in the full guide. Local development builds are separate.",
      zh: "在 macOS 或 Linux 上安装 GitHub 已发布版本。之后运行 codewhale update 获取新版本。完整指南中也提供 npm 和 Cargo 安装方式。本地开发构建与已发布版本分开。",
    },
    commands: ["curl -fsSL https://codewhale.net/install.sh | sh", "codewhale doctor"],
    link: {
      href: "/install",
      label: { en: "Full install guide", zh: "完整安装指南" },
    },
  },
  {
    id: "first-session",
    title: { en: "Open a first session — no key needed", zh: "打开第一个会话——无需密钥" },
    body: {
      en: "The terminal can open without an API key; model replies need a configured provider. Plan blocks file mutation and shell execution. Permitted research can still contact external services.",
      zh: "终端可在没有 API 密钥时打开；模型回复需要配置提供商。Plan 禁止文件修改与 shell 执行，但获准的研究请求仍可能访问外部服务。",
    },
    commands: ["codewhale"],
    link: {
      href: "/docs/vocabulary",
      label: { en: "Learn the product nouns first", zh: "先了解产品名词" },
    },
  },
  {
    id: "connect-provider",
    title: { en: "Connect a provider", zh: "连接提供商" },
    body: {
      en: "Configure a supported provider with your own credentials, or connect a local model server. Check the selected provider, endpoint and model before sending a task. Local servers may require authentication.",
      zh: "用你自己的凭据配置支持的提供商，或连接本地模型服务。发送任务前检查所选提供商、端点与模型。本地服务也可能要求身份验证。",
    },
    commands: ["codewhale auth set --provider deepseek"],
    link: {
      href: "/models",
      label: { en: "Providers and models", zh: "提供商与模型" },
    },
  },
  {
    id: "fleet-workflow",
    title: { en: "Optional: set up a fleet", zh: "可选：配置 fleet" },
    body: {
      en: "Optional in v0.9.11: /fleet setup edits the selected named fleet, or opens profile setup when none is selected. Review the model and save scope before saving. Fleet configuration does not grant execution permissions. You can start with a single agent.",
      zh: "v0.9.11 的可选步骤：/fleet setup 编辑当前选中的命名 fleet；未选中时则打开角色档案设置。保存前检查模型与保存范围。Fleet 配置不会授予执行权限。你可以先从一个智能体开始。",
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
