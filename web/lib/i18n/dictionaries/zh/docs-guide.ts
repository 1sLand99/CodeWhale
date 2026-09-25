import type { DocsGuideDict } from "../types";

/**
 * Simplified-Chinese dictionary for the docs "Getting started" page.
 * Copy moved verbatim from the former `isZh` branches in
 * `app/[locale]/docs/guide/page.tsx`.
 */
export const docsGuide: DocsGuideDict = {
  metaTitle: "新手指引 · Codewhale 文档",
  metaDescription:
    "安装 Codewhale、连接模型并开始第一项任务。需要模型与角色列表时，再配置 Fleet。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "新手指引",
  overviewLead:
    "安装 Codewhale，连接模型，然后交给它一项任务。Fleet 配置为可选步骤。",
  sessionTitle: "看一次真实会话",
  sessionLead:
    "查看一项任务从首次请求到完成的全过程。",
  nextTitle: "接下来",
  sourceNote:
    "更多细节见文档中的用户指南与快捷键说明。",
};
