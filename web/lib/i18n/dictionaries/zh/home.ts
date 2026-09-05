import type { HomeDict } from "../types";

/**
 * Simplified Chinese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — 你的模型，协作更强。",
  metaDescription:
    "Codewhale 是开源的智能体计算系统。把你已经在用的模型——托管、网关或本地——带进终端，让它们在你的机器上协同工作，由你掌控。Rust 编写，MIT 许可。",
  kicker: "智能体计算，由你定规矩",
  heroTitleA: "你的模型，",
  heroTitleB: "协作更强。",
  heroIntro:
    "{brand} 把编程智能体带进终端，用来读代码、改文件、跑检查。选择支持的模型，并设置会话的权限策略。开源，运行在你自己的机器上。",
  getCodewhale: "获取 Codewhale",
  exploreProduct: "了解产品",
  shotPreview: "终端预览",
  shotBuild: "v{version} 开发版本",
  screenshotAlt:
    "终端中的 Codewhale v0.9.12 开发版本：盲文点阵鲸鱼标志、尚无历史记录的新会话、消息输入框，以及显示 Full Access、Work 模式、两个计划任务、MCP 服务器连接中和 GLM-5.3 最高强度的状态栏",
  latestRelease: "最新发布 {tag}",
  releaseUnavailable: "发布状态暂不可用",
  currentSource: "源码",
  sourceCandidate: "未发布",
  providerRoutes: "{count} 个提供商",
  publishedRelease: "已发布",
  figcaptionSourceCandidate: "未发布",
  chapterTerminal: "你的终端",
  chapterTerminalTitle: "从熟悉的地方开始。",
  gainHeading: "让你的模型把任务做完。",
  gainLede: "先从一个模型开始。用 Fleet 保存成员配置；当任务适合分工时，再把部分工作委派给其他智能体。",
  gain: [
    ["你的模型", "使用支持的托管提供商、网关或本地模型服务。Fleet 保存可复用智能体角色的模型选择。"],
    ["能干的智能体", "Plan、Work、Operate 三种模式；一支 fleet 为同一件事分工；文件、shell、网页和 MCP 工具；可保存、恢复、回滚的会话。"],
    ["本机掌控", "Ask、Auto-Review 或 Full Access——它在问你之前能做多少，由你设定。本地运行，系统允许时启用沙箱，审计日志你随时能读。"],
  ],
  chapterModels: "你的模型",
  modelsHeading: "为你选择的模型留一个位置。",
  modelsBody:
    "连接支持的托管提供商、网关或本地模型服务。开始工作前，检查所选提供商与模型。本地服务是否需要 API 密钥，取决于它的配置。",
  modelsFacts: [
    ["托管", "你自己的 API 密钥，用 codewhale auth set 保存"],
    ["网关", "一个端点接多个模型，提供商仍由你选"],
    ["本地", "localhost 上的 vLLM、SGLang、Ollama——通常无需密钥"],
  ],
  modelsLink: "了解提供商选项",
  startHeading: "四步开始第一个会话。",
  startLede: "安装，无需密钥打开会话，接入提供商；一个模型不够时，再配置 fleet。",
  startGuideLink: "阅读新手指引",
  startVocabularyLink: "查名词",
  chapterAccount: "现在能在哪里运行",
  availabilityHeading: "已可用、开发中、暂不可用——如实说明。",
  availabilityLede: "终端是已发布的产品。其余的按实际状态列出。",
  availability: [
    ["终端", "已发布", "优先使用 GitHub Releases 中适用于 Linux、macOS、Windows 的二进制；npm 和 Cargo 为其他安装方式。Android 上的 Termux 为预览。"],
    ["网页应用", "开发预览", "账户页面与浏览器配对已在开发版本中实现。公开服务上的端到端远程控制尚未验证；已发布的任务执行入口是终端。"],
    ["桌面端", "开发版本", "本地 macOS 开发构建已做过测试。尚无已发布的桌面应用可供下载。"],
    ["云端计算机", "暂不可用", "在托管计算机上运行工作仍在开发中。等它真正可用时，本页会如实说明。"],
  ],
  availabilityNote: "终端不需要 Codewhale 账户。托管模型按你自己的提供商账户计费；创建 Codewhale 账户不会购买模型访问权限。",
  accountLink: "创建账户",
  surfacesHeading: "活在哪里干，就在哪里用。",
  surfaces: [
    ["TUI", "交互式终端工作"],
    ["codewhale exec", "脚本与 CI"],
    ["本地 Web 客户端", "本机界面；托管浏览器工作台仍在开发中"],
    ["运行时 API + MCP", "本地集成"],
    ["fleet", "多个智能体协作一件事"],
  ],
  runtimeLink: "运行时界面与稳定程度",
  installBandHeading: "在 macOS 或 Linux 上安装。",
  copy: "复制",
  copied: "已复制 ✓",
  binaries: "预编译包",
  chinaMirrors: "中国镜像",
  installGuideLink: "阅读安装指南",
  communityHeading: "公开构建",
  communityBody: "MIT 许可。贡献者的工作覆盖运行时、提供商、平台、文档与测试。",
  communityLinksAria: "社区链接",
  contribute: "参与贡献",
};
