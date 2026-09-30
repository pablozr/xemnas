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
  /**
   * `false` only for an assistant message the server reports as still being
   * written (`info.time` present without `completed`). Absent timing is treated
   * as finished, so fixtures and older servers keep working.
   */
  completed: boolean;
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

/**
 * Structural request accepted by the plugin's `session.messages`/`session.diff`.
 *
 * Mirrors the generated SDK options (`path`, `query`, `signal`) without taking
 * a dependency on the SDK package: the object the OpenCode runtime hands to the
 * plugin already carries a client bound to the right instance, directory and
 * authorization.
 */
export interface PluginClientRequest {
  path: { id: string };
  query?: Record<string, unknown>;
  signal?: AbortSignal;
}

/** Structural result of one plugin client call (`data`/`error`/`response`). */
export interface PluginClientResult {
  data?: unknown;
  error?: unknown;
  response?: {
    ok: boolean;
    status: number;
    headers: { get(name: string): string | null };
  };
}

/** Structural view of the OpenCode session sub-client. */
export interface PluginSessionClient {
  messages(request: PluginClientRequest): Promise<PluginClientResult>;
  diff(request: PluginClientRequest): Promise<PluginClientResult>;
}

/** Structural view of the OpenCode client passed to the plugin factory. */
export interface OpenCodePluginClient {
  session: PluginSessionClient;
}

/**
 * Returns whether `value` is a usable OpenCode plugin client.
 *
 * Only the two methods this adapter reads are required; anything else (an
 * absent client, a partial stub, a future shape) is treated as unavailable so
 * the caller can fall back to the legacy HTTP source.
 */
export function isPluginClient(value: unknown): value is OpenCodePluginClient {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const session = (value as { session?: unknown }).session;
  if (typeof session !== "object" || session === null) {
    return false;
  }
  const methods = session as Record<string, unknown>;
  return (
    typeof methods.messages === "function" && typeof methods.diff === "function"
  );
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
  return {
    id: infoRecord.id,
    role: infoRecord.role,
    parts,
    completed: isCompleted(infoRecord),
  };
}

/** Whether a message is finished: see [`OpenCodeMessage.completed`]. */
function isCompleted(info: Record<string, unknown>): boolean {
  if (info.role !== "assistant") {
    return true;
  }
  const time = info.time;
  if (typeof time !== "object" || time === null) {
    return true;
  }
  return typeof (time as Record<string, unknown>).completed === "number";
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

/** Base used to resolve relative `Link` targets without dereferencing them. */
const LINK_BASE = "http://localhost";

/**
 * Parses the `rel="next"` target from a `Link` header into a `URL`.
 *
 * The target is only ever inspected (path, query) — it is never fetched: the
 * in-process plugin client already knows the instance, directory and auth.
 */
function linkNextUrl(header: string | null, base: string): URL | null {
  if (header === null) {
    return null;
  }
  for (const part of header.split(",")) {
    const match = /<([^>]+)>\s*;\s*rel\s*=\s*"?next"?/i.exec(part.trim());
    if (match !== null) {
      try {
        return new URL(match[1], base);
      } catch {
        return null;
      }
    }
  }
  return null;
}

/** Extracts the `rel="next"` target from a `Link` header, if present. */
function parseNextLink(header: string | null, baseUrl: string): string | null {
  return linkNextUrl(header, baseUrl)?.toString() ?? null;
}

/** Path of the documented message-listing route for one session. */
function messageRoute(sessionId: string): string {
  return `/session/${encodeURIComponent(sessionId)}/message`;
}

/**
 * Extracts the opaque `before` cursor from a `Link: rel="next"` header.
 *
 * The server's cursor is opaque (`JSON.stringify({id, time})` base64url), so it
 * is never synthesized. The link target must be the same session's message
 * route over HTTP(S); a link that would switch session or scheme is ignored
 * rather than followed.
 */
export function nextBeforeFromLink(
  header: string | null,
  sessionId: string,
): string | null {
  const target = linkNextUrl(header, LINK_BASE);
  if (target === null) {
    return null;
  }
  if (target.protocol !== "http:" && target.protocol !== "https:") {
    return null;
  }
  if (target.pathname !== messageRoute(sessionId)) {
    return null;
  }
  const before = target.searchParams.get("before");
  return before !== null && before.length > 0 ? before : null;
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

/** One page of raw items plus the token for the next (older) page. */
interface PageResult {
  items: unknown[];
  next: string | null;
}

/**
 * Walks pages newest→oldest, deduplicating by id and stopping at the
 * checkpoint, then returns the collected messages ascending and filtered.
 *
 * `fetchPage(null)` reads the newest page; a non-null token is whatever the
 * previous page returned (an HTTP URL or an opaque `before` cursor). The
 * parsing/dedup/checkpoint rules are shared by the HTTP and client sources so
 * they cannot drift.
 */
async function collectMessages(
  options: ListMessagesOptions,
  maxPages: number,
  fetchPage: (token: string | null) => Promise<PageResult>,
): Promise<OpenCodeMessage[]> {
  const collected: OpenCodeMessage[] = [];
  const seen = new Set<string>();
  let token: string | null = null;

  for (let page = 0; page < maxPages; page++) {
    const result = await fetchPage(token);
    let reachedCheckpoint = false;
    for (const item of result.items) {
      const message = parseMessageWithParts(item);
      if (message === null || seen.has(message.id)) {
        continue;
      }
      seen.add(message.id);
      collected.push(message);
      if (
        options.afterId !== null &&
        compareIds(message.id, options.afterId) <= 0
      ) {
        reachedCheckpoint = true;
      }
    }
    if (reachedCheckpoint || result.next === null) {
      break;
    }
    token = result.next;
  }

  collected.sort((left, right) => compareIds(left.id, right.id));
  return options.afterId === null
    ? collected
    : collected.filter(
        (message) => compareIds(message.id, options.afterId ?? "") > 0,
      );
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
      return collectMessages(listOptions, maxPages, async (token) => {
        const url = token ?? firstPageUrl(sessionId, pageSize);
        const result = await fetchMessagePage(fetchImpl, url, timeoutMs);
        return { items: result.items, next: result.nextUrl };
      });
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

/**
 * Runs `call` with a real deadline.
 *
 * The call receives an abort signal, and the result races a timer: a client
 * that ignores the signal (or never settles) cannot pin the reconciliation
 * forever. The timer is always cleared, and a call that settles after the
 * deadline is swallowed so it never becomes an unhandled rejection.
 */
async function withTimeout<T>(
  call: (signal: AbortSignal) => Promise<T>,
  timeoutMs: number,
): Promise<T> {
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const deadline = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new Error(`opencode client timed out after ${timeoutMs}ms`));
    }, timeoutMs);
  });
  // If the call throws synchronously the race never observes the deadline;
  // keep it handled so clearing the timer cannot leave a stray rejection.
  deadline.catch(() => {});

  try {
    const pending = call(controller.signal);
    // Discard a late outcome: the race has already settled the caller.
    pending.catch(() => {});
    return await Promise.race([pending, deadline]);
  } finally {
    if (timer !== undefined) {
      clearTimeout(timer);
    }
  }
}

/**
 * Unwraps a plugin client result.
 *
 * A transport error, a non-2xx response or a missing payload is an error, **not
 * an empty page**: a failed read must never masquerade as "no messages" and
 * advance the checkpoint.
 */
function requireClientData(result: PluginClientResult): unknown {
  if (typeof result !== "object" || result === null) {
    throw new Error("opencode client returned no result");
  }
  const response = result.response;
  if (response === undefined || !response.ok) {
    throw new Error(`opencode responded ${response?.status ?? "unknown"}`);
  }
  if (result.error !== undefined && result.error !== null) {
    throw new Error("opencode client returned an error");
  }
  if (result.data === undefined) {
    throw new Error("opencode client returned no data");
  }
  return result.data;
}

/**
 * Unwraps a plugin client result whose payload must be an array.
 *
 * Both native operations (`messages`, `diff`) document an array response;
 * anything else (`null`, an object, a scalar) is an invalid payload and must be
 * an error rather than silently treated as an empty list.
 */
function requireClientArray(
  result: PluginClientResult,
  what: string,
): unknown[] {
  const data = requireClientData(result);
  if (!Array.isArray(data)) {
    throw new Error(`opencode returned a non-array ${what} payload`);
  }
  return data;
}

/** Options for the plugin-client-backed message source. */
export interface PluginClientMessageSourceOptions {
  /** Client supplied by OpenCode, already bound to directory/auth. */
  client: OpenCodePluginClient;
  /** Project directory forwarded as the `directory` query parameter. */
  directory: string;
  /** Request timeout, in milliseconds. */
  timeoutMs?: number;
}

/**
 * Creates a message source backed by the plugin's native OpenCode client.
 *
 * The client runs the request in-process against the server that owns the
 * session (its directory and authorization), so the adapter never re-derives a
 * base URL nor dereferences the pagination `Link`: it only extracts the opaque
 * `before` cursor and re-calls the same client. Pagination, dedup and checkpoint
 * filtering are shared with the HTTP source via [`collectMessages`].
 */
export function createClientMessageSource(
  options: PluginClientMessageSourceOptions,
): MessageSource {
  const timeoutMs = options.timeoutMs ?? 5_000;

  return {
    async listMessages(sessionId, listOptions) {
      const pageSize = Math.max(1, listOptions.limit);
      const maxPages = Math.max(
        1,
        listOptions.maxPages ?? DEFAULT_MAX_MESSAGE_PAGES,
      );
      return collectMessages(listOptions, maxPages, async (before) => {
        const query: Record<string, unknown> = {
          directory: options.directory,
          limit: pageSize,
        };
        if (before !== null) {
          query.before = before;
        }
        const result = await withTimeout(
          (signal) =>
            options.client.session.messages({
              path: { id: sessionId },
              query,
              signal,
            }),
          timeoutMs,
        );
        const items = requireClientArray(result, "messages");
        return {
          items,
          next: nextBeforeFromLink(
            result.response?.headers.get("link") ?? null,
            sessionId,
          ),
        };
      });
    },

    async getDiff(sessionId, messageId) {
      const result = await withTimeout(
        (signal) =>
          options.client.session.diff({
            path: { id: sessionId },
            query: { directory: options.directory, messageID: messageId },
            signal,
          }),
        timeoutMs,
      );
      const data = requireClientArray(result, "diff");
      return data
        .map(parseFileDiff)
        .filter((diff): diff is OpenCodeFileDiff => diff !== null);
    },
  };
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
