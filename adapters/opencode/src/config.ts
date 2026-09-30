//! Adapter configuration and discovery resolution.
//!
//! The adapter is a *thin plugin*: it holds no domain rules and no AI. This
//! module only turns the environment and the local API discovery files into an
//! [`AdapterConfig`]; everything else is wiring performed by `index.ts`.
//!
//! Locations mirror the Rust side (`crates/storage-sqlite` `default_data_dir`):
//! `XEMNAS_DATA_DIR` wins, then `%LOCALAPPDATA%\xemnas`, then the system temp
//! directory. The discovery file and its sibling `api-token` are written by the
//! local API (ticket 09): `{protocol_version, port, instance_id}` plus a raw
//! bearer token (`crates/local-api/src/discovery.rs`).

import { readFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join } from "node:path";

/** Adapter name written into every envelope's `source.adapter`. */
export const ADAPTER_NAME = "opencode";

/** Package name used to locate the adapter root at runtime. */
export const PACKAGE_NAME = "xemnas-adapter-opencode";

/**
 * Protocol version this adapter speaks with the local API.
 *
 * Mirrors `PROTOCOL_VERSION` in `crates/local-api/src/discovery.rs`. A discovery
 * file advertising another version is treated as unavailable rather than
 * guessed at.
 */
export const EXPECTED_PROTOCOL_VERSION = 1;

/**
 * Default OpenCode server base URL.
 *
 * **Assumption (configurable):** the OpenCode HTTP server listens on
 * `127.0.0.1:4096` by default (official server docs:
 * <https://opencode.ai/docs/server/>). It is overridable with `OPENCODE_URL`
 * because the port is not part of this project's contract.
 */
export const DEFAULT_OPENCODE_URL = "http://127.0.0.1:4096";

/** Debounce applied to idle events before reconciliation starts. */
export const DEFAULT_DEBOUNCE_MS = 1_000;
/** Timeout for one `POST /v1/captures` attempt. */
export const DEFAULT_REQUEST_TIMEOUT_MS = 5_000;
/** Maximum UTF-8 bytes for a single artifact `content`. */
export const DEFAULT_MAX_ARTIFACT_BYTES = 64 * 1024;
/** Maximum number of artifacts kept per envelope. */
export const DEFAULT_MAX_ARTIFACTS = 64;
/** Maximum UTF-8 bytes of diff content kept per envelope (reduced to hunks). */
export const DEFAULT_MAX_DIFF_BYTES = 32 * 1024;
/** Maximum number of messages fetched per reconciliation. */
export const DEFAULT_MESSAGE_LIMIT = 200;
/**
 * Maximum backward pages fetched per reconciliation.
 *
 * Bounds how far the adapter pages back looking for the checkpoint, so a very
 * long session cannot make one idle event unbounded.
 */
export const DEFAULT_MAX_MESSAGE_PAGES = 10;

/** Default wait for a context block before the turn proceeds without it. */
export const DEFAULT_CONTEXT_TIMEOUT_MS = 300;

/** Resolved, immutable adapter configuration. */
export interface AdapterConfig {
  /** OpenCode server base URL, no trailing slash. */
  opencodeUrl: string;
  /** Absolute path of `discovery.json`. */
  discoveryPath: string;
  /** Absolute path of the sibling `api-token` file. */
  tokenPath: string;
  /** Directory holding this adapter's own state files. */
  stateDir: string;
  /** Absolute path of `checkpoints.json`. */
  checkpointsPath: string;
  /** Absolute path of `failures.json`. */
  failuresPath: string;
  /**
   * Root of the outbox (`outbox/`), sibling of `state/`.
   *
   * The adapter only writes `pending/`; the app owns the other transitions
   * (`sending/`, `accepted/`, `rejected/`) during import.
   */
  outboxDir: string;
  /** Absolute path of the adapter's `outbox/pending` directory. */
  pendingDir: string;
  /** Idle debounce, in milliseconds. */
  debounceMs: number;
  /** HTTP timeout per capture POST, in milliseconds. */
  requestTimeoutMs: number;
  /** Maximum UTF-8 bytes per artifact content. */
  maxArtifactBytes: number;
  /** Maximum artifacts per envelope. */
  maxArtifacts: number;
  /** Maximum UTF-8 bytes of diff content per envelope. */
  maxDiffBytes: number;
  /** Maximum messages fetched per reconciliation. */
  messageLimit: number;
  /** Maximum backward pages fetched per reconciliation. */
  maxMessagePages: number;
  /** Maximum wait for a context block, in milliseconds. */
  contextTimeoutMs: number;
}

/** Contents of `discovery.json` written by the local API. */
export interface DiscoveryInfo {
  /** Protocol version the server speaks. */
  protocol_version: number;
  /** Bound loopback port. */
  port: number;
  /** Per-session instance identifier. */
  instance_id: string;
}

function envValue(
  env: NodeJS.ProcessEnv,
  name: string,
): string | undefined {
  const value = env[name];
  return value !== undefined && value.length > 0 ? value : undefined;
}

function envInt(
  env: NodeJS.ProcessEnv,
  name: string,
  fallback: number,
): number {
  const raw = envValue(env, name);
  if (raw === undefined) {
    return fallback;
  }
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) && parsed >= 0 ? parsed : fallback;
}

function stripTrailingSlash(value: string): string {
  return value.endsWith("/") ? value.slice(0, -1) : value;
}

/**
 * Returns the platform data directory, mirroring the Rust default.
 *
 * `XEMNAS_DATA_DIR` wins; otherwise `%LOCALAPPDATA%\xemnas` on Windows and the
 * system temp directory elsewhere (the Rust fallback uses `temp_dir`).
 */
export function defaultDataDir(
  env: NodeJS.ProcessEnv = process.env,
  home: string = homedir(),
): string {
  const override = envValue(env, "XEMNAS_DATA_DIR");
  if (override !== undefined) {
    return override;
  }
  const localAppData = envValue(env, "LOCALAPPDATA");
  if (localAppData !== undefined) {
    return join(localAppData, "xemnas");
  }
  // Non-Windows fallback. `home` keeps the function deterministic in tests.
  return join(tmpdir() || home, "xemnas");
}

/** Resolves the adapter configuration from the environment. */
export function resolveConfig(
  env: NodeJS.ProcessEnv = process.env,
): AdapterConfig {
  const dataDir = defaultDataDir(env);
  const discoveryPath =
    envValue(env, "XEMNAS_DISCOVERY") ??
    join(dataDir, "state", "discovery.json");
  const stateDir =
    envValue(env, "XEMNAS_ADAPTER_STATE_DIR") ?? join(dataDir, "adapter");
  const outboxDir =
    envValue(env, "XEMNAS_OUTBOX_DIR") ?? join(dataDir, "outbox");
  return {
    opencodeUrl: stripTrailingSlash(
      envValue(env, "OPENCODE_URL") ?? DEFAULT_OPENCODE_URL,
    ),
    discoveryPath,
    tokenPath: join(dirname(discoveryPath), "api-token"),
    stateDir,
    checkpointsPath: join(stateDir, "checkpoints.json"),
    failuresPath: join(stateDir, "failures.json"),
    outboxDir,
    pendingDir: join(outboxDir, "pending"),
    debounceMs: envInt(env, "XEMNAS_ADAPTER_DEBOUNCE_MS", DEFAULT_DEBOUNCE_MS),
    requestTimeoutMs: envInt(
      env,
      "XEMNAS_ADAPTER_TIMEOUT_MS",
      DEFAULT_REQUEST_TIMEOUT_MS,
    ),
    maxArtifactBytes: envInt(
      env,
      "XEMNAS_ADAPTER_MAX_ARTIFACT_BYTES",
      DEFAULT_MAX_ARTIFACT_BYTES,
    ),
    maxArtifacts: envInt(
      env,
      "XEMNAS_ADAPTER_MAX_ARTIFACTS",
      DEFAULT_MAX_ARTIFACTS,
    ),
    maxDiffBytes: envInt(
      env,
      "XEMNAS_ADAPTER_MAX_DIFF_BYTES",
      DEFAULT_MAX_DIFF_BYTES,
    ),
    messageLimit: envInt(
      env,
      "XEMNAS_ADAPTER_MESSAGE_LIMIT",
      DEFAULT_MESSAGE_LIMIT,
    ),
    maxMessagePages: envInt(
      env,
      "XEMNAS_ADAPTER_MAX_MESSAGE_PAGES",
      DEFAULT_MAX_MESSAGE_PAGES,
    ),
    contextTimeoutMs: envInt(
      env,
      "XEMNAS_CONTEXT_TIMEOUT_MS",
      DEFAULT_CONTEXT_TIMEOUT_MS,
    ),
  };
}

/**
 * Reads and validates `discovery.json`.
 *
 * Returns `null` on any I/O or shape error so a missing/partial file is treated
 * as "local app unavailable" rather than crashing the plugin.
 */
export function readDiscovery(
  config: Pick<AdapterConfig, "discoveryPath">,
): DiscoveryInfo | null {
  try {
    const parsed = JSON.parse(
      readFileSync(config.discoveryPath, "utf8"),
    ) as Record<string, unknown>;
    if (
      typeof parsed.protocol_version !== "number" ||
      typeof parsed.port !== "number" ||
      typeof parsed.instance_id !== "string"
    ) {
      return null;
    }
    return {
      protocol_version: parsed.protocol_version,
      port: parsed.port,
      instance_id: parsed.instance_id,
    };
  } catch {
    return null;
  }
}

/** Reads the bearer token; returns `null` when missing or empty. */
export function readToken(
  config: Pick<AdapterConfig, "tokenPath">,
): string | null {
  try {
    const token = readFileSync(config.tokenPath, "utf8").trim();
    return token.length > 0 ? token : null;
  } catch {
    return null;
  }
}

/**
 * Walks up from `start` until the adapter's `package.json` is found.
 *
 * The compiled modules live under `dist/src/`, one level deeper than the
 * source, so the root is discovered rather than hardcoded.
 */
export function packageRoot(start: string = import.meta.dirname): string {
  let current = start;
  for (;;) {
    try {
      const parsed = JSON.parse(
        readFileSync(join(current, "package.json"), "utf8"),
      ) as { name?: unknown };
      if (parsed.name === PACKAGE_NAME) {
        return current;
      }
    } catch {
      // Not this directory; keep walking.
    }
    const parent = dirname(current);
    if (parent === current) {
      throw new Error("adapter package root not found");
    }
    current = parent;
  }
}

let cachedVersion: string | null = null;

/** Returns the adapter version from `package.json` (cached). */
export function adapterVersion(): string {
  if (cachedVersion !== null) {
    return cachedVersion;
  }
  try {
    const parsed = JSON.parse(
      readFileSync(join(packageRoot(), "package.json"), "utf8"),
    ) as { version?: unknown };
    cachedVersion = typeof parsed.version === "string" ? parsed.version : "0.0.0";
  } catch {
    cachedVersion = "0.0.0";
  }
  return cachedVersion;
}
