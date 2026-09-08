import type { HomeDict } from "../types";

/**
 * Simplified Chinese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — 用你选择的模型，完成开发和自动化任务",
  metaDescription:
    "借助开源智能体和你选择的云端或本地 AI 模型，开发软件、处理文件，并自动完成日常任务。",
  heroTitle: "用你选择的模型，完成开发和自动化任务",
  heroIntro:
    "{brand} 为你提供能够开发软件、处理文件，并将重复任务转化为可复用工作流程的智能体。告诉它们你想完成什么，再选择适合这项工作的云端或本地模型，你还可以在工作过程中自由切换提供商。",
  getCodewhale: "获取 Codewhale",
  exploreProduct: "了解产品",
  shotPreview: "终端预览",
  shotBuild: "v{version} 开发版本",
  screenshotAlt:
    "Codewhale v0.9.12 终端，构建版本 171acee689aa：一个新会话，显示鲸鱼标志、消息输入区、Full Access 和 Operate 模式、两个定时任务、21 个正在连接的 MCP 服务器，以及推理强度设为最高的 GLM-5.3。",
  latestRelease: "最新发布 {tag}",
  releaseUnavailable: "发布状态暂不可用",
  currentSource: "源码",
  sourceCandidate: "未发布",
  providerRoutes: "{count} 个提供商",
  publishedRelease: "已发布",
  figcaptionSourceCandidate: "未发布",
  chapterTerminal: "你的终端",
  chapterTerminalTitle: "从你想做的项目开始",
  gainHeading: "你可以用 Codewhale 做什么",
  gainLede: "从一个项目、一个问题，或一项希望自动处理的任务开始，你可以与一个智能体一起完成，也可以将较大的工作拆分给多个智能体。",
  gain: [
    [
      "构建项目",
      "描述你想做什么，然后与能够阅读代码、编辑文件、运行命令并检查结果的智能体一起完成。"
    ],
    [
      "自动处理日常工作",
      "为重复执行的任务创建脚本和工作流程，以便在需要时随时从终端再次运行。"
    ],
    [
      "使用不同的模型",
      "为智能体选择云端或本地模型，让不同模型和角色承担各自适合的工作。"
    ]
  ],
  chapterModels: "你的模型",
  modelsHeading: "为每项任务选择合适的模型",
  modelsBody:
    "你可以直接连接云端模型提供商，通过网关访问多家提供商，或在本地运行模型，并在工作过程中选择每个会话使用的模型。",
  modelsFacts: [
    ["托管", "你自己的 API 密钥，用 codewhale auth set --provider <id> 保存"],
    ["网关", "一个端点接多个模型，提供商仍由你选"],
    ["本地", "localhost 上的 vLLM、SGLang、Ollama——通常无需密钥"],
  ],
  modelsLink: "了解模型与提供商",
  startHeading: "开始使用 Codewhale",
  startLede: "安装 Codewhale 并连接模型后，你就可以在终端中描述第一项任务，并在希望多个智能体分担工作时添加一个 Fleet。",
  startGuideLink: "阅读新手指引",
  startVocabularyLink: "查名词",
  chapterAccount: "获取 Codewhale",
  availabilityHeading: "你可以在哪里使用 Codewhale",
  availabilityLede: "你现在就可以在终端中使用 Codewhale，同时我们正在开发网页应用、桌面应用和云端计算机。",
  availability: [
    [
      "终端",
      "已发布",
      "GitHub 提供适用于 Linux、macOS 和 Windows 的发布版二进制文件；也可通过 npm 或 Cargo 安装。在 Android 上通过 Termux 运行的版本为预览版。"
    ],
    [
      "网页应用",
      "开发预览",
      "开发预览版提供账户访问与浏览器配对。"
    ],
    [
      "桌面端",
      "开发版本",
      "macOS 应用仍在开发中，稍后将提供公开下载。"
    ],
    [
      "云端计算机",
      "开发中",
      "用于运行任务的托管计算机。"
    ]
  ],
  availabilityNote: "使用终端不需要 Codewhale 账户，云端模型的使用费用由你的提供商收取。",
  accountLink: "创建账户",
  surfacesHeading: "使用 Codewhale 的方式",
  surfaces: [
    ["TUI", "交互式终端工作"],
    ["codewhale exec", "脚本与 CI"],
    ["本地 Web 客户端", "本机界面；托管浏览器工作台仍在开发中"],
    ["运行时 API + MCP", "本地集成"],
    ["fleet", "多个智能体协作一件事"],
  ],
  runtimeLink: "了解集成",
  installBandHeading: "在 macOS 或 Linux 上安装 Codewhale",
  copy: "复制",
  copied: "已复制 ✓",
  binaries: "预编译包",
  chinaMirrors: "中国镜像",
  installGuideLink: "阅读安装指南",
  communityHeading: "一起让 Codewhale 变得更好",
  communityBody: "无论你是发现了错误、有功能方面的想法，还是准备提交第一个 pull request，我们都希望听到你的意见，与你一起推进接下来的工作。",
  communityLinksAria: "社区链接",
  contribute: "提交 pull request",
};
