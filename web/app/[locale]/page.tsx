import Image from "next/image";
import Link from "next/link";
import { GettingStartedSteps } from "@/components/getting-started-steps";
import { InstallCodeBlock } from "@/components/install-code-block";
import { Whale } from "@/components/whale";
import { getFacts } from "@/lib/facts";
import { fill, getHome } from "@/lib/i18n/dictionaries";

const REPO = "https://github.com/Hmbown/CodeWhale";
const CAPTURED_SESSION_VERSION = "0.9.1";

// Revalidate against source-proven runtime facts without giving up static edge
// caching. `getFacts()` rejects legacy or older KV snapshots.
export const revalidate = 300;

const WORKFLOW = [
  {
    en: ["Inspect", "Read the repository, its instructions, and the task."],
    zh: ["检查", "读取仓库、项目说明与任务。"],
  },
  {
    en: ["Act", "Edit files through explicit approval boundaries."],
    zh: ["执行", "在明确的审批边界内修改文件。"],
  },
  {
    en: ["Verify", "Run checks and inspect the result."],
    zh: ["验证", "运行检查并核对结果。"],
  },
  {
    en: ["Report", "Leave a concise, durable receipt."],
    zh: ["报告", "留下简洁、可追溯的工作收据。"],
  },
] as const;

const SURFACES = [
  {
    en: ["TUI", "Interactive terminal work"],
    zh: ["TUI", "交互式终端工作"],
  },
  {
    en: ["codewhale exec", "Scripts and CI"],
    zh: ["codewhale exec", "脚本与 CI"],
  },
  {
    en: ["Web client", "Loopback-only browser client"],
    zh: ["Web 客户端", "仅限本机回环的浏览器客户端"],
  },
  {
    en: ["Runtime API + MCP", "Local integrations"],
    zh: ["运行时 API + MCP", "本地集成"],
  },
  {
    en: ["Fleet", "Durable multi-agent work"],
    zh: ["Fleet", "持久化多智能体工作"],
  },
] as const;

export default async function HomePage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const isZh = locale === "zh";
  // en/zh copy stays inline (copy-contract tests read it from this file);
  // every other routed locale renders from the dictionary layer with the
  // English dictionary as the build-time-guaranteed fallback.
  const foreign = !isZh && locale !== "en";
  const d = getHome(locale);
  const facts = await getFacts();
  const sourceVersion = facts.version ?? "unknown";
  const publishedRelease = facts.latestPublishedRelease;
  const sourceIsPublished = publishedRelease?.version === sourceVersion;
  const providerCount = facts.providers.length;

  return (
    <div className="product-home">
      <section className="product-hero">
        <div className="product-container product-hero-grid">
          <div className="product-hero-copy">
            <div className="product-hero-brandline">
              <Whale size={34} />
              <span>Codewhale</span>
              <em>{facts.license ?? "MIT"}</em>
            </div>
            <p className="product-kicker">
              {isZh ? "数据与代码如海" : foreign ? d.kicker : "An ocean of data and code"}
            </p>
            <h1>
              {isZh ? (
                <>
                  潜入深海，
                  <br />
                  <span>你不必亲自下潜。</span>
                </>
              ) : foreign ? (
                <>
                  {d.heroTitleA}
                  <br />
                  <span>{d.heroTitleB}</span>
                </>
              ) : (
                <>
                  Dive into the deep
                  <br />
                  <span>so you don&apos;t have to.</span>
                </>
              )}
            </h1>
            <p>
              {isZh
                ? "Codewhale 把大模型的杠杆交给普通人：在你的终端里读取仓库、修改文件、运行检查、留下收据。不必已经是程序员，也能把东西做出来——运行在你自己的机器上。"
                : foreign
                  ? d.heroIntro
                  : "Codewhale gives ordinary people the leverage of LLMs to build things. In your terminal it reads the repo, edits files, runs checks, and leaves a receipt — without assuming you already speak code. It runs on your machine."}
            </p>
            <div className="product-actions">
              <Link href={`/${locale}/install`} className="product-button product-button-primary">
                {isZh ? "安装" : foreign ? d.install : "Install"}
              </Link>
              <Link href={`/${locale}/docs`} className="product-button">
                {isZh ? "文档" : foreign ? d.docs : "Docs"}
              </Link>
              <a href={REPO} className="product-button">
                GitHub
              </a>
            </div>
            <div className="product-install">
              <InstallCodeBlock
                cmd="npm install -g codewhale"
                copyLabel={isZh ? "复制" : foreign ? d.copy : "Copy"}
                copiedLabel={isZh ? "已复制 ✓" : foreign ? d.copied : "Copied ✓"}
              />
            </div>
            <p
              className="product-facts"
              data-source-state={sourceIsPublished ? "published release" : "source candidate"}
              data-source-state-label={sourceIsPublished ? d.publishedRelease : d.figcaptionSourceCandidate}
            >
              {publishedRelease
                ? isZh
                  ? `最新发布 ${publishedRelease.tag}`
                  : foreign
                    ? fill(d.latestRelease, { tag: publishedRelease.tag })
                    : `Latest release ${publishedRelease.tag}`
                : isZh
                  ? "发布状态暂不可用"
                  : foreign
                    ? d.releaseUnavailable
                    : "Release status unavailable"}{" "}
              <span>·</span>{" "}
              {isZh
                ? `${sourceIsPublished ? "当前源码" : "源码候选版"} v${sourceVersion}：`
                : foreign
                  ? `${sourceIsPublished ? d.currentSource : d.sourceCandidate} v${sourceVersion}: `
                  : `${sourceIsPublished ? "Current source" : "Source candidate"} v${sourceVersion}: `}
              {isZh ? (
                `${providerCount} 个提供商路由`
              ) : foreign ? (
                fill(d.providerRoutes, { count: providerCount })
              ) : (
                `${providerCount} provider routes`
              )}{" "}
              <span>·</span> {facts.license ?? "MIT"}
            </p>
          </div>

          <figure className="product-shot">
            <div className="product-shot-toolbar">
              <span>
                <Whale size={20} />
                Codewhale TUI
              </span>
              <span>v{CAPTURED_SESSION_VERSION}</span>
            </div>
            <Image
              src="/codewhale-tui.png"
              alt={
                isZh
                  ? `Codewhale v${CAPTURED_SESSION_VERSION} 的全新终端会话，使用本地 Ollama 路由且没有空的 Work 栏`
                  : foreign
                    ? fill(d.screenshotAlt, { version: CAPTURED_SESSION_VERSION })
                    : `Fresh Codewhale v${CAPTURED_SESSION_VERSION} terminal session using a local Ollama route, with no empty Work bar`
              }
              width={1280}
              height={720}
              sizes="(max-width: 900px) calc(100vw - 2rem), 58vw"
              priority
            />
            <figcaption>
              {isZh
                ? `v${CAPTURED_SESSION_VERSION} 已发布版 · 本地 Ollama 路由 · Plan / Act / Operate`
                : foreign
                  ? fill(d.figcaption, {
                      version: CAPTURED_SESSION_VERSION,
                      state: d.publishedRelease,
                    })
                  : `v${CAPTURED_SESSION_VERSION} published release · local Ollama route · Plan / Act / Operate`}
            </figcaption>
          </figure>
        </div>
      </section>

      <section className="product-proof">
        <div className="product-container product-proof-grid">
          <h2>
            {isZh ? (
              <>终端原生的水下壳。模型与提供商中立。本地优先。</>
            ) : foreign ? (
              <>{d.proofHeading}</>
            ) : (
              <>An underwater terminal shell. Model-neutral. Local-first.</>
            )}
          </h2>
          <p>
            {isZh
              ? "连接你已有的托管、网关或本地模型。Codewhale 在你的机器上运行；模型是可选择的组件，不是产品本身。Plan / Act / Operate 与明确的审批边界，让深潜也保持可控。"
              : foreign
                ? d.proofBody
                : "Bring the hosted, gateway, or local model you already use. Codewhale runs on your machine and treats the model as a selectable component—not the product. Plan / Act / Operate and explicit permission postures keep the deep dive under your control."}
          </p>
        </div>
      </section>

      <section className="product-workflow">
        <div className="product-container">
          <h2>
            {isZh ? "从任务到经过验证的改动。" : foreign ? d.workflowHeading : "From task to verified change."}
          </h2>
          <ol className="product-workflow-steps">
            {WORKFLOW.map((step, index) => {
              const [title, description] = isZh
                ? step.zh
                : foreign
                  ? d.workflow[index]
                  : step.en;
              return (
                <li key={title}>
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <h3>{title}</h3>
                  <p>{description}</p>
                </li>
              );
            })}
          </ol>
          <div className="product-receipt" aria-label={isZh ? "工作流程示例" : foreign ? d.receiptAria : "Example work receipt"}>
            <span>$ codewhale exec &quot;fix the failing test&quot;</span>
            <span>inspect&nbsp;&nbsp; repository and instructions</span>
            <span>act&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp; edit through the selected permission posture</span>
            <span>verify&nbsp;&nbsp;&nbsp; cargo test --locked</span>
            <strong>report&nbsp;&nbsp;&nbsp; checks passed · receipt saved</strong>
          </div>
        </div>
      </section>

      <section className="product-start">
        <div className="product-container">
          <h2>
            {isZh ? "第一次使用？四步走完。" : "New to Codewhale? Four steps end to end."}
          </h2>
          <p className="product-start-lede">
            {isZh
              ? "安装 → 无需密钥的首次会话 → 连接提供商 → 第一个 Fleet Workflow。名词含义见产品名词页。"
              : "Install → a first keyless session → provider connection → a first Fleet workflow. The nouns are defined on the vocabulary page."}
          </p>
          <GettingStartedSteps locale={locale} />
          <div className="product-start-links">
            <Link href={`/${locale}/docs/guide`}>
              {isZh ? "阅读新手指引 →" : "Read the getting-started guide →"}
            </Link>
            <Link href={`/${locale}/docs/vocabulary`}>
              {isZh ? "查看产品名词 →" : "See the product vocabulary →"}
            </Link>
          </div>
        </div>
      </section>

      <section className="product-boundaries">
        <div className="product-container product-boundaries-grid">
          <div>
            <h2>
              {isZh ? (
                <>
                  你的模型。
                  <br />
                  <span>你的边界。</span>
                </>
              ) : foreign ? (
                <>
                  {d.boundariesHeadingA}
                  <br />
                  <span>{d.boundariesHeadingB}</span>
                </>
              ) : (
                <>
                  Your model.
                  <br />
                  <span>Your boundaries.</span>
                </>
              )}
            </h2>
            <p>
              {isZh
                ? "显式选择模型、工作模式与权限姿态。Codewhale 不会把未知成本显示成零，也不会把预览功能说成已发布产品。"
                : foreign
                  ? d.boundariesBody
                  : "Choose the model, working mode, and permission posture explicitly. Unknown cost stays unknown, and preview surfaces stay labeled as such."}
            </p>
          </div>
          <dl className="product-boundary-list">
            <div>
              <dt>
                {isZh
                  ? `${providerCount} 个提供商路由`
                  : foreign
                    ? fill(d.providerRoutes, { count: providerCount })
                    : `${providerCount} provider routes`}
              </dt>
              <dd>{isZh ? "托管、网关与本地模型" : foreign ? d.hostedGatewayLocal : "Hosted, gateway, and local models"}</dd>
            </div>
            <div>
              <dt>Plan · Act · Operate</dt>
              <dd>{isZh ? "从只读规划到自主执行" : foreign ? d.planActOperateDesc : "Read-only planning through autonomous operation"}</dd>
            </div>
            <div>
              <dt>Ask · Auto-Review · Full Access</dt>
              <dd>{isZh ? "为任务选择权限姿态" : foreign ? d.askAutoReviewDesc : "Choose the permission posture for the work"}</dd>
            </div>
            <div>
              <dt>TUI · exec · web · API</dt>
              <dd>{isZh ? "交互式与无头运行时界面" : foreign ? d.tuiExecWebDesc : "Interactive and headless runtime surfaces"}</dd>
            </div>
          </dl>
        </div>
      </section>

      <section className="product-surfaces">
        <div className="product-container">
          <h2>
            {isZh ? "在工作发生的地方使用运行时。" : foreign ? d.surfacesHeading : "Use the runtime where the work happens."}
          </h2>
          <div className="product-surface-list">
            {SURFACES.map((surface, index) => {
              const [name, description] = isZh
                ? surface.zh
                : foreign
                  ? d.surfaces[index]
                  : surface.en;
              return (
                <div key={name}>
                  <strong>{name}</strong>
                  <span>{description}</span>
                </div>
              );
            })}
          </div>
          <Link href={`/${locale}/runtime`}>
            {isZh ? "查看运行时界面与稳定性说明 →" : foreign ? d.runtimeLink : "See runtime surfaces and stability notes →"}
          </Link>
        </div>
      </section>

      <section className="product-install-band">
        <div className="product-container product-install-grid">
          <h2>{isZh ? "从一条命令开始。" : foreign ? d.installBandHeading : "Start with one command."}</h2>
          <div>
            <InstallCodeBlock
              cmd="npm install -g codewhale"
              copyLabel={isZh ? "复制" : foreign ? d.copy : "Copy"}
              copiedLabel={isZh ? "已复制 ✓" : foreign ? d.copied : "Copied ✓"}
            />
            <p>
              Cargo · {isZh ? "预编译包" : foreign ? d.binaries : "Binaries"} · Docker · Nix · Windows · Android / Termux ·{" "}
              {isZh ? "中国镜像" : foreign ? d.chinaMirrors : "China mirrors"}
            </p>
            <Link href={`/${locale}/install`}>
              {isZh ? "阅读安装指南 →" : foreign ? d.installGuideLink : "Read the install guide →"}
            </Link>
          </div>
        </div>
      </section>

      <section className="product-community">
        <div className="product-container product-community-grid">
          <div className="product-community-illustration" aria-hidden="true">
            <Whale size={180} />
          </div>
          <div>
            <h2>{isZh ? "公开构建" : foreign ? d.communityHeading : "Built in public"}</h2>
            <p>
              {isZh
                ? "Codewhale 采用 MIT 许可证，由来自不同时区、语言和技术背景的贡献者共同塑造。"
                : foreign
                  ? d.communityBody
                  : "MIT-licensed and shaped by contributors across runtimes, providers, platforms, documentation, and tests."}
            </p>
          </div>
          <nav aria-label={isZh ? "社区链接" : foreign ? d.communityLinksAria : "Community links"}>
            <a href={REPO}>GitHub</a>
            <a href={`${REPO}/issues`}>Issues</a>
            <Link href={`/${locale}/contribute`}>{isZh ? "参与贡献" : foreign ? d.contribute : "Contribute"}</Link>
            {publishedRelease ? (
              <a href={publishedRelease.url}>{publishedRelease.tag}</a>
            ) : (
              <a href={`${REPO}/releases`}>Releases</a>
            )}
          </nav>
        </div>
      </section>
    </div>
  );
}
