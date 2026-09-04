import { existsSync, readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { handleProductTelemetry, ingestUrl, CANONICAL_INGEST_URL } from "../../app/api/product-telemetry/route";
import {
  CONSENT_STORAGE_KEY,
  CONSENT_VERSION,
  COUNTERS_STORAGE_KEY,
  INSTALL_ID_ROTATION_MS,
  INSTALL_STORAGE_KEY,
  buildEnvelope,
  createUsageRecorder,
  emptyCounters,
  readConsent,
  resolveInstallId,
  validateEnvelope,
  type StorageLike,
} from "./product-usage";

/** The backend's golden browser fixture, when the ingest checkout carries it. */
const GOLDEN_PATH = new URL("../../../telemetry-ingest/test/golden/browser-v2.json", import.meta.url);

const UUID = "82b77c4f-4cce-4c74-8e17-8a38ba0581ee";
const NOW = Date.parse("2026-09-04T20:00:00Z");

function memoryStorage(seed: Record<string, string> = {}): StorageLike & { data: Map<string, string> } {
  const data = new Map(Object.entries(seed));
  return {
    data,
    getItem: (key) => data.get(key) ?? null,
    setItem: (key, value) => void data.set(key, value),
    removeItem: (key) => void data.delete(key),
  };
}

function recorderWith(storage: StorageLike, sent: string[] = [], now = () => NOW) {
  const timers: (() => void)[] = [];
  const recorder = createUsageRecorder({
    surface: "website",
    appVersion: "0.9.12",
    endpoint: "/api/product-telemetry",
    storage,
    now,
    randomUuid: () => UUID,
    send: async (_endpoint, body) => {
      sent.push(body);
      return true;
    },
    setTimer: (callback) => {
      timers.push(callback);
      return timers.length;
    },
    clearTimer: (handle) => {
      timers[(handle as number) - 1] = () => {};
    },
  });
  return { recorder, timers, sent };
}

describe("product usage envelope", () => {
  it("matches the closed browser contract and the backend's golden fixture", () => {
    const counters = emptyCounters();
    counters.page_view = 1;
    const envelope = buildEnvelope({ counters, installId: UUID, appVersion: "0.9.12", surface: "website", now: NOW });
    expect(validateEnvelope(envelope)).toEqual({ ok: true, envelope });
    expect(envelope.sent_at).toBe("2026-09-04T20:00:00Z");
    if (existsSync(GOLDEN_PATH)) {
      const golden = JSON.parse(readFileSync(GOLDEN_PATH, "utf8"));
      expect(envelope).toEqual(golden);
      expect(validateEnvelope(golden).ok).toBe(true);
    }
  });

  it("rejects anything outside the closed field set", () => {
    const base = buildEnvelope({ counters: emptyCounters(), installId: UUID, appVersion: "0.9.12", surface: "website", now: NOW });
    expect(validateEnvelope({ ...base, referrer: "x" }).ok).toBe(false);
    expect(validateEnvelope({ ...base, git_sha: "abc" }).ok).toBe(false);
    expect(validateEnvelope({ ...base, surface: "tui" }).ok).toBe(false);
    expect(validateEnvelope({ ...base, consent_version: 3 }).ok).toBe(false);
    expect(validateEnvelope({ ...base, install_id: "not-a-uuid" }).ok).toBe(false);
    expect(validateEnvelope({ ...base, events: [] }).ok).toBe(false);
    const extraCounter = structuredClone(base) as unknown as { events: [{ counters: Record<string, number> }] };
    extraCounter.events[0].counters.url = 1;
    expect(validateEnvelope(extraCounter).ok).toBe(false);
    const negative = structuredClone(base) as unknown as { events: [{ counters: Record<string, number> }] };
    negative.events[0].counters.page_view = -1;
    expect(validateEnvelope(negative).ok).toBe(false);
    // The website route only ever accepts the website surface.
    expect(validateEnvelope({ ...base, surface: "web-app" }, { surfaces: ["website"] }).ok).toBe(false);
  });
});

describe("consent", () => {
  it("fails closed on missing, unreadable, declined, and older decisions", () => {
    expect(readConsent(null)).toBe("undecided");
    expect(readConsent("{not json")).toBe("undecided");
    expect(readConsent(JSON.stringify({ version: 3, granted: true }))).toBe("undecided");
    expect(readConsent(JSON.stringify({ version: CONSENT_VERSION, granted: false }))).toBe("declined");
    expect(readConsent(JSON.stringify({ version: CONSENT_VERSION, granted: true }))).toBe("granted");
  });

  it("counts nothing and stores nothing without consent", () => {
    const storage = memoryStorage();
    const { recorder, timers, sent } = recorderWith(storage);
    recorder.record("page_view");
    recorder.record("install_copy");
    expect(recorder.pending()).toEqual(emptyCounters());
    expect(storage.data.has(COUNTERS_STORAGE_KEY)).toBe(false);
    expect(storage.data.has(INSTALL_STORAGE_KEY)).toBe(false);
    expect(timers).toHaveLength(0);
    expect(sent).toHaveLength(0);
  });

  it("delivers one aggregate after consent and discards it after the attempt", async () => {
    const storage = memoryStorage();
    const { recorder, timers, sent } = recorderWith(storage);
    recorder.grant();
    expect(recorder.consent()).toBe("granted");
    recorder.record("page_view");
    recorder.record("page_view");
    recorder.record("docs_view");
    expect(timers).toHaveLength(1);
    timers[0]();
    await Promise.resolve();
    expect(sent).toHaveLength(1);
    const body = JSON.parse(sent[0]);
    expect(validateEnvelope(body).ok).toBe(true);
    expect(body.events[0].counters.page_view).toBe(2);
    expect(body.events[0].counters.docs_view).toBe(1);
    expect(body.install_id).toBe(UUID);
    expect(recorder.pending()).toEqual(emptyCounters());
    expect(storage.data.has(COUNTERS_STORAGE_KEY)).toBe(false);
  });

  it("clears queued counts and identity on opt-out and cancels pending delivery", async () => {
    const storage = memoryStorage();
    const { recorder, timers, sent } = recorderWith(storage);
    recorder.grant();
    recorder.record("page_view");
    await recorder.flush();
    expect(sent).toHaveLength(1);
    expect(storage.data.has(INSTALL_STORAGE_KEY)).toBe(true);
    recorder.record("download");
    expect(timers).toHaveLength(2);
    recorder.decline();
    expect(recorder.consent()).toBe("declined");
    expect(recorder.pending()).toEqual(emptyCounters());
    expect(storage.data.has(INSTALL_STORAGE_KEY)).toBe(false);
    expect(storage.data.has(COUNTERS_STORAGE_KEY)).toBe(false);
    timers[1]();
    await Promise.resolve();
    expect(sent).toHaveLength(1);
  });

  it("honours another tab's revocation through sync()", () => {
    const storage = memoryStorage();
    const { recorder } = recorderWith(storage);
    recorder.grant();
    recorder.record("page_view");
    expect(recorder.pending().page_view).toBe(1);
    // Another tab declines: the shared storage changes underneath us.
    storage.setItem(CONSENT_STORAGE_KEY, JSON.stringify({ version: CONSENT_VERSION, granted: false }));
    recorder.sync();
    expect(recorder.pending()).toEqual(emptyCounters());
    expect(storage.data.has(COUNTERS_STORAGE_KEY)).toBe(false);
  });
});

describe("install id", () => {
  it("is a random v4 id, kept for 90 days and then rotated", () => {
    const fresh = resolveInstallId(null, NOW, () => UUID);
    expect(fresh).toEqual({ id: UUID, raw: JSON.stringify({ id: UUID, createdAt: NOW }), rotated: true });
    const kept = resolveInstallId(fresh.raw, NOW + INSTALL_ID_ROTATION_MS - 1, () => "unused");
    expect(kept.id).toBe(UUID);
    expect(kept.rotated).toBe(false);
    const rotated = resolveInstallId(fresh.raw, NOW + INSTALL_ID_ROTATION_MS, () => "9f1a2b3c-4d5e-4f60-8a1b-2c3d4e5f6a7b");
    expect(rotated.id).toBe("9f1a2b3c-4d5e-4f60-8a1b-2c3d4e5f6a7b");
    expect(rotated.rotated).toBe(true);
    expect(resolveInstallId("garbage", NOW, () => UUID).rotated).toBe(true);
  });
});

describe("same-origin forwarder", () => {
  const envelope = buildEnvelope({ counters: { ...emptyCounters(), page_view: 1 }, installId: UUID, appVersion: "0.9.12", surface: "website", now: NOW });
  const post = (body: unknown) =>
    new Request("http://localhost/api/product-telemetry", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: typeof body === "string" ? body : JSON.stringify(body),
    });

  it("is inert without the exact canonical ingest configured", async () => {
    expect(ingestUrl({})).toBeNull();
    expect(ingestUrl({ CODEWHALE_TELEMETRY_INGEST_URL: "https://example.com/v1/telemetry" })).toBeNull();
    expect(ingestUrl({ CODEWHALE_TELEMETRY_INGEST_URL: CANONICAL_INGEST_URL })).toBe(CANONICAL_INGEST_URL);
    let forwarded = 0;
    const response = await handleProductTelemetry(post(envelope), {
      ingestUrl: null,
      forward: async () => {
        forwarded += 1;
        return { ok: true };
      },
    });
    expect(response.status).toBe(200);
    expect(await response.json()).toEqual({ accepted: false, reason: "disabled" });
    expect(forwarded).toBe(0);
  });

  it("forwards only a validated website batch, and only the batch", async () => {
    const calls: { url: string; body: string }[] = [];
    const forward = async (url: string, body: string) => {
      calls.push({ url, body });
      return { ok: true };
    };
    const accepted = await handleProductTelemetry(post(envelope), { ingestUrl: CANONICAL_INGEST_URL, forward });
    expect(await accepted.json()).toEqual({ accepted: true });
    expect(calls).toHaveLength(1);
    expect(calls[0].url).toBe(CANONICAL_INGEST_URL);
    expect(JSON.parse(calls[0].body)).toEqual(envelope);

    const rejected = await handleProductTelemetry(post({ ...envelope, surface: "web-app" }), { ingestUrl: CANONICAL_INGEST_URL, forward });
    expect(rejected.status).toBe(422);
    expect(calls).toHaveLength(1);

    const invalid = await handleProductTelemetry(post("{"), { ingestUrl: CANONICAL_INGEST_URL, forward });
    expect(invalid.status).toBe(422);

    const oversized = await handleProductTelemetry(post({ ...envelope, app_version: "0.9.12-" + "x".repeat(5000) }), { ingestUrl: CANONICAL_INGEST_URL, forward });
    expect(oversized.status).toBe(413);
    expect(calls).toHaveLength(1);
  });

  it("reports an unreachable ingest as unavailable without retrying", async () => {
    let attempts = 0;
    const response = await handleProductTelemetry(post(envelope), {
      ingestUrl: CANONICAL_INGEST_URL,
      forward: async () => {
        attempts += 1;
        throw new Error("down");
      },
    });
    expect(await response.json()).toEqual({ accepted: false, reason: "unavailable" });
    expect(attempts).toBe(1);
  });
});
