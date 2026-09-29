//! Outbox writer unit tests: atomic `pending/` files, overwrite semantics and
//! non-throwing filesystem failures.

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  mkdtempSync,
  readdirSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import type { CaptureEnvelope } from "../src/envelope.js";
import {
  createOutboxWriter,
  pendingFileName,
  type OutboxWriteResult,
} from "../src/outbox.js";

function sampleEnvelope(idempotencyKey: string): CaptureEnvelope {
  return {
    schema_version: 1,
    capture_id: "018f2d3c-4b5a-7c6d-8e9f-000000000001",
    idempotency_key: idempotencyKey,
    source: {
      adapter: "opencode",
      adapter_version: "0.0.0",
      session_id: "session-1",
      message_id: "msg-0001",
    },
    project: { canonical_path: "C:/synthetic/projects/p" },
    observed_at: "2026-09-28T12:00:00.000Z",
    artifacts: [
      {
        artifact_id: "018f2d3c-4b5a-7c6d-8e9f-000000000101",
        kind: "user_text",
        content: "synthetic",
        metadata: {},
        fingerprint: "a".repeat(64),
      },
    ],
  };
}

function tempDir(): string {
  return mkdtempSync(join(tmpdir(), "xemnas-outbox-test-"));
}

test("pendingFileName is the lowercase sha256 of the idempotency key", () => {
  const key = "opencode:session-1:msg-0001:deadbeef";
  const expected = `${createHash("sha256").update(key, "utf8").digest("hex")}.json`;
  assert.equal(pendingFileName(key), expected);
  assert.match(pendingFileName(key), /^[0-9a-f]{64}\.json$/);
});

test("writePending creates pending/<sha256>.json with the full envelope", () => {
  const pendingDir = join(tempDir(), "outbox", "pending");
  const writer = createOutboxWriter({ pendingDir });
  const envelope = sampleEnvelope("opencode:s:m:hash");

  const result = writer.writePending(envelope);
  assert.equal(result.ok, true);

  const files = readdirSync(pendingDir);
  assert.deepEqual(files, [pendingFileName(envelope.idempotency_key)]);
  const parsed = JSON.parse(readFileSync(join(pendingDir, files[0]), "utf8"));
  assert.deepEqual(parsed, envelope);
});

test("writePending overwrites the same key and never leaves a .tmp file", () => {
  const pendingDir = join(tempDir(), "outbox", "pending");
  const writer = createOutboxWriter({ pendingDir });

  writer.writePending(sampleEnvelope("opencode:s:m:same"));
  const updated = sampleEnvelope("opencode:s:m:same");
  updated.capture_id = "018f2d3c-4b5a-7c6d-8e9f-000000000002";
  writer.writePending(updated);

  const files = readdirSync(pendingDir);
  assert.equal(files.length, 1);
  assert.ok(!files.some((name) => name.endsWith(".tmp")));
  assert.deepEqual(
    JSON.parse(readFileSync(join(pendingDir, files[0]), "utf8")),
    updated,
  );
});

test("writePending reports a filesystem failure without throwing", () => {
  const root = tempDir();
  // `blocker` is a regular file, so creating `blocker/pending` fails.
  const blocker = join(root, "blocker");
  writeFileSync(blocker, "not a directory", "utf8");
  const writer = createOutboxWriter({ pendingDir: join(blocker, "pending") });

  let result: OutboxWriteResult | undefined;
  assert.doesNotThrow(() => {
    result = writer.writePending(sampleEnvelope("opencode:s:m:fs"));
  });
  assert.equal(result?.ok, false);
  if (result?.ok === false) {
    assert.ok(result.reason.length > 0);
  }
});
