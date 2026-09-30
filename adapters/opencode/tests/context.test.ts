//! Context injection hook: modes, prompt extraction, never blocking, never
//! logging the prompt, and capture never re-reading injected blocks.

import assert from "node:assert/strict";
import { createServer } from "node:http";
import type { AddressInfo } from "node:net";
import { test } from "node:test";

import { resolveConfig } from "../src/config.js";
import {
  createChatMessageHook,
  createContextClient,
  promptText,
  stripContextBlocks,
  type ChatMessageOutput,
  type ContextAnswer,
  type ContextClient,
  type ContextRequest,
} from "../src/context.js";
import { boundContent } from "../src/redact.js";
import { createRecordingLog } from "./test-helpers.js";

const BLOCK = '<xemnas-context note="x">\nregra:abc Erros em português\n</xemnas-context>';

function answer(mode: string, context: string | null, items = 1): ContextAnswer {
  return { mode, context, tokens: 20, items, omitted: 0 };
}

function scriptedClient(result: ContextAnswer | null): {
  client: ContextClient;
  seen: ContextRequest[];
} {
  const seen: ContextRequest[] = [];
  return {
    seen,
    client: {
      async prepare(request) {
        seen.push(request);
        return result;
      },
    },
  };
}

function output(text: string): ChatMessageOutput {
  return {
    message: { sessionID: "s1" },
    parts: [
      { type: "text", text },
      { type: "file" },
      { type: "text", text: "gerado", synthetic: true },
    ],
  };
}

test("a block from the app is appended to the user's last text part", async () => {
  const { client, seen } = scriptedClient(answer("inject", BLOCK));
  const recording = createRecordingLog();
  const hook = createChatMessageHook({
    client,
    directory: "C:/projeto",
    log: recording.log,
  });
  const out = output("SECRET-PROMPT melhorar o cache");
  await hook({ sessionID: "s1" }, out);

  assert.equal(out.parts[0]?.text, `SECRET-PROMPT melhorar o cache\n\n${BLOCK}`);
  assert.equal(out.parts[2]?.text, "gerado", "synthetic parts are untouched");
  assert.deepEqual(seen[0], {
    canonical_path: "C:/projeto",
    session_id: "s1",
    prompt: "SECRET-PROMPT melhorar o cache",
  });
  assert.ok(recording.lines.some((line) => line.includes("context-prepared")));
  assert.ok(
    recording.lines.every((line) => !line.includes("SECRET-PROMPT")),
    "the prompt never reaches a log",
  );
});

test("off and shadow answers leave the message untouched", async () => {
  for (const mode of ["off", "shadow"]) {
    const { client, seen } = scriptedClient(answer(mode, null, mode === "off" ? 0 : 1));
    const hook = createChatMessageHook({
      client,
      directory: "C:/projeto",
      log: createRecordingLog().log,
    });
    const out = output("cache");
    await hook({}, out);
    assert.equal(out.parts[0]?.text, "cache");
    assert.equal(seen[0]?.session_id, "s1", "falls back to the message session");
  }
});

test("an unavailable or failing API leaves the turn untouched", async () => {
  const failing: ContextClient = {
    async prepare() {
      throw new Error("boom");
    },
  };
  for (const client of [scriptedClient(null).client, failing]) {
    const hook = createChatMessageHook({
      client,
      directory: "C:/projeto",
      log: createRecordingLog().log,
    });
    const out = output("cache");
    await hook({ sessionID: "s1" }, out);
    assert.equal(out.parts[0]?.text, "cache");
  }
});

test("prompt text ignores synthetic parts and earlier injected blocks", () => {
  assert.equal(
    promptText([
      { type: "text", text: `pergunta\n\n${BLOCK}` },
      { type: "text", text: "sintético", synthetic: true },
      { type: "text", text: "mais" },
    ]),
    "pergunta\nmais",
  );
  assert.equal(stripContextBlocks(`a ${BLOCK} b`), "a b");
  assert.equal(stripContextBlocks('a <xemnas-context note="x">sem fim'), "a");
});

test("captured content never keeps an injected block", () => {
  assert.equal(boundContent(`pedido\n\n${BLOCK}`, 1_000), "pedido");
});

test("the client gives up after its timeout and returns null", async () => {
  const server = createServer(() => {
    // Never answers.
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = (server.address() as AddressInfo).port;
  const client = createContextClient({
    resolveEndpoint: () => ({ baseUrl: `http://127.0.0.1:${port}`, token: "t" }),
    timeoutMs: 50,
  });
  const started = Date.now();
  const result = await client.prepare({
    canonical_path: "C:/p",
    session_id: "s1",
    prompt: "x",
  });
  assert.equal(result, null);
  assert.ok(Date.now() - started < 1_000);
  server.closeAllConnections();
  await new Promise<void>((resolve) => server.close(() => resolve()));
});

test("the client sends the bearer token and validates the answer shape", async () => {
  let authorization = "";
  const server = createServer((request, response) => {
    authorization = request.headers.authorization ?? "";
    response.writeHead(200, { "Content-Type": "application/json" });
    response.end(
      JSON.stringify({ mode: "inject", context: BLOCK, tokens: 20, items: 1, omitted: 0 }),
    );
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const port = (server.address() as AddressInfo).port;
  const client = createContextClient({
    resolveEndpoint: () => ({ baseUrl: `http://127.0.0.1:${port}`, token: "tok" }),
    timeoutMs: 1_000,
  });
  const result = await client.prepare({
    canonical_path: "C:/p",
    session_id: "s1",
    prompt: "x",
  });
  assert.equal(authorization, "Bearer tok");
  assert.equal(result?.context, BLOCK);
  await new Promise<void>((resolve) => server.close(() => resolve()));
});

test("the plugin has no mode or budget settings, only a timeout", () => {
  const config = resolveConfig({ XEMNAS_CONTEXT_MODE: "inject" });
  assert.equal("contextMode" in config, false);
  assert.equal("contextBudgetTokens" in config, false);
  assert.equal(config.contextTimeoutMs, 300);
  assert.equal(resolveConfig({ XEMNAS_CONTEXT_TIMEOUT_MS: "150" }).contextTimeoutMs, 150);
});
