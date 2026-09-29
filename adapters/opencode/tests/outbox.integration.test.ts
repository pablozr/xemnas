//! Adapter integration tests for the outbox fallback path.
//!
//! Only `unavailable` results persist to `outbox/pending`; `4xx` stays in
//! `failures.json`; `2xx` advances the checkpoint and writes nothing.

import assert from "node:assert/strict";
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { checkpointPaths, createCheckpointStore } from "../src/checkpoint.js";
import {
  createCaptureClient,
  createLocalApiEndpointResolver,
} from "../src/client.js";
import { createAdapter } from "../src/index.js";
import { createFakeMessageSource } from "../src/opencode.js";
import { pendingFileName } from "../src/outbox.js";
import {
  createRecordingClient,
  createRecordingLog,
  createTempStateDir,
  rawMessage,
  testConfig,
} from "./test-helpers.js";

const sessionId = "session-outbox-1";

function textPart(messageId: string, text: string): unknown {
  return {
    id: `${messageId}-t`,
    sessionID: sessionId,
    messageID: messageId,
    type: "text",
    text,
  };
}

const user1 = rawMessage(
  "msg-0001",
  "user",
  [textPart("msg-0001", "hello")],
  sessionId,
);
const assistant1 = rawMessage(
  "msg-0002",
  "assistant",
  [textPart("msg-0002", "done")],
  sessionId,
);

function source() {
  return createFakeMessageSource({
    messages: { [sessionId]: [user1, assistant1] },
  });
}

const fixedNow = (): Date => new Date("2026-09-28T12:00:00Z");

function pendingFiles(pendingDir: string): string[] {
  return existsSync(pendingDir) ? readdirSync(pendingDir) : [];
}

test("unavailable writes one pending file and leaves the checkpoint unchanged", async () => {
  const stateDir = createTempStateDir();
  const config = testConfig(stateDir);
  const recording = createRecordingClient([
    { kind: "unavailable", reason: "connection" },
  ]);
  const store = createCheckpointStore(checkpointPaths(stateDir));
  const adapter = createAdapter({
    config,
    source: source(),
    client: recording.client,
    checkpoints: store,
    directory: "C:/synthetic/projects/outbox",
    log: createRecordingLog().log,
    now: fixedNow,
  });

  await adapter.reconcileSession(sessionId);

  const envelope = recording.seen[0];
  const files = pendingFiles(config.pendingDir);
  assert.deepEqual(files, [pendingFileName(envelope.idempotency_key)]);
  assert.deepEqual(
    JSON.parse(readFileSync(join(config.pendingDir, files[0]), "utf8")),
    envelope,
  );
  assert.equal(store.get(sessionId).last_message_id, null);
  assert.equal(store.lastFailure(sessionId), null);
});

test("repeated unavailable overwrites the same pending file", async () => {
  const stateDir = createTempStateDir();
  const config = testConfig(stateDir);
  const recording = createRecordingClient([
    { kind: "unavailable", reason: "connection" },
    { kind: "unavailable", reason: "connection" },
  ]);
  const adapter = createAdapter({
    config,
    source: source(),
    client: recording.client,
    checkpoints: createCheckpointStore(checkpointPaths(stateDir)),
    directory: "C:/synthetic/projects/outbox",
    log: createRecordingLog().log,
    now: fixedNow,
  });

  await adapter.reconcileSession(sessionId);
  await adapter.reconcileSession(sessionId);

  const files = pendingFiles(config.pendingDir);
  assert.equal(files.length, 1);
  assert.ok(!files.some((name) => name.endsWith(".tmp")));
});

test("a 4xx rejection never creates an outbox file and records a failure", async () => {
  const stateDir = createTempStateDir();
  const config = testConfig(stateDir);
  const recording = createRecordingClient([
    { kind: "rejected", status: 422 },
  ]);
  const store = createCheckpointStore(checkpointPaths(stateDir));
  const adapter = createAdapter({
    config,
    source: source(),
    client: recording.client,
    checkpoints: store,
    directory: "C:/synthetic/projects/outbox",
    log: createRecordingLog().log,
    now: fixedNow,
  });

  await adapter.reconcileSession(sessionId);

  assert.equal(existsSync(config.pendingDir), false);
  const failure = store.lastFailure(sessionId);
  assert.equal(failure?.status, 422);
  assert.equal(failure?.reason, "http-422");
  assert.equal(store.get(sessionId).last_message_id, null);
});

test("a 2xx acceptance writes nothing to the outbox and advances the checkpoint", async () => {
  const stateDir = createTempStateDir();
  const config = testConfig(stateDir);
  const recording = createRecordingClient([]);
  const store = createCheckpointStore(checkpointPaths(stateDir));
  const adapter = createAdapter({
    config,
    source: source(),
    client: recording.client,
    checkpoints: store,
    directory: "C:/synthetic/projects/outbox",
    log: createRecordingLog().log,
    now: fixedNow,
  });

  await adapter.reconcileSession(sessionId);

  assert.equal(existsSync(config.pendingDir), false);
  assert.equal(store.get(sessionId).last_message_id, "msg-0001");
});

test("a missing discovery falls back to the outbox", async () => {
  const stateDir = createTempStateDir();
  const config = testConfig(stateDir, {
    discoveryPath: join(stateDir, "missing", "discovery.json"),
  });
  const adapter = createAdapter({
    config,
    source: source(),
    client: createCaptureClient({
      resolveEndpoint: createLocalApiEndpointResolver(config),
      timeoutMs: 500,
    }),
    checkpoints: createCheckpointStore(checkpointPaths(stateDir)),
    directory: "C:/synthetic/projects/outbox",
    log: createRecordingLog().log,
    now: fixedNow,
  });

  await adapter.reconcileSession(sessionId);

  const files = pendingFiles(config.pendingDir);
  assert.equal(files.length, 1);
  assert.match(files[0], /^[0-9a-f]{64}\.json$/);
});

test("an outbox filesystem failure is recorded without throwing", async () => {
  const root = mkdtempSync(join(tmpdir(), "xemnas-outbox-fail-"));
  const blocker = join(root, "blocker");
  writeFileSync(blocker, "not a directory", "utf8");
  const config = testConfig(join(root, "state"), {
    outboxDir: blocker,
    pendingDir: join(blocker, "pending"),
  });
  const recording = createRecordingClient([
    { kind: "unavailable", reason: "connection" },
  ]);
  const store = createCheckpointStore(checkpointPaths(config.stateDir));
  const adapter = createAdapter({
    config,
    source: source(),
    client: recording.client,
    checkpoints: store,
    directory: "C:/synthetic/projects/outbox",
    log: createRecordingLog().log,
    now: fixedNow,
  });

  await assert.doesNotReject(() => adapter.reconcileSession(sessionId));

  const failure = store.lastFailure(sessionId);
  assert.equal(failure?.reason, "outbox-write");
  assert.equal(failure?.status, null);
  assert.equal(store.get(sessionId).last_message_id, null);
});
