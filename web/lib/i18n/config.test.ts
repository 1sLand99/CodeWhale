import { describe, expect, it } from "vitest";
import {
  ALL_LOCALES,
  defaultLocale,
  isPartialLocale,
  isTrackedLocale,
  isValidLocale,
  locales,
  partialLocales,
} from "./config";

describe("locale registry (single canonical taxonomy)", () => {
  it("routes exactly the shipped and partial locales", () => {
    const routed = ALL_LOCALES.filter(
      (l) => l.status === "shipped" || l.status === "partial",
    ).map((l) => l.code);
    expect([...locales]).toEqual(routed);
  });

  it("keeps planned and deferred locales out of route generation", () => {
    const notRouted = ALL_LOCALES.filter(
      (l) => l.status === "planned" || l.status === "deferred",
    ).map((l) => l.code);
    expect(notRouted.length).toBeGreaterThan(0);
    for (const code of notRouted) {
      expect(locales).not.toContain(code);
      expect(isValidLocale(code)).toBe(false);
      expect(isTrackedLocale(code)).toBe(true);
    }
  });

  it("ships the v0.9.2 website wave as partial with visible status", () => {
    for (const code of ["ja", "vi", "ko", "ru", "uk", "es", "pt-BR"]) {
      expect(isValidLocale(code)).toBe(true);
      expect(isPartialLocale(code)).toBe(true);
    }
    expect(partialLocales).not.toContain("en");
    expect(partialLocales).not.toContain("zh");
    expect(isPartialLocale("fr")).toBe(false);
    expect(isValidLocale("fr")).toBe(false);
  });

  it("has unique codes and native-script labels", () => {
    const codes = ALL_LOCALES.map((l) => l.code);
    expect(new Set(codes).size).toBe(codes.length);
    const labels = Object.fromEntries(ALL_LOCALES.map((l) => [l.code, l.label]));
    expect(labels["ru"]).toBe("Русский");
    expect(labels["uk"]).toBe("Українська");
    expect(labels["ja"]).toBe("日本語");
    expect(labels["ko"]).toBe("한국어");
    expect(labels["vi"]).toBe("Tiếng Việt");
    expect(labels["pt-BR"]).toBe("Português (BR)");
    expect(labels["hi"]).toBe("हिन्दी");
  });

  it("keeps the default locale routed", () => {
    expect(locales).toContain(defaultLocale);
    expect(defaultLocale).toBe("en");
  });
});
