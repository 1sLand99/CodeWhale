import type { HomeDict } from "../types";

/**
 * Simplified Chinese home dictionary — native copy for the Tidal Folio landing page,
 * in the current direction: your models, more capable together; agents
 * and control on your own machine; availability stated per surface as it
 * is today. Product vocabulary stays literal (Plan / Work / Operate, Ask /
 * Auto-Review / Full Access, Codewhale, TUI, codewhale exec, Fleet).
 */

export const home: HomeDict = {
  metaTitle: "Codewhale — 把想做的做出来。",
  metaDescription:
    "用 Codewhale 开发软件、处理文件、自动化任务。选择托管或本地模型，并随工作需要切换提供商。",
  kicker: "开源 AI 智能体",
  heroTitleA: "把想做的做出来。",
  heroTitleB: "选择你的模型。",
  heroIntro:
    "{brand} 提供能开发软件、处理文件、自动化任务的智能体。使用你选择的模型，并随工作需要切换提供商。",
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
  chapterTerminalTitle: "从你想做的东西开始。",
  gainHeading: "让想法付诸行动。",
  gainLede: "开发一个项目、研究一个问题，或自动化一项任务。从一个智能体开始，较大的工作可以交给多个智能体分担。",
  gain: [
    [
      "动手创造",
      "把想法变成可运行的软件。智能体可以编辑文件、运行命令，并检查结果。"
    ],
    [
      "自动化重复工作",
      "为重复任务创建脚本和工作流，再从终端运行。"
    ],
    [
      "选择你的模型",
      "连接托管或本地模型。将较大的工作拆分，交给使用不同模型、担任不同角色的智能体。"
    ]
  ],
  chapterModels: "你的模型",
  modelsHeading: "找到适合任务的模型。",
  modelsBody:
    "使用提供商的托管服务，通过网关连接，或在本地运行模型。为每个会话选择提供商和模型，并在工作过程中调整。",
  modelsFacts: [
    ["托管", "你自己的 API 密钥，用 codewhale auth set --provider <id> 保存"],
    ["网关", "一个端点接多个模型，提供商仍由你选"],
    ["本地", "localhost 上的 vLLM、SGLang、Ollama——通常无需密钥"],
  ],
  modelsLink: "了解模型与提供商",
  startHeading: "开始你的第一个任务。",
  startLede: "安装 Codewhale，连接一个模型，再告诉它你想做什么。需要多个智能体分担工作时，就添加一个 Fleet。",
  startGuideLink: "阅读新手指引",
  startVocabularyLink: "查名词",
  chapterAccount: "获取 Codewhale",
  availabilityHeading: "在哪里使用 Codewhale。",
  availabilityLede: "从终端开始。应用和云端计算机仍在开发中。",
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
  availabilityNote: "使用终端无需 Codewhale 账户。托管模型的使用费用由你的提供商收取。",
  accountLink: "创建账户",
  surfacesHeading: "活在哪里干，就在哪里用。",
  surfaces: [
    ["TUI", "交互式终端工作"],
    ["codewhale exec", "脚本与 CI"],
    ["本地 Web 客户端", "本机界面；托管浏览器工作台仍在开发中"],
    ["运行时 API + MCP", "本地集成"],
    ["fleet", "多个智能体协作一件事"],
  ],
  runtimeLink: "了解集成",
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
