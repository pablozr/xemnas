//! Trigger and fast-return tests for the OpenCode adapter.
//!
//! These prove the plugin reacts only to idle signals and that the event hook
//! returns before any network work finishes (MVP-SPEC §7.1: "o adapter retorna
//! rapidamente e nunca executa modelo").

import assert from "node:assert/strict";
import { test } from "node:test";

import { createCheckpointStore, checkpointPaths } from "../src/checkpoint.js";
import type { CaptureClient, PostResult } from "../src/client.js";
import {
  createAdapter,
  createOpenCodePlugin,
  idleSessionId,
  type OpenCodeEvent,
} from "../src/index.js";
import { createFakeMessageSource } from "../src/opencode.js";
import {
  createManualScheduler,
  createRecordingClient,
  createRecordingLog,
  createTempStateDir,
  delay,
  rawMessage,
  testConfig,
} from "./test-helpers.js";

const sessionId = "session-trigger-1";

const idleStatus: OpenCodeEvent = {
  type: "session.status",
  properties: { sessionID: sessionId, status: { type: "idle" } },
};
const busyStatus: OpenCodeEvent = {
  type: "session.status",
  properties: { sessionID: sessionId, status: { type: "busy" } },
};
const idleCompat: OpenCodeEvent = {
  type: "session.idle",
  properties: { sessionID: sessionId },
};

function sampleSource() {
  const user = rawMessage(
    "msg-0001",
    "user",
    [
      {
        id: "msg-0001-t",
        sessionID: sessionId,
        messageID: "msg-0001",
        type: "text",
        text: "hello",
      },
    ],
    sessionId,
  );
  return createFakeMessageSource({ messages: { [sessionId]: [user] } });
}

function buildAdapter() {
  const stateDir = createTempStateDir();
  const scheduler = createManualScheduler();
  const { log } = createRecordingLog();
  const { client } = createRecordingClient([]);
  const adapter = createAdapter({
    config: testConfig(stateDir, { debounceMs: 250 }),
    source: sampleSource(),
    client,
    checkpoints: createCheckpointStore(checkpointPaths(stateDir)),
    directory: "C:/synthetic/projects/trigger",
    log,
    now: () => new Date("2026-09-28T12:00:00Z"),
    schedule: scheduler.schedule,
  });
  return { adapter, scheduler };
}

test("session.status with status idle schedules one reconciliation", () => {
  const { adapter, scheduler } = buildAdapter();
  adapter.handleEvent(idleStatus);
  assert.equal(scheduler.tasks.length, 1);
  assert.equal(scheduler.tasks[0].delayMs, 250);
});

test("session.status with a non-idle status does not schedule", () => {
  const { adapter, scheduler } = buildAdapter();
  adapter.handleEvent(busyStatus);
  assert.equal(scheduler.tasks.length, 0);
});

test("the deprecated session.idle event still schedules", () => {
  const { adapter, scheduler } = buildAdapter();
  adapter.handleEvent(idleCompat);
  assert.equal(scheduler.tasks.length, 1);
});

test("unrelated or malformed events are ignored", () => {
  const { adapter, scheduler } = buildAdapter();
  adapter.handleEvent({ type: "message.updated", properties: { sessionID: sessionId } });
  adapter.handleEvent({ type: "session.status" });
  adapter.handleEvent({ type: "session.status", properties: {} });
  assert.equal(scheduler.tasks.length, 0);
});

test("idleSessionId accepts only idle status and session.idle", () => {
  assert.equal(idleSessionId(idleStatus), sessionId);
  assert.equal(idleSessionId(idleCompat), sessionId);
  assert.equal(idleSessionId(busyStatus), null);
  assert.equal(
    idleSessionId({ type: "session.status", properties: { sessionID: 1 } }),
    null,
  );
});

test("the plugin event hook resolves while the capture POST is still pending", async () => {
  const stateDir = createTempStateDir();
  let postStarted = false;
  let postSettled = false;
  let resolvePost: (result: PostResult) => void = () => {};
  const client: CaptureClient = {
    post(): Promise<PostResult> {
      postStarted = true;
      return new Promise<PostResult>((resolve) => {
        resolvePost = resolve;
      }).finally(() => {
        postSettled = true;
      });
    },
  };

  const plugin = createOpenCodePlugin((directory) =>
    createAdapter({
      config: testConfig(stateDir, { debounceMs: 0 }),
      source: sampleSource(),
      client,
      checkpoints: createCheckpointStore(checkpointPaths(stateDir)),
      directory,
      log: createRecordingLog().log,
      now: () => new Date("2026-09-28T12:00:00Z"),
    }),
  );

  const hooks = await plugin({ directory: "C:/synthetic/projects/trigger" });
  const outcome = await Promise.race([
    Promise.resolve(hooks.event?.({ event: idleStatus })).then(() => "resolved"),
    delay(500).then(() => "timeout"),
  ]);
  assert.equal(outcome, "resolved", "the event hook must not await the POST");

  await delay(20);
  assert.equal(postStarted, true, "the debounced POST must start in the background");
  assert.equal(postSettled, false, "the POST is still pending while the hook resolved");

  resolvePost({ kind: "accepted", status: 201, captureId: "cap-hook" });
  await delay(20);
  assert.equal(postSettled, true);
});
