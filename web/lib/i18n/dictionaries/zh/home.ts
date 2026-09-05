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
    "{brand} 把你已经在用的模型带进同一个终端，让它们像一支船员一样协作——读代码、改文件、跑检查——而每个模型能做什么，由你来定。开源，跑在你自己的机器上。",
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
  gainLede: "一个会话可以同时容纳多个模型，各司其职，在同一个仓库里按同一套规则工作。",
  gain: [
    ["你的模型", "托管密钥、网关，或者完全不需要密钥的本地运行时。给每个角色固定一个模型，提供商始终是你选的那个——模型名不会替你切换。"],
    ["能干的智能体", "Plan、Work、Operate 三种模式；一支 fleet 为同一件事分工；文件、shell、网页和 MCP 工具；可保存、恢复、回滚的会话。"],
    ["本机掌控", "Ask、Auto-Review 或 Full Access——它在问你之前能做多少，由你设定。本地运行，系统允许时启用沙箱，审计日志你随时能读。"],
  ],
  chapterModels: "你的模型",
  modelsHeading: "带上你已有的，不改你未选的。",
  modelsBody:
    "Codewhale 内置 {count} 个提供商，一视同仁。保存一次密钥，指定一个模型，路由就保持你设定的样子。vLLM、SGLang 或 Ollama 上的本地模型不需要密钥。",
  modelsFacts: [
    ["托管", "你自己的 API 密钥，用 codewhale auth set 保存"],
    ["网关", "一个端点接多个模型，提供商仍由你选"],
    ["本地", "localhost 上的 vLLM、SGLang、Ollama——通常无需密钥"],
  ],
  modelsLink: "查看所有提供商",
  startHeading: "四步开始第一个会话。",
  startLede: "安装，无需密钥打开会话，接入提供商；一个模型不够时，再配置 fleet。",
  startGuideLink: "阅读新手指引",
  startVocabularyLink: "查名词",
  chapterAccount: "现在能在哪里运行",
  availabilityHeading: "已可用、开发中、暂不可用——如实说明。",
  availabilityLede: "终端是已发布的产品。其余的按实际状态列出。",
  availability: [
    ["终端", "已发布", "npm、Cargo 与 Linux、macOS、Windows 的预编译二进制。Android 上的 Termux 为预览。"],
    ["网页应用", "登录与远程控制可用", "登录或创建账户，然后在正在运行的本地会话里输入 /rc，即可在浏览器中继续这同一个会话。浏览器工作台的其余部分仍是开发预览。"],
    ["桌面端", "开发版本", "macOS、Linux、Windows 有 alpha 构建。桌面应用尚未正式发布。"],
    ["云端计算机", "暂不可用", "在托管计算机上运行工作仍在开发中。等它真正可用时，本页会如实说明。"],
  ],
  availabilityNote: "终端不需要账户。账户本身从不等于付费方案，本站也无法向你收费。",
  accountLink: "创建账户",
  surfacesHeading: "活在哪里干，就在哪里用。",
  surfaces: [
    ["TUI", "交互式终端工作"],
    ["codewhale exec", "脚本与 CI"],
    ["本地 Web 客户端", "本机界面；托管远程控制使用网页应用"],
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
