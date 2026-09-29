//! Capture transport to the local API.
//!
//! Sends one validated envelope to `POST /v1/captures` with the bearer token
//! discovered at runtime (ticket 09). Outcomes are classified for the
//! checkpoint logic:
//!
//! - 2xx ⇒ `accepted` (advance the checkpoint);
//! - 4xx ⇒ `rejected` (record a sanitized failure, do **not** advance);
//! - 5xx, timeout or refused connection ⇒ `unavailable` (do **not** advance and
//!   do not crash).
//!
//! The outbox (MVP-SPEC §7.2) is ticket 11; this module deliberately has no
//! fallback persistence. A `rejected` or `unavailable` outcome is left for the
//! next idle to retry with the same idempotency key.

import {
  EXPECTED_PROTOCOL_VERSION,
  readDiscovery,
  readToken,
  type AdapterConfig,
} from "./config.js";
import type { CaptureEnvelope } from "./envelope.js";

/** Resolved local API coordinates for one attempt. */
export interface CaptureEndpoint {
  baseUrl: string;
  token: string;
}

/** Resolves the current local API endpoint, or `null` when unavailable. */
export type EndpointResolver = () => CaptureEndpoint | null;

/** Classified result of one POST attempt. */
export type PostResult =
  | { kind: "accepted"; status: number; captureId: string }
  | { kind: "rejected"; status: number }
  | { kind: "unavailable"; reason: string };

/** Sends capture envelopes. */
export interface CaptureClient {
  post(envelope: CaptureEnvelope): Promise<PostResult>;
}

/** Options for [`createCaptureClient`]. */
export interface CaptureClientOptions {
  resolveEndpoint: EndpointResolver;
  timeoutMs: number;
  fetchImpl?: typeof fetch;
}

/** Returns `capture_id` from a receipt body, or `null` when absent/invalid. */
function receiptCaptureId(body: string): string | null {
  try {
    const parsed = JSON.parse(body) as unknown;
    if (typeof parsed !== "object" || parsed === null) {
      return null;
    }
    const captureId = (parsed as Record<string, unknown>).capture_id;
    return typeof captureId === "string" && captureId.length > 0
      ? captureId
      : null;
  } catch {
    return null;
  }
}

/** Creates the HTTP capture client. */
export function createCaptureClient(
  options: CaptureClientOptions,
): CaptureClient {
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;
  return {
    async post(envelope: CaptureEnvelope): Promise<PostResult> {
      const endpoint = options.resolveEndpoint();
      if (endpoint === null) {
        return { kind: "unavailable", reason: "endpoint-unresolved" };
      }

      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), options.timeoutMs);
      try {
        const response = await fetchImpl(`${endpoint.baseUrl}/v1/captures`, {
          method: "POST",
          headers: {
            Authorization: `Bearer ${endpoint.token}`,
            "Content-Type": "application/json",
            "Idempotency-Key": envelope.idempotency_key,
          },
          body: JSON.stringify(envelope),
          signal: controller.signal,
        });
        const status = response.status;
        // Read the receipt body (small); never log it. The local API returns
        // `{ capture_id, idempotency_key, received_at, artifact_count }` and,
        // on an idempotent replay, preserves the **original** `capture_id`.
        const body = await response.text().catch(() => "");
        if (status >= 200 && status < 300) {
          return {
            kind: "accepted",
            status,
            captureId: receiptCaptureId(body) ?? envelope.capture_id,
          };
        }
        if (status >= 400 && status < 500) {
          return { kind: "rejected", status };
        }
        return { kind: "unavailable", reason: `http-${status}` };
      } catch (error) {
        const reason =
          error instanceof Error && error.name === "AbortError"
            ? "timeout"
            : "connection";
        return { kind: "unavailable", reason };
      } finally {
        clearTimeout(timer);
      }
    },
  };
}

/**
 * Builds an endpoint resolver backed by the discovery file.
 *
 * The discovery and token are read on every attempt because the local API
 * rewrites them on each start (new port and token), and a stale cached endpoint
 * would fail after a restart.
 */
export function createLocalApiEndpointResolver(
  config: Pick<AdapterConfig, "discoveryPath" | "tokenPath">,
): EndpointResolver {
  return () => {
    const discovery = readDiscovery(config);
    if (
      discovery === null ||
      discovery.protocol_version !== EXPECTED_PROTOCOL_VERSION
    ) {
      return null;
    }
    const token = readToken(config);
    if (token === null) {
      return null;
    }
    return { baseUrl: `http://127.0.0.1:${discovery.port}`, token };
  };
}
