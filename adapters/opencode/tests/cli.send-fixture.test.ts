//! Driver tests for `src/cli/send-fixture.ts`.
//!
//! Hermetic: the fixture server and the fake capture server both bind loopback
//! on port 0; no external network is used.

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { once } from "node:events";
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { packageRoot } from "../src/config.js";
import { runSendFixture } from "../src/cli/send-fixture.js";
import { createRecordingLog } from "./test-helpers.js";

const sessionId = "session-cli-1";

function tempDir(tag: string): string {
  return mkdtempSync(join(tmpdir(), `xemnas-${tag}-`));
}

function textPart(messageId: string, text: string): unknown {
  return {
    id: `${messageId}-t`,
    sessionID: sessionId,
    messageID: messageId,
    type: "text",
    text,
  };
}

function message(
  id: string,
  role: string,
  parts: unknown[],
): Record<string, unknown> {
  // Assistant messages are finished, as OpenCode reports after the turn.
  const time = role === "assistant" ? { created: 1, completed: 2 } : { created: 1 };
  return { info: { id, role, sessionID: sessionId, time }, parts };
}

function writeFixture(
  dir: string,
  pages: unknown[][],
  diffs: Record<string, unknown[]> = {},
): void {
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, "session.json"),
    JSON.stringify({
      session_id: sessionId,
      directory: "C:/synthetic/cli",
      pages,
      diffs,
    }),
    "utf8",
  );
}

function writeDiscovery(runtimeDir: string, port: number): string {
  mkdirSync(runtimeDir, { recursive: true });
  const discoveryPath = join(runtimeDir, "discovery.json");
  writeFileSync(
    discoveryPath,
    JSON.stringify({ protocol_version: 1, port, instance_id: "instance-1" }),
    "utf8",
  );
  writeFileSync(
    join(runtimeDir, "api-token"),
    "test-token-0123456789abcdef",
    "utf8",
  );
  return discoveryPath;
}

interface CaptureServer {
  port: number;
  stats: { posts: number };
  close: () => Promise<void>;
}

async function startCaptureServer(status: number, captureId: string): Promise<CaptureServer> {
  const stats = { posts: 0 };
  const server = createServer((request, response) => {
    if (request.method !== "POST" || request.url !== "/v1/captures") {
      response.writeHead(404);
      response.end();
      return;
    }
    let body = "";
    request.on("data", (chunk) => {
      body += chunk;
    });
    request.on("end", () => {
      stats.posts += 1;
      const envelope = JSON.parse(body) as {
        idempotency_key: string;
        artifacts: unknown[];
      };
      response.writeHead(status, { "content-type": "application/json" });
      if (status >= 200 && status < 300) {
        response.end(
          JSON.stringify({
            capture_id: captureId,
            idempotency_key: envelope.idempotency_key,
            received_at: "2026-09-28T12:00:00Z",
            artifact_count: envelope.artifacts.length,
          }),
        );
      } else {
        response.end(JSON.stringify({ code: "ERR", message: "rejected" }));
      }
    });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  const port =
    typeof address === "object" && address !== null ? address.port : 0;
  return {
    port,
    stats,
    close: () =>
      new Promise((done) => {
        server.closeAllConnections?.();
        server.close(() => done());
      }),
  };
}

test("the shipped E2E fixture parses and carries a synthetic secret", () => {
  const path = join(packageRoot(), "tests", "fixtures", "e2e", "session.json");
  const text = readFileSync(path, "utf8");
  const parsed = JSON.parse(text) as {
    session_id: string;
    pages: unknown[][];
  };
  assert.equal(parsed.session_id, "session-e2e-1");
  assert.ok(Array.isArray(parsed.pages) && parsed.pages.length >= 1);
  assert.ok(text.includes("sk-test-"));
});

test("without discovery the driver reports outbox and redacts the fixture", async () => {
  const root = tempDir("cli-outbox");
  const summary = await runSendFixture({
    fixtureDir: join(packageRoot(), "tests", "fixtures", "e2e"),
    stateDir: join(root, "state"),
    outboxDir: join(root, "outbox"),
    env: {},
    log: createRecordingLog().log,
  });

  assert.equal(summary.outcome, "outbox");
  assert.equal(summary.detail, "endpoint-unresolved");
  assert.ok(summary.outbox_path !== null);
  assert.equal(existsSync(summary.outbox_path), true);
  assert.ok(summary.artifact_count >= 1);

  const persisted = readFileSync(summary.outbox_path, "utf8");
  assert.ok(!persisted.includes("sk-test-"), "the secret must be redacted");
  assert.ok(!persisted.includes("Private chain of thought"));
  assert.ok(persisted.includes("[REDACTED]"));
});

test("with discovery the driver posts each reconciled turn and reports sent", async () => {
  const root = tempDir("cli-sent");
  const fixtureDir = join(root, "fixture");
  writeFixture(fixtureDir, [
    [
      message("msg-0003", "user", [textPart("msg-0003", "three")]),
      message("msg-0004", "assistant", [textPart("msg-0004", "four")]),
    ],
    [
      message("msg-0001", "user", [textPart("msg-0001", "one")]),
      message("msg-0002", "assistant", [textPart("msg-0002", "two")]),
    ],
  ]);
  const capture = await startCaptureServer(201, "cap-cli-1");
  try {
    const summary = await runSendFixture({
      fixtureDir,
      discoveryPath: writeDiscovery(join(root, "runtime"), capture.port),
      stateDir: join(root, "state"),
      outboxDir: join(root, "outbox"),
      env: {},
      log: createRecordingLog().log,
    });

    assert.equal(summary.outcome, "sent");
    assert.equal(summary.capture_id, "cap-cli-1");
    assert.equal(capture.stats.posts, 2, "both turns must be reconciled");

    const checkpoints = JSON.parse(
      readFileSync(join(root, "state", "checkpoints.json"), "utf8"),
    ) as Record<string, { last_message_id: string; last_capture_id: string }>;
    assert.equal(checkpoints[sessionId].last_message_id, "msg-0003");
    assert.equal(checkpoints[sessionId].last_capture_id, "cap-cli-1");
    assert.equal(existsSync(join(root, "outbox", "pending")), false);
  } finally {
    await capture.close();
  }
});

test("a 4xx response reports rejected_4xx without touching the outbox", async () => {
  const root = tempDir("cli-rejected");
  const fixtureDir = join(root, "fixture");
  writeFixture(fixtureDir, [
    [
      message("msg-0001", "user", [textPart("msg-0001", "one")]),
      message("msg-0002", "assistant", [textPart("msg-0002", "two")]),
    ],
  ]);
  const capture = await startCaptureServer(422, "unused");
  try {
    const summary = await runSendFixture({
      fixtureDir,
      discoveryPath: writeDiscovery(join(root, "runtime"), capture.port),
      stateDir: join(root, "state"),
      outboxDir: join(root, "outbox"),
      env: {},
      log: createRecordingLog().log,
    });

    assert.equal(summary.outcome, "rejected_4xx");
    assert.equal(summary.detail, "http-422");
    assert.equal(existsSync(join(root, "outbox", "pending")), false);
    const failures = JSON.parse(
      readFileSync(join(root, "state", "failures.json"), "utf8"),
    ) as Record<string, { status: number; reason: string }>;
    assert.equal(failures[sessionId].status, 422);
    assert.equal(failures[sessionId].reason, "http-422");
  } finally {
    await capture.close();
  }
});

test("a failed outbox write is reported as failed, never as outbox", async () => {
  const root = tempDir("cli-outbox-fail");
  // `blocker` is a regular file, so `blocker/pending` cannot be created.
  const blocker = join(root, "blocker");
  writeFileSync(blocker, "not a directory", "utf8");

  const summary = await runSendFixture({
    fixtureDir: join(packageRoot(), "tests", "fixtures", "e2e"),
    stateDir: join(root, "state"),
    outboxDir: blocker,
    env: {},
    log: createRecordingLog().log,
  });

  assert.equal(summary.outcome, "failed");
  assert.equal(summary.outbox_path, null);
  assert.ok(summary.error !== null && summary.error.length > 0);
  assert.ok(summary.idempotency_key !== null);
  assert.equal(summary.artifact_count >= 1, true);
});

test("the CLI exits non-zero when the outbox cannot be persisted", () => {
  const root = tempDir("cli-main-fail");
  const blocker = join(root, "blocker");
  writeFileSync(blocker, "not a directory", "utf8");
  const packageDir = packageRoot();
  const script = join(packageDir, "dist", "src", "cli", "send-fixture.js");

  const run = spawnSync(
    process.execPath,
    [
      script,
      "--fixture",
      join(packageDir, "tests", "fixtures", "e2e"),
      "--state-dir",
      join(root, "state"),
      "--outbox-dir",
      blocker,
    ],
    { cwd: packageDir, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
  );

  assert.notEqual(run.status, 0);
  const summary = JSON.parse(run.stdout) as {
    outcome: string;
    outbox_path: string | null;
  };
  assert.equal(summary.outcome, "failed");
  assert.equal(summary.outbox_path, null);
});
