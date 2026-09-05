# Website ingestion bound and install priority — 2026-09-04

Follow-up to the local website integration e71e57376. The existing form stream
reader now lives in `web/lib/bounded-body.ts`; both login forms and the telemetry
forwarder consume it. Form media-type handling and `FormBodyError` remain intact.
The reader counts raw bytes before retaining each chunk, cancels on overflow,
releases its lock, and returns a controlled error for failed reads. Telemetry
uses fatal UTF-8 decoding and never forwards rejected input. The exact canonical
ingest setting, closed schema, consent path, no redirects, and no retries remain
unchanged. Comments now distinguish headers set in source from headers the
hosting transport could add; this is not a claim of network anonymity.

Homepage availability text in all 18 locales, product availability, installation
metadata/details, docs task descriptions, and EN/ZH FAQ installation answers now
put GitHub release binaries first. npm and Cargo are alternatives. The FAQ reuses
the existing getting-started command. Local GT export updated EN/ZH catalogs;
no translation service was called. Mirror-specific and package-update recipes
remain labeled alternatives.

## Local proof

- Website tests: 47 files, 407 passed, 0 failed. Nine added checks cover streaming
  overflow without length and with misleading lengths; cancellation and lock
  release; UTF-8 byte size; exact 4096-byte valid payload; malformed UTF-8 and
  lengths; read failure; and split multibyte form data at its exact byte boundary.
  Rejected telemetry cases assert zero forwarding.
- Lint: 0 errors, 2 existing wordmark image warnings. Production webpack build,
  locale/catalog checks, facts check, and docs check passed.
- Initial test run exposed stale local GT exports (405 pass / 1 fail); exporting
  the local catalogs resolved it. Final log is `site-byte-tests.log`.
- Built preview `http://127.0.0.1:3136/en`: EN desktop and ZH 390px homepage copy
  verified in the DOM; expanded EN FAQ verified at 390px. Document scroll/client
  widths match (1265 desktop; 375 within a 390px viewport with scrollbar).
- `/tmp/cwa-app-parity-20260904/screenshots/site-byte-en-desktop.png`,
  `site-byte-zh-390.png`, `site-byte-faq-en-390.png` were captured and inspected.
  Consent was dismissed without granting counting. No installer was executed.

This runtime repository has no root `npm test` / `check:web` scripts; the website
scoped test/lint/build/check scripts are the gate. No hosted CI, Cloudflare
transport behavior, deploy, configuration change, telemetry delivery, or customer
acceptance is claimed. App account-key work is a separate local commit c5aaef9b.
