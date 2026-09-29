# Capture fixtures

Shared between the Rust contract tests (`crates/integration-contracts/tests/contract.rs`)
and the TypeScript adapter tests. The filename prefix is the contract between the
two sides:

- `valid-*` — the schema accepts it and the Rust `CaptureEnvelope` deserializes it.
- `invalid-*` — the schema rejects it: extra field, wrong type, bad UUID, bad
  timestamp, bad fingerprint, unknown `kind`, or an empty `artifacts` array.
- `incomplete-*` — a required top-level field is missing.
- `incompatible-*` — `schema_version` is not the supported `1`.
- `valid-large-*` — long content and many artifacts; still valid.

All content is synthetic: no real prompts, diffs, credentials, sessions or
paths (PRIV-001, MVP-SPEC §19.8).

Validation source of truth is
`adapters/opencode/schemas/capture-envelope.schema.json` (JSON Schema draft
2020-12, generated from the Rust types). Rust validates with `jsonschema` and
`should_validate_formats(true)`; the TypeScript side must enable `ajv-formats`
so `observed_at` is asserted rather than only annotated.
