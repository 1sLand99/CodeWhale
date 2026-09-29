import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { handleProductTelemetry } from "@/app/api/product-telemetry/route";

const APP_DIR = fileURLToPath(new URL("../app", import.meta.url));

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(path);
    return /\.(ts|tsx)$/.test(entry.name) ? [path] : [];
  });
}

describe("/api/product-telemetry", () => {
  // @opennextjs/cloudflare does not support the edge runtime; the deployed
  // telemetry route answered every request with a 500 while it declared it.
  it("no app route or page opts into the edge runtime", () => {
    const edge = sourceFiles(APP_DIR).filter((file) =>
      /export\s+const\s+runtime\s*=\s*["']edge["']/.test(readFileSync(file, "utf8")),
    );
    expect(edge).toEqual([]);
  });

  it("answers an empty POST without a server error", async () => {
    const post = () =>
      new Request("https://codewhale.net/api/product-telemetry", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: "{}",
      });

    const disabled = await handleProductTelemetry(post(), { ingestUrl: null });
    expect(disabled.status).toBe(200);
    expect(await disabled.json()).toEqual({ accepted: false, reason: "disabled" });

    const enabled = await handleProductTelemetry(post(), {
      ingestUrl: "https://telemetry.codewhale.net/v1/telemetry",
      forward: async () => {
        throw new Error("an invalid envelope must not be forwarded");
      },
    });
    expect(enabled.status).toBe(422);
    expect(await enabled.json()).toEqual({ accepted: false, reason: "schema" });
  });
});
