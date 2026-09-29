//! Message source port: the OpenCode HTTP API and an in-memory fake.
//!
//! The adapter reads the canonical turn data from OpenCode's public server API
//! (MVP-SPEC §7.1). The endpoints and payload shapes live **only** in this
//! module, and follow the documented server API
//! (<https://opencode.ai/docs/server/>) and the generated SDK types
//! (`@opencode-ai/sdk` `gen/types.gen.ts`):
//!
//! - `GET /session/:id/message?limit=N[&before=cursor]` returns
//!   `Array<{ info: Message, parts: Part[] }>`. `info.id`/`info.role` carry the
//!   message identity and role; `parts` carries the content.
//! - `GET /session/:id/diff?messageID=` returns `FileDiff[]`, where each item is
//!   `{ file, before, after, additions, deletions }` (line snapshots, not a
//!   pre-rendered patch).
//!
//! Pagination: the server's `before` parameter is an **opaque base64url cursor**
//! (`JSON.stringify({id, time})`), not a raw message id, and it is only valid
//! when `limit` is also present (`MessagesQuery` in
//! `packages/opencode/src/server/routes/instance/httpapi/groups/session.ts`).
//! The next cursor is delivered in the `Link: <url>; rel="next"` response
//! header, so this module follows that header rather than guessing a cursor.
//! Message ids are opaque and ascending (`Identifier.ascending`); this module
//! compares them lexicographically only as a documented best-effort ordering.

import { DEFAULT_MAX_MESSAGE_PAGES } from "./config.js";

export interface OpenCodePart {
  /** Part discriminator, for example `text`, `reasoning`, `tool`. */
  type: string;
  /** Present on text and reasoning parts. */
  text?: string;
  /** Tool name, present on tool parts. */
  tool?: string;
  /** Tool state subset; only `status` is read (never tool output). */
  state?: { status?: string };
}

/** Normalized view of one OpenCode message. */
export interface OpenCodeMessage {
  id: string;
  role: string;
  parts: OpenCodePart[];
}

/** Normalized view of one OpenCode file diff (a before/after line snapshot). */
export interface OpenCodeFileDiff {
  file: string;
  before: string;
  after: string;
}

/** Documented response item of `GET /session/:id/message`. */
export interface MessageWithParts {
  info: { id: string; role: string } & Record<string, unknown>;
  parts: unknown[];
}

/** Documented `FileDiff` response item. */
export interface RawFileDiff {
  file: string;
  before: string;
  after: string;
  additions?: number;
  deletions?: number;
}

/** Options for listing messages since a checkpoint. */
export interface ListMessagesOptions {
  /** Exclusive lower bound; `null` means "from the beginning of the window". */
  afterId: string | null;
  /** Page size (the server's `limit`). */
  limit: number;
  /** Maximum backward pages to fetch; defaults to the configured bound. */
  maxPages?: number;
}

/** Source of messages and diffs for one OpenCode session. */
export interface MessageSource {
  /** Returns messages ascending by id, filtered to `id > afterId`. */
  listMessages(
    sessionId: string,
    options: ListMessagesOptions,
  ): Promise<OpenCodeMessage[]>;
  /** Returns the file diffs associated with a message. */
  getDiff(sessionId: string, messageId: string): Promise<OpenCodeFileDiff[]>;
}

function toPart(value: unknown): OpenCodePart | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (typeof record.type !== "string") {
    return null;
  }
  const part: OpenCodePart = { type: record.type };
  if (typeof record.text === "string") {
    part.text = record.text;
  }
  if (typeof record.tool === "string") {
    part.tool = record.tool;
  }
  if (typeof record.state === "object" && record.state !== null) {
    const state = record.state as Record<string, unknown>;
    part.state =
      typeof state.status === "string" ? { status: state.status } : {};
  }
  return part;
}

/**
 * Parses one documented `{ info, parts }` item into a normalized message.
 *
 * Returns `null` when the item does not carry the documented message identity
 * (`info.id`/`info.role`); malformed items are skipped rather than guessed at.
 */
export function parseMessageWithParts(value: unknown): OpenCodeMessage | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  const info = record.info;
  if (typeof info !== "object" || info === null) {
    return null;
  }
  const infoRecord = info as Record<string, unknown>;
  if (
    typeof infoRecord.id !== "string" ||
    typeof infoRecord.role !== "string"
  ) {
    return null;
  }
  const parts = Array.isArray(record.parts)
    ? record.parts
        .map(toPart)
        .filter((part): part is OpenCodePart => part !== null)
    : [];
  return { id: infoRecord.id, role: infoRecord.role, parts };
}

/**
 * Parses one documented `FileDiff` item.
 *
 * Requires the `file`/`before`/`after` line snapshots the server returns.
 */
export function parseFileDiff(value: unknown): OpenCodeFileDiff | null {
  if (typeof value !== "object" || value === null) {
    return null;
  }
  const record = value as Record<string, unknown>;
  if (
    typeof record.file !== "string" ||
    typeof record.before !== "string" ||
    typeof record.after !== "string"
  ) {
    return null;
  }
  return { file: record.file, before: record.before, after: record.after };
}

function compareIds(left: string, right: string): number {
  // Opaque ascending ids: lexicographic order is a documented best-effort.
  return left < right ? -1 : left > right ? 1 : 0;
}

/** Options for the HTTP message source. */
export interface HttpMessageSourceOptions {
  /** OpenCode base URL, without a trailing slash. */
  baseUrl: string;
  /** Request timeout, in milliseconds. */
  timeoutMs?: number;
  /** Injectable fetch, defaulting to the global implementation. */
  fetchImpl?: typeof fetch;
}

interface MessagePage {
  items: unknown[];
  nextUrl: string | null;
}

/** Extracts the `rel="next"` target from a `Link` header, if present. */
function parseNextLink(header: string | null, baseUrl: string): string | null {
  if (header === null) {
    return null;
  }
  for (const part of header.split(",")) {
    const match = /<([^>]+)>\s*;\s*rel\s*=\s*"?next"?/i.exec(part.trim());
    if (match !== null) {
      try {
        return new URL(match[1], baseUrl).toString();
      } catch {
        return null;
      }
    }
  }
  return null;
}

async function fetchMessagePage(
  fetchImpl: typeof fetch,
  url: string,
  timeoutMs: number,
): Promise<MessagePage> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchImpl(url, { signal: controller.signal });
    if (!response.ok) {
      throw new Error(`opencode responded ${response.status}`);
    }
    const body = (await response.json()) as unknown;
    return {
      items: Array.isArray(body) ? body : [],
      nextUrl: parseNextLink(response.headers.get("link"), url),
    };
  } finally {
    clearTimeout(timer);
  }
}

/** Creates the real HTTP-backed message source. */
export function createHttpMessageSource(
  options: HttpMessageSourceOptions,
): MessageSource {
  const fetchImpl = options.fetchImpl ?? globalThis.fetch;
  const timeoutMs = options.timeoutMs ?? 5_000;

  const firstPageUrl = (sessionId: string, limit: number): string =>
    `${options.baseUrl}/session/${encodeURIComponent(sessionId)}/message?limit=${limit}`;

  const diffUrl = (sessionId: string, messageId: string): string =>
    `${options.baseUrl}/session/${encodeURIComponent(sessionId)}/diff?messageID=${encodeURIComponent(messageId)}`;

  return {
    async listMessages(sessionId, listOptions) {
      const pageSize = Math.max(1, listOptions.limit);
      const maxPages = Math.max(
        1,
        listOptions.maxPages ?? DEFAULT_MAX_MESSAGE_PAGES,
      );
      const collected: OpenCodeMessage[] = [];
      const seen = new Set<string>();
      let url: string | null = firstPageUrl(sessionId, pageSize);

      for (let page = 0; page < maxPages && url !== null; page++) {
        const result = await fetchMessagePage(fetchImpl, url, timeoutMs);
        let reachedCheckpoint = false;
        for (const item of result.items) {
          const message = parseMessageWithParts(item);
          if (message === null || seen.has(message.id)) {
            continue;
          }
          seen.add(message.id);
          collected.push(message);
          if (
            listOptions.afterId !== null &&
            compareIds(message.id, listOptions.afterId) <= 0
          ) {
            reachedCheckpoint = true;
          }
        }
        if (reachedCheckpoint) {
          break;
        }
        url = result.nextUrl;
      }

      collected.sort((left, right) => compareIds(left.id, right.id));
      return listOptions.afterId === null
        ? collected
        : collected.filter(
            (message) => compareIds(message.id, listOptions.afterId ?? "") > 0,
          );
    },

    async getDiff(sessionId, messageId) {
      const raw = await getJson(
        fetchImpl,
        diffUrl(sessionId, messageId),
        timeoutMs,
      );
      return Array.isArray(raw)
        ? raw
            .map(parseFileDiff)
            .filter((diff): diff is OpenCodeFileDiff => diff !== null)
        : [];
    },
  };
}

async function getJson(
  fetchImpl: typeof fetch,
  url: string,
  timeoutMs: number,
): Promise<unknown> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetchImpl(url, { signal: controller.signal });
    if (!response.ok) {
      throw new Error(`opencode responded ${response.status}`);
    }
    return (await response.json()) as unknown;
  } finally {
    clearTimeout(timer);
  }
}

/** Options for the deterministic in-memory source used by tests. */
export interface FakeMessageSourceOptions {
  /** Documented `{ info, parts }` items keyed by session id. */
  messages?: Record<string, MessageWithParts[]>;
  /** Documented `FileDiff` items keyed by `${sessionId}:${messageId}`. */
  diffs?: Record<string, RawFileDiff[]>;
}

/**
 * Creates a deterministic in-memory message source.
 *
 * The fake consumes the **same documented payload shapes** as the HTTP source
 * and runs them through the same parsers, so it cannot mask a parser
 * regression.
 */
export function createFakeMessageSource(
  options: FakeMessageSourceOptions = {},
): MessageSource {
  const messages = options.messages ?? {};
  const diffs = options.diffs ?? {};
  return {
    async listMessages(sessionId, listOptions) {
      const all = (messages[sessionId] ?? [])
        .map(parseMessageWithParts)
        .filter((message): message is OpenCodeMessage => message !== null)
        .sort((left, right) => compareIds(left.id, right.id));
      const after = listOptions.afterId;
      const filtered =
        after === null
          ? all
          : all.filter((message) => compareIds(message.id, after) > 0);
      return filtered.slice(-Math.max(1, listOptions.limit));
    },
    async getDiff(sessionId, messageId) {
      return (diffs[`${sessionId}:${messageId}`] ?? [])
        .map(parseFileDiff)
        .filter((diff): diff is OpenCodeFileDiff => diff !== null);
    },
  };
}
