import type { DocsConstitutionDict } from "../types";

export const docsConstitution: DocsConstitutionDict = {
  metaTitle: "宪章与 /constitution · Codewhale 文档",
  metaDescription: "用户全局宪章、仓库本地法、项目说明和运行时边界。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "宪章与 /constitution",
  overviewCompanion: "Constitution",
  overviewLead:
    "Codewhale 先给 Agent 一个可追责的地址，再给上下文冲突一套法律。{constitutionCommand} 是管理个人常驻宪章的主入口：它把结构化的用户全局设置保存在 {globalPath}，再渲染成模型可读的 prose block。仓库仍可通过 {repoPath} 增加本地 law；runtime policy 独立负责模式、审批、沙箱、成本和工具边界。",
  scopes: [
    [
      "User-global",
      "用户全局",
      "用 /constitution 管理跨项目个人常驻法。它是结构化数据渲染成 prose，不是裸 prompt 编辑器。",
    ],
    [
      "Repo-local",
      "仓库本地",
      ".codewhale/constitution.json 是可选项目 law，用于不变量、分支规则、验证和升级条件。",
    ],
    [
      "Runtime",
      "运行时",
      "宪章文本可以表达偏好；审批、沙箱、Shell、网络、信任和 MCP 权限仍由运行时配置强制执行。",
    ],
  ],
  authorityNote:
    "普通项目说明仍放在 AGENTS.md；记忆和交接低于宪章与项目说明；完整 base prompt Markdown 覆盖只是专家逃生口，不是普通设置路径。详见 {configurationDocs}。",
  configurationLink: "配置文档",
  sourceNote: "来源文档：docs/ARCHITECTURE.md · 更新时请同步修改 docs-map.ts。",
};
