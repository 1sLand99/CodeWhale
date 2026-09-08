/**
 * Ed25519 verifying keys for the CodeWhale cloud facts channel (facts/v1).
 *
 * Mirror of `crates/config/src/cloud_facts/keys.rs` — `check:facts` fails if
 * the two diverge. Keys are pinned here (and in the binary); the Supabase
 * `facts_key` table is informational and never a trust root.
 *
 * No production signing anchor has been approved for this release. An empty
 * table disables delivery before any transport read. Tests supply their own
 * public fixture keys; those keys never belong in this table.
 */
export type KeyStatus = "active" | "retired";

export interface TrustedKey {
  keyId: string;
  /** Standard base64 of the raw 32-byte Ed25519 public key. */
  publicKey: string;
  status: KeyStatus;
}

export const DOMAIN = "codewhale-facts/v1\0";
export const ENVELOPE_VERSION = 1;
export const SUPPORTED_SCHEMA_VERSION = 1;
export const MAX_PAYLOAD_BYTES = 512 * 1024;

export const TRUSTED_KEYS: readonly TrustedKey[] = [];

export function trustedKey(keyId: string): TrustedKey | undefined {
  return TRUSTED_KEYS.find((key) => key.keyId === keyId);
}
