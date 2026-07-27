/**
 * Deterministic locale detection for the website middleware (#3091).
 *
 * Resolution order (first match wins, no ambient state):
 * 1. The NEXT_LOCALE cookie (a previous explicit choice).
 * 2. Accept-Language, in the header's preference order. Each tag matches:
 *    a. exact full tag against the routed set (pt-BR → pt-BR);
 *    b. its primary subtag against the routed set (ru-RU → ru, zh-Hant → zh);
 *    c. a declared base→variant mapping for bases we only serve as a
 *       regional variant (pt → pt-BR).
 * 3. The default locale (en).
 *
 * The mapping table is deliberately tiny and explicit — no guessing that
 * e.g. es-419 should route anywhere other than the shipped `es`.
 */
import { defaultLocale, locales } from "./config";

const ROUTED = locales as readonly string[];

/** Base subtags that route to a specific regional variant. */
const BASE_TO_VARIANT: Record<string, string> = {
  pt: "pt-BR",
};

/** Match one language tag (any case, optional region/script) to a routed locale. */
export function matchLocaleTag(tag: string): string | null {
  const t = tag.trim().toLowerCase();
  if (!t || t === "*") return null;

  // Exact full-tag match (case-insensitive; routed codes are lowercase).
  const exact = ROUTED.find((l) => l.toLowerCase() === t);
  if (exact) return exact;

  const base = t.split("-")[0];
  if (ROUTED.includes(base)) return base;

  const variant = BASE_TO_VARIANT[base];
  if (variant && ROUTED.includes(variant)) return variant;

  return null;
}

/** Resolve the locale for a request from its cookie and Accept-Language header. */
export function detectLocaleFromHeaders(
  cookie: string | undefined,
  acceptLanguage: string | null,
): string {
  if (cookie) {
    const match = matchLocaleTag(cookie);
    if (match) return match;
  }

  if (acceptLanguage) {
    const preferred = acceptLanguage.split(",").map((s) => s.split(";")[0]);
    for (const tag of preferred) {
      const match = matchLocaleTag(tag);
      if (match) return match;
    }
  }

  return defaultLocale;
}
