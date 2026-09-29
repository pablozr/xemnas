//! Development driver for the PowerShell E2E: serve a synthetic fixture like
//! the OpenCode server and run the **real** adapter pipeline against a
//! discovered local API.
//!
//! This is a tool, not adapter API. It lets the E2E prove the end-to-end path
//! (fixture → adapter → POST or outbox) without a real OpenCode or desktop app,
//! and prints a JSON summary on stdout for assertions. Nothing here is imported
//! by the plugin entry point.
//!
//! Usage:
//!
//! ```text
//! node dist/src/cli/send-fixture.js \
//!   --fixture <dir>        # directory with session.json
//!   [--discovery <path>]   # discovery.json to use; absent/invalid ⇒ outbox
//!   [--state-dir <path>]   # adapter checkpoint/failure state
//!   [--outbox-dir <path>]  # outbox root (default: <data>/outbox)
//!   [--data-dir <path>]    # XEMNAS_DATA_DIR override
//! ```

import { once } from "node:events";
import { existsSync, readFileSync } from "node:fs";
import { createServer, type ServerResponse } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import {
  checkpointPaths,
  createCheckpointStore,
} from "../checkpoint.js";
import {
  createCaptureClient,
  createLocalApiEndpointResolver,
  type CaptureClient,
  type PostResult,
} from "../client.js";
import { resolveConfig } from "../config.js";
import type { CaptureEnvelope } from "../envelope.js";
import { createAdapter, createStderrLog, type AdapterLog } from "../index.js";
import { createOutboxWriter, type OutboxWriter } from "../outbox.js";
import {
  createHttpMessageSource,
  type MessageWithParts,
  type RawFileDiff,
} from "../opencode.js";

/** Fixture document read from `<fixture>/session.json`. */
export interface FixtureDocument {
  session_id: string;
  /** Project directory used as `project.canonical_path`. */
  directory?: string;
  /** Newest page first; each page is a documented `{ info, parts }[]` array. */
  pages: MessageWithParts[][];
  /** Diffs keyed by user message id (documented `FileDiff` items). */
  diffs?: Record<string, RawFileDiff[]>;
}

/** Options accepted by [`runSendFixture`] and the CLI flags. */
export interface SendFixtureOptions {
  /** Directory containing `session.json`. */
  fixtureDir: string;
  /** `discovery.json` to use; absent or invalid ⇒ unavailable ⇒ outbox. */
  discoveryPath?: string;
  /** Adapter checkpoint/failure state directory. */
  stateDir?: string;
  /** Outbox root override (`XEMNAS_OUTBOX_DIR`). */
  outboxDir?: string;
  /** Data directory override (`XEMNAS_DATA_DIR`). */
  dataDir?: string;
  /** Base environment; defaults to `process.env`. */
  env?: NodeJS.ProcessEnv;
  /** Log sink; defaults to a stderr JSON logger. */
  log?: AdapterLog;
}

/** JSON summary printed on stdout for the E2E. */
export interface SendFixtureSummary {
  outcome: "sent" | "outbox" | "rejected_4xx" | "failed" | "none";
  capture_id: string | null;
  idempotency_key: string | null;
  artifacts: Array<{ kind: string; fingerprint: string }>;
  artifact_count: number;
  session_id: string;
  message_id: string | null;
  outbox_path: string | null;
  detail: string | null;
  /** Sanitized error code for a local failure (never content); else `null`. */
  error: string | null;
}

interface FixtureServer {
  url: string;
  close: () => Promise<void>;
}

function loadFixture(fixtureDir: string): FixtureDocument {
  const path = join(fixtureDir, "session.json");
  const parsed = JSON.parse(readFileSync(path, "utf8")) as Record<
    string,
    unknown
  >;
  if (typeof parsed.session_id !== "string" || !Array.isArray(parsed.pages)) {
    throw new Error(`fixture ${path} must declare session_id and pages`);
  }
  return parsed as unknown as FixtureDocument;
}

function indexToCursor(index: number): string {
  return Buffer.from(String(index), "utf8").toString("base64url");
}

function cursorToIndex(cursor: string): number {
  const parsed = Number.parseInt(
    Buffer.from(cursor, "base64url").toString("utf8"),
    10,
  );
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : 0;
}

function writeJson(
  response: ServerResponse,
  status: number,
  body: unknown,
  headers: Record<string, string> = {},
): void {
  response.writeHead(status, { "content-type": "application/json", ...headers });
  response.end(JSON.stringify(body));
}

/** Starts a loopback server that mimics the documented OpenCode endpoints. */
async function startFixtureServer(
  fixture: FixtureDocument,
): Promise<FixtureServer> {
  const server = createServer((request, response) => {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");

    const messageMatch = /^\/session\/([^/]+)\/message$/.exec(url.pathname);
    if (request.method === "GET" && messageMatch !== null) {
      const sessionId = decodeURIComponent(messageMatch[1]);
      if (sessionId !== fixture.session_id) {
        writeJson(response, 404, { error: "session not found" });
        return;
      }
      const before = url.searchParams.get("before");
      const pageIndex = before === null ? 0 : cursorToIndex(before);
      const page = fixture.pages[pageIndex] ?? [];
      const headers: Record<string, string> = {};
      if (pageIndex + 1 < fixture.pages.length) {
        const limit = url.searchParams.get("limit") ?? "50";
        headers.link = `</session/${encodeURIComponent(sessionId)}/message?limit=${limit}&before=${indexToCursor(pageIndex + 1)}>; rel="next"`;
      }
      writeJson(response, 200, page, headers);
      return;
    }

    const diffMatch = /^\/session\/([^/]+)\/diff$/.exec(url.pathname);
    if (request.method === "GET" && diffMatch !== null) {
      const sessionId = decodeURIComponent(diffMatch[1]);
      if (sessionId !== fixture.session_id) {
        writeJson(response, 404, { error: "session not found" });
        return;
      }
      const messageId = url.searchParams.get("messageID") ?? "";
      writeJson(response, 200, fixture.diffs?.[messageId] ?? []);
      return;
    }

    writeJson(response, 404, { error: "not found" });
  });
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address();
  const port =
    typeof address === "object" && address !== null ? address.port : 0;
  return {
    url: `http://127.0.0.1:${port}`,
    close: () =>
      new Promise((done) => {
        server.closeAllConnections?.();
        server.close(() => done());
      }),
  };
}

function buildEnv(
  options: SendFixtureOptions,
  opencodeUrl: string,
): NodeJS.ProcessEnv {
  const env: NodeJS.ProcessEnv = {
    ...(options.env ?? process.env),
    OPENCODE_URL: opencodeUrl,
  };
  if (options.discoveryPath !== undefined) {
    env.XEMNAS_DISCOVERY = options.discoveryPath;
  } else {
    // `--discovery` absent is the deliberate "desktop closed" signal: force an
    // unresolved endpoint instead of inheriting a machine-level discovery file.
    env.XEMNAS_DISCOVERY = join(
      options.stateDir ?? options.dataDir ?? tmpdir(),
      "no-discovery.json",
    );
  }
  if (options.stateDir !== undefined) {
    env.XEMNAS_ADAPTER_STATE_DIR = options.stateDir;
  }
  if (options.outboxDir !== undefined) {
    env.XEMNAS_OUTBOX_DIR = options.outboxDir;
  }
  if (options.dataDir !== undefined) {
    env.XEMNAS_DATA_DIR = options.dataDir;
  } else if (
    options.stateDir !== undefined &&
    env.XEMNAS_DATA_DIR === undefined &&
    env.XEMNAS_OUTBOX_DIR === undefined
  ) {
    // Keep the default outbox next to the adapter state: <data>/outbox.
    env.XEMNAS_DATA_DIR = dirname(resolve(options.stateDir));
  }
  return env;
}

/** In-run observations used to build the summary. */
interface Recording {
  envelope: CaptureEnvelope | null;
  last: PostResult | null;
  /** Result reported by the OutboxWriter for the last pending write. */
  outbox: { path: string | null; error: string | null };
}

function summarize(
  fixture: FixtureDocument,
  recorded: Recording,
): SendFixtureSummary {
  const envelope = recorded.envelope;
  const last = recorded.last;
  let outcome: SendFixtureSummary["outcome"] = "none";
  let detail: string | null = null;
  let error: string | null = null;
  let outboxPath: string | null = null;

  if (last?.kind === "accepted") {
    outcome = "sent";
  } else if (last?.kind === "rejected") {
    outcome = "rejected_4xx";
    detail = `http-${last.status}`;
  } else if (last?.kind === "unavailable") {
    detail = last.reason;
    // Never claim `outbox` without a persisted file: trust the OutboxWriter's
    // own result and confirm the file exists before reporting success.
    if (recorded.outbox.path !== null && existsSync(recorded.outbox.path)) {
      outcome = "outbox";
      outboxPath = recorded.outbox.path;
    } else {
      outcome = "failed";
      error = recorded.outbox.error ?? "outbox-not-persisted";
    }
  }

  return {
    outcome,
    // When the app accepted the capture, report the receipt's id (the app
    // preserves the original on replay); otherwise the envelope's own id.
    capture_id:
      last?.kind === "accepted"
        ? last.captureId
        : (envelope?.capture_id ?? null),
    idempotency_key: envelope?.idempotency_key ?? null,
    artifacts: (envelope?.artifacts ?? []).map((artifact) => ({
      kind: artifact.kind,
      fingerprint: artifact.fingerprint,
    })),
    artifact_count: envelope?.artifacts.length ?? 0,
    session_id: fixture.session_id,
    message_id: envelope?.source.message_id ?? null,
    outbox_path: outboxPath,
    detail,
    error,
  };
}

/**
 * Runs the real adapter once against the fixture server and returns the summary.
 *
 * The capture POST targets the discovered local API (or the outbox when the
 * discovery is absent/invalid); the fixture server only stands in for OpenCode.
 */
export async function runSendFixture(
  options: SendFixtureOptions,
): Promise<SendFixtureSummary> {
  const fixture = loadFixture(options.fixtureDir);
  const server = await startFixtureServer(fixture);
  try {
    const env = buildEnv(options, server.url);
    const config = resolveConfig(env);
    const recorded: Recording = {
      envelope: null,
      last: null,
      outbox: { path: null, error: null },
    };

    const transport = createCaptureClient({
      resolveEndpoint: createLocalApiEndpointResolver(config),
      timeoutMs: config.requestTimeoutMs,
    });
    const client: CaptureClient = {
      async post(candidate: CaptureEnvelope): Promise<PostResult> {
        recorded.envelope = candidate;
        const result = await transport.post(candidate);
        recorded.last = result;
        return result;
      },
    };

    // Observe the real writer result so the summary can tell whether the
    // pending file was actually persisted (the writer never throws by design).
    const realOutbox = createOutboxWriter(config);
    const outbox: OutboxWriter = {
      writePending(candidate: CaptureEnvelope) {
        const result = realOutbox.writePending(candidate);
        recorded.outbox = result.ok
          ? { path: result.path, error: null }
          : { path: null, error: result.reason };
        return result;
      },
    };

    const adapter = createAdapter({
      config,
      source: createHttpMessageSource({
        baseUrl: server.url,
        timeoutMs: config.requestTimeoutMs,
      }),
      client,
      checkpoints: createCheckpointStore(checkpointPaths(config.stateDir)),
      outbox,
      directory: fixture.directory ?? "C:/synthetic/e2e",
      log: options.log ?? createStderrLog(),
      now: () => new Date(),
    });

    await adapter.reconcileSession(fixture.session_id);
    return summarize(fixture, recorded);
  } finally {
    await server.close();
  }
}

/** Parsed CLI options. */
export interface CliOptions {
  fixtureDir: string;
  discoveryPath?: string;
  stateDir?: string;
  outboxDir?: string;
  dataDir?: string;
}

function requireValue(flag: string, value: string | undefined): string {
  if (value === undefined || value.startsWith("--")) {
    throw new Error(`${flag} requires a value`);
  }
  return value;
}

/** Parses the CLI arguments (see the module header). */
export function parseArgs(argv: string[]): CliOptions {
  let fixtureDir: string | undefined;
  let discoveryPath: string | undefined;
  let stateDir: string | undefined;
  let outboxDir: string | undefined;
  let dataDir: string | undefined;
  for (let index = 0; index < argv.length; index++) {
    const flag = argv[index];
    const value = argv[index + 1];
    switch (flag) {
      case "--fixture":
        fixtureDir = requireValue(flag, value);
        index++;
        break;
      case "--discovery":
        discoveryPath = requireValue(flag, value);
        index++;
        break;
      case "--state-dir":
        stateDir = requireValue(flag, value);
        index++;
        break;
      case "--outbox-dir":
        outboxDir = requireValue(flag, value);
        index++;
        break;
      case "--data-dir":
        dataDir = requireValue(flag, value);
        index++;
        break;
      default:
        throw new Error(`unknown argument: ${flag}`);
    }
  }
  if (fixtureDir === undefined) {
    throw new Error("--fixture <dir> is required");
  }
  return { fixtureDir, discoveryPath, stateDir, outboxDir, dataDir };
}

/** Parses the flags, runs the fixture once and prints the JSON summary. */
export async function main(
  argv: string[] = process.argv.slice(2),
): Promise<number> {
  try {
    const summary = await runSendFixture(parseArgs(argv));
    process.stdout.write(`${JSON.stringify(summary)}\n`);
    // A local failure (for example an outbox write that did not persist) must
    // not look like success to the caller.
    return summary.outcome === "failed" ? 1 : 0;
  } catch (error) {
    process.stderr.write(
      `send-fixture failed: ${error instanceof Error ? error.message : "error"}\n`,
    );
    return 1;
  }
}

const invokedPath = process.argv[1];
if (
  invokedPath !== undefined &&
  import.meta.url === pathToFileURL(invokedPath).href
) {
  void main().then((code) => {
    process.exitCode = code;
  });
}
