"use client";

/**
 * <UsageConsent> — the website's usage-counting recorder and its consent UI.
 *
 * One client component does three jobs so there is exactly one owner of the
 * recorder on the page:
 *
 *   1. Counts. `page_view` on every route change (`docs_view` too on docs
 *      routes), plus any element carrying `data-usage="<counter>"` when it
 *      is clicked — server components mark links; nothing else changes.
 *      `recordUsage()` is the import for client components (copy buttons,
 *      the error boundary).
 *   2. Consent. A quiet sheet at the foot of the viewport until the person
 *      decides; the footer's "Usage data" control reopens it. Explicit,
 *      reversible, versioned, and fail-closed — see lib/telemetry.
 *   3. Delivery. One flush after a pause or on pagehide; other tabs'
 *      revocation is honoured through the `storage` event.
 *
 * Nothing renders on the server, so the page is complete and static without
 * it; reduced motion is respected because there is no motion.
 */

import { usePathname } from "next/navigation";
import { useCallback, useEffect, useRef, useState } from "react";
import { USAGE_CONSENT_COPY } from "@/lib/content/usage-consent";
import { pickText } from "@/lib/i18n/dictionaries";
import { isDocsPath } from "@/lib/i18n/path";
import {
  CONSENT_STORAGE_KEY,
  PRODUCT_COUNTER_FIELDS,
  createUsageRecorder,
  type ConsentState,
  type ProductCounter,
  type UsageRecorder,
} from "@/lib/telemetry/product-usage";

const OPEN_EVENT = "cw-usage-consent:open";
const ENDPOINT = "/api/product-telemetry";

let recorder: UsageRecorder | null = null;

function getRecorder(appVersion: string): UsageRecorder | null {
  if (typeof window === "undefined") return null;
  if (!recorder) {
    let storage: Storage | null = null;
    try {
      storage = window.localStorage;
    } catch {
      storage = null;
    }
    if (!storage) return null;
    recorder = createUsageRecorder({
      surface: "website",
      appVersion,
      endpoint: ENDPOINT,
      storage,
    });
  }
  return recorder;
}

/** Count one interaction. A no-op without current consent. */
export function recordUsage(counter: ProductCounter): void {
  recorder?.record(counter);
}

/** Reopen the consent sheet (the footer's "Usage data" control). */
export function openUsageConsent(): void {
  if (typeof window !== "undefined") window.dispatchEvent(new Event(OPEN_EVENT));
}

function isCounter(value: string | undefined): value is ProductCounter {
  return value !== undefined && (PRODUCT_COUNTER_FIELDS as readonly string[]).includes(value);
}

export function UsageConsent({ locale, appVersion }: { locale: string; appVersion: string }) {
  const pathname = usePathname();
  const [state, setState] = useState<ConsentState>("undecided");
  const [open, setOpen] = useState(false);
  const [mounted, setMounted] = useState(false);
  const sheetRef = useRef<HTMLDivElement>(null);
  const openerRef = useRef<HTMLElement | null>(null);

  // Mount: resolve consent, open the sheet when undecided, and wire the
  // listeners that make the choice reversible from anywhere.
  useEffect(() => {
    const current = getRecorder(appVersion);
    if (!current) return;
    setMounted(true);
    const consent = current.consent();
    setState(consent);
    if (consent === "undecided") setOpen(true);

    const onOpen = () => {
      openerRef.current = document.activeElement as HTMLElement | null;
      setState(current.consent());
      setOpen(true);
    };
    const onStorage = (event: StorageEvent) => {
      if (event.key !== null && event.key !== CONSENT_STORAGE_KEY) return;
      current.sync();
      setState(current.consent());
    };
    const onClick = (event: MouseEvent) => {
      const target = event.target as HTMLElement | null;
      const marked = target?.closest<HTMLElement>("[data-usage]");
      const counter = marked?.dataset.usage;
      if (isCounter(counter)) current.record(counter);
    };
    const onPageHide = () => {
      void current.flush();
    };
    window.addEventListener(OPEN_EVENT, onOpen);
    window.addEventListener("storage", onStorage);
    document.addEventListener("click", onClick);
    window.addEventListener("pagehide", onPageHide);
    return () => {
      window.removeEventListener(OPEN_EVENT, onOpen);
      window.removeEventListener("storage", onStorage);
      document.removeEventListener("click", onClick);
      window.removeEventListener("pagehide", onPageHide);
    };
  }, [appVersion]);

  // Every route change is one page view; docs routes are also a docs view.
  useEffect(() => {
    const current = getRecorder(appVersion);
    if (!current || !pathname) return;
    current.record("page_view");
    if (isDocsPath(pathname)) current.record("docs_view");
  }, [appVersion, pathname]);

  // Focus follows an explicit footer request, never the first page visit.
  // Dismissal leaves an undecided preference off; only grant() enables counts.
  useEffect(() => {
    if (!open) return;
    const sheet = sheetRef.current;
    if (openerRef.current) sheet?.querySelector<HTMLElement>("button")?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        setOpen(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      openerRef.current?.focus();
      openerRef.current = null;
    };
  }, [open, state]);

  const decide = useCallback(
    (granted: boolean) => {
      const current = getRecorder(appVersion);
      if (!current) return;
      if (granted) current.grant();
      else current.decline();
      setState(current.consent());
      setOpen(false);
      // The visit that produced the decision is itself one page view.
      if (granted) current.record("page_view");
    },
    [appVersion],
  );

  if (!mounted || !open) return null;

  const t = (text: { en: string; zh: string }) => pickText(text, locale);
  const undecided = state === "undecided";

  return (
    <div
      ref={sheetRef}
      className="usage-consent"
      role="dialog"
      aria-modal="false"
      aria-label={t(USAGE_CONSENT_COPY.dialogAria)}
      data-state={state}
    >
      <div className="usage-consent-copy">
        <div className="usage-consent-heading"><strong>{t(USAGE_CONSENT_COPY.title)}</strong><button type="button" className="usage-consent-dismiss" aria-label={t(USAGE_CONSENT_COPY.close)} onClick={() => setOpen(false)}>×</button></div>
        <p>{t(USAGE_CONSENT_COPY.summary)}</p>
        <details><summary>{t(USAGE_CONSENT_COPY.details)}</summary><p>{t(USAGE_CONSENT_COPY.body)}</p></details>
        {!undecided && <p className="usage-consent-status">{t(USAGE_CONSENT_COPY.status[state])}</p>}
      </div>
      <div className="usage-consent-actions">
        {undecided ? (
          <>
            <button type="button" className="usage-consent-button usage-consent-button-primary" onClick={() => decide(true)}>
              {t(USAGE_CONSENT_COPY.allow)}
            </button>
            <button type="button" className="usage-consent-button" onClick={() => decide(false)}>
              {t(USAGE_CONSENT_COPY.decline)}
            </button>
          </>
        ) : (
          <>
            <button
              type="button"
              className="usage-consent-button usage-consent-button-primary"
              onClick={() => decide(state !== "granted")}
            >
              {state === "granted" ? t(USAGE_CONSENT_COPY.turnOff) : t(USAGE_CONSENT_COPY.turnOn)}
            </button>
          </>
        )}
      </div>
    </div>
  );
}

/** The footer control that reopens the sheet. */
export function UsageDataLink({ label }: { label: string }) {
  return (
    <button type="button" className="usage-data-link" onClick={openUsageConsent}>
      {label}
    </button>
  );
}
