//! TypeScript half of the Capture Envelope contract.
//!
//! The JSON Schema generated from the Rust types
//! (`adapters/opencode/schemas/capture-envelope.schema.json`) is the single
//! source of truth. This suite validates the shared fixtures in
//! `tests/fixtures/capture/` with Ajv in draft 2020-12 mode, mirroring the Rust
//! `jsonschema` validator (`should_validate_formats(true)`): the filename
//! prefix is the contract between the two sides.
//!
//! - `valid-*`        — schema accepts;
//! - `invalid-*`      — schema rejects (extra field, wrong type, bad pattern, ...);
//! - `incomplete-*`   — a required field is missing;
//! - `incompatible-*` — `schema_version` is not the supported `1`.

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { describe, test } from "node:test";

import type { AnySchema } from "ajv";
import { Ajv2020 } from "ajv/dist/2020.js";
import type { FormatsPlugin } from "ajv-formats";

// `ajv` and `ajv-formats` are CommonJS packages. Under `moduleResolution:
// nodenext` an ESM `import Ajv2020 from "ajv/dist/2020.js"` /
// `import addFormats from "ajv-formats"` is typed as the CJS namespace, not a
// constructor/callable. `Ajv2020` is available as a named export; the
// `formatsPlugin` default has no named export, so it is loaded explicitly and
// narrowed to its real `FormatsPlugin` type.
const require = createRequire(import.meta.url);
const addFormats = require("ajv-formats") as FormatsPlugin;

/**
 * Walk up from `start` until a directory holding `Cargo.toml` is found.
 *
 * Tests must not depend on the process `cwd`: the compiled file lives in
 * `dist/tests/`, a level deeper than the TypeScript source, so the repo root is
 * derived from `import.meta.dirname` instead.
 */
function findRepoRoot(start: string): string {
  let current = start;
  for (;;) {
    if (existsSync(join(current, "Cargo.toml"))) {
      return current;
    }
    const parent = dirname(current);
    if (parent === current) {
      throw new Error(`no Cargo.toml found above ${start}`);
    }
    current = parent;
  }
}

const repoRoot = findRepoRoot(import.meta.dirname);
const fixturesDir = join(repoRoot, "tests", "fixtures", "capture");
const schemaPath = join(
  repoRoot,
  "adapters",
  "opencode",
  "schemas",
  "capture-envelope.schema.json",
);

const schemaDocument = JSON.parse(
  readFileSync(schemaPath, "utf8"),
) as Record<string, unknown>;

// Ajv 8 defaults to draft 2020-12 for `Ajv2020`, `validateFormats: true` and
// `strict: true`; all three are stated explicitly so the configuration is
// self-documenting. Strict mode compiles the schemars-generated schema as-is
// (no option had to be relaxed). `validateFormats` keeps `observed_at`'s
// `format: date-time` asserted, matching the Rust side.
const ajv = new Ajv2020({ validateFormats: true, strict: true });
addFormats(ajv);
const validate = ajv.compile(schemaDocument as AnySchema);

const fixtureNames = readdirSync(fixturesDir)
  .filter((name) => name.endsWith(".json"))
  .sort();

const validFixtures = fixtureNames.filter((name) => name.startsWith("valid-"));
const invalidFixtures = fixtureNames.filter((name) => name.startsWith("invalid-"));
const incompleteFixtures = fixtureNames.filter((name) =>
  name.startsWith("incomplete-"),
);
const incompatibleFixtures = fixtureNames.filter((name) =>
  name.startsWith("incompatible-"),
);
const rejectedFixtures = [
  ...invalidFixtures,
  ...incompleteFixtures,
  ...incompatibleFixtures,
];

function readFixture(name: string): unknown {
  return JSON.parse(readFileSync(join(fixturesDir, name), "utf8"));
}

describe("capture envelope contract", () => {
  test("the versioned schema declares JSON Schema draft 2020-12", () => {
    assert.equal(
      schemaDocument.$schema,
      "https://json-schema.org/draft/2020-12/schema",
    );
  });

  test("the fixture corpus covers every required shape", () => {
    assert.ok(
      validFixtures.length >= 3,
      `expected at least 3 valid fixtures, found ${validFixtures.length}`,
    );
    assert.ok(
      invalidFixtures.length >= 7,
      `expected at least 7 invalid fixtures, found ${invalidFixtures.length}`,
    );
    assert.ok(
      incompleteFixtures.length >= 3,
      `expected at least 3 incomplete fixtures, found ${incompleteFixtures.length}`,
    );
    assert.ok(
      incompatibleFixtures.length >= 1,
      `expected at least 1 incompatible fixture, found ${incompatibleFixtures.length}`,
    );
  });

  for (const name of validFixtures) {
    test(`valid fixture ${name} is accepted`, () => {
      const accepted = validate(readFixture(name));
      assert.equal(
        accepted,
        true,
        `${name} must validate: ${ajv.errorsText(validate.errors)}`,
      );
    });
  }

  test("valid fixture fingerprints match the SHA-256 of their content", () => {
    let checked = 0;
    for (const name of validFixtures) {
      const envelope = readFixture(name) as {
        artifacts?: Array<{ content?: unknown; fingerprint?: unknown }>;
      };
      const artifacts = envelope.artifacts;
      assert.ok(Array.isArray(artifacts), `${name} must have an artifacts array`);
      assert.ok(
        artifacts.length >= 1,
        `${name} must have at least one artifact`,
      );
      for (const [index, artifact] of artifacts.entries()) {
        const { content, fingerprint } = artifact;
        assert.ok(
          typeof content === "string",
          `${name} artifact ${index} content must be a string`,
        );
        assert.ok(
          typeof fingerprint === "string",
          `${name} artifact ${index} fingerprint must be a string`,
        );
        assert.equal(
          fingerprint,
          createHash("sha256").update(content, "utf8").digest("hex"),
          `${name} artifact ${index}: fingerprint is not the SHA-256 of its content`,
        );
        checked += 1;
      }
    }
    assert.ok(checked > 0, "expected at least one artifact to verify");
  });

  for (const name of rejectedFixtures) {
    test(`rejected fixture ${name} fails with at least one error`, () => {
      const accepted = validate(readFixture(name));
      assert.equal(accepted, false, `${name} must be rejected by the schema`);
      assert.ok(
        (validate.errors?.length ?? 0) >= 1,
        `${name} must report at least one validation error`,
      );
    });
  }
});
