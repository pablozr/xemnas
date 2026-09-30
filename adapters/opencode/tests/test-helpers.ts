//! Shared, hermetic test helpers for the OpenCode adapter.
//!
//! Nothing here touches the network, the real OpenCode server or the local API:
//! tests drive the adapter through injected ports and temporary directories.

import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  DEFAULT_MAX_ARTIFACTS,
  DEFAULT_MAX_ARTIFACT_BYTES,
  DEFAULT_MAX_DIFF_BYTES,
  DEFAULT_MAX_MESSAGE_PAGES,
  DEFAULT_MESSAGE_LIMIT,
  DEFAULT_OPENCODE_URL,
  type AdapterConfig,
} from "../src/config.js";
import type { CaptureClient, PostResult } from "../src/client.js";
import type { CaptureEnvelope } from "../src/envelope.js";
import type { AdapterLog } from "../src/index.js";
import type { MessageWithParts, RawFileDiff } from "../src/opencode.js";

/** Creates a unique temporary directory for adapter state. */
export function createTempStateDir(): string {
  return mkdtempSync(join(tmpdir(), "xemnas-adapter-test-"));
}

/** Builds a full [`AdapterConfig`] rooted in a temporary directory. */
export function testConfig(
  stateDir: string,
  overrides: Partial<AdapterConfig> = {},
): AdapterConfig {
  return {
    opencodeUrl: DEFAULT_OPENCODE_URL,
    discoveryPath: join(stateDir, "discovery.json"),
    tokenPath: join(stateDir, "api-token"),
    stateDir,
    checkpointsPath: join(stateDir, "checkpoints.json"),
    failuresPath: join(stateDir, "failures.json"),
    outboxDir: join(stateDir, "outbox"),
    pendingDir: join(stateDir, "outbox", "pending"),
    debounceMs: 0,
    requestTimeoutMs: 5_000,
    maxArtifactBytes: DEFAULT_MAX_ARTIFACT_BYTES,
    maxArtifacts: DEFAULT_MAX_ARTIFACTS,
    maxDiffBytes: DEFAULT_MAX_DIFF_BYTES,
    messageLimit: DEFAULT_MESSAGE_LIMIT,
    maxMessagePages: DEFAULT_MAX_MESSAGE_PAGES,
    contextTimeoutMs: 300,
    ...overrides,
  };
}

/** A scheduler that queues tasks for explicit execution and supports cancel. */
export interface ManualScheduler {
  schedule: (task: () => void, delayMs: number) => () => void;
  tasks: Array<{ task: () => void; delayMs: number }>;
  runAll(): void;
}

/** Creates a [`ManualScheduler`]. */
export function createManualScheduler(): ManualScheduler {
  const tasks: Array<{ task: () => void; delayMs: number }> = [];
  return {
    tasks,
    schedule(task, delayMs) {
      const entry = { task, delayMs };
      tasks.push(entry);
      return () => {
        const index = tasks.indexOf(entry);
        if (index >= 0) {
          tasks.splice(index, 1);
        }
      };
    },
    runAll() {
      const pending = [...tasks];
      tasks.length = 0;
      for (const entry of pending) {
        entry.task();
      }
    },
  };
}

/** A log that records rendered lines for assertions. */
export interface RecordingLog {
  log: AdapterLog;
  lines: string[];
}

/** Creates a [`RecordingLog`]. */
export function createRecordingLog(): RecordingLog {
  const lines: string[] = [];
  const record =
    (level: string) =>
    (event: string, fields: Record<string, string | number> = {}): void => {
      lines.push(JSON.stringify({ level, event, ...fields }));
    };
  return {
    lines,
    log: { info: record("info"), warn: record("warn"), error: record("error") },
  };
}

/** A capture client that records envelopes and replays scripted outcomes. */
export interface RecordingClient {
  client: CaptureClient;
  seen: CaptureEnvelope[];
}

/**
 * Creates a [`RecordingClient`].
 *
 * Outcomes are consumed in order; once exhausted, every call is `accepted`.
 */
export function createRecordingClient(results: PostResult[]): RecordingClient {
  const seen: CaptureEnvelope[] = [];
  return {
    seen,
    client: {
      async post(envelope: CaptureEnvelope): Promise<PostResult> {
        seen.push(envelope);
        return (
          results.shift() ?? {
            kind: "accepted",
            status: 201,
            captureId: envelope.capture_id,
          }
        );
      },
    },
  };
}

/** Builds a documented `{ info, parts }` message item. */
export function rawMessage(
  id: string,
  role: string,
  parts: unknown[],
  sessionId = "session",
): MessageWithParts {
  return {
    info: { id, role, sessionID: sessionId, time: { created: 2 } },
    parts,
  };
}

/** Builds a documented `FileDiff` item with `before`/`after` line snapshots. */
export function rawDiff(
  file: string,
  before: string,
  after: string,
): RawFileDiff {
  return { file, before, after, additions: 1, deletions: 1 };
}

/** Resolves after `ms` milliseconds (for timing assertions). */
export function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
