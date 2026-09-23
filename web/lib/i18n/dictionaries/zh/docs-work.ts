import type { DocsWorkDict } from "../types";

/**
 * Chinese dictionary for `app/[locale]/docs/work/page.tsx`.
 * Copy moved verbatim from the page's `isZh` ternaries.
 *
 * Spacing note: the spaces around `{todoWrite}`, `{checklistAlias}` and
 * `{todoAlias}` came from `{" "}` in the JSX, and the ones before `To-do`
 * and `回放` came from mid-sentence line breaks inside a single text node.
 * `{inProgress}`, `{completed}` and `{cancelled}` have no leading space
 * because JSX dropped the trailing whitespace of the text node that ended
 * in a newline. Do not "fix" either as a typo.
 */
export const docsWork: DocsWorkDict = {
  metaTitle: "工作面板 · Codewhale 文档",
  metaDescription: "唯一的 To-do 列表、模型如何看到它，以及同一份工作状态的延续路径。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "工作面板",
  overviewLead:
    "Codewhale 的 TUI 侧栏有一块 Work 区域，显示当前工作的实时状态。它不只是视觉上的待办清单：同一份工作状态同时由模型可见的工具、会话接力（relay）和子 Agent 交接共同维护。Codewhale 只有一个 Work 面板——带计数的 To-do 执行台账。update_plan 是对话式的推理笔记，不是第二个进度面板。",
  checklistTitle: "To-do：唯一的执行台账",
  checklistBody:
    "To-do 是具体工作的进度台账：一组带状态的条目（pending / in_progress / completed / cancelled），外加完成百分比和当前进行中的条目。模型通过 canonical 的 {todoWrite} 工具替换活动线程或持久任务的 To-do 投影——这是模型可见的进度表面。旧的 {checklistAlias} 和 {todoAlias} 名字仍是隐藏的兼容别名：它们对同一份 To-do 状态保持可派发，以便旧 transcript 回放，但不会出现在模型目录里。",
  strategyTitle: "策略是对话式推理：update_plan",
  strategyLead:
    "update_plan 承载的是可选的高层策略，不是第二份清单。它的字段面向阶段级理解：标题、目标、上下文摘要、说明、来源、关键文件、约束、推荐方案、验证计划、风险与未知、交接包，以及一组步骤。它帮助父会话或后续 worker 理解“为什么这么做”；具体执行进度始终属于 To-do 列表。侧栏有意不把策略状态渲染成第二条进度列表，各个 To-do 快照出口也不会包含它——只有 update_plan 而 To-do 为空时，不会产生任何 To-do 快照。",
  continuityTitle: "延续性：同一份状态流向各处",
  continuityLead:
    "模型通过自己的工具结果了解 To-do：todo_write 返回的结果就是普通的会话历史，因此无需在每一步重复注入清单。只有在有人明确要求的节点，才会用同一个渲染器展示一次当前 To-do：分叉（fork_context）的子 Agent 在其结构化状态块里收到该正文；/relay 把同样的正文写进交接指令。两处的 To-do 正文逐字节一致——子 Agent 与下一个线程因此从父级真实的进度位置继续，而不是从转述的摘要开始。侧栏的 To-do 区域则完整实时渲染同一份状态。",
  captureTitle: "终端实拍（文本复原）",
  captureLead:
    "下面的文本块按 crates/tui/src/tui/sidebar.rs 的渲染逻辑逐行复原侧栏 Work 区域：目标是带 ◆ 图标的 Goal 行、耗时、token 预算条；然后是完成度计数和带编号的状态条目。",
  captureLegend:
    "条目前缀对应四种状态：{pending} 待办、{inProgress} 进行中、{completed} 完成、{cancelled} 取消。空间不够时侧栏窗口化到进行中条目附近，并用 “+N more To-do items” 标注被省略的条目。",
  modelFacingTitle: "哪些是模型可见的，哪些只是界面",
  modelFacingLead:
    "已被实现和测试证实的模型可见路径有三条：todo_write 工具本身是模型目录里的活跃工具，它返回的工具结果就是模型看到清单的方式；分叉子 Agent 的结构化状态块（<codewhale:fork_state> 中的 To-do 小节，在真正 fork 的那一刻解析）；以及 /relay 输出。没有任何一步请求会重复注入 To-do——一条结构化测试直接断言真实出站请求体里不含该清单。侧栏渲染是视觉呈现——它给人看，不注入模型上下文。",
  modelFacingBoundaries:
    "边界值得说清楚：因为没有逐步注入，稳定的系统与工具前缀完全不受 To-do 变化影响，前缀缓存也不会因此失效。fork 那一次的快照读取的是权威状态（有 work graph 时读它暂存的投影，而不是尚未发布的旧视图），所以同一回合里较早的一次 todo_write 也会被带上。条目数与字符数都有硬上限，进行中的条目优先保留，被省略的部分带省略标记。To-do 为空时不输出任何内容。渲染器只保证包裹结构、控制字符与上限这三件事——它不会审查条目文本的含义，任意 To-do 内容不因此变成可信指令。",
  sourceNote: "来源文档：docs/TOOL_SURFACE.md, docs/TOOL_LIFECYCLE.md · 更新时请同步修改 docs-map.ts。",
};
