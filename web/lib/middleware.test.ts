import { describe, expect, it } from "vitest";
import { NextRequest } from "next/server";
import { middleware } from "../middleware";

/**
 * `www.codewhale.net` and `codewhale.net` are both bound to this worker as
 * custom domains, so without a canonical-host redirect the whole site is
 * reachable — and indexable — twice.
 */
function request(url: string, host: string, headers: Record<string, string> = {}) {
  return new NextRequest(new URL(url), {
    headers: new Headers({ host, ...headers }),
  });
}

describe("canonical host", () => {
  it("301s www to the apex, preserving path and query", () => {
    const res = middleware(
      request("https://www.codewhale.net/en/docs/hooks?x=1", "www.codewhale.net"),
    );
    expect(res.status).toBe(301);
    expect(res.headers.get("location")).toBe("https://codewhale.net/en/docs/hooks?x=1");
  });

  it("redirects assets and API routes too, so a moved document stops pulling from www", () => {
    for (const path of ["/_next/static/chunk.js", "/api/curated", "/opengraph-image"]) {
      const res = middleware(request(`https://www.codewhale.net${path}`, "www.codewhale.net"));
      expect(res.status, path).toBe(301);
      expect(res.headers.get("location"), path).toBe(`https://codewhale.net${path}`);
    }
  });

  it("leaves the apex host alone", () => {
    const res = middleware(request("https://codewhale.net/en", "codewhale.net"));
    expect(res.status).not.toBe(301);
  });

  it("leaves localhost and preview hosts alone", () => {
    for (const host of ["localhost:3000", "codewhale-web.pages.dev"]) {
      const res = middleware(request("https://example.test/en", host));
      expect(res.status, host).not.toBe(301);
    }
  });

  it("still applies security headers to the redirect", () => {
    const res = middleware(request("https://www.codewhale.net/en", "www.codewhale.net"));
    expect(res.headers.get("X-Content-Type-Options")).toBe("nosniff");
    expect(res.headers.get("Strict-Transport-Security")).toContain("max-age=");
  });
});

describe("dotted well-known paths", () => {
  it("does not locale-prefix /llms.txt", () => {
    const res = middleware(request("https://codewhale.net/llms.txt", "codewhale.net"));
    expect(res.status).not.toBe(307);
    expect(res.status).not.toBe(308);
    expect(res.headers.get("location")).toBeNull();
  });
});
