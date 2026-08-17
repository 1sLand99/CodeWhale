import Link from "next/link";
import { DocsSidebar } from "@/components/docs-sidebar";
import { Whale } from "@/components/whale";

/* ------------------------------------------------------------------ */
/*  Layout (Next.js App Router)                                        */
/* ------------------------------------------------------------------ */

export default async function DocsLayout({
  children,
  params,
}: {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;
  const isZh = locale === "zh";

  return (
    <div className="docs-theme docs-portal min-h-screen">
      <section className="hero">
        <div className="portal-current" aria-hidden="true" />
        <div className="portal-container docs-portal-hero-inner">
          <div className="portal-mark">
            <Whale size={28} />
            <span>{isZh ? "Codewhale 文档" : "Codewhale documentation"}</span>
          </div>
          <h1>{isZh ? "查找准确的使用说明。" : "Find the guidance you need."}</h1>
          <p>
            {isZh
              ? "从新手指引和安装开始，或直接查看名词、模式、权限、工具、提供商、Fleet、钩子、MCP 与运行时 API。每页都链接到仓库中的源文档。"
              : "Start with the guide and install pages, or go straight to vocabulary, modes, permissions, tools, providers, Fleet, hooks, MCP, and the Runtime API. Each page links to its source document in the repository."}
          </p>
          <div className="portal-actions">
            <Link href={`/${locale}/install`} className="portal-button portal-button-primary">
              {isZh ? "安装 Codewhale" : "Install Codewhale"}
            </Link>
            <Link
              href="https://github.com/Hmbown/CodeWhale/tree/main/docs"
              target="_blank"
              rel="noreferrer"
              className="portal-button portal-button-secondary"
            >
              {isZh ? "浏览源文档 ↗" : "Browse source docs ↗"}
            </Link>
          </div>
        </div>
      </section>

      <div className="portal-container docs-shell min-w-0">
        <article className="docs-content min-w-0">{children}</article>
        <DocsSidebar locale={locale} />
      </div>
    </div>
  );
}
