import type { MetadataRoute } from "next";
import { locales } from "@/lib/i18n/config";
import { SITE_URL } from "@/lib/page-meta";

// Public, indexable routes (locale-prefixed). /admin and /api are
// intentionally excluded; see app/robots.ts.
const PATHS = ["", "/install", "/constitution", "/models", "/runtime", "/docs", "/docs/configuration", "/docs/constitution", "/docs/fleet", "/docs/guide", "/docs/hooks", "/docs/mcp", "/docs/modes", "/docs/runtime-api", "/docs/sandbox", "/docs/subagents", "/docs/tools", "/docs/troubleshooting", "/docs/vocabulary", "/docs/web", "/docs/work", "/faq", "/roadmap", "/feed", "/digest", "/contribute", "/community"];

export default function sitemap(): MetadataRoute.Sitemap {
  const lastModified = new Date();
  // hreflang alternates derive from the canonical locale registry so new
  // routed locales propagate without a sitemap edit.
  return PATHS.flatMap((path) =>
    locales.map((locale) => ({
      url: `${SITE_URL}/${locale}${path}`,
      lastModified,
      alternates: {
        languages: Object.fromEntries(
          locales.map((l) => [l, `${SITE_URL}/${l}${path}`]),
        ),
      },
    })),
  );
}
