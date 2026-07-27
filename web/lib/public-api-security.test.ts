import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

function routeSource(path: string): string {
  return readFileSync(new URL(`../app/api/${path}/route.ts`, import.meta.url), "utf8");
}

function librarySource(path: string): string {
  return readFileSync(new URL(path, import.meta.url), "utf8");
}

describe("public API security contracts", () => {
  it("keeps the unauthenticated feed cached and detached from the server token", () => {
    const source = routeSource("github/feed");
    expect(source).toContain('export const dynamic = "force-static"');
    expect(source).toContain("fetchFeed(undefined, 50)");
    expect(source).not.toContain("GITHUB_TOKEN");
    expect(source).not.toContain('dynamic = "force-dynamic"');
  });

  it("validates the draft namespace before admin discard can reach KV", () => {
    const source = routeSource("admin/post");
    expect(source).toContain("parseDraftKey(draftKey)");
    expect(source.indexOf("parseDraftKey(draftKey)")).toBeLessThan(
      source.indexOf("getDraft(env.CURATED_KV, draftKey)"),
    );
    expect(source.indexOf("getDraft(env.CURATED_KV, draftKey)")).toBeLessThan(
      source.indexOf("deleteDraft(env.CURATED_KV, draftKey)"),
    );
  });

  it("bounds the public login body before comparing the maintainer token", () => {
    const source = routeSource("admin/login");
    expect(source).toContain("readBoundedUrlEncodedForm(req, MAX_LOGIN_BODY_BYTES)");
    expect(source).not.toContain("req.formData()");
    expect(source.indexOf("readBoundedUrlEncodedForm(req, MAX_LOGIN_BODY_BYTES)")).toBeLessThan(
      source.indexOf("safeEqual(submitted, env.MAINTAINER_TOKEN)"),
    );
  });

  it("keeps paid review callers on the canonical persisted draft namespaces", () => {
    const source = librarySource("./community-agent-tasks.ts");
    expect(source).toContain('hasFreshDraft(env.CURATED_KV, "triage"');
    expect(source).toContain('hasFreshDraft(env.CURATED_KV, "pr-review"');
    expect(source).not.toContain('hasFreshDraft(env.CURATED_KV, "issue"');
    expect(source).not.toContain('hasFreshDraft(env.CURATED_KV, "pr"');
  });
});
