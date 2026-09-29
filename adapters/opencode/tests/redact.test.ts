//! Redaction and UTF-8 bounding unit tests.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  boundContent,
  buildDiffHunks,
  collectText,
  omitReasoningParts,
  redactSecrets,
  reduceDiffs,
  truncateUtf8,
} from "../src/redact.js";

test("redactSecrets masks sk- and ghp_ key prefixes", () => {
  const redacted = redactSecrets(
    "use sk-abcdefghijklmnopqrstuvwx and ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 now",
  );
  assert.ok(!redacted.includes("sk-abcdefghijklmnopqrstuvwx"));
  assert.ok(!redacted.includes("ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"));
  assert.equal(redacted.match(/\[REDACTED\]/g)?.length, 2);
});

test("redactSecrets masks PEM private key blocks", () => {
  const pem = [
    "-----BEGIN PRIVATE KEY-----",
    "MIIEvQIBADANBgkqhkiG9w0BAQEFAASC",
    "-----END PRIVATE KEY-----",
  ].join("\n");
  assert.equal(redactSecrets(`before\n${pem}\nafter`), "before\n[REDACTED]\nafter");
});

test("redactSecrets masks credential assignment lines", () => {
  const redacted = redactSecrets(
    ["TOKEN=abc123def456", "API_KEY: sk-zzzzzzzzzzzz", "note = keep me"].join(
      "\n",
    ),
  );
  assert.ok(redacted.includes("TOKEN=[REDACTED]"));
  assert.ok(redacted.includes("API_KEY: [REDACTED]"));
  assert.ok(redacted.includes("note = keep me"));
});

test("redactSecrets leaves ordinary text unchanged", () => {
  const text = "Refactor the parser and update the call sites.";
  assert.equal(redactSecrets(text), text);
});

test("truncateUtf8 fits the byte budget without splitting code points", () => {
  const value = "a\u{1F600}b"; // "a" + emoji (4 bytes) + "b"
  assert.equal(truncateUtf8(value, 100), value);
  assert.equal(truncateUtf8(value, 5), "a\u{1F600}");
  assert.equal(truncateUtf8(value, 4), "a");
  assert.equal(truncateUtf8(value, 0), "");
  assert.ok(Buffer.byteLength(truncateUtf8(value, 3), "utf8") <= 3);
});

test("boundContent redacts before truncating", () => {
  const secret = "sk-abcdefghijklmnopqrstuvwx";
  const bounded = boundContent(`token ${secret} end`, 12);
  assert.ok(bounded.length <= 12);
  assert.ok(!bounded.includes(secret));
});

test("omitReasoningParts and collectText drop reasoning content", () => {
  const parts = [
    { type: "reasoning", text: "private chain of thought" },
    { type: "text", text: "visible answer" },
    { type: "thinking", text: "more private thought" },
  ];
  const kept = omitReasoningParts(parts);
  assert.deepEqual(
    kept.map((part) => part.type),
    ["text"],
  );
  assert.equal(collectText(kept), "visible answer");
});

test("buildDiffHunks emits unified hunks from before/after line snapshots", () => {
  const hunks = buildDiffHunks("a\nb\nc\n", "a\nB\nc\n");
  assert.ok(hunks.startsWith("@@ "));
  assert.ok(hunks.includes("-b"));
  assert.ok(hunks.includes("+B"));
  assert.equal(buildDiffHunks("same\n", "same\n"), "");
});

test("reduceDiffs bounds the generated hunks", () => {
  const before = Array.from({ length: 200 }, (_, i) => `line ${i}`).join("\n");
  const after = before.replace("line 100", "LINE 100");
  const reduced = reduceDiffs([{ file: "x.ts", before, after }], 128);
  assert.equal(reduced.length, 1);
  assert.ok(Buffer.byteLength(reduced[0].content, "utf8") <= 128);
  assert.ok(reduced[0].content.includes("@@"));
});
