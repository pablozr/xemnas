//! Minimal context injection on each user turn (`chat.message` hook).
//!
//! The hook asks the local API for a compact, already-deduplicated block and
//! appends it to the user's text when the app returns one. The mode (off,
//! shadow, inject) is a per-project setting in the app, not in the plugin. It never throws and never
//! waits longer than `contextTimeoutMs`: without an answer the turn proceeds
//! untouched. The prompt is sent only to the loopback API and never logged.

import type { EndpointResolver } from "./client.js";
import type { AdapterLog } from "./index.js";

const OPEN_TAG = "<xemnas-context";
const CLOSE_TAG = "</xemnas-context>";

/** Request sent to `POST /v1/context`. */
export interface ContextRequest {
  canonical_path: string;
  session_id: string;
  prompt: string;
}

/** Answer of `POST /v1/context`. */
export interface ContextAnswer {
  mode: string;
  context: string | null;
  tokens: number;
  items: number;
  omitted: number;
}

/** Fetches context blocks. */
export interface ContextClient {
  prepare(request: ContextRequest): Promise<ContextAnswer | null>;
}

/** Options for [`createContextClient`]. */
export interface ContextClientOptions {
  resolveEndpoint: EndpointResolver;
  timeoutMs: number;
  fetchImpl?: typeof fetch;
}

/** A structural view of an OpenCode message part. */
export interface ChatPart {
  type: string;
  text?: string;
  synthetic?: boolean;
}

/** Input of the `chat.message` hook. */
export interface ChatMessageInput {
  sessionID?: string;
}

/** Output of the `chat.message` hook; `parts` may be mutated. */
export interface ChatMessageOutput {
  message?: { sessionID?: string };
  parts: ChatPart[];
}

/** The `chat.message` hook signature. */
export type ChatMessageHook = (
  input: ChatMessageInput,
  output: ChatMessageOutput,
) => Promise<void>;

/** Removes every `<xemnas-context …>…</xemnas-context>` block from `text`. */
export function stripContextBlocks(text: string): string {
  let result = text;
  for (;;) {
    const start = result.indexOf(OPEN_TAG);
    if (start === -1) {
      return result;
    }
    const end = result.indexOf(CLOSE_TAG, start);
    const stop = end === -1 ? result.length : end + CLOSE_TAG.length;
    result = `${result.slice(0, start).trimEnd()}${result.slice(stop)}`;
  }
}

/** The user's own text: non-synthetic text parts, joined. */
export function promptText(parts: ChatPart[]): string {
  return parts
    .filter((part) => part.type === "text" && part.synthetic !== true)
    .map((part) => stripContextBlocks(part.text ?? ""))
    .join("\n")
    .trim();
}

function isAnswer(value: unknown): value is ContextAnswer {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const record = value as Record<string, unknown>;
  return (
    typeof record.mode === "string" &&
    (record.context === null || typeof record.context === "string") &&
    typeof record.tokens === "number" &&
    typeof record.items === "number" &&
    typeof record.omitted === "number"
  );
}

/** Creates the HTTP client for `POST /v1/context`. */
export function createContextClient(
  options: ContextClientOptions,
): ContextClient {
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;
  return {
    async prepare(request: ContextRequest): Promise<ContextAnswer | null> {
      const endpoint = options.resolveEndpoint();
      if (endpoint === null) {
        return null;
      }
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), options.timeoutMs);
      try {
        const response = await fetchImpl(`${endpoint.baseUrl}/v1/context`, {
          method: "POST",
          headers: {
            Authorization: `Bearer ${endpoint.token}`,
            "Content-Type": "application/json",
          },
          body: JSON.stringify(request),
          signal: controller.signal,
        });
        if (response.status !== 200) {
          return null;
        }
        const body = (await response.json()) as unknown;
        return isAnswer(body) ? body : null;
      } catch {
        return null;
      } finally {
        clearTimeout(timer);
      }
    },
  };
}

/** Dependencies of [`createChatMessageHook`]. */
export interface ChatMessageHookDeps {
  client: ContextClient;
  directory: string;
  log: AdapterLog;
}

/** Builds the `chat.message` hook. */
export function createChatMessageHook(deps: ChatMessageHookDeps): ChatMessageHook {
  return async (input, output) => {
    try {
      const sessionId = input.sessionID ?? output.message?.sessionID;
      const prompt = promptText(output.parts);
      if (sessionId === undefined || prompt.length === 0) {
        return;
      }
      const answer = await deps.client.prepare({
        canonical_path: deps.directory,
        session_id: sessionId,
        prompt,
      });
      if (answer === null) {
        deps.log.warn("context-unavailable", { session_id: sessionId });
        return;
      }
      if (answer.items > 0) {
        deps.log.info("context-prepared", {
          session_id: sessionId,
          mode: answer.mode,
          items: answer.items,
          tokens: answer.tokens,
          omitted: answer.omitted,
        });
      }
      if (answer.context === null) {
        return;
      }
      const target = [...output.parts]
        .reverse()
        .find((part) => part.type === "text" && part.synthetic !== true);
      if (target !== undefined) {
        target.text = `${target.text ?? ""}\n\n${answer.context}`;
      }
    } catch {
      deps.log.warn("context-failed", {});
    }
  };
}
