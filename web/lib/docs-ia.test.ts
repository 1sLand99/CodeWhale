/**
 * Information-architecture contracts: docs-map registration, sitemap and
 * hreflang preservation, navigation parity across breakpoints and locales,
 * and the accessibility hooks (skip link, labelled nav, aria-current).
 *
 * These are deterministic source/unit contracts in the same style as
 * public-copy.test.ts: they read the real sources and assert structure, so a
 * future IA change fails here first instead of drifting silently.
 */
import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { DOC_TOPICS, docTopicHref, getTopic } from "./docs-map";
import { docsTopicIsCurrent } from "./docs-navigation";
import { locales } from "./i18n/config";

const webRoot = new URL("../", import.meta.url);
const repoRoot = new URL("../../", import.meta.url);

function webText(path: string): string {
  return readFileSync(new URL(path, webRoot), "utf8");
}

const sitemap = webText("app/sitemap.ts");
const nav = webText("components/nav.tsx");
const navLinks = webText("components/nav-links.tsx");
const mobileMenu = webText("components/mobile-menu.tsx");
const footer = webText("components/footer.tsx");
const localeLayout = webText("app/[locale]/layout.tsx");
const css = webText("app/globals.css");

describe("docs-map registration", () => {
  it("registers the guide and vocabulary topics as first-party pages", () => {
    const guide = getTopic("guide");
    const vocabulary = getTopic("vocabulary");
    expect(guide?.hasPage).toBe(true);
    expect(vocabulary?.hasPage).toBe(true);
    expect(vocabulary?.category).toBe("core-concepts");
    expect(docTopicHref(guide!, "en")).toBe("/en/docs/guide");
    expect(docTopicHref(vocabulary!, "zh")).toBe("/zh/docs/vocabulary");
    expect(docsTopicIsCurrent(vocabulary!, "en", "/en/docs/vocabulary")).toBe(true);
  });

  it("keeps every docs topic repo source on disk", () => {
    for (const topic of DOC_TOPICS) {
      const sources = Array.isArray(topic.repoSource) ? topic.repoSource : [topic.repoSource];
      for (const source of sources) {
        expect(existsSync(new URL(source, repoRoot)), `${topic.id}: ${source}`).toBe(true);
      }
    }
  });

  it("keeps topic labels and descriptions bilingual", () => {
    for (const topic of DOC_TOPICS) {
      for (const pair of [topic.label, topic.description]) {
        expect(pair.en.trim().length, `${topic.id} en`).toBeGreaterThan(0);
        expect(pair.zh.trim().length, `${topic.id} zh`).toBeGreaterThan(0);
      }
    }
  });
});

describe("sitemap and hreflang preservation", () => {
  it("indexes every first-party docs page", () => {
    for (const topic of DOC_TOPICS) {
      if (!topic.hasPage) continue;
      const path = topic.sitePath ? `/${topic.sitePath}` : `/docs/${topic.slug}`;
      expect(sitemap, path).toContain(`"${path}"`);
    }
    expect(sitemap).toContain('"/docs/guide"');
    expect(sitemap).toContain('"/docs/vocabulary"');
  });

  it("keeps per-locale alternate pairs for every indexed route", () => {
    expect(sitemap).toContain("alternates");
    expect(sitemap).toContain("en: `${SITE_URL}/en${path}`");
    expect(sitemap).toContain("zh: `${SITE_URL}/zh${path}`");
    // Route generation is driven by the shipped locale set, never hardcoded.
    expect(sitemap).toContain("locales.map");
    expect(locales).toEqual(["en", "zh"]);
  });

  it("keeps the new docs pages on the shared metadata helper", () => {
    for (const route of ["guide", "vocabulary"]) {
      const page = webText(`app/[locale]/docs/${route}/page.tsx`);
      expect(page, route).toContain('import { buildPageMetadata } from "@/lib/page-meta"');
      expect(page, route).toContain(`path: "/docs/${route}"`);
    }
  });
});

describe("navigation parity and accessibility", () => {
  it("keeps desktop and mobile navigation on one shared link set", () => {
    // Both surfaces consume the same `links` prop from nav.tsx — assert the
    // wiring rather than duplicating the arrays.
    expect(nav).toContain("<NavLinks links={links} isZh={isZh} />");
    expect(nav).toContain("links={links}");
    expect(mobileMenu).toContain("links.map");
    expect(navLinks).toContain("links.map");
  });

  it("keeps en/zh nav link paths in exact locale-swap parity", () => {
    const enHrefs = [...nav.matchAll(/href: "\/en\/([^"]+)"/g)].map((m) => m[1]);
    const zhHrefs = [...nav.matchAll(/href: "\/zh\/([^"]+)"/g)].map((m) => m[1]);
    expect(enHrefs.length).toBeGreaterThanOrEqual(4);
    expect(enHrefs).toEqual(zhHrefs);
    expect(enHrefs).toContain("docs/guide");
    expect(enHrefs).toContain("faq");
  });

  it("labels the primary nav and marks the current page accessibly", () => {
    expect(navLinks).toContain('aria-label={isZh ? "主导航" : "Primary"}');
    expect(navLinks).toContain('aria-current={isActive ? "page" : undefined}');
    expect(mobileMenu).toContain('aria-current={isActive ? "page" : undefined}');
    expect(mobileMenu).toContain('aria-expanded={open}');
    expect(mobileMenu).toContain('aria-controls="mobile-menu"');
    expect(mobileMenu).toContain('role="dialog"');
  });

  it("ships a keyboard-reachable skip link to the main landmark", () => {
    expect(localeLayout).toContain('href="#main-content"');
    expect(localeLayout).toContain('className="skip-link"');
    expect(localeLayout).toContain('<main id="main-content">');
    expect(css).toContain(".skip-link:focus-visible");
  });

  it("keeps responsive breakpoints for the getting-started steps", () => {
    // 4-up grid by default, 2-up at the tablet breakpoint, 1-up on phones —
    // the same responsive ladder as the existing workflow steps.
    expect(css).toMatch(/\.gs-steps\s*\{[^}]*repeat\(4, minmax\(0, 1fr\)\)/);
    expect(css).toMatch(
      /@media \(max-width: 760px\)[\s\S]*?\.gs-steps\s*\{[^}]*repeat\(2, minmax\(0, 1fr\)\)/,
    );
    expect(css).toMatch(
      /@media \(max-width: 520px\)[\s\S]*?\.gs-steps\s*\{\s*grid-template-columns: 1fr/,
    );
  });

  it("keeps the footer discovery links alongside the pinned legal links", () => {
    expect(footer).toContain('href: "/en/docs/guide"');
    expect(footer).toContain('href: "/zh/docs/guide"');
    expect(footer).toContain('href: "/en/faq"');
    expect(footer).toContain(
      '{ label: "MIT license", href: "https://github.com/Hmbown/CodeWhale/blob/main/LICENSE" }',
    );
  });
});

describe("homepage integration", () => {
  const homepage = webText("app/[locale]/page.tsx");

  it("renders the shared getting-started path on the homepage", () => {
    expect(homepage).toContain('import { GettingStartedSteps } from "@/components/getting-started-steps"');
    expect(homepage).toContain("<GettingStartedSteps locale={locale} />");
    expect(homepage).toContain("product-start");
    expect(homepage).toContain("/docs/guide");
    expect(homepage).toContain("/docs/vocabulary");
  });

  it("keeps the previously pinned homepage facts intact", () => {
    // Guard against the new band accidentally displacing the public-copy
    // gate's required surface (the full contract lives in public-copy.test.ts).
    expect(homepage).toContain("facts.latestPublishedRelease");
    expect(homepage).toContain("Source candidate");
    expect(homepage).toContain('src="/codewhale-tui.png"');
    for (const label of ["Plan", "Act", "Operate", "Ask", "Auto-Review", "Full Access"]) {
      expect(homepage).toContain(label);
    }
  });
});
