//! Versioned capture contract exchanged with external adapters.
//!
//! The **Capture Envelope** is the wire shape an adapter (for example OpenCode)
//! produces from a completed turn. These types are the source of truth: the
//! JSON Schema in `adapters/opencode/schemas/capture-envelope.schema.json` is
//! generated from them and consumed by the TypeScript adapter, and the fixtures
//! in `tests/fixtures/capture/` are validated against that schema in Rust and
//! TypeScript.
//!
//! Every struct denies unknown fields ([`serde`]'s `deny_unknown_fields`): a
//! model or adapter that returns extra fields is rejected rather than silently
//! extended (MVP-SPEC §13). Evolution happens through `schema_version`, whose
//! JSON Schema pins `const: 1`.
//!
//! This module deliberately carries **no size limits**: the ingest API (ticket
//! 09) owns `body limits`, and the spec has no numeric bound here.

use schemars::{json_schema, schema_for, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// The only capture contract version accepted by this build.
///
/// The generated schema pins it with `const`, so a version-2 fixture fails
/// validation in Rust and TypeScript rather than being coerced.
pub const CAPTURE_ENVELOPE_SCHEMA_VERSION: u32 = 1;

/// Canonical UUID v7 pattern: version nibble `7` and RFC 4122 variant nibble.
const UUID_V7_PATTERN: &str =
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-7[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$";

/// Lowercase SHA-256 pattern (64 hex characters).
const SHA256_PATTERN: &str = r"^[0-9a-f]{64}$";

/// A versioned package of Source Artifacts produced by one adapter.
///
/// Consumers written in Rust **must validate the value against the generated
/// JSON Schema before deserializing**. Typed deserialization alone does not
/// enforce the `const: 1` on `schema_version`, because the field is a plain
/// `u32`; version enforcement is the ingest API's responsibility (MVP-SPEC
/// §7.3) and lands with ticket 09.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureEnvelope {
    /// Contract version; the schema pins this to
    /// [`CAPTURE_ENVELOPE_SCHEMA_VERSION`].
    ///
    /// Deserialization accepts any `u32`; only schema validation applies the
    /// `const: 1`. Validate against the schema before trusting this value.
    #[schemars(schema_with = "schema_version_schema")]
    pub schema_version: u32,
    /// Stable identifier of this capture (UUID v7).
    #[schemars(extend("pattern" = UUID_V7_PATTERN))]
    pub capture_id: String,
    /// Adapter-provided idempotency key; never empty.
    #[schemars(length(min = 1))]
    pub idempotency_key: String,
    /// Where the capture came from.
    pub source: CaptureSource,
    /// Which Project the capture belongs to.
    pub project: ProjectRef,
    /// RFC 3339 timestamp of when the adapter observed the turn.
    #[schemars(extend("format" = "date-time"))]
    pub observed_at: String,
    /// The captured artifacts, at least one per turn.
    #[schemars(length(min = 1))]
    pub artifacts: Vec<SourceArtifact>,
}

/// Adapter and session coordinates of a capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CaptureSource {
    /// Adapter name, for example `opencode`.
    pub adapter: String,
    /// Adapter version that produced the envelope.
    pub adapter_version: String,
    /// Adapter session identifier.
    pub session_id: String,
    /// Adapter message identifier within the session.
    pub message_id: String,
}

/// Reference to the Project a capture belongs to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectRef {
    /// Canonicalized project path; never empty. Canonicalization and the
    /// allow-list check are the ingest API's job (MVP-SPEC §7.3).
    #[schemars(length(min = 1))]
    pub canonical_path: String,
}

/// A single immutable artifact within a capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceArtifact {
    /// Stable identifier of this artifact (UUID v7).
    #[schemars(extend("pattern" = UUID_V7_PATTERN))]
    pub artifact_id: String,
    /// What kind of content the artifact carries.
    pub kind: ArtifactKind,
    /// Opaque content string; never empty. No size bound is imposed here.
    #[schemars(length(min = 1))]
    pub content: String,
    /// Open metadata object; extra keys are allowed inside it.
    pub metadata: Map<String, Value>,
    /// Lowercase SHA-256 of the artifact content.
    #[schemars(extend("pattern" = SHA256_PATTERN))]
    pub fingerprint: String,
}

/// The kinds of Source Artifact the contract recognises.
///
/// Serialized in `snake_case`, matching the conceptual contract in MVP-SPEC §10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    /// Text authored by the user.
    UserText,
    /// Text authored by the assistant.
    AssistantText,
    /// A diff hunk from the turn.
    DiffHunk,
    /// A summary of a tool call.
    ToolSummary,
}

impl ArtifactKind {
    /// Returns the persisted literal for this kind.
    ///
    /// Matches the `snake_case` serialization exactly, without going through
    /// `serde`, so the ingest use case can bucket artifacts by kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserText => "user_text",
            Self::AssistantText => "assistant_text",
            Self::DiffHunk => "diff_hunk",
            Self::ToolSummary => "tool_summary",
        }
    }
}

/// JSON Schema for [`CaptureEnvelope::schema_version`].
///
/// Hand-written so the schema carries exactly `type`, `const` and `minimum`.
/// The derived `u32` schema would add `format: "uint32"`, which is an OpenAPI
/// extension rather than a JSON Schema 2020-12 keyword and would force strict
/// consumers (the Ajv adapter) to relax their strictness.
fn schema_version_schema(_generator: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "integer",
        "const": CAPTURE_ENVELOPE_SCHEMA_VERSION,
        "minimum": 0
    })
}

/// Returns the JSON Schema for [`CaptureEnvelope`], generated from the types.
///
/// The generated schema (draft 2020-12) is the validation source of truth and is
/// versioned at `adapters/opencode/schemas/capture-envelope.schema.json`.
///
/// Regenerate the artifact after changing any contract type:
///
/// ```text
/// cargo test -p integration-contracts --test contract -- --ignored regenerate_capture_envelope_schema
/// ```
pub fn capture_envelope_schema() -> schemars::Schema {
    schema_for!(CaptureEnvelope)
}

/// Returns the lowercase SHA-256 of `content` encoded as UTF-8.
///
/// This is the canonical artifact fingerprint: the ingest API recomputes it and
/// rejects any envelope whose declared `fingerprint` disagrees (MVP-SPEC §10).
pub fn artifact_fingerprint(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Error returned when a raw envelope fails JSON Schema validation.
///
/// It deliberately carries **no instance content**: only the number of failures
/// and the schema-defined keywords, so it is always safe to log (PRIV-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeValidationError {
    error_count: usize,
    keywords: Vec<String>,
    schema_detail: Option<String>,
}

impl EnvelopeValidationError {
    /// Failure carrying the schema keywords that did not match.
    fn validation(keywords: Vec<String>) -> Self {
        Self {
            error_count: keywords.len(),
            keywords,
            schema_detail: None,
        }
    }

    /// Failure compiling the generated schema; the detail is our own schema text.
    fn schema(detail: String) -> Self {
        Self {
            error_count: 0,
            keywords: Vec::new(),
            schema_detail: Some(detail),
        }
    }

    /// Number of schema validation failures.
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Schema keywords that failed (for example `const`, `required`, `pattern`).
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    /// Schema compilation detail, when the generated schema itself failed.
    pub fn schema_detail(&self) -> Option<&str> {
        self.schema_detail.as_deref()
    }
}

impl std::fmt::Display for EnvelopeValidationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("capture envelope failed JSON Schema validation")
    }
}

impl std::error::Error for EnvelopeValidationError {}

/// Compiled draft 2020-12 validator for the capture contract.
///
/// Compiling the schema is done once and cached; `format` is asserted (the
/// schema's default vocabulary only annotates it), so an invalid `observed_at`
/// fails validation.
fn envelope_validator() -> Result<&'static jsonschema::Validator, EnvelopeValidationError> {
    static VALIDATOR: OnceLock<Result<jsonschema::Validator, EnvelopeValidationError>> =
        OnceLock::new();
    VALIDATOR
        .get_or_init(|| {
            let schema = serde_json::to_value(capture_envelope_schema())
                .map_err(|error| EnvelopeValidationError::schema(error.to_string()))?;
            jsonschema::draft202012::options()
                .should_validate_formats(true)
                .build(&schema)
                .map_err(|error| EnvelopeValidationError::schema(error.to_string()))
        })
        .as_ref()
        .map_err(Clone::clone)
}

/// Validates a raw capture envelope against the generated JSON Schema.
///
/// This is the ingest gate (MVP-SPEC §7.3 step 2): it enforces the supported
/// `schema_version`, required fields, patterns and formats. Deserializing with
/// `serde` alone does not apply `const: 1`, so callers must validate here first.
///
/// The error carries only safe metadata, never the offending instance.
pub fn validate_envelope(value: &Value) -> Result<(), EnvelopeValidationError> {
    let validator = envelope_validator()?;
    let keywords: Vec<String> = validator
        .iter_errors(value)
        // jsonschema 0.58 moved `keyword()` off `ValidationError` and onto
        // `ValidationErrorKind`; the validation keyword is now reached through
        // `.kind()`. The collected value is unchanged: the JSON Schema keyword
        // that rejected the envelope, never the offending instance (PRIV-001).
        .map(|error| error.kind().keyword().to_string())
        .collect();
    if keywords.is_empty() {
        Ok(())
    } else {
        Err(EnvelopeValidationError::validation(keywords))
    }
}

#[cfg(test)]
mod tests {
    use super::{ArtifactKind, CAPTURE_ENVELOPE_SCHEMA_VERSION};
    use serde_json::json;

    #[test]
    fn artifact_kinds_serialize_in_snake_case() {
        let pairs = [
            (ArtifactKind::UserText, "user_text"),
            (ArtifactKind::AssistantText, "assistant_text"),
            (ArtifactKind::DiffHunk, "diff_hunk"),
            (ArtifactKind::ToolSummary, "tool_summary"),
        ];
        for (kind, literal) in pairs {
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(literal));
            let parsed: ArtifactKind = serde_json::from_value(json!(literal)).unwrap();
            assert_eq!(parsed, kind);
        }
    }

    #[test]
    fn schema_version_constant_is_pinned_in_the_schema() {
        let schema = serde_json::to_value(super::capture_envelope_schema()).unwrap();
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            json!(CAPTURE_ENVELOPE_SCHEMA_VERSION)
        );
    }

    #[test]
    fn schema_declares_draft_2020_12() {
        let schema = serde_json::to_value(super::capture_envelope_schema()).unwrap();
        assert_eq!(
            schema["$schema"],
            json!("https://json-schema.org/draft/2020-12/schema")
        );
    }
}
