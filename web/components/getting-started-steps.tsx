/**
 * <GettingStartedSteps> — renders the shared new-user path from
 * web/lib/content/getting-started.ts: install → first offline session →
 * provider connection → first Fleet workflow.
 *
 * Used by the homepage band and the /docs/guide page so the path reads
 * identically in both places. Server component, SSG-safe.
 */

import Link from "next/link";
import { GETTING_STARTED_STEPS } from "@/lib/content/getting-started";

export function GettingStartedSteps({ locale = "en" }: { locale?: string }) {
  const isZh = locale === "zh";

  return (
    <ol className="gs-steps">
      {GETTING_STARTED_STEPS.map((step, index) => (
        <li key={step.id} data-step-id={step.id}>
          <span className="gs-step-index" aria-hidden="true">
            {String(index + 1).padStart(2, "0")}
          </span>
          <h3>{isZh ? step.title.zh : step.title.en}</h3>
          <p>{isZh ? step.body.zh : step.body.en}</p>
          {step.commands.length > 0 && (
            <pre className="code-block gs-step-commands"><code>{step.commands.join("\n")}</code></pre>
          )}
          <Link href={`/${locale}${step.link.href}`} className="gs-step-link">
            {isZh ? step.link.label.zh : step.link.label.en} →
          </Link>
        </li>
      ))}
    </ol>
  );
}
