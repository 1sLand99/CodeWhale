/** Optional processor for the existing validated ingest; no second collector. */
import { CONSENT_VERSION, SCHEMA_VERSION, type Batch } from "./schema";

export interface PostHogConfig {
  /** Unset by default. Exactly one of the two regional HTTPS capture origins. */
  POSTHOG_HOST?: string;
  /** Project token, configured as a Worker secret only after activation approval. */
  POSTHOG_PROJECT_TOKEN?: string;
  /** Operator prerequisite, never code-level proof of Cloudflare header removal. */
  POSTHOG_IP_SAFE_EGRESS_VERIFIED?: string;
}

export const POSTHOG_TIMEOUT_MS = 1_500;
const HOSTS = ["https://us.i.posthog.com", "https://eu.i.posthog.com"];

/** Only v2 proves consent to third-party processing. Old accepted batches stay first-party. */
export async function deliverPostHog(batch: Batch, config: PostHogConfig): Promise<void> {
  if (
    batch.schema_version !== SCHEMA_VERSION ||
    batch.consent_version !== CONSENT_VERSION ||
    batch.events.length === 0 ||
    config.POSTHOG_IP_SAFE_EGRESS_VERIFIED !== "true" ||
    !config.POSTHOG_HOST || !HOSTS.includes(config.POSTHOG_HOST) ||
    !config.POSTHOG_PROJECT_TOKEN || !/^phc_[A-Za-z0-9_-]{1,256}$/.test(config.POSTHOG_PROJECT_TOKEN)
  ) return;

  const { install_id, sent_at, events, ...envelope } = batch;
  const body = JSON.stringify({
    api_key: config.POSTHOG_PROJECT_TOKEN,
    batch: events.map(({ event, ...properties }) => ({
      event: `codewhale_${event}`,
      timestamp: sent_at,
      properties: {
        ...envelope,
        ...properties,
        distinct_id: `codewhale:${install_id}`,
        $process_person_profile: false,
        $geoip_disable: true,
        $ip: null,
      },
    })),
  });
  try {
    // Copy no incoming headers or connection metadata. Cloudflare may still
    // add platform headers; the operator guard requires a staging receipt for
    // the actual egress path. Never follow a redirect carrying the token.
    const response = await fetch(`${config.POSTHOG_HOST}/batch/`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body,
      redirect: "error",
      credentials: "omit",
      signal: AbortSignal.timeout(POSTHOG_TIMEOUT_MS),
    });
    // No retries, response parsing, shared queue, or log containing a token or payload.
    void response.body?.cancel().catch(() => {});
  } catch {
    // Processor failure must not change the first-party ingest result.
  }
}
