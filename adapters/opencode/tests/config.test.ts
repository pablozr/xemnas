//! Configuration and discovery resolution tests.

import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

import {
  adapterVersion,
  defaultDataDir,
  EXPECTED_PROTOCOL_VERSION,
  readDiscovery,
  readToken,
  resolveConfig,
} from "../src/config.js";

test("resolveConfig honors XEMNAS_DATA_DIR for the default locations", () => {
  const dataDir = join(tmpdir(), "xemnas-config-data");
  const config = resolveConfig({ XEMNAS_DATA_DIR: dataDir });
  assert.equal(defaultDataDir({ XEMNAS_DATA_DIR: dataDir }), dataDir);
  assert.equal(
    config.discoveryPath,
    join(dataDir, "state", "discovery.json"),
  );
  assert.equal(config.stateDir, join(dataDir, "adapter"));
  assert.equal(config.checkpointsPath, join(dataDir, "adapter", "checkpoints.json"));
  assert.equal(config.failuresPath, join(dataDir, "adapter", "failures.json"));
});

test("resolveConfig lets XEMNAS_DISCOVERY move the token sibling", () => {
  const discoveryPath = join(tmpdir(), "custom-runtime", "discovery.json");
  const config = resolveConfig({
    XEMNAS_DATA_DIR: join(tmpdir(), "unused"),
    XEMNAS_DISCOVERY: discoveryPath,
  });
  assert.equal(config.discoveryPath, discoveryPath);
  assert.equal(config.tokenPath, join(tmpdir(), "custom-runtime", "api-token"));
});

test("resolveConfig honors XEMNAS_ADAPTER_STATE_DIR", () => {
  const stateDir = join(tmpdir(), "custom-adapter-state");
  const config = resolveConfig({
    XEMNAS_DATA_DIR: join(tmpdir(), "unused"),
    XEMNAS_ADAPTER_STATE_DIR: stateDir,
  });
  assert.equal(config.stateDir, stateDir);
});

test("resolveConfig applies OPENCODE_URL without a trailing slash", () => {
  assert.equal(
    resolveConfig({ XEMNAS_DATA_DIR: join(tmpdir(), "d") }).opencodeUrl,
    "http://127.0.0.1:4096",
  );
  assert.equal(
    resolveConfig({
      XEMNAS_DATA_DIR: join(tmpdir(), "d"),
      OPENCODE_URL: "http://127.0.0.1:9999/",
    }).opencodeUrl,
    "http://127.0.0.1:9999",
  );
});

test("resolveConfig honors limit, debounce and timeout overrides", () => {
  const config = resolveConfig({
    XEMNAS_DATA_DIR: join(tmpdir(), "d"),
    XEMNAS_ADAPTER_DEBOUNCE_MS: "25",
    XEMNAS_ADAPTER_TIMEOUT_MS: "1500",
    XEMNAS_ADAPTER_MAX_ARTIFACT_BYTES: "1024",
    XEMNAS_ADAPTER_MAX_ARTIFACTS: "7",
    XEMNAS_ADAPTER_MAX_DIFF_BYTES: "2048",
    XEMNAS_ADAPTER_MESSAGE_LIMIT: "42",
    XEMNAS_ADAPTER_MAX_MESSAGE_PAGES: "3",
  });
  assert.equal(config.debounceMs, 25);
  assert.equal(config.requestTimeoutMs, 1500);
  assert.equal(config.maxArtifactBytes, 1024);
  assert.equal(config.maxArtifacts, 7);
  assert.equal(config.maxDiffBytes, 2048);
  assert.equal(config.messageLimit, 42);
  assert.equal(config.maxMessagePages, 3);
});

test("readDiscovery parses a valid file and rejects malformed ones", () => {
  const dir = mkdtempSync(join(tmpdir(), "xemnas-discovery-test-"));
  const discoveryPath = join(dir, "discovery.json");

  writeFileSync(
    discoveryPath,
    JSON.stringify({
      protocol_version: EXPECTED_PROTOCOL_VERSION,
      port: 4321,
      instance_id: "instance-1",
    }),
    "utf8",
  );
  assert.deepEqual(readDiscovery({ discoveryPath }), {
    protocol_version: 1,
    port: 4321,
    instance_id: "instance-1",
  });

  writeFileSync(discoveryPath, JSON.stringify({ port: 4321 }), "utf8");
  assert.equal(readDiscovery({ discoveryPath }), null);
  assert.equal(readDiscovery({ discoveryPath: join(dir, "missing.json") }), null);
});

test("readToken trims the token and treats an empty file as missing", () => {
  const dir = mkdtempSync(join(tmpdir(), "xemnas-token-test-"));
  const tokenPath = join(dir, "api-token");

  writeFileSync(tokenPath, "secret-token\n", "utf8");
  assert.equal(readToken({ tokenPath }), "secret-token");

  writeFileSync(tokenPath, "   \n", "utf8");
  assert.equal(readToken({ tokenPath }), null);
  assert.equal(readToken({ tokenPath: join(dir, "missing") }), null);
});

test("adapterVersion matches the package version", () => {
  assert.equal(adapterVersion(), "0.0.0");
});
