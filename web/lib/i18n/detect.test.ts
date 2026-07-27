import { describe, expect, it } from "vitest";
import { detectLocaleFromHeaders, matchLocaleTag } from "./detect";

describe("matchLocaleTag", () => {
  it("matches exact full tags case-insensitively", () => {
    expect(matchLocaleTag("pt-BR")).toBe("pt-BR");
    expect(matchLocaleTag("PT-br")).toBe("pt-BR");
    expect(matchLocaleTag("ru")).toBe("ru");
    expect(matchLocaleTag("uk")).toBe("uk");
  });

  it("maps regional variants to the routed base tag", () => {
    expect(matchLocaleTag("ru-RU")).toBe("ru");
    expect(matchLocaleTag("uk-UA")).toBe("uk");
    expect(matchLocaleTag("es-MX")).toBe("es");
    expect(matchLocaleTag("es-419")).toBe("es");
    expect(matchLocaleTag("zh-Hant")).toBe("zh");
    expect(matchLocaleTag("zh-TW")).toBe("zh");
    expect(matchLocaleTag("ja-JP")).toBe("ja");
    expect(matchLocaleTag("ko-KR")).toBe("ko");
    expect(matchLocaleTag("vi-VN")).toBe("vi");
  });

  it("routes pt to the only shipped Portuguese variant", () => {
    expect(matchLocaleTag("pt")).toBe("pt-BR");
    expect(matchLocaleTag("pt-PT")).toBe("pt-BR");
  });

  it("rejects unrouted and empty tags deterministically", () => {
    expect(matchLocaleTag("fr")).toBeNull();
    expect(matchLocaleTag("de-DE")).toBeNull();
    expect(matchLocaleTag("ar")).toBeNull();
    expect(matchLocaleTag("")).toBeNull();
    expect(matchLocaleTag("*")).toBeNull();
  });
});

describe("detectLocaleFromHeaders", () => {
  it("prefers an explicit cookie choice over Accept-Language", () => {
    expect(detectLocaleFromHeaders("ru", "ja,en;q=0.8")).toBe("ru");
  });

  it("ignores stale cookies for unrouted locales", () => {
    expect(detectLocaleFromHeaders("fr", "uk,en;q=0.8")).toBe("uk");
  });

  it("honors Accept-Language preference order", () => {
    expect(detectLocaleFromHeaders(undefined, "fr,vi;q=0.9,ru;q=0.8")).toBe("vi");
    expect(detectLocaleFromHeaders(undefined, "de,pt;q=0.7")).toBe("pt-BR");
  });

  it("falls back to the default locale with no signal", () => {
    expect(detectLocaleFromHeaders(undefined, null)).toBe("en");
    expect(detectLocaleFromHeaders(undefined, "")).toBe("en");
    expect(detectLocaleFromHeaders(undefined, "fr,de;q=0.8")).toBe("en");
  });
});
