import { DocsSearch } from "@/components/docs-search";
import { getDocsShell } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsShell(locale);
  return buildPageMetadata({
    path: "/docs",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function DocsHubPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  return <DocsSearch locale={locale} />;
}
