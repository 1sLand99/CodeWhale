import type { DocsFleetDict } from "../types";

export const docsFleet: DocsFleetDict = {
  metaTitle: "Fleet 与 Workflow · Codewhale 文档",
  metaDescription: "持久多 worker 运行的本地控制平面，以及可选的 Workflow 编排层。",
  bodyClassName: "text-ink-soft leading-[1.9] tracking-wide",
  overviewTitle: "Fleet 与 Workflow",
  overviewLead:
    "Fleet 是面向持久多 worker 运行的控制平面。它不是独立的执行引擎：一个 Fleet worker 就是一次由 Fleet 启动并持久跟踪的 codewhale exec 无头运行。当工作需要重试、睡眠/重启后存活、远程执行、收据或可审计的台账时，使用 Fleet 而不是短寿命的 agent 扇出。",
  runTitle: "运行一次 Fleet",
  runLead:
    "Fleet 状态存放在工作区的 .codewhale/fleet.jsonl 台账中，worker 日志在 .codewhale/fleet/ 下。codewhale fleet resume <run-id> 是重启恢复命令：它重放台账、调和停止心跳的在途租约，且幂等——在管理进程退出、笔记本睡眠或运行时重启后都可以安全运行。",
  statusLead:
    "注意两个同名状态面：TUI 里的 {fleetStatusTui}（或 {subagents}）只显示当前交互会话的子 Agent；shell 里的 {fleetStatusShell} 才读取持久 Fleet 台账。",
  profilesTitle: "角色与 /fleet setup",
  profilesLead:
    "/fleet setup 打开一个渐进式向导，编写可复用的 agent 团队档案：一次只做一个选择——角色，然后是模型（可继承，或任何已配置提供商的具体模型），再是思考档位（inherit、off、low、medium、high、max 或 auto）——最后在审查页确认完整姿态（路由、思考、权限、工具、范围与审查策略）。档案可以写在项目级（.codewhale/agents/<role>.toml，随仓库走）或个人级（$CODEWHALE_HOME/agents/<role>.toml，本机所有仓库可用）；同名项目档案优先。档案的存储范围不会扩大运行操作的权限。",
  workflowTitle: "Workflow 编排",
  workflowLead:
    "普通多 Agent 工作不需要 Workflow：在 Operate 里直接发消息，需要并行、隔离或长时间工作时让 Codewhale 优先委派后台 worker 即可。只有当工作需要有序阶段、门禁、共享预算、回放或确定性汇总时才用 Workflow。Workflow 脚本是纯协调者：没有自己的文件系统和 shell，真正的工作由它启动的子 Agent 完成。脚本以编译专用的声明式 JS 子集编写，降低到类型化的 WorkflowSpec 后由 Rust 校验与执行；import、fetch、process、eval、async/await 等会产生副作用的写法会被编译器拒绝。",
  workflowLimits:
    "默认校验边界：每次 Workflow 运行最多 100 个 worker Agent、最多 5 层递归 Fleet 环、循环必须声明 max_iterations、动态 expand 节点必须声明 max_children 和模板。这些是数量上限而非并发要求——一个合法的 100 Agent Workflow 仍会按配置好的 Fleet worker 池排水执行。Workflow JS 沙箱内单 run 最多 16 个并发存活 Agent、整个 VM 生命周期最多 1,000 次启动。",
  sourceNote: "来源文档：docs/FLEET.md, docs/WORKFLOW_AUTHORING.md · 更新时请同步修改 docs-map.ts。",
};
