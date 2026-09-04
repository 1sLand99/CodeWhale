/**
 * product.ts — the copy for /product, the "what is this and what do I get"
 * page behind the primary nav's first link.
 *
 * TRUTH CONTRACT: availability is stated per surface as it is today and
 * mirrors the homepage's availability chapter and docs/public-surface-facts.json.
 * Nothing here claims cloud execution; the web app is described as account
 * sign-in plus a development preview; desktop is a development build.
 * Counts (providers, tools, sandbox backends) come from the facts layer at
 * render time, never typed here.
 */

import type { LocalizedText } from "./vocabulary";

export interface ProductRow {
  title: LocalizedText;
  body: LocalizedText;
}

export interface ProductAvailabilityRow {
  surface: LocalizedText;
  status: LocalizedText;
  detail: LocalizedText;
  /** Locale-relative route with the full story, or null. */
  href: string | null;
  linkLabel: LocalizedText | null;
}

export const PRODUCT_COPY = {
  metadata: {
    title: { en: "Product · Codewhale", zh: "产品 · Codewhale" },
    description: {
      en: "What Codewhale is and what a person gains: their own models, capable agents, and control on their own machine — with availability stated per surface.",
      zh: "Codewhale 是什么、你能得到什么：你自己的模型、能干的智能体、以及在自己机器上的掌控——并按界面如实说明可用状态。",
    },
  },
  title: {
    en: "One terminal. Your models, working as a crew.",
    zh: "一个终端。你的模型，像一支船员一样协作。",
  },
  lede: {
    en: "Codewhale is an open-source agentic computing system. It holds the models you already use — hosted, through a gateway, or on your own machine — and lets them work together on a repository under rules you set. You keep the keys, the choice of model, and the last word on what runs.",
    zh: "Codewhale 是开源的智能体计算系统。它容纳你已经在用的模型——托管、网关或本机——让它们在你设定的规则下协同处理一个仓库。密钥、模型选择和最终决定权都在你手里。",
  },

  gainHeading: { en: "What you gain", zh: "你能得到什么" },
  gain: [
    {
      title: { en: "Every model you already pay for, in one place", zh: "你已付费的每个模型，都在一处" },
      body: {
        en: "{count} providers are built in and treated as peers. Save a key once, name a model, and the route stays exactly as you set it. Local models over vLLM, SGLang, or Ollama need no key at all.",
        zh: "内置 {count} 个提供商，一视同仁。保存一次密钥，指定一个模型，路由就保持你设定的样子。vLLM、SGLang 或 Ollama 上的本地模型完全不需要密钥。",
      },
    },
    {
      title: { en: "A crew, not a single assistant", zh: "一支船员，而不是一个助手" },
      body: {
        en: "Pin a different model to each role. Run a fleet of sub-agents on one job. Give them files, a shell, the web, and MCP servers as tools. Sessions save, resume, and roll back.",
        zh: "给每个角色固定不同的模型。让一支 fleet 子智能体分工处理同一件事。文件、shell、网页和 MCP 服务器都是它们的工具。会话可保存、恢复、回滚。",
      },
    },
    {
      title: { en: "Control that stays on your machine", zh: "掌控权留在你的机器上" },
      body: {
        en: "Plan is read-only. Work executes. Operate runs a fleet. Independently, Ask, Auto-Review, and Full Access set how much happens before it asks you. The OS sandbox is used where the platform provides one, and an audit log records sensitive events.",
        zh: "Plan 只读。Work 执行。Operate 调度 fleet。另一维度上，Ask、Auto-Review 和 Full Access 决定它在问你之前能做多少。平台提供沙箱时就启用沙箱，审计日志记录敏感事件。",
      },
    },
  ] satisfies ProductRow[],

  availabilityHeading: { en: "Where it runs today", zh: "现在能在哪里运行" },
  availabilityLede: {
    en: "The terminal is the released product. Everything else is listed with the state it is actually in, and this page changes when that state does.",
    zh: "终端是已发布的产品。其余按实际状态列出；状态变化时，本页也会随之更新。",
  },
  availability: [
    {
      surface: { en: "Terminal", zh: "终端" },
      status: { en: "Released", zh: "已发布" },
      detail: {
        en: "Install with npm, Cargo, or a prebuilt binary for Linux, macOS, and Windows. Android on Termux is a preview. The interactive TUI and codewhale exec for scripts ship together.",
        zh: "通过 npm、Cargo 或 Linux、macOS、Windows 的预编译二进制安装。Android 上的 Termux 为预览。交互式 TUI 与用于脚本的 codewhale exec 一同发布。",
      },
      href: "/install",
      linkLabel: { en: "Install guide", zh: "安装指南" },
    },
    {
      surface: { en: "Web app", zh: "网页应用" },
      status: { en: "Account sign-in available", zh: "账户登录可用" },
      detail: {
        en: "Sign in or create an account. The browser workbench itself is a development preview; no account is required for the terminal, and an account is never a paid plan by itself.",
        zh: "登录或创建账户。浏览器工作台本身仍是开发预览；终端不需要账户，账户本身也从不等于付费方案。",
      },
      href: "/signin",
      linkLabel: { en: "Sign in", zh: "登录" },
    },
    {
      surface: { en: "Desktop", zh: "桌面端" },
      status: { en: "Development build", zh: "开发版本" },
      detail: {
        en: "Alpha builds exist for macOS, Linux, and Windows. There is no released desktop app yet.",
        zh: "macOS、Linux、Windows 有 alpha 构建。桌面应用尚未正式发布。",
      },
      href: null,
      linkLabel: null,
    },
    {
      surface: { en: "Cloud computers", zh: "云端计算机" },
      status: { en: "Not available yet", zh: "暂不可用" },
      detail: {
        en: "Running work on a hosted computer is in development. Codewhale will say so here when it works; a passing local test is not that.",
        zh: "在托管计算机上运行工作仍在开发中。等它真正可用时，这里会如实说明；本地测试通过不等于可用。",
      },
      href: null,
      linkLabel: null,
    },
  ] satisfies ProductAvailabilityRow[],

  controlHeading: { en: "How much it does before it asks", zh: "它在问你之前能做多少" },
  controlLede: {
    en: "Two independent dials. The mode says what kind of work a session may do; the permission says how much of it happens without a question.",
    zh: "两个独立的旋钮。模式决定会话可以做哪类工作；权限决定其中多少无需询问。",
  },
  modes: [
    { title: { en: "Plan", zh: "Plan" }, body: { en: "Read-only. It inspects and proposes; nothing is written.", zh: "只读。它检查并提出方案，不写入任何东西。" } },
    { title: { en: "Work", zh: "Work" }, body: { en: "Edits files and runs commands within the permission you set.", zh: "在你设定的权限内修改文件、运行命令。" } },
    { title: { en: "Operate", zh: "Operate" }, body: { en: "Runs a fleet: several agents on one job, each in its role.", zh: "调度 fleet：多个智能体各司其职，处理同一件事。" } },
  ] satisfies ProductRow[],
  permissions: [
    { title: { en: "Ask", zh: "Ask" }, body: { en: "Every write and every command waits for you.", zh: "每次写入、每条命令都等你确认。" } },
    { title: { en: "Auto-Review", zh: "Auto-Review" }, body: { en: "Routine steps proceed; risky ones still ask.", zh: "常规步骤自动进行，高风险步骤仍会询问。" } },
    { title: { en: "Full Access", zh: "Full Access" }, body: { en: "Runs without asking. You choose this; it is never the default.", zh: "不询问直接运行。由你主动选择，绝不是默认。" } },
  ] satisfies ProductRow[],

  surfacesHeading: { en: "Surfaces in the released terminal", zh: "已发布终端中的界面" },
  surfacesLede: {
    en: "One runtime, several ways in. Every surface runs the same engine, tools, and permissions on your machine.",
    zh: "一个运行时，多种入口。每个界面都跑同一套引擎、工具与权限，都在你的机器上。",
  },
  surfacesLink: { en: "Runtime surfaces and what is stable", zh: "运行时界面与稳定程度" },

  actions: {
    install: { en: "Get Codewhale", zh: "获取 Codewhale" },
    models: { en: "See every provider", zh: "查看所有提供商" },
    docs: { en: "Read the docs", zh: "阅读文档" },
  },
} as const;
