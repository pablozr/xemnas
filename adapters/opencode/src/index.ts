//! OpenCode adapter plugin: idle trigger → reconcile → validate → POST.
//!
//! This is the thin wiring layer of the adapter. It holds no domain rules and
//! never calls a model (MVP-SPEC §7.1, stack doc "Adapter do OpenCode"): its
//! responsibility ends after delivering one valid capture envelope.
//!
//! The event hook only filters idle events, applies a short debounce and
//! enqueues background work, so it returns immediately — the network POST is
//! never awaited on the hook.

import { checkpointPaths, createCheckpointStore, type CheckpointStore } from "./checkpoint.js";
import {
  createCaptureClient,
  createLocalApiEndpointResolver,
  type CaptureClient,
} from "./client.js";
import {
  resolveConfig,
  type AdapterConfig,
} from "./config.js";
import {
  createChatMessageHook,
  createContextClient,
  type ChatMessageHook,
} from "./context.js";
import { buildEnvelope, validateEnvelope } from "./envelope.js";
import {
  createOutboxWriter,
  type OutboxWriter,
} from "./outbox.js";
import {
  createHttpMessageSource,
  isExtractionSession,
  type MessageSource,
  type OpenCodeFileDiff,
  type OpenCodeMessage,
} from "./opencode.js";

/** Structured, sanitized adapter log. Fields must never carry content/tokens. */
export interface AdapterLog {
  info(event: string, fields?: Record<string, string | number>): void;
  warn(event: string, fields?: Record<string, string | number>): void;
  error(event: string, fields?: Record<string, string | number>): void;
}

/**
 * Creates a stderr JSON logger.
 *
 * Only primitive fields are emitted, and callers pass ids/counts/status codes
 * exclusively (PRIV-001, MVP-SPEC §14): tokens and message content are never
 * logged.
 */
export function createStderrLog(
  write: (line: string) => void = (line) => {
    process.stderr.write(line);
  },
): AdapterLog {
  const emit =
    (level: string) =>
    (event: string, fields: Record<string, string | number> = {}): void => {
      const safe: Record<string, string | number> = {};
      for (const [key, value] of Object.entries(fields)) {
        if (typeof value === "string" || typeof value === "number") {
          safe[key] = value;
        }
      }
      write(`${JSON.stringify({ level, event, ...safe })}\n`);
    };
  return { info: emit("info"), warn: emit("warn"), error: emit("error") };
}

/** A structural view of the OpenCode plugin event. */
export interface OpenCodeEvent {
  type: string;
  properties?: unknown;
}

/**
 * Minimal structural OpenCode plugin contract.
 *
 * Mirrors `@opencode-ai/plugin` (`Plugin = (input) => Promise<Hooks>`) as
 * documented at <https://opencode.ai/docs/plugins.md>; declared locally to avoid
 * pulling the SDK into this thin adapter.
 */
export interface OpenCodePluginInput {
  directory: string;
  worktree?: string;
  client?: unknown;
  project?: unknown;
  $?: unknown;
}

/** Input passed to the plugin's `event` hook. */
export interface OpenCodePluginEventInput {
  event: OpenCodeEvent;
}

/** Hooks returned by the plugin factory. */
export interface OpenCodePluginHooks {
  event?: (input: OpenCodePluginEventInput) => Promise<void> | void;
  "chat.message"?: ChatMessageHook;
}

/** The OpenCode plugin signature. */
export type OpenCodePlugin = (
  input: OpenCodePluginInput,
) => Promise<OpenCodePluginHooks>;

/** Dependencies of [`createAdapter`], all injectable for hermetic tests. */
export interface AdapterDeps {
  config: AdapterConfig;
  source: MessageSource;
  client: CaptureClient;
  checkpoints: CheckpointStore;
  /**
   * Persists envelopes when the local API is unavailable. Defaults to the real
   * filesystem writer rooted at `config.pendingDir`.
   */
  outbox?: OutboxWriter;
  /** Project directory from the plugin input; canonicalization is the server's. */
  directory: string;
  log?: AdapterLog;
  now?: () => Date;
  /**
   * Schedules a debounced task and returns a cancel function. The default uses
   * `setTimeout`/`clearTimeout`; tests inject a deterministic scheduler.
   */
  schedule?: (task: () => void, delayMs: number) => () => void;
}

/** One turn: a user message and the assistant responses that follow it. */
interface Turn {
  user: OpenCodeMessage;
  assistants: OpenCodeMessage[];
}

/** Summary of one reconciliation pass. */
export interface SessionOutcome {
  session_id: string;
  sent: number;
  skipped: number;
  stopped: boolean;
}

/** The adapter's runtime surface. */
export interface Adapter {
  /** Handles one plugin event, returning immediately. */
  handleEvent(event: OpenCodeEvent): void;
  /** Reconciles one session to completion (used directly by tests). */
  reconcileSession(sessionId: string): Promise<SessionOutcome>;
  /** Resolves when the session's in-flight work (including coalesced reruns) ends. */
  whenIdle(sessionId: string): Promise<void>;
}

/**
 * Returns the session id when `event` is an idle signal, else `null`.
 *
 * Accepts `session.status` with an `idle` status (recommended) and the
 * deprecated-but-emitted `session.idle` (MVP-SPEC §7.1). The event property
 * `sessionID` is the only field read; shapes are documented at
 * <https://opencode.ai/docs/plugins.md>.
 */
export function idleSessionId(event: OpenCodeEvent): string | null {
  if (typeof event?.type !== "string") {
    return null;
  }
  const properties = (event.properties ?? {}) as Record<string, unknown>;
  const sessionId =
    typeof properties.sessionID === "string" ? properties.sessionID : null;
  if (sessionId === null) {
    return null;
  }
  if (event.type === "session.idle") {
    return sessionId;
  }
  if (event.type === "session.status") {
    const status = properties.status;
    if (typeof status === "object" && status !== null) {
      if ((status as Record<string, unknown>).type === "idle") {
        return sessionId;
      }
    }
  }
  return null;
}

/** Groups ascending messages into user turns with their child responses. */
function buildTurns(messages: OpenCodeMessage[]): Turn[] {
  const turns: Turn[] = [];
  let current: Turn | null = null;
  for (const message of messages) {
    if (message.role === "user") {
      if (current !== null) {
        turns.push(current);
      }
      current = { user: message, assistants: [] };
    } else if (message.role === "assistant" && current !== null) {
      current.assistants.push(message);
    }
  }
  if (current !== null) {
    turns.push(current);
  }
  return turns;
}

/** Creates the adapter from injected dependencies. */
export function createAdapter(deps: AdapterDeps): Adapter {
  const log = deps.log ?? createStderrLog();
  const now = deps.now ?? (() => new Date());
  const outbox = deps.outbox ?? createOutboxWriter(deps.config);
  const schedule =
    deps.schedule ??
    ((task: () => void, delayMs: number): (() => void) => {
      const timer = setTimeout(task, delayMs);
      return () => clearTimeout(timer);
    });

  async function reconcileSession(sessionId: string): Promise<SessionOutcome> {
    // Extraction sessions created by the xemnas desktop are never captured
    // (ADR-0004). An unreadable title does not block a real capture.
    if (deps.source.getSessionTitle !== undefined) {
      let title: string | null = null;
      try {
        title = await deps.source.getSessionTitle(sessionId);
      } catch {
        title = null;
      }
      if (isExtractionSession(title)) {
        log.info("extraction-session-skipped", { session_id: sessionId });
        return { session_id: sessionId, sent: 0, skipped: 0, stopped: false };
      }
    }

    const checkpoint = deps.checkpoints.get(sessionId);

    let messages: OpenCodeMessage[];
    try {
      messages = await deps.source.listMessages(sessionId, {
        afterId: checkpoint.last_message_id,
        limit: deps.config.messageLimit,
        maxPages: deps.config.maxMessagePages,
      });
    } catch {
      log.warn("messages-unavailable", { session_id: sessionId });
      return { session_id: sessionId, sent: 0, skipped: 0, stopped: true };
    }

    const turns = buildTurns(messages);
    let sent = 0;
    let skipped = 0;

    for (const turn of turns) {
      let diffs: OpenCodeFileDiff[] = [];
      try {
        diffs = await deps.source.getDiff(sessionId, turn.user.id);
      } catch {
        log.warn("diff-unavailable", {
          session_id: sessionId,
          message_id: turn.user.id,
        });
      }

      const envelope = buildEnvelope({
        sessionId,
        directory: deps.directory,
        userMessage: turn.user,
        assistantMessages: turn.assistants,
        diffs,
        limits: {
          maxArtifactBytes: deps.config.maxArtifactBytes,
          maxArtifacts: deps.config.maxArtifacts,
          maxDiffBytes: deps.config.maxDiffBytes,
        },
        now: now(),
      });

      if (envelope === null) {
        // Nothing capturable in this turn; advance so it is not rescanned.
        deps.checkpoints.advance(sessionId, turn.user.id);
        skipped += 1;
        continue;
      }

      const validation = validateEnvelope(envelope);
      if (!validation.valid) {
        log.error("envelope-invalid", {
          session_id: sessionId,
          message_id: turn.user.id,
          error_count: validation.errorCount,
        });
        return { session_id: sessionId, sent, skipped, stopped: true };
      }

      const result = await deps.client.post(envelope);
      if (result.kind === "accepted") {
        deps.checkpoints.advance(sessionId, turn.user.id, result.captureId);
        sent += 1;
        log.info("capture-accepted", {
          session_id: sessionId,
          message_id: turn.user.id,
          status: result.status,
        });
        continue;
      }

      if (result.kind === "rejected") {
        deps.checkpoints.recordFailure(sessionId, {
          at: now().toISOString(),
          message_id: turn.user.id,
          status: result.status,
          reason: `http-${result.status}`,
        });
        log.warn("capture-rejected", {
          session_id: sessionId,
          message_id: turn.user.id,
          status: result.status,
        });
        return { session_id: sessionId, sent, skipped, stopped: true };
      }

      // Local API unavailable: persist the envelope in the outbox. The
      // checkpoint stays put, so a later idle re-sends the same key and the app
      // deduplicates whichever copy arrives first (MVP-SPEC §7.2).
      log.warn("app-unavailable", {
        session_id: sessionId,
        reason: result.reason,
      });
      const written = outbox.writePending(envelope);
      if (written.ok) {
        log.info("outbox-pending", {
          session_id: sessionId,
          message_id: turn.user.id,
        });
      } else {
        deps.checkpoints.recordFailure(sessionId, {
          at: now().toISOString(),
          message_id: turn.user.id,
          status: null,
          reason: "outbox-write",
        });
        log.error("outbox-write-failed", {
          session_id: sessionId,
          message_id: turn.user.id,
        });
      }
      return { session_id: sessionId, sent, skipped, stopped: true };
    }

    return { session_id: sessionId, sent, skipped, stopped: false };
  }

  const pendingTimers = new Map<string, () => void>();

  function handleEvent(event: OpenCodeEvent): void {
    const sessionId = idleSessionId(event);
    if (sessionId === null) {
      return;
    }
    // Short debounce: a newer idle replaces the pending one for this session.
    pendingTimers.get(sessionId)?.();
    pendingTimers.set(
      sessionId,
      schedule(() => {
        pendingTimers.delete(sessionId);
        startSession(sessionId);
      }, deps.config.debounceMs),
    );
  }

  // One in-flight reconciliation per session, with coalescing: an idle signal
  // that arrives while a session is running schedules exactly one more pass
  // after it finishes, so overlapping idles cannot run concurrently or end out
  // of order (which would regress the checkpoint).
  interface SessionTask {
    running: boolean;
    rerun: boolean;
    active: Promise<void> | null;
  }
  const tasks = new Map<string, SessionTask>();

  function startSession(sessionId: string): void {
    const state = tasks.get(sessionId) ?? {
      running: false,
      rerun: false,
      active: null,
    };
    tasks.set(sessionId, state);
    if (state.running) {
      state.rerun = true;
      return;
    }
    state.running = true;
    state.active = (async () => {
      try {
        do {
          state.rerun = false;
          await reconcileSession(sessionId);
        } while (state.rerun);
      } catch {
        log.error("reconcile-failed", { session_id: sessionId });
      } finally {
        state.running = false;
        state.active = null;
      }
    })();
  }

  function whenIdle(sessionId: string): Promise<void> {
    return tasks.get(sessionId)?.active ?? Promise.resolve();
  }

  return { handleEvent, reconcileSession, whenIdle };
}

/**
 * Builds an [`OpenCodePlugin`] from an adapter factory.
 *
 * The factory receives the plugin input's `directory`, which becomes the
 * envelope's `project.canonical_path`; canonicalization and the allow-list
 * check happen on the server (MVP-SPEC §7.3).
 */
export function createOpenCodePlugin(
  buildAdapter: (directory: string) => Adapter,
  buildChatHook?: (directory: string) => ChatMessageHook,
): OpenCodePlugin {
  return async (input: OpenCodePluginInput): Promise<OpenCodePluginHooks> => {
    const directory =
      typeof input.directory === "string" ? input.directory : "";
    const adapter = buildAdapter(directory);
    const hooks: OpenCodePluginHooks = {
      event: async ({ event }): Promise<void> => {
        adapter.handleEvent(event);
      },
    };
    if (buildChatHook !== undefined) {
      hooks["chat.message"] = buildChatHook(directory);
    }
    return hooks;
  };
}

function buildDefaultAdapter(
  config: AdapterConfig,
  directory: string,
): Adapter {
  return createAdapter({
    config,
    directory,
    source: createHttpMessageSource({
      baseUrl: config.opencodeUrl,
      timeoutMs: config.requestTimeoutMs,
    }),
    client: createCaptureClient({
      resolveEndpoint: createLocalApiEndpointResolver(config),
      timeoutMs: config.requestTimeoutMs,
    }),
    checkpoints: createCheckpointStore(checkpointPaths(config.stateDir)),
  });
}

function buildDefaultChatHook(
  config: AdapterConfig,
  directory: string,
): ChatMessageHook {
  return createChatMessageHook({
    directory,
    client: createContextClient({
      resolveEndpoint: createLocalApiEndpointResolver(config),
      timeoutMs: config.contextTimeoutMs,
    }),
    log: createStderrLog(),
  });
}

/** OpenCode plugin entry point. */
export const XemnasOpenCodeAdapter: OpenCodePlugin = (input) => {
  const config = resolveConfig();
  return createOpenCodePlugin(
    (directory) => buildDefaultAdapter(config, directory),
    (directory) => buildDefaultChatHook(config, directory),
  )(input);
};
