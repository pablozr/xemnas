//! Context injection: on each user turn (`chat.message` hook) and right after
//! the agent edits a file (`tool.execute.after` hook, ADR-0005).
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
  /** Files involved: recently edited (prompt) or just edited (edit). */
  files?: string[];
  /** `prompt` (default) or `edit`. */
  trigger?: "prompt" | "edit";
}

/** Tools that change files; only these trigger context on edit. */
export const EDIT_TOOLS: ReadonlySet<string> = new Set([
  "edit",
  "write",
  "multiedit",
  "patch",
  "apply_patch",
]);

/** Edited files remembered per session, for follow-up prompts. */
const MAX_RECENT_FILES = 8;

/** Files each session edited recently, newest last. In memory only. */
export class RecentFiles {
  private readonly bySession = new Map<string, string[]>();

  /** Records `files` as the newest edits of `sessionId`. */
  add(sessionId: string, files: string[]): void {
    const current = (this.bySession.get(sessionId) ?? []).filter(
      (file) => !files.includes(file),
    );
    current.push(...files);
    this.bySession.set(sessionId, current.slice(-MAX_RECENT_FILES));
  }

  /** The session's recent edits, oldest first. */
  get(sessionId: string): string[] {
    return [...(this.bySession.get(sessionId) ?? [])];
  }
}

/** Input of the `tool.execute.after` hook. */
export interface ToolAfterInput {
  tool: string;
  sessionID: string;
  callID?: string;
  args?: unknown;
}

/** Output of the `tool.execute.after` hook; `output` is what the model reads. */
export interface ToolAfterOutput {
  title?: string;
  output: string;
  metadata?: unknown;
}

/** The `tool.execute.after` hook signature. */
export type ToolAfterHook = (
  input: ToolAfterInput,
  output: ToolAfterOutput,
) => Promise<void>;

/**
 * Files an edit tool changed, from its arguments: `filePath` for `edit`,
 * `write` and `multiedit`; the `*** Add/Update File:` and `+++ b/` headers of
 * a patch for `patch` and `apply_patch`. Deletions are ignored.
 */
export function editedFiles(tool: string, args: unknown): string[] {
  if (!EDIT_TOOLS.has(tool) || typeof args !== "object" || args === null) {
    return [];
  }
  const record = args as Record<string, unknown>;
  const files: string[] = [];
  for (const key of ["filePath", "file_path", "path"]) {
    const value = record[key];
    if (typeof value === "string" && value.trim().length > 0) {
      files.push(value.trim());
    }
  }
  for (const key of ["patchText", "patch", "input"]) {
    const value = record[key];
    if (typeof value !== "string") {
      continue;
    }
    for (const line of value.split(/\r?\n/)) {
      const match =
        /^\*\*\* (?:Add|Update) File:\s*(.+)$/.exec(line) ??
        /^\+\+\+ (?:b\/)?(.+)$/.exec(line);
      const file = match?.[1]?.trim();
      if (file !== undefined && file.length > 0 && file !== "/dev/null") {
        files.push(file);
      }
    }
  }
  return [...new Set(files)];
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

/** Dependencies of [`createChatMessageHook`] and [`createToolAfterHook`]. */
export interface ChatMessageHookDeps {
  client: ContextClient;
  directory: string;
  log: AdapterLog;
  /** Shared with the edit hook so follow-up prompts see recent edits. */
  recent?: RecentFiles;
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
      const files = deps.recent?.get(sessionId) ?? [];
      const answer = await deps.client.prepare({
        canonical_path: deps.directory,
        session_id: sessionId,
        prompt,
        ...(files.length > 0 ? { files } : {}),
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

/**
 * Builds the `tool.execute.after` hook: after an edit tool ran, appends to its
 * result the decisions and rules the project map ties to the edited files, so
 * the agent reads them while it is still working there. The app returns each
 * item once per session and nothing for files outside mapped components; any
 * failure leaves the tool result untouched.
 */
export function createToolAfterHook(deps: ChatMessageHookDeps): ToolAfterHook {
  return async (input, output) => {
    try {
      const files = editedFiles(input.tool, input.args);
      if (files.length === 0 || typeof input.sessionID !== "string") {
        return;
      }
      deps.recent?.add(input.sessionID, files);
      const answer = await deps.client.prepare({
        canonical_path: deps.directory,
        session_id: input.sessionID,
        prompt: "",
        files,
        trigger: "edit",
      });
      if (answer === null || answer.items === 0) {
        return;
      }
      deps.log.info("context-on-edit", {
        session_id: input.sessionID,
        mode: answer.mode,
        items: answer.items,
        tokens: answer.tokens,
      });
      if (answer.context !== null && typeof output.output === "string") {
        output.output = `${output.output}\n\n${answer.context}`;
      }
    } catch {
      deps.log.warn("context-on-edit-failed", {});
    }
  };
}
