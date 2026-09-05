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
      title: { en: "Supported models, in one place", zh: "支持的模型，都在一处" },
      body: {
        en: "Connect a supported hosted provider, a gateway, or a local model server. Check the selected provider and model before starting work. Local servers may run without an API key, depending on their configuration.",
        zh: "连接支持的托管提供商、网关或本地模型服务。开始工作前，检查所选提供商与模型。本地服务是否需要 API 密钥，取决于它的配置。",
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
        en: "Plan blocks file mutation and shell execution; permitted research can still contact external services. Work executes tasks. Operate emphasizes delegation. Independently, Ask, Auto-Review, and Full Access set how much happens before it asks you. The OS sandbox is used where the platform provides one, and an audit log records sensitive events.",
        zh: "Plan 禁止文件修改与 shell 执行；获准的研究仍可访问外部服务。Work 执行任务，Operate 侧重分工。另一维度上，Ask、Auto-Review 和 Full Access 决定它在问你之前能做多少。平台提供沙箱时就启用沙箱，审计日志记录敏感事件。",
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
        en: "Install GitHub release binaries for Linux, macOS, and Windows. npm and Cargo are alternatives. Android on Termux is a preview. The interactive TUI and codewhale exec for scripts ship together.",
        zh: "优先使用 GitHub Releases 中适用于 Linux、macOS、Windows 的二进制；npm 和 Cargo 为其他安装方式。Android 上的 Termux 为预览。交互式 TUI 与用于脚本的 codewhale exec 一同发布。",
      },
      href: "/install",
      linkLabel: { en: "Install guide", zh: "安装指南" },
    },
    {
      surface: { en: "Web app", zh: "网页应用" },
      status: { en: "Development preview", zh: "开发预览" },
      detail: {
        en: "Account pages and browser pairing are implemented in development. Public end-to-end remote control has not been verified; use the terminal for released task execution.",
        zh: "账户页面与浏览器配对已在开发版本中实现。公开服务上的端到端远程控制尚未验证；已发布的任务执行入口是终端。",
      },
      href: "/signin",
      linkLabel: { en: "Sign in", zh: "登录" },
    },
    {
      surface: { en: "Desktop", zh: "桌面端" },
      status: { en: "Development build", zh: "开发版本" },
      detail: {
        en: "A local macOS development build has been tested. There is no released desktop app to download.",
        zh: "本地 macOS 开发构建已做过测试。尚无已发布的桌面应用可供下载。",
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
    { title: { en: "Plan", zh: "Plan" }, body: { en: "Blocks file mutation and shell execution. Permitted research may contact external services; session state can still be saved.", zh: "禁止文件修改与 shell 执行。获准的研究可访问外部服务；会话状态仍可保存。" } },
    { title: { en: "Work", zh: "Work" }, body: { en: "Edits files and runs commands within the permission you set.", zh: "在你设定的权限内修改文件、运行命令。" } },
    { title: { en: "Operate", zh: "Operate" }, body: { en: "Runs a fleet: several agents on one job, each in its role.", zh: "调度 fleet：多个智能体各司其职，处理同一件事。" } },
  ] satisfies ProductRow[],
  permissions: [
    { title: { en: "Ask", zh: "Ask" }, body: { en: "Prompts according to the active approval rules; saved permissions and hard policy boundaries still apply.", zh: "按当前审批规则询问；已保存的权限与强制策略边界仍然生效。" } },
    { title: { en: "Auto-Review", zh: "Auto-Review" }, body: { en: "Uses automated review for eligible actions; unresolved approval decisions return to you.", zh: "对符合条件的操作进行自动审核；未解决的审批仍交给你。" } },
    { title: { en: "Full Access", zh: "Full Access" }, body: { en: "Reduces approval prompts. It does not bypass hard policy boundaries or grant access outside the allowed scope.", zh: "减少审批提示，但不会绕过强制策略边界，也不会授予允许范围之外的访问权限。" } },
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
