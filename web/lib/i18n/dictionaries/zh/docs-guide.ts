import type { DocsGuideDict } from "../types";

/**
 * Simplified-Chinese dictionary for the docs "Getting started" page.
 * Copy moved verbatim from the former `isZh` branches in
 * `app/[locale]/docs/guide/page.tsx`.
 */
export const docsGuide: DocsGuideDict = {
  metaTitle: "新手指引 · Codewhale 文档",
  metaDescription:
    "从安装到配置理想 Fleet 的完整路径：安装、无需密钥的首次会话、连接提供商、设置 Fleet。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "新手指引",
  overviewLead:
    "从一条安装命令到配置好你的 Fleet，四步走完。每一步都只陈述当前版本真实的行为；未发布或待录制的内容会明确标注。",
  sessionTitle: "看一次真实会话",
  sessionLead:
    "下方是真实会话媒体位。它当前处于待录制状态——这是有意为之：在 v0.9.2 候选版 dogfood 录制完成前，本站不展示任何占位或摆拍影像。",
  nextTitle: "接下来",
  sourceNote:
    "来源文档：docs/GUIDE.md, docs/KEYBINDINGS.md · 步骤文案来自 web/lib/content/getting-started.ts；更新时请同步修改 docs-map.ts。",
};
