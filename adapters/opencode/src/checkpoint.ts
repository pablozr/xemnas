//! Per-session checkpoints and failure records.
//!
//! The checkpoint is what makes the online path lossless and idempotent
//! (MVP-SPEC §7.1). `last_message_id` is the last **user turn** successfully
//! delivered; it only moves after the local API answered `2xx`. A failed POST
//! leaves the checkpoint untouched, so the next idle rebuilds the same envelope
//! (same `message_id` and `diff_hash` ⇒ same `Idempotency-Key`) and the server
//! deduplicates it (ticket 09 §7.3.5).
//!
//! Both files are written with a temporary file plus an atomic rename so a
//! crash never leaves a half-written JSON document.

import {
  mkdirSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { dirname, join } from "node:path";

/** Checkpoint of one OpenCode session. */
export interface SessionCheckpoint {
  /** Last user message id delivered (or adopted) for this session. */
  last_message_id: string | null;
  /** Capture id of the last accepted envelope, when one was sent. */
  last_capture_id: string | null;
}

/** Sanitized record of a failed capture; carries no content. */
export interface CaptureFailure {
  /** ISO-8601 timestamp of the failure. */
  at: string;
  /** User message id whose envelope failed. */
  message_id: string;
  /** HTTP status when the server rejected the capture; `null` for local failures. */
  status: number | null;
  /** Short, sanitized reason code (for example `http-422`, `outbox-write`). */
  reason: string;
}

/** Persistence port for checkpoints and failures. */
export interface CheckpointStore {
  /** Returns the stored checkpoint, defaulting to an empty one. */
  get(sessionId: string): SessionCheckpoint;
  /**
   * Moves the checkpoint forward. `captureId` is optional: a turn with no
   * capturable artifacts advances the message id while preserving the last
   * accepted capture id.
   */
  advance(sessionId: string, messageId: string, captureId?: string): void;
  /** Records the most recent rejection for a session (a sanitized record). */
  recordFailure(sessionId: string, failure: CaptureFailure): void;
  /** Returns the most recent rejection record, when present. */
  lastFailure(sessionId: string): CaptureFailure | null;
}

/** Paths backing a [`CheckpointStore`]. */
export interface CheckpointPaths {
  checkpointsPath: string;
  failuresPath: string;
}

let temporaryCounter = 0;

/** Writes `data` as JSON to a temp sibling and renames it into place. */
function writeAtomic(path: string, data: unknown): void {
  mkdirSync(dirname(path), { recursive: true });
  const temporary = join(
    dirname(path),
    `.${process.pid}.${temporaryCounter++}.tmp`,
  );
  writeFileSync(temporary, `${JSON.stringify(data, null, 2)}\n`, "utf8");
  renameSync(temporary, path);
}

function readJsonObject(path: string): Record<string, unknown> {
  try {
    const parsed = JSON.parse(readFileSync(path, "utf8")) as unknown;
    return typeof parsed === "object" && parsed !== null
      ? (parsed as Record<string, unknown>)
      : {};
  } catch {
    return {};
  }
}

function toCheckpoint(value: unknown): SessionCheckpoint {
  if (typeof value !== "object" || value === null) {
    return { last_message_id: null, last_capture_id: null };
  }
  const record = value as Record<string, unknown>;
  return {
    last_message_id:
      typeof record.last_message_id === "string"
        ? record.last_message_id
        : null,
    last_capture_id:
      typeof record.last_capture_id === "string"
        ? record.last_capture_id
        : null,
  };
}

/** Creates a filesystem-backed checkpoint store. */
export function createCheckpointStore(paths: CheckpointPaths): CheckpointStore {
  const readAll = (): Record<string, unknown> =>
    readJsonObject(paths.checkpointsPath);

  return {
    get(sessionId: string): SessionCheckpoint {
      return toCheckpoint(readAll()[sessionId]);
    },

    advance(sessionId: string, messageId: string, captureId?: string): void {
      const all = readAll();
      const current = toCheckpoint(all[sessionId]);
      // Monotonic guard: an out-of-order or duplicated completion must never
      // move the checkpoint backwards, even though sessions are serialized.
      if (
        current.last_message_id !== null &&
        messageId <= current.last_message_id
      ) {
        return;
      }
      all[sessionId] = {
        last_message_id: messageId,
        last_capture_id: captureId ?? current.last_capture_id,
      };
      writeAtomic(paths.checkpointsPath, all);
    },

    recordFailure(sessionId: string, failure: CaptureFailure): void {
      const all = readJsonObject(paths.failuresPath);
      all[sessionId] = failure;
      writeAtomic(paths.failuresPath, all);
    },

    lastFailure(sessionId: string): CaptureFailure | null {
      const value = readJsonObject(paths.failuresPath)[sessionId];
      if (typeof value !== "object" || value === null) {
        return null;
      }
      const record = value as Record<string, unknown>;
      if (typeof record.at !== "string" || typeof record.message_id !== "string") {
        return null;
      }
      return {
        at: record.at,
        message_id: record.message_id,
        status: typeof record.status === "number" ? record.status : null,
        reason: typeof record.reason === "string" ? record.reason : "",
      };
    },
  };
}

/** Builds the checkpoint paths for a given state directory. */
export function checkpointPaths(stateDir: string): CheckpointPaths {
  return {
    checkpointsPath: join(stateDir, "checkpoints.json"),
    failuresPath: join(stateDir, "failures.json"),
  };
}
