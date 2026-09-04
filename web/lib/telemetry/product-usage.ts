/**
 * product-usage.ts — aggregate, consented usage counting for the website.
 *
 * This is the browser side of the first-party telemetry contract
 * (telemetry-ingest/src/schema.ts, docs/TELEMETRY.md): closed schema version
 * 2, explicit consent version 4, one `product_usage` event carrying thirteen
 * unsigned counters, a random v4 install id unrelated to any person and
 * rotated every 90 days, and nothing else — no page, URL, referrer, error
 * text, account, or content ever enters the envelope. There is no analytics
 * SDK and no processor token in the browser; the same-origin route
 * (app/api/product-telemetry) forwards a validated batch to the canonical
 * ingest only when the operator has configured that exact endpoint.
 *
 * Consent fails closed. A missing, unreadable, declined, or older-version
 * decision means nothing is counted and nothing is stored. Revoking consent
 * clears the queued counts and the install id, cancels any pending delivery,
 * and — through the `storage` event — does the same in every other open tab.
 *
 * Framework-free and injectable so the contract is testable in Node: the
 * storage, clock, id source, and transport are parameters with browser
 * defaults.
 */

export const SCHEMA_VERSION = 2;
export const CONSENT_VERSION = 4;
export const INSTALL_ID_ROTATION_MS = 90 * 24 * 60 * 60 * 1000;
export const MAX_ENVELOPE_BYTES = 4 * 1024;
/** Counts wait this long after the last interaction before one delivery. */
export const FLUSH_DELAY_MS = 20_000;

export const CONSENT_STORAGE_KEY = "cw-usage-consent";
export const INSTALL_STORAGE_KEY = "cw-usage-install";
export const COUNTERS_STORAGE_KEY = "cw-usage-counters";

export const PRODUCT_COUNTER_FIELDS = [
  "page_view",
  "docs_view",
  "install_copy",
  "download",
  "signup",
  "login",
  "session_create",
  "session_resume",
  "turn_submit",
  "turn_complete",
  "settings_open",
  "integration_connect",
  "error_shown",
] as const;

export type ProductCounter = (typeof PRODUCT_COUNTER_FIELDS)[number];
export type ProductCounters = Record<ProductCounter, number>;

export const SURFACES = ["website", "web-app", "desktop"] as const;
export type Surface = (typeof SURFACES)[number];

export const ENVELOPE_FIELDS = [
  "schema_version",
  "consent_version",
  "sent_at",
  "install_id",
  "app_version",
  "git_sha",
  "surface",
  "os",
  "arch",
  "libc",
  "tty",
  "events",
] as const;

export interface ProductUsageEnvelope {
  schema_version: 2;
  consent_version: 4;
  sent_at: string;
  install_id: string;
  app_version: string;
  git_sha: null;
  surface: Surface;
  os: "other";
  arch: "other";
  libc: "none";
  tty: false;
  events: [{ event: "product_usage"; counters: ProductCounters }];
}

const U32_MAX = 4294967295;
const SENT_AT_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;
const INSTALL_ID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const VERSION_RE = /^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/;

export function emptyCounters(): ProductCounters {
  return Object.fromEntries(PRODUCT_COUNTER_FIELDS.map((field) => [field, 0])) as ProductCounters;
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function keysExactly(value: Record<string, unknown>, expected: readonly string[]): string | null {
  const actual = Object.keys(value);
  for (const key of actual) if (!expected.includes(key)) return `unexpected key ${key}`;
  for (const key of expected) if (!(key in value)) return `missing key ${key}`;
  return null;
}

/**
 * The closed-set validator, mirroring the ingest's rules for a browser
 * batch: exact key sets everywhere, constant envelope values, one
 * `product_usage` event, every counter a u32. Unknown keys reject the whole
 * envelope — there is no sanitising path.
 */
export function validateEnvelope(
  value: unknown,
  options: { surfaces?: readonly Surface[] } = {},
): { ok: true; envelope: ProductUsageEnvelope } | { ok: false; reason: string } {
  const surfaces = options.surfaces ?? SURFACES;
  if (!isPlainObject(value)) return { ok: false, reason: "not an object" };
  const keyError = keysExactly(value, ENVELOPE_FIELDS);
  if (keyError) return { ok: false, reason: `envelope: ${keyError}` };
  if (value.schema_version !== SCHEMA_VERSION) return { ok: false, reason: "schema_version" };
  if (value.consent_version !== CONSENT_VERSION) return { ok: false, reason: "consent_version" };
  if (typeof value.sent_at !== "string" || !SENT_AT_RE.test(value.sent_at)) return { ok: false, reason: "sent_at" };
  if (typeof value.install_id !== "string" || !INSTALL_ID_RE.test(value.install_id)) return { ok: false, reason: "install_id" };
  if (typeof value.app_version !== "string" || value.app_version.length > 64 || !VERSION_RE.test(value.app_version)) {
    return { ok: false, reason: "app_version" };
  }
  if (value.git_sha !== null) return { ok: false, reason: "git_sha" };
  if (typeof value.surface !== "string" || !surfaces.includes(value.surface as Surface)) return { ok: false, reason: "surface" };
  if (value.os !== "other") return { ok: false, reason: "os" };
  if (value.arch !== "other") return { ok: false, reason: "arch" };
  if (value.libc !== "none") return { ok: false, reason: "libc" };
  if (value.tty !== false) return { ok: false, reason: "tty" };
  if (!Array.isArray(value.events) || value.events.length !== 1) return { ok: false, reason: "events" };
  const event = value.events[0];
  if (!isPlainObject(event)) return { ok: false, reason: "event: not an object" };
  const eventKeyError = keysExactly(event, ["event", "counters"]);
  if (eventKeyError) return { ok: false, reason: `event: ${eventKeyError}` };
  if (event.event !== "product_usage") return { ok: false, reason: "event: name" };
  if (!isPlainObject(event.counters)) return { ok: false, reason: "counters: not an object" };
  const counterKeyError = keysExactly(event.counters, PRODUCT_COUNTER_FIELDS);
  if (counterKeyError) return { ok: false, reason: `counters: ${counterKeyError}` };
  for (const field of PRODUCT_COUNTER_FIELDS) {
    const item = event.counters[field];
    if (typeof item !== "number" || !Number.isInteger(item) || item < 0 || item > U32_MAX) {
      return { ok: false, reason: `counters: ${field}` };
    }
  }
  return { ok: true, envelope: value as unknown as ProductUsageEnvelope };
}

/** RFC3339 UTC at second precision, exactly `to_rfc3339_opts(Secs, true)`. */
export function sentAt(now: number): string {
  return new Date(Math.floor(now / 1000) * 1000).toISOString().replace(/\.\d{3}Z$/, "Z");
}

export function buildEnvelope(input: {
  counters: ProductCounters;
  installId: string;
  appVersion: string;
  surface: Surface;
  now: number;
}): ProductUsageEnvelope {
  return {
    schema_version: SCHEMA_VERSION,
    consent_version: CONSENT_VERSION,
    sent_at: sentAt(input.now),
    install_id: input.installId,
    app_version: input.appVersion,
    git_sha: null,
    surface: input.surface,
    os: "other",
    arch: "other",
    libc: "none",
    tty: false,
    events: [{ event: "product_usage", counters: { ...input.counters } }],
  };
}

// ------------------------------------------------------------------ consent

export interface ConsentRecord {
  version: number;
  granted: boolean;
  decidedAt: string;
}

export type ConsentState = "granted" | "declined" | "undecided";

/**
 * Reads the stored decision and fails closed: anything that is not a
 * current-version, explicitly granted record counts as no consent. An older
 * version's grant is `undecided` — the person has to see the current
 * disclosure again — while any current-version refusal stays `declined`.
 */
export function readConsent(raw: string | null | undefined): ConsentState {
  if (!raw) return "undecided";
  try {
    const parsed = JSON.parse(raw) as Partial<ConsentRecord>;
    if (!isPlainObject(parsed) || parsed.version !== CONSENT_VERSION) return "undecided";
    return parsed.granted === true ? "granted" : "declined";
  } catch {
    return "undecided";
  }
}

export function consentRecord(granted: boolean, now: number): string {
  const record: ConsentRecord = { version: CONSENT_VERSION, granted, decidedAt: sentAt(now) };
  return JSON.stringify(record);
}

// --------------------------------------------------------------- install id

interface InstallRecord {
  id: string;
  createdAt: number;
}

/** The current install id, or a fresh one when missing, malformed, or older than 90 days. */
export function resolveInstallId(
  raw: string | null | undefined,
  now: number,
  randomUuid: () => string,
): { id: string; raw: string; rotated: boolean } {
  try {
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<InstallRecord>;
      if (
        isPlainObject(parsed) &&
        typeof parsed.id === "string" &&
        INSTALL_ID_RE.test(parsed.id) &&
        typeof parsed.createdAt === "number" &&
        now - parsed.createdAt >= 0 &&
        now - parsed.createdAt < INSTALL_ID_ROTATION_MS
      ) {
        return { id: parsed.id, raw, rotated: false };
      }
    }
  } catch {
    /* unreadable: rotate */
  }
  const id = randomUuid();
  const record: InstallRecord = { id, createdAt: now };
  return { id, raw: JSON.stringify(record), rotated: true };
}

// ----------------------------------------------------------------- counters

export function readCounters(raw: string | null | undefined): ProductCounters {
  const counters = emptyCounters();
  if (!raw) return counters;
  try {
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    if (!isPlainObject(parsed)) return counters;
    for (const field of PRODUCT_COUNTER_FIELDS) {
      const value = parsed[field];
      if (typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= U32_MAX) {
        counters[field] = value;
      }
    }
  } catch {
    /* unreadable: start from zero */
  }
  return counters;
}

export function hasCounts(counters: ProductCounters): boolean {
  return PRODUCT_COUNTER_FIELDS.some((field) => counters[field] > 0);
}

// ----------------------------------------------------------------- recorder

export interface StorageLike {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

export interface RecorderOptions {
  surface: Surface;
  appVersion: string;
  /** Same-origin route that forwards to the canonical ingest. */
  endpoint: string;
  storage: StorageLike;
  now?: () => number;
  randomUuid?: () => string;
  /** Transport; returns whether the batch was accepted. Never retried. */
  send?: (endpoint: string, body: string) => Promise<boolean>;
  setTimer?: (callback: () => void, delayMs: number) => unknown;
  clearTimer?: (handle: unknown) => void;
  flushDelayMs?: number;
}

export interface UsageRecorder {
  consent(): ConsentState;
  grant(): void;
  decline(): void;
  /** Re-read consent from storage (another tab may have changed it). */
  sync(): void;
  record(counter: ProductCounter): void;
  /** Deliver whatever is queued now (also used on pagehide). */
  flush(): Promise<void>;
  pending(): ProductCounters;
}

function browserSend(endpoint: string, body: string): Promise<boolean> {
  if (typeof fetch !== "function") return Promise.resolve(false);
  const controller = typeof AbortController === "function" ? new AbortController() : null;
  const timer = controller ? setTimeout(() => controller.abort(), 1500) : null;
  return fetch(endpoint, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body,
    keepalive: true,
    credentials: "omit",
    referrerPolicy: "no-referrer",
    signal: controller?.signal,
  })
    .then((response) => response.ok)
    .catch(() => false)
    .finally(() => {
      if (timer !== null) clearTimeout(timer);
    });
}

export function createUsageRecorder(options: RecorderOptions): UsageRecorder {
  const now = options.now ?? (() => Date.now());
  const randomUuid = options.randomUuid ?? (() => crypto.randomUUID());
  const send = options.send ?? browserSend;
  const setTimer = options.setTimer ?? ((callback, delay) => setTimeout(callback, delay));
  const clearTimer = options.clearTimer ?? ((handle) => clearTimeout(handle as ReturnType<typeof setTimeout>));
  const flushDelayMs = options.flushDelayMs ?? FLUSH_DELAY_MS;
  const { storage } = options;

  let counters = emptyCounters();
  let timer: unknown = null;
  let inFlight = false;

  const read = (key: string) => {
    try {
      return storage.getItem(key);
    } catch {
      return null;
    }
  };
  const write = (key: string, value: string) => {
    try {
      storage.setItem(key, value);
    } catch {
      /* storage unavailable: counting stays in memory for this page only */
    }
  };
  const remove = (key: string) => {
    try {
      storage.removeItem(key);
    } catch {
      /* nothing to clear */
    }
  };

  const cancelTimer = () => {
    if (timer !== null) clearTimer(timer);
    timer = null;
  };

  /** Everything queued and every identity goes; nothing pending survives. */
  const clearAll = () => {
    cancelTimer();
    counters = emptyCounters();
    remove(COUNTERS_STORAGE_KEY);
    remove(INSTALL_STORAGE_KEY);
  };

  const consent = () => readConsent(read(CONSENT_STORAGE_KEY));

  const schedule = () => {
    if (timer !== null) return;
    timer = setTimer(() => {
      timer = null;
      void flush();
    }, flushDelayMs);
  };

  const flush = async () => {
    if (inFlight) return;
    cancelTimer();
    if (consent() !== "granted") {
      clearAll();
      return;
    }
    const batch = counters;
    if (!hasCounts(batch)) return;
    const install = resolveInstallId(read(INSTALL_STORAGE_KEY), now(), randomUuid);
    if (install.rotated) write(INSTALL_STORAGE_KEY, install.raw);
    const envelope = buildEnvelope({
      counters: batch,
      installId: install.id,
      appVersion: options.appVersion,
      surface: options.surface,
      now: now(),
    });
    const body = JSON.stringify(envelope);
    if (!validateEnvelope(envelope).ok || body.length > MAX_ENVELOPE_BYTES) return;
    // Discard after one attempt, accepted or not: there is no retry queue,
    // and a count that did not land is not worth remembering.
    counters = emptyCounters();
    remove(COUNTERS_STORAGE_KEY);
    inFlight = true;
    try {
      await send(options.endpoint, body);
    } finally {
      inFlight = false;
    }
  };

  // Hydrate any counts a previous page on this origin left behind, but only
  // when consent is current; otherwise clear them as a stale grant's debris.
  if (consent() === "granted") {
    counters = readCounters(read(COUNTERS_STORAGE_KEY));
  } else {
    clearAll();
  }

  return {
    consent,
    grant() {
      write(CONSENT_STORAGE_KEY, consentRecord(true, now()));
    },
    decline() {
      write(CONSENT_STORAGE_KEY, consentRecord(false, now()));
      clearAll();
    },
    sync() {
      if (consent() !== "granted") clearAll();
    },
    record(counter) {
      if (consent() !== "granted") {
        clearAll();
        return;
      }
      if (counters[counter] < U32_MAX) counters[counter] += 1;
      write(COUNTERS_STORAGE_KEY, JSON.stringify(counters));
      schedule();
    },
    flush,
    pending: () => ({ ...counters }),
  };
}
