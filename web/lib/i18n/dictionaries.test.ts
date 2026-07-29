import { describe, expect, it } from "vitest";
import {
  DICTIONARY_LOCALES,
  EN_CHROME,
  EN_HOME,
  fill,
  getChrome,
  getHome,
} from "./dictionaries";
import type { ChromeDict, HomeDict } from "./dictionaries/types";

function templateTokens(value: string): string[] {
  return [...value.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();
}

function flattenStrings(dict: object): Record<string, string> {
  const out: Record<string, string> = {};
  for (const [key, value] of Object.entries(dict)) {
    if (typeof value === "string") {
      out[key] = value;
    } else if (Array.isArray(value)) {
      value.forEach((pair, i) => {
        out[`${key}[${i}][0]`] = pair[0];
        out[`${key}[${i}][1]`] = pair[1];
      });
    }
  }
  return out;
}

describe("website dictionaries", () => {
  it("cover exactly the v0.9.2 partial locales", () => {
    expect([...DICTIONARY_LOCALES].sort()).toEqual(
      ["es", "id", "ja", "ko", "pt-BR", "ru", "uk", "vi"].sort(),
    );
  });

  it("holds every dictionary to exact key parity with the English reference", () => {
    const enChromeKeys = Object.keys(EN_CHROME).sort();
    const enHomeKeys = Object.keys(EN_HOME).sort();
    for (const locale of DICTIONARY_LOCALES) {
      expect(Object.keys(getChrome(locale)).sort(), `${locale} chrome keys`).toEqual(
        enChromeKeys,
      );
      expect(Object.keys(getHome(locale)).sort(), `${locale} home keys`).toEqual(
        enHomeKeys,
      );
    }
  });

  it("preserves {token} template placeholders through translation", () => {
    const enChromeTokens = flattenStrings(EN_CHROME);
    const enHomeTokens = flattenStrings(EN_HOME);
    for (const locale of DICTIONARY_LOCALES) {
      const chrome = flattenStrings(getChrome(locale));
      const home = flattenStrings(getHome(locale));
      for (const key of Object.keys(enChromeTokens)) {
        expect(templateTokens(chrome[key]), `${locale} chrome ${key}`).toEqual(
          templateTokens(enChromeTokens[key]),
        );
      }
      for (const key of Object.keys(enHomeTokens)) {
        expect(templateTokens(home[key]), `${locale} home ${key}`).toEqual(
          templateTokens(enHomeTokens[key]),
        );
      }
    }
  });

  it("keeps workflow and surface lists structurally aligned", () => {
    for (const locale of DICTIONARY_LOCALES) {
      const home = getHome(locale);
      expect(home.workflow, `${locale} workflow`).toHaveLength(4);
      expect(home.surfaces, `${locale} surfaces`).toHaveLength(5);
      for (const pair of [...home.workflow, ...home.surfaces]) {
        expect(pair[0].length, `${locale} empty title`).toBeGreaterThan(0);
        expect(pair[1].length, `${locale} empty description`).toBeGreaterThan(0);
      }
    }
  });

  it("falls back to the English dictionary for unrouted locales — no missing markers", () => {
    for (const key of Object.keys(EN_CHROME) as (keyof ChromeDict)[]) {
      expect(getChrome("fr")[key]).toBe(EN_CHROME[key]);
      expect(getChrome("en")[key]).toBe(EN_CHROME[key]);
    }
    for (const key of Object.keys(EN_HOME) as (keyof HomeDict)[]) {
      expect(getHome("de")[key]).toEqual(EN_HOME[key]);
    }
  });

  it("has no empty strings anywhere", () => {
    for (const locale of ["en", ...DICTIONARY_LOCALES]) {
      for (const [key, value] of Object.entries(flattenStrings(getChrome(locale)))) {
        expect(value.trim().length, `${locale} chrome ${key}`).toBeGreaterThan(0);
      }
      for (const [key, value] of Object.entries(flattenStrings(getHome(locale)))) {
        expect(value.trim().length, `${locale} home ${key}`).toBeGreaterThan(0);
      }
    }
  });

  it("keeps the Cyrillic packs script-pure (no cross-leakage, no mixed copy)", () => {
    const cyrillic = /[Ѐ-ӿ]/;
    for (const [key, value] of Object.entries(flattenStrings(getChrome("uk")))) {
      expect(value, `uk chrome ${key}`).not.toMatch(/[ыэъЫЭЪ]/);
      void cyrillic;
    }
    for (const [key, value] of Object.entries(flattenStrings(getHome("uk")))) {
      expect(value, `uk home ${key}`).not.toMatch(/[ыэъЫЭЪ]/);
    }
    for (const [key, value] of Object.entries(flattenStrings(getChrome("ru")))) {
      expect(value, `ru chrome ${key}`).not.toMatch(/[іІїЇєЄґҐ]/);
    }
    for (const [key, value] of Object.entries(flattenStrings(getHome("ru")))) {
      expect(value, `ru home ${key}`).not.toMatch(/[іІїЇєЄґҐ]/);
    }
    // Prose values are actually translated, not English pass-through.
    expect(getHome("ru").heroIntro).toMatch(cyrillic);
    expect(getHome("uk").heroIntro).toMatch(cyrillic);
    expect(getChrome("ru").navDocs).not.toBe(EN_CHROME.navDocs);
    expect(getChrome("uk").navDocs).not.toBe(EN_CHROME.navDocs);
    expect(getChrome("ru").navDocs).not.toBe(getChrome("uk").navDocs);
  });

  it("interpolates templates with fill() and leaves unknown tokens visible", () => {
    expect(fill("Latest release {tag}", { tag: "v0.9.2" })).toBe("Latest release v0.9.2");
    expect(fill("{count} provider routes", { count: 30 })).toBe("30 provider routes");
    expect(fill("v{version} {state}", { version: "0.9.2" })).toBe("v0.9.2 {state}");
  });
});
