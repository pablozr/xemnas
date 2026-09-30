//! Automatic capture through OpenCode's native plugin client.
//!
//! The plugin input carries a `client` bound to the session's instance,
//! directory and authorization, whose requests run in-process. These tests
//! prove the adapter prefers that client, pages by the opaque `Link` cursor,
//! treats a failing client as a hard error (never an empty page, never a
//! silent fallback to `OPENCODE_URL`) and keeps two instances isolated.

import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import { XemnasOpenCodeAdapter } from "../src/index.js";
import {
  createClientMessageSource,
  isPluginClient,
  type OpenCodePluginClient,
  type PluginClientRequest,
  type PluginClientResult,
} from "../src/opencode.js";
import { delay, rawDiff, rawMessage } from "./test-helpers.js";

const sessionId = "session-client-1";
const directory = "C:/synthetic/projects/client";

function textPart(messageId: string, text: string): unknown {
  return {
    id: `${messageId}-t`,
    sessionID: sessionId,
    messageID: messageId,
    type: "text",
    text,
  };
}

function okResult(data: unknown, link?: string): PluginClientResult {
  return {
    data,
    error: undefined,
    response: {
      ok: true,
      status: 200,
      headers: new Headers(
        link === undefined ? {} : { link: `<${link}>; rel="next"` },
      ),
    },
  };
}

function errorResult(status: number): PluginClientResult {
  return {
    data: undefined,
    error: { status },
    response: { ok: false, status, headers: new Headers() },
  };
}

interface FakeClient {
  client: OpenCodePluginClient;
  messageCalls: PluginClientRequest[];
  diffCalls: PluginClientRequest[];
}

/**
 * Builds a structural plugin client scripted per call.
 *
 * `messages` receives the 0-based call index so tests can return successive
 * pages; `diff` is called once per turn.
 */
function fakeClient(handlers: {
  messages?: (
    call: PluginClientRequest,
    index: number,
  ) => Promise<PluginClientResult> | PluginClientResult;
  diff?: (
    call: PluginClientRequest,
  ) => Promise<PluginClientResult> | PluginClientResult;
}): FakeClient {
  const messageCalls: PluginClientRequest[] = [];
  const diffCalls: PluginClientRequest[] = [];
  const client: OpenCodePluginClient = {
    session: {
      async messages(request) {
        messageCalls.push(request);
        if (handlers.messages === undefined) {
          throw new Error("no messages handler");
        }
        return handlers.messages(request, messageCalls.length - 1);
      },
      async diff(request) {
        diffCalls.push(request);
        if (handlers.diff === undefined) {
          throw new Error("no diff handler");
        }
        return handlers.diff(request);
      },
    },
  };
  return { client, messageCalls, diffCalls };
}

async function waitFor(
  condition: () => boolean,
  timeoutMs: number,
): Promise<void> {
  const start = Date.now();
  while (!condition()) {
    if (Date.now() - start > timeoutMs) {
      throw new Error("waitFor timed out");
    }
    await delay(5);
  }
}

test("isPluginClient accepts only a session with messages and diff", () => {
  assert.equal(isPluginClient(undefined), false);
  assert.equal(isPluginClient(null), false);
  assert.equal(isPluginClient({}), false);
  assert.equal(isPluginClient({ session: {} }), false);
  assert.equal(isPluginClient({ session: { messages: "no", diff: 1 } }), false);
  assert.equal(
    isPluginClient({ session: { messages() {}, diff() {} } }),
    true,
  );
});

test("the client source pages by the opaque Link cursor and honors the checkpoint", async () => {
  const pages = [
    {
      items: [
        rawMessage("m4", "user", [], sessionId),
        rawMessage("m5", "assistant", [], sessionId),
      ],
      link: `http://localhost/session/${sessionId}/message?limit=2&before=cursor1`,
    },
    {
      items: [
        rawMessage("m2", "user", [], sessionId),
        rawMessage("m3", "assistant", [], sessionId),
      ],
      link: `http://localhost/session/${sessionId}/message?limit=2&before=cursor2`,
    },
    {
      items: [rawMessage("m1", "user", [], sessionId)],
      link: undefined,
    },
  ];
  const { client, messageCalls } = fakeClient({
    messages: (_call, index) => okResult(pages[index].items, pages[index].link),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 1_000,
  });

  const messages = await source.listMessages(sessionId, {
    afterId: "m1",
    limit: 2,
    maxPages: 10,
  });

  assert.deepEqual(
    messages.map((message) => message.id),
    ["m2", "m3", "m4", "m5"],
  );
  assert.equal(messageCalls.length, 3);
  assert.equal(messageCalls[0].query?.directory, directory);
  assert.equal(messageCalls[0].query?.limit, 2);
  assert.equal(messageCalls[0].query?.before, undefined);
  assert.equal(messageCalls[1].query?.before, "cursor1");
  assert.equal(messageCalls[2].query?.before, "cursor2");
});

test("the client source stops as soon as the checkpoint is reached", async () => {
  const pages = [
    { items: [rawMessage("m4", "user", [], sessionId)], link: "cursor1" },
    { items: [rawMessage("m2", "user", [], sessionId)], link: "cursor2" },
    { items: [rawMessage("m1", "user", [], sessionId)], link: undefined },
  ];
  const { client, messageCalls } = fakeClient({
    messages: (_call, index) =>
      okResult(
        pages[index].items,
        pages[index].link === undefined
          ? undefined
          : `http://localhost/session/${sessionId}/message?limit=1&before=${pages[index].link}`,
      ),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 1_000,
  });

  const messages = await source.listMessages(sessionId, {
    afterId: "m2",
    limit: 1,
    maxPages: 10,
  });

  assert.deepEqual(
    messages.map((message) => message.id),
    ["m4"],
  );
  assert.equal(messageCalls.length, 2, "must not page past the checkpoint");
});

test("maxPages bounds the number of client pages", async () => {
  const { client, messageCalls } = fakeClient({
    messages: (_call, index) =>
      okResult(
        [rawMessage(`m${index}`, "user", [], sessionId)],
        `http://localhost/session/${sessionId}/message?limit=1&before=c${index + 1}`,
      ),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 1_000,
  });

  const messages = await source.listMessages(sessionId, {
    afterId: null,
    limit: 1,
    maxPages: 3,
  });

  assert.equal(messageCalls.length, 3);
  assert.equal(messages.length, 3);
});

test("a Link that would switch session is ignored", async () => {
  const { client, messageCalls } = fakeClient({
    messages: () =>
      okResult(
        [rawMessage("m2", "user", [], sessionId)],
        "http://localhost/session/another-session/message?limit=1&before=cursor1",
      ),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 1_000,
  });

  const messages = await source.listMessages(sessionId, {
    afterId: null,
    limit: 1,
    maxPages: 5,
  });

  assert.deepEqual(
    messages.map((message) => message.id),
    ["m2"],
  );
  assert.equal(messageCalls.length, 1, "a cross-session Link must not be followed");
});

test("a client error or non-ok response is an error, not an empty page", async () => {
  const failing = fakeClient({ messages: () => errorResult(500) });
  const source = createClientMessageSource({
    client: failing.client,
    directory,
    timeoutMs: 1_000,
  });
  await assert.rejects(
    () => source.listMessages(sessionId, { afterId: null, limit: 10 }),
    /opencode responded 500/,
  );

  const failingDiff = fakeClient({ diff: () => errorResult(404) });
  const diffSource = createClientMessageSource({
    client: failingDiff.client,
    directory,
    timeoutMs: 1_000,
  });
  await assert.rejects(
    () => diffSource.getDiff(sessionId, "m1"),
    /opencode responded 404/,
  );
});

test("a client timeout aborts deterministically instead of returning []", async () => {
  const { client } = fakeClient({
    messages: (call) =>
      new Promise<PluginClientResult>((_resolve, reject) => {
        call.signal?.addEventListener("abort", () => {
          reject(Object.assign(new Error("aborted"), { name: "AbortError" }));
        });
      }),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 15,
  });

  await assert.rejects(
    () => source.listMessages(sessionId, { afterId: null, limit: 10 }),
    /timed out|aborted/i,
  );
});

test("a client that ignores the abort signal still hits the deadline", async () => {
  const { client } = fakeClient({
    // Never resolves and never observes the signal: a broken/misbehaving client
    // must not pin the reconciliation forever.
    messages: () => new Promise<PluginClientResult>(() => {}),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 20,
  });

  // Bound the wait so a regression fails instead of hanging the suite.
  const started = Date.now();
  const outcome = await Promise.race([
    source
      .listMessages(sessionId, { afterId: null, limit: 10 })
      .then(() => "resolved", (error: Error) => `rejected:${error.message}`),
    delay(300).then(() => "hung"),
  ]);

  assert.match(outcome, /^rejected:.*(timed out|abort)/i);
  assert.ok(Date.now() - started < 300, "the deadline must fire before the guard");
});

test("a synchronous client throw rejects instead of leaking", async () => {
  const client: OpenCodePluginClient = {
    session: {
      messages() {
        throw new Error("sync boom");
      },
      diff() {
        throw new Error("sync boom");
      },
    },
  };
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 1_000,
  });
  await assert.rejects(
    () => source.listMessages(sessionId, { afterId: null, limit: 10 }),
    /sync boom/,
  );
});

test("a late client resolution after the deadline is discarded", async () => {
  let release: (result: PluginClientResult) => void = () => {};
  const { client } = fakeClient({
    messages: () =>
      new Promise<PluginClientResult>((resolve) => {
        release = resolve;
      }),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 15,
  });

  const outcome = await Promise.race([
    source
      .listMessages(sessionId, { afterId: null, limit: 10 })
      .then(() => "resolved", (error: Error) => `rejected:${error.message}`),
    delay(300).then(() => "hung"),
  ]);
  assert.match(outcome, /^rejected:/);

  // Settling the abandoned promise afterwards must not throw (no unhandled
  // rejection) nor change the already-settled caller.
  release(okResult([rawMessage("m9", "user", [], sessionId)]));
  await delay(10);
});

test("listMessages rejects a non-array payload instead of an empty list", async () => {
  for (const payload of [null, {}, "text", 42]) {
    const { client } = fakeClient({ messages: () => okResult(payload) });
    const source = createClientMessageSource({
      client,
      directory,
      timeoutMs: 1_000,
    });
    await assert.rejects(
      () => source.listMessages(sessionId, { afterId: null, limit: 10 }),
      /non-array messages/,
    );
  }
});

test("getDiff rejects a non-array payload instead of an empty list", async () => {
  for (const payload of [null, {}, "text"]) {
    const { client } = fakeClient({ diff: () => okResult(payload) });
    const source = createClientMessageSource({
      client,
      directory,
      timeoutMs: 1_000,
    });
    await assert.rejects(
      () => source.getDiff(sessionId, "m1"),
      /non-array diff/,
    );
  }
});

test("getDiff forwards the session, directory and message id", async () => {
  const { client, diffCalls } = fakeClient({
    diff: () => okResult([rawDiff("src/a.ts", "-old\n", "+new\n")]),
  });
  const source = createClientMessageSource({
    client,
    directory,
    timeoutMs: 1_000,
  });

  const diffs = await source.getDiff(sessionId, "msg-0001");

  assert.deepEqual(diffs, [
    { file: "src/a.ts", before: "-old\n", after: "+new\n" },
  ]);
  assert.equal(diffCalls.length, 1);
  assert.equal(diffCalls[0].path.id, sessionId);
  assert.equal(diffCalls[0].query?.directory, directory);
  assert.equal(diffCalls[0].query?.messageID, "msg-0001");
});

test("two client sources stay isolated in directory and data", async () => {
  const first = fakeClient({
    messages: () => okResult([rawMessage("a1", "user", [], "s")]),
  });
  const second = fakeClient({
    messages: () => okResult([rawMessage("b1", "user", [], "s")]),
  });
  const firstSource = createClientMessageSource({
    client: first.client,
    directory: "C:/one",
    timeoutMs: 1_000,
  });
  const secondSource = createClientMessageSource({
    client: second.client,
    directory: "C:/two",
    timeoutMs: 1_000,
  });

  const [fromFirst, fromSecond] = await Promise.all([
    firstSource.listMessages("s", { afterId: null, limit: 10 }),
    secondSource.listMessages("s", { afterId: null, limit: 10 }),
  ]);

  assert.deepEqual(fromFirst.map((message) => message.id), ["a1"]);
  assert.deepEqual(fromSecond.map((message) => message.id), ["b1"]);
  assert.equal(first.messageCalls[0].query?.directory, "C:/one");
  assert.equal(second.messageCalls[0].query?.directory, "C:/two");
});

interface EnvSnapshot {
  dataDir: string | undefined;
  debounce: string | undefined;
}

function preparePluginDataDir(): { dataDir: string; previous: EnvSnapshot } {
  const dataDir = mkdtempSync(join(tmpdir(), "xemnas-plugin-client-"));
  mkdirSync(join(dataDir, "state"), { recursive: true });
  writeFileSync(
    join(dataDir, "state", "discovery.json"),
    JSON.stringify({ protocol_version: 1, port: 4321, instance_id: "i-1" }),
    "utf8",
  );
  writeFileSync(join(dataDir, "state", "api-token"), "token\n", "utf8");
  const previous: EnvSnapshot = {
    dataDir: process.env.XEMNAS_DATA_DIR,
    debounce: process.env.XEMNAS_ADAPTER_DEBOUNCE_MS,
  };
  process.env.XEMNAS_DATA_DIR = dataDir;
  process.env.XEMNAS_ADAPTER_DEBOUNCE_MS = "0";
  return { dataDir, previous };
}

function restoreEnv(previous: EnvSnapshot): void {
  if (previous.dataDir === undefined) {
    delete process.env.XEMNAS_DATA_DIR;
  } else {
    process.env.XEMNAS_DATA_DIR = previous.dataDir;
  }
  if (previous.debounce === undefined) {
    delete process.env.XEMNAS_ADAPTER_DEBOUNCE_MS;
  } else {
    process.env.XEMNAS_ADAPTER_DEBOUNCE_MS = previous.debounce;
  }
}

function interceptFetch(): {
  bodies: Array<Record<string, unknown>>;
  unexpected: string[];
  restore: () => void;
} {
  const original = globalThis.fetch;
  const bodies: Array<Record<string, unknown>> = [];
  const unexpected: string[] = [];
  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.includes("/v1/captures")) {
      bodies.push(JSON.parse(String(init?.body)) as Record<string, unknown>);
      return new Response(JSON.stringify({ capture_id: "cap-1" }), {
        status: 201,
      });
    }
    unexpected.push(url);
    throw new Error(`unexpected fetch ${url}`);
  }) as typeof fetch;
  return { bodies, unexpected, restore: () => (globalThis.fetch = original) };
}

test("the real plugin factory captures through the injected client", async () => {
  const { previous } = preparePluginDataDir();
  const net = interceptFetch();
  try {
    const client = fakeClient({
      messages: () =>
        okResult([
          rawMessage(
            "msg-0001",
            "user",
            [textPart("msg-0001", "hello")],
            sessionId,
          ),
          rawMessage(
            "msg-0002",
            "assistant",
            [textPart("msg-0002", "world")],
            sessionId,
          ),
        ]),
      diff: () => okResult([]),
    }).client;

    const hooks = await XemnasOpenCodeAdapter({ directory, client });
    await hooks.event?.({
      event: {
        type: "session.status",
        properties: { sessionID: sessionId, status: { type: "idle" } },
      },
    });
    await waitFor(() => net.bodies.length === 1, 1_000);

    assert.deepEqual(net.unexpected, [], "must not use the legacy HTTP source");
    const source = net.bodies[0].source as Record<string, unknown>;
    assert.equal(source.session_id, sessionId);
  } finally {
    net.restore();
    restoreEnv(previous);
  }
});

test("a present but failing client never falls back to the HTTP source", async () => {
  const { previous } = preparePluginDataDir();
  const net = interceptFetch();
  try {
    const client = fakeClient({
      messages: () => {
        throw new Error("client unavailable");
      },
    }).client;

    const hooks = await XemnasOpenCodeAdapter({ directory, client });
    await hooks.event?.({
      event: {
        type: "session.status",
        properties: { sessionID: sessionId, status: { type: "idle" } },
      },
    });
    await delay(50);

    assert.equal(net.bodies.length, 0);
    assert.deepEqual(net.unexpected, [], "no HTTP request may be attempted");
  } finally {
    net.restore();
    restoreEnv(previous);
  }
});

test("a structurally unusable client falls back to the legacy HTTP source", async () => {
  const { previous } = preparePluginDataDir();
  const original = globalThis.fetch;
  const openCodeUrls: string[] = [];
  const bodies: Array<Record<string, unknown>> = [];
  globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
    const url =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    if (url.includes("/v1/captures")) {
      bodies.push(JSON.parse(String(init?.body)) as Record<string, unknown>);
      return new Response(JSON.stringify({ capture_id: "cap-http" }), {
        status: 201,
      });
    }
    openCodeUrls.push(url);
    return new Response(
      JSON.stringify([
        rawMessage(
          "msg-0001",
          "user",
          [textPart("msg-0001", "via http")],
          sessionId,
        ),
      ]),
      { status: 200 },
    );
  }) as typeof fetch;

  try {
    const hooks = await XemnasOpenCodeAdapter({
      directory,
      // Not a usable client: missing the `session.diff` method.
      client: { session: { messages: () => undefined } },
    });
    await hooks.event?.({
      event: {
        type: "session.status",
        properties: { sessionID: sessionId, status: { type: "idle" } },
      },
    });
    await waitFor(() => bodies.length === 1, 1_000);

    assert.ok(
      openCodeUrls.some((url) =>
        /\/session\/session-client-1\/message\?limit=200/.test(url),
      ),
      `expected a legacy HTTP message fetch, got ${openCodeUrls.join(", ")}`,
    );
  } finally {
    globalThis.fetch = original;
    restoreEnv(previous);
  }
});
