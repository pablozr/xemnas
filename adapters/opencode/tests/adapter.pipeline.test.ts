//! End-to-end (hermetic) pipeline tests: reconcile → redact → envelope → POST.
//!
//! Every dependency is injected, so these tests prove the online path without a
//! real OpenCode server, local API or network. Fixtures use the **documented**
//! OpenCode payload shapes (`{ info, parts }` messages, `{ file, before, after }`
//! diffs) so the fake cannot mask a parser regression.

import assert from "node:assert/strict";
import { test } from "node:test";

import { createCheckpointStore, checkpointPaths } from "../src/checkpoint.js";
import type { CaptureClient, PostResult } from "../src/client.js";
import type { AdapterConfig } from "../src/config.js";
import { sha256Hex, validateEnvelope } from "../src/envelope.js";
import { createAdapter, type OpenCodeEvent } from "../src/index.js";
import {
  createFakeMessageSource,
  type RawFileDiff,
} from "../src/opencode.js";
import {
  createManualScheduler,
  createRecordingClient,
  createRecordingLog,
  createTempStateDir,
  delay,
  rawDiff,
  rawMessage,
  testConfig,
} from "./test-helpers.js";

const sessionId = "session-pipeline-1";
const directory = "C:/synthetic/projects/pipeline";

function textPart(messageId: string, text: string): unknown {
  return { id: `${messageId}-t`, sessionID: sessionId, messageID: messageId, type: "text", text };
}

function reasoningPart(messageId: string, text: string): unknown {
  return {
    id: `${messageId}-r`,
    sessionID: sessionId,
    messageID: messageId,
    type: "reasoning",
    text,
  };
}

function toolPart(messageId: string, tool: string, status: string): unknown {
  return {
    id: `${messageId}-tool`,
    sessionID: sessionId,
    messageID: messageId,
    type: "tool",
    callID: `${messageId}-call`,
    tool,
    state: { status },
  };
}

const user1 = rawMessage(
  "msg-0001",
  "user",
  [textPart("msg-0001", "Refactor the parser.")],
  sessionId,
);
const assistant1 = rawMessage(
  "msg-0002",
  "assistant",
  [
    textPart("msg-0002", "Done."),
    reasoningPart("msg-0002", "private chain of thought"),
    toolPart("msg-0002", "edit", "completed"),
  ],
  sessionId,
);
const user2 = rawMessage(
  "msg-0003",
  "user",
  [textPart("msg-0003", "Add tests.")],
  sessionId,
);
const assistant2 = rawMessage(
  "msg-0004",
  "assistant",
  [textPart("msg-0004", "Added.")],
  sessionId,
);

function diffKey(messageId: string): string {
  return `${sessionId}:${messageId}`;
}

interface HarnessOptions {
  messages: ReturnType<typeof rawMessage>[];
  diffs?: Record<string, RawFileDiff[]>;
  results?: PostResult[];
  overrides?: Partial<AdapterConfig>;
  directory?: string;
}

function buildHarness(options: HarnessOptions) {
  const stateDir = createTempStateDir();
  const source = createFakeMessageSource({
    messages: { [sessionId]: options.messages },
    diffs: options.diffs,
  });
  const recording = createRecordingClient(options.results ?? []);
  const store = createCheckpointStore(checkpointPaths(stateDir));
  const { log, lines } = createRecordingLog();
  const adapter = createAdapter({
    config: testConfig(stateDir, options.overrides),
    source,
    client: recording.client,
    checkpoints: store,
    directory: options.directory ?? directory,
    log,
    now: () => new Date("2026-09-28T12:00:00Z"),
  });
  return { adapter, store, seen: recording.seen, lines, stateDir, source };
}

test("a first reconciliation sends every new turn and advances the checkpoint", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1, user2, assistant2],
    diffs: {
      [diffKey("msg-0001")]: [
        rawDiff("src/parser.ts", "const a = 1;\n", "const a = 2;\n"),
      ],
      [diffKey("msg-0003")]: [
        rawDiff("src/parser.test.ts", "assert(1);\n", "assert(2);\n"),
      ],
    },
  });
  const outcome = await harness.adapter.reconcileSession(sessionId);

  assert.equal(outcome.sent, 2);
  assert.equal(outcome.stopped, false);
  assert.equal(harness.seen.length, 2);
  assert.equal(harness.seen[0].source.message_id, "msg-0001");
  assert.equal(harness.seen[1].source.message_id, "msg-0003");
  assert.equal(harness.store.get(sessionId).last_message_id, "msg-0003");
  assert.equal(
    harness.store.get(sessionId).last_capture_id,
    harness.seen[1].capture_id,
  );
});

test("the newest turn waits until its answer is finished", async () => {
  const writing = rawMessage("msg-0004", "assistant", [textPart("msg-0004", "Wri")], sessionId);
  writing.info.time = { created: 2 };
  const harness = buildHarness({
    messages: [user1, assistant1, user2, writing],
  });
  const outcome = await harness.adapter.reconcileSession(sessionId);

  assert.equal(outcome.sent, 1, "only the finished turn is sent");
  assert.equal(harness.seen[0].source.message_id, "msg-0001");
  assert.equal(
    harness.store.get(sessionId).last_message_id,
    "msg-0001",
    "the checkpoint stays before the unfinished turn",
  );
});

test("a second reconciliation with an up-to-date checkpoint sends nothing", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1, user2, assistant2],
  });
  await harness.adapter.reconcileSession(sessionId);
  const sentAfterFirst = harness.seen.length;

  const outcome = await harness.adapter.reconcileSession(sessionId);
  assert.equal(outcome.sent, 0);
  assert.equal(harness.seen.length, sentAfterFirst);
});

test("the checkpoint advances only after a 2xx response", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    results: [{ kind: "unavailable", reason: "connection" }],
  });
  const outcome = await harness.adapter.reconcileSession(sessionId);

  assert.equal(outcome.stopped, true);
  assert.equal(harness.store.get(sessionId).last_message_id, null);
});

test("an unavailable app neither advances the checkpoint nor throws", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    results: [{ kind: "unavailable", reason: "timeout" }],
  });
  const outcome = await harness.adapter.reconcileSession(sessionId);
  assert.equal(outcome.stopped, true);
  assert.equal(harness.store.get(sessionId).last_message_id, null);
  assert.equal(harness.store.lastFailure(sessionId), null);
});

test("a rejected POST records a sanitized failure and does not advance", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    results: [{ kind: "rejected", status: 422 }],
  });
  const outcome = await harness.adapter.reconcileSession(sessionId);

  assert.equal(outcome.stopped, true);
  assert.equal(harness.store.get(sessionId).last_message_id, null);
  const failure = harness.store.lastFailure(sessionId);
  assert.equal(failure?.status, 422);
  assert.equal(failure?.message_id, "msg-0001");
});

test("a failed POST keeps the checkpoint and the retry reuses the idempotency key", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    diffs: {
      [diffKey("msg-0001")]: [rawDiff("src/parser.ts", "const a = 1;\n", "const a = 2;\n")],
    },
    results: [
      { kind: "unavailable", reason: "connection" },
      { kind: "accepted", status: 201, captureId: "cap-retry" },
    ],
  });

  await harness.adapter.reconcileSession(sessionId);
  assert.equal(harness.store.get(sessionId).last_message_id, null);

  await harness.adapter.reconcileSession(sessionId);
  assert.equal(harness.seen.length, 2);
  assert.equal(
    harness.seen[0].idempotency_key,
    harness.seen[1].idempotency_key,
    "the retry must reuse the same Idempotency-Key",
  );
  assert.equal(harness.store.get(sessionId).last_message_id, "msg-0001");
});

test("an idempotent replay records the server-returned capture id", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    results: [{ kind: "accepted", status: 200, captureId: "server-cap-1" }],
  });
  await harness.adapter.reconcileSession(sessionId);

  assert.notEqual(harness.seen[0].capture_id, "server-cap-1");
  assert.equal(harness.store.get(sessionId).last_capture_id, "server-cap-1");
});

test("the checkpoint never moves backwards", () => {
  const stateDir = createTempStateDir();
  const store = createCheckpointStore(checkpointPaths(stateDir));
  store.advance(sessionId, "msg-0003", "cap-3");
  store.advance(sessionId, "msg-0001", "cap-1");
  assert.equal(store.get(sessionId).last_message_id, "msg-0003");
  assert.equal(store.get(sessionId).last_capture_id, "cap-3");
  store.advance(sessionId, "msg-0005", "cap-5");
  assert.equal(store.get(sessionId).last_message_id, "msg-0005");
});

test("concurrent idle signals coalesce and never regress the checkpoint", async () => {
  const stateDir = createTempStateDir();
  const source = createFakeMessageSource({
    messages: { [sessionId]: [user1, assistant1] },
  });
  let resolvePost: (result: PostResult) => void = () => {};
  let posts = 0;
  const client: CaptureClient = {
    post(): Promise<PostResult> {
      posts += 1;
      return new Promise<PostResult>((resolve) => {
        resolvePost = resolve;
      });
    },
  };
  const store = createCheckpointStore(checkpointPaths(stateDir));
  const scheduler = createManualScheduler();
  const adapter = createAdapter({
    config: testConfig(stateDir),
    source,
    client,
    checkpoints: store,
    directory,
    log: createRecordingLog().log,
    now: () => new Date("2026-09-28T12:00:00Z"),
    schedule: scheduler.schedule,
  });
  const idle: OpenCodeEvent = {
    type: "session.status",
    properties: { sessionID: sessionId, status: { type: "idle" } },
  };

  adapter.handleEvent(idle);
  adapter.handleEvent(idle);
  assert.equal(scheduler.tasks.length, 1, "debounce collapses the two idles");
  scheduler.runAll();
  await delay(0);
  assert.equal(posts, 1);

  adapter.handleEvent(idle);
  scheduler.runAll();
  await delay(0);
  assert.equal(posts, 1, "an idle during execution must not start a second POST");

  resolvePost({ kind: "accepted", status: 201, captureId: "cap-1" });
  await adapter.whenIdle(sessionId);

  assert.equal(posts, 1);
  assert.equal(store.get(sessionId).last_message_id, "msg-0001");
  assert.equal(store.get(sessionId).last_capture_id, "cap-1");
});

test("the built envelope validates against the versioned schema", async () => {
  const harness = buildHarness({ messages: [user1, assistant1] });
  await harness.adapter.reconcileSession(sessionId);
  const result = validateEnvelope(harness.seen[0]);
  assert.equal(result.valid, true, `errorCount=${result.errorCount}`);
});

test("artifact fingerprints are the SHA-256 of their content", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    diffs: {
      [diffKey("msg-0001")]: [rawDiff("src/parser.ts", "const a = 1;\n", "const a = 2;\n")],
    },
  });
  await harness.adapter.reconcileSession(sessionId);
  const artifacts = harness.seen[0].artifacts;
  assert.ok(artifacts.length >= 1);
  for (const artifact of artifacts) {
    assert.equal(artifact.fingerprint, sha256Hex(artifact.content));
    assert.match(artifact.fingerprint, /^[0-9a-f]{64}$/);
  }
});

test("the idempotency key follows the documented format", async () => {
  const harness = buildHarness({ messages: [user1, assistant1] });
  await harness.adapter.reconcileSession(sessionId);
  assert.match(
    harness.seen[0].idempotency_key,
    /^opencode:session-pipeline-1:msg-0001:[0-9a-f]{64}$/,
  );
});

test("the project canonical path is the plugin directory", async () => {
  const harness = buildHarness({
    messages: [user1, assistant1],
    directory: "C:/synthetic/projects/canonical",
  });
  await harness.adapter.reconcileSession(sessionId);
  assert.equal(
    harness.seen[0].project.canonical_path,
    "C:/synthetic/projects/canonical",
  );
});

test("reasoning parts are omitted and secrets are redacted", async () => {
  const secret = "sk-abcdefghijklmnopqrstuvwx";
  const harness = buildHarness({
    messages: [
      rawMessage(
        "msg-0001",
        "user",
        [textPart("msg-0001", `Use token ${secret} please`)],
        sessionId,
      ),
      rawMessage(
        "msg-0002",
        "assistant",
        [
          textPart("msg-0002", "Understood."),
          reasoningPart("msg-0002", `hidden ${secret} chain`),
        ],
        sessionId,
      ),
    ],
  });
  await harness.adapter.reconcileSession(sessionId);

  const contents = harness.seen[0].artifacts.map((artifact) => artifact.content);
  for (const content of contents) {
    assert.ok(!content.includes(secret), "the secret must not be captured");
    assert.ok(!content.includes("hidden"), "reasoning must be omitted");
  }
  assert.ok(contents.some((content) => content.includes("[REDACTED]")));
  assert.equal(validateEnvelope(harness.seen[0]).valid, true);
});

test("an oversized diff is bounded and reduced to hunks", async () => {
  const maxDiffBytes = 256;
  const beforeLines = Array.from({ length: 200 }, (_, i) => `line ${i}`);
  const afterLines = [...beforeLines];
  afterLines[100] = "LINE 100 CHANGED";
  const harness = buildHarness({
    messages: [user1, assistant1],
    diffs: {
      [diffKey("msg-0001")]: [
        rawDiff("src/huge.ts", beforeLines.join("\n"), afterLines.join("\n")),
      ],
    },
    overrides: { maxDiffBytes },
  });
  await harness.adapter.reconcileSession(sessionId);

  const diffs = harness.seen[0].artifacts.filter(
    (artifact) => artifact.kind === "diff_hunk",
  );
  assert.ok(diffs.length >= 1, "the diff must survive as a reduced hunk");
  for (const diff of diffs) {
    assert.ok(Buffer.byteLength(diff.content, "utf8") <= maxDiffBytes);
    assert.ok(diff.content.includes("@@"), "hunks carry a range header");
  }
  assert.equal(validateEnvelope(harness.seen[0]).valid, true);
});

test("a turn with no capturable artifacts is not sent", async () => {
  const harness = buildHarness({
    messages: [
      rawMessage(
        "msg-0001",
        "user",
        [reasoningPart("msg-0001", "only private reasoning")],
        sessionId,
      ),
    ],
  });
  const outcome = await harness.adapter.reconcileSession(sessionId);

  assert.equal(harness.seen.length, 0);
  assert.equal(outcome.skipped, 1);
  assert.equal(harness.store.get(sessionId).last_message_id, "msg-0001");
});

test("sensitive content never reaches the log", async () => {
  const secret = "sk-abcdefghijklmnopqrstuvwx";
  const harness = buildHarness({
    messages: [
      rawMessage(
        "msg-0001",
        "user",
        [textPart("msg-0001", `Refactor ${secret} now`)],
        sessionId,
      ),
    ],
  });
  await harness.adapter.reconcileSession(sessionId);

  const rendered = harness.lines.join("\n");
  assert.ok(!rendered.includes(secret));
  assert.ok(!rendered.includes("Refactor"));
  assert.ok(rendered.includes("capture-accepted"));
});
