//! Parser and pagination tests against the documented OpenCode shapes.
//!
//! These feed literal `{ info, parts }` messages and `{ file, before, after }`
//! diffs (as documented at <https://opencode.ai/docs/server/>) into the HTTP
//! source, and drive its `Link: rel="next"` pagination with a stub fetch.

import assert from "node:assert/strict";
import { test } from "node:test";

import {
  createHttpMessageSource,
  parseFileDiff,
  parseMessageWithParts,
} from "../src/opencode.js";
import { rawDiff, rawMessage } from "./test-helpers.js";

const sessionId = "session-parser-1";
const baseUrl = "http://127.0.0.1:4096";

function textPart(messageId: string, text: string): unknown {
  return {
    id: `${messageId}-text`,
    sessionID: sessionId,
    messageID: messageId,
    type: "text",
    text,
  };
}

function reasoningPart(messageId: string, text: string): unknown {
  return {
    id: `${messageId}-reasoning`,
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
    state: { status, input: {}, output: "tool output", title: "t" },
  };
}

test("parseMessageWithParts reads id/role from info and parts from the item", () => {
  const message = parseMessageWithParts(
    rawMessage(
      "msg-0001",
      "assistant",
      [
        textPart("msg-0001", "hello"),
        reasoningPart("msg-0001", "private"),
        toolPart("msg-0001", "edit", "completed"),
      ],
      sessionId,
    ),
  );
  assert.ok(message !== null);
  assert.equal(message.id, "msg-0001");
  assert.equal(message.role, "assistant");
  assert.deepEqual(
    message.parts.map((part) => part.type),
    ["text", "reasoning", "tool"],
  );
  assert.equal(message.parts[0].text, "hello");
  assert.equal(message.parts[2].tool, "edit");
  assert.equal(message.parts[2].state?.status, "completed");
});

test("parseMessageWithParts rejects items without info.id/info.role", () => {
  assert.equal(
    parseMessageWithParts({ info: { role: "user" }, parts: [] }),
    null,
  );
  assert.equal(parseMessageWithParts({ id: "msg-1", role: "user" }), null);
  assert.equal(parseMessageWithParts(null), null);
});

test("parseFileDiff reads file/before/after snapshots", () => {
  const diff = parseFileDiff(rawDiff("src/a.ts", "-old\n", "+new\n"));
  assert.deepEqual(diff, { file: "src/a.ts", before: "-old\n", after: "+new\n" });
  assert.equal(parseFileDiff({ file: "a.ts", patch: "@@" }), null);
});

test("listMessages follows Link rel=next until the checkpoint page", async () => {
  const pages = [
    {
      body: [
        rawMessage("m4", "user", [textPart("m4", "four")], sessionId),
        rawMessage("m5", "assistant", [textPart("m5", "five")], sessionId),
      ],
      link: `${baseUrl}/session/${sessionId}/message?limit=2&before=cursor1`,
    },
    {
      body: [
        rawMessage("m2", "user", [textPart("m2", "two")], sessionId),
        rawMessage("m3", "assistant", [textPart("m3", "three")], sessionId),
      ],
      link: `${baseUrl}/session/${sessionId}/message?limit=2&before=cursor2`,
    },
    {
      body: [rawMessage("m1", "user", [textPart("m1", "one")], sessionId)],
      link: null,
    },
  ];
  let calls = 0;
  const fetchImpl = (async () => {
    const page = pages[calls];
    calls += 1;
    return new Response(JSON.stringify(page.body), {
      status: 200,
      headers: page.link === null ? {} : { link: `<${page.link}>; rel="next"` },
    });
  }) as unknown as typeof fetch;

  const source = createHttpMessageSource({ baseUrl, fetchImpl });
  const messages = await source.listMessages(sessionId, {
    afterId: "m1",
    limit: 2,
    maxPages: 10,
  });

  assert.deepEqual(
    messages.map((message) => message.id),
    ["m2", "m3", "m4", "m5"],
  );
  assert.equal(calls, 3, "pagination must reach the third page");
});

test("listMessages stops as soon as the checkpoint is reached", async () => {
  const pages = [
    {
      body: [rawMessage("m4", "user", [textPart("m4", "four")], sessionId)],
      link: `${baseUrl}/session/${sessionId}/message?limit=1&before=cursor1`,
    },
    {
      body: [rawMessage("m2", "user", [textPart("m2", "two")], sessionId)],
      link: `${baseUrl}/session/${sessionId}/message?limit=1&before=cursor2`,
    },
    {
      body: [rawMessage("m1", "user", [textPart("m1", "one")], sessionId)],
      link: null,
    },
  ];
  let calls = 0;
  const fetchImpl = (async () => {
    const page = pages[calls];
    calls += 1;
    return new Response(JSON.stringify(page.body), {
      status: 200,
      headers: page.link === null ? {} : { link: `<${page.link}>; rel="next"` },
    });
  }) as unknown as typeof fetch;

  const source = createHttpMessageSource({ baseUrl, fetchImpl });
  const messages = await source.listMessages(sessionId, {
    afterId: "m2",
    limit: 1,
  });

  assert.deepEqual(
    messages.map((message) => message.id),
    ["m4"],
  );
  assert.equal(calls, 2, "must not fetch older pages past the checkpoint");
});
