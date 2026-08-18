import Link from "next/link";
import { DocsSidebar } from "@/components/docs-sidebar";
import { Whale } from "@/components/whale";
import { getDocsShell } from "@/lib/i18n/dictionaries";

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
  const t = getDocsShell(locale);

  return (
    <div className="docs-theme docs-portal min-h-screen">
      <section className="hero">
        <div className="portal-current" aria-hidden="true" />
        <div className="portal-container docs-portal-hero-inner">
          <div className="portal-mark">
            <Whale size={28} />
            <span>{t.portalMark}</span>
          </div>
          <h1>{t.heroTitle}</h1>
          <p>{t.heroLead}</p>
          <div className="portal-actions">
            <Link href={`/${locale}/install`} className="portal-button portal-button-primary">
              {t.installCta}
            </Link>
            <Link
              href="https://github.com/Hmbown/CodeWhale/tree/main/docs"
              target="_blank"
              rel="noreferrer"
              className="portal-button portal-button-secondary"
            >
              {t.sourceDocsCta}
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
