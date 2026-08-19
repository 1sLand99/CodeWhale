import { Fragment } from "react";
import Link from "next/link";
import { getDocsConstitution, splitTokens } from "@/lib/i18n/dictionaries";
import { buildPageMetadata } from "@/lib/page-meta";

const CODE_SPANS: Record<string, string> = {
  constitutionCommand: "/constitution",
  globalPath: "$CODEWHALE_HOME/constitution.json",
  repoPath: ".codewhale/constitution.json",
};

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsConstitution(locale);
  return buildPageMetadata({
    path: "/docs/constitution",
    locale,
    title: t.metaTitle,
    description: t.metaDescription,
  });
}

export default async function ConstitutionPage({ params }: { params: Promise<{ locale: string }> }) {
  const { locale } = await params;
  const t = getDocsConstitution(locale);

  return (
    <section className="space-y-10">
      <section id="overview" className="scroll-mt-32">
        <h1 className="font-display text-3xl mb-1">
          {t.overviewTitle}{" "}
          <span className="font-cjk text-indigo text-2xl ml-2">{t.overviewCompanion}</span>
        </h1>
        <p className={`${t.bodyClassName} mt-3`}>
          {splitTokens(t.overviewLead).map((part, i) =>
            "token" in part ? (
              <code key={`${i}-${part.token}`} className="inline">
                {CODE_SPANS[part.token] ?? `{${part.token}}`}
              </code>
            ) : (
              <Fragment key={`${i}-text`}>{part.text}</Fragment>
            ),
          )}
        </p>
        <div className="hairline-t hairline-b mt-6 grid md:grid-cols-3 col-rule">
          {t.scopes.map(([name, companion, detail]) => (
            <div key={name} className="p-5">
              <div className="font-display text-lg text-indigo mb-1">
                {name} <span className="font-cjk text-sm ml-1.5">{companion}</span>
              </div>
              <p className={`${t.bodyClassName} text-sm`}>{detail}</p>
            </div>
          ))}
        </div>
        <p className={`${t.bodyClassName} mt-4 text-sm`}>
          {splitTokens(t.authorityNote).map((part, i) =>
            "token" in part && part.token === "configurationDocs" ? (
              <Link
                key={`${i}-${part.token}`}
                href="https://github.com/Hmbown/CodeWhale/blob/main/docs/CONFIGURATION.md#constitution-project-instructions-and-repo-authority"
                className="body-link"
              >
                {t.configurationLink}
              </Link>
            ) : "token" in part ? (
              <Fragment key={`${i}-${part.token}`}>{`{${part.token}}`}</Fragment>
            ) : (
              <Fragment key={`${i}-text`}>{part.text}</Fragment>
            ),
          )}
        </p>
      </section>
      <section id="source" className="hairline-t pt-8">
        <p className="text-sm text-ink-mute">{t.sourceNote}</p>
      </section>
    </section>
  );
}
