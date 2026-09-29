//! Outbox writer: atomic `pending/` files for when the local app is unavailable.
//!
//! When the local API cannot be reached (MVP-SPEC §7.2), the adapter persists
//! the full capture envelope so the app can import it on its next start. The
//! outbox is fallback transport, not a second database (stack doc "Outbox"):
//!
//! - root: `XEMNAS_OUTBOX_DIR`, default `<XEMNAS_DATA_DIR>/outbox`;
//! - the adapter only creates/writes `pending/`; the app owns `sending/`,
//!   `accepted/` and `rejected/` during import;
//! - file name: `<sha256hex(idempotency_key)>.json`, so re-writing the same key
//!   overwrites the same file and never duplicates it;
//! - the content is the complete envelope JSON;
//! - the write is a temporary sibling followed by an atomic rename, so no
//!   partially written file is ever visible under `pending/`.
//!
//! Filesystem failures never throw: they are returned as a failed result so the
//! caller can record a sanitized failure (PRIV-001) without crashing the hook.

import { createHash } from "node:crypto";
import { mkdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import type { AdapterConfig } from "./config.js";
import type { CaptureEnvelope } from "./envelope.js";

/** Successful pending write. */
export interface OutboxWriteOk {
  ok: true;
  /** Absolute path of the visible `pending/<sha256>.json` file. */
  path: string;
}

/** Failed pending write; `reason` is a short, sanitized code. */
export interface OutboxWriteError {
  ok: false;
  reason: string;
}

/** Result of persisting one envelope to the outbox. */
export type OutboxWriteResult = OutboxWriteOk | OutboxWriteError;

/** Persists envelopes to `outbox/pending`. */
export interface OutboxWriter {
  writePending(envelope: CaptureEnvelope): OutboxWriteResult;
}

let temporaryCounter = 0;

/** Contract file name for a key: `<sha256hex(idempotency_key)>.json`. */
export function pendingFileName(idempotencyKey: string): string {
  return `${createHash("sha256").update(idempotencyKey, "utf8").digest("hex")}.json`;
}

/** Creates a filesystem-backed outbox writer rooted at `config.pendingDir`. */
export function createOutboxWriter(
  config: Pick<AdapterConfig, "pendingDir">,
): OutboxWriter {
  return {
    writePending(envelope: CaptureEnvelope): OutboxWriteResult {
      const name = pendingFileName(envelope.idempotency_key);
      const finalPath = join(config.pendingDir, name);
      // Unique temporary so concurrent writers cannot clobber each other's
      // partial file; the rename below is what publishes it.
      const temporaryPath = join(
        config.pendingDir,
        `${name}.${process.pid}.${temporaryCounter++}.tmp`,
      );
      try {
        mkdirSync(config.pendingDir, { recursive: true });
        writeFileSync(temporaryPath, `${JSON.stringify(envelope)}\n`, "utf8");
        renameSync(temporaryPath, finalPath);
        return { ok: true, path: finalPath };
      } catch (error) {
        try {
          rmSync(temporaryPath, { force: true });
        } catch {
          // Best effort: never mask the original failure.
        }
        const code =
          error instanceof Error
            ? ((error as NodeJS.ErrnoException).code ?? error.name)
            : "io-error";
        return { ok: false, reason: code };
      }
    },
  };
}
