//! Contract tests for the Capture Envelope.

use std::fs;
use std::path::{Path, PathBuf};

use integration_contracts::capture::{capture_envelope_schema, CaptureEnvelope};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

/// Workspace root, two levels above this package.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tests live two levels below the workspace root")
        .to_path_buf()
}

/// Directory holding the shared capture fixtures.
fn fixtures_dir() -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("capture")
}

/// Versioned schema artifact consumed by the TypeScript adapter.
fn schema_path() -> PathBuf {
    workspace_root()
        .join("adapters")
        .join("opencode")
        .join("schemas")
        .join("capture-envelope.schema.json")
}

/// The in-memory schema serialized as a JSON value.
fn schema_value() -> Value {
    serde_json::to_value(capture_envelope_schema()).expect("the schema serializes")
}

/// Fixture files paired with their filename.
fn fixture_files() -> Vec<(String, PathBuf)> {
    let mut files: Vec<(String, PathBuf)> = fs::read_dir(fixtures_dir())
        .expect("the capture fixtures directory must exist")
        .map(|entry| entry.expect("readable directory entry"))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .map(|entry| {
            (
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            )
        })
        .collect();
    files.sort();
    files
}

/// Reads and parses a fixture.
fn read_fixture(path: &Path) -> Value {
    let text =
        fs::read_to_string(path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {}: {error}", path.display()))
}

/// Returns the lowercase SHA-256 of the UTF-8 bytes of `value`.
fn sha256_hex(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Builds a draft 2020-12 validator that also asserts `format`.
fn capture_validator() -> jsonschema::Validator {
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema_value())
        .expect("the generated schema compiles")
}

#[test]
fn schema_matches_the_versioned_artifact() {
    let current = schema_value();
    let path = schema_path();
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let versioned: Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));

    assert_eq!(
        current,
        versioned,
        "{} is stale; regenerate it with the documented command",
        path.display()
    );
    assert_eq!(
        versioned["$schema"],
        json!("https://json-schema.org/draft/2020-12/schema")
    );
}

#[test]
#[ignore = "writes the versioned schema artifact; run explicitly to regenerate"]
fn regenerate_capture_envelope_schema() {
    let path = schema_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create the schema directory");
    }
    let mut text = serde_json::to_string_pretty(&schema_value()).expect("the schema serializes");
    text.push('\n');
    fs::write(&path, text).expect("write the schema artifact");
    println!("wrote {}", path.display());
}

#[test]
fn valid_fixtures_validate_deserialize_and_round_trip() {
    let validator = capture_validator();
    let mut checked = 0;
    for (name, path) in fixture_files() {
        if !name.starts_with("valid-") {
            continue;
        }
        checked += 1;
        let value = read_fixture(&path);
        assert!(
            validator.is_valid(&value),
            "{name} must validate against the schema"
        );

        let envelope: CaptureEnvelope = serde_json::from_value(value.clone())
            .unwrap_or_else(|error| panic!("{name} must deserialize: {error}"));

        assert_eq!(
            serde_json::to_value(&envelope).expect("serializes"),
            value,
            "{name} must round-trip without losing semantics"
        );

        assert_eq!(json!(envelope.schema_version), value["schema_version"]);
        assert_eq!(json!(envelope.idempotency_key), value["idempotency_key"]);
        assert_eq!(json!(envelope.capture_id), value["capture_id"]);
        assert_eq!(json!(envelope.observed_at), value["observed_at"]);

        let artifacts = value["artifacts"]
            .as_array()
            .expect("artifacts is an array");
        assert_eq!(artifacts.len(), envelope.artifacts.len());
        for (artifact_value, artifact) in artifacts.iter().zip(&envelope.artifacts) {
            assert_eq!(json!(artifact.artifact_id), artifact_value["artifact_id"]);
            assert_eq!(json!(artifact.fingerprint), artifact_value["fingerprint"]);
            assert_eq!(json!(artifact.kind), artifact_value["kind"]);
            assert_eq!(json!(artifact.content), artifact_value["content"]);
        }
    }
    assert!(checked >= 2, "expected at least two valid fixtures");
}

#[test]
fn valid_fixture_fingerprints_match_sha256_of_content() {
    let mut checked = 0;
    for (name, path) in fixture_files() {
        if !name.starts_with("valid-") {
            continue;
        }
        let value = read_fixture(&path);
        let artifacts = value["artifacts"]
            .as_array()
            .unwrap_or_else(|| panic!("{name} must have an artifacts array"));
        assert!(
            !artifacts.is_empty(),
            "{name} must have at least one artifact"
        );
        for (index, artifact) in artifacts.iter().enumerate() {
            let content = artifact["content"]
                .as_str()
                .unwrap_or_else(|| panic!("{name} artifact {index} content must be a string"));
            let fingerprint = artifact["fingerprint"]
                .as_str()
                .unwrap_or_else(|| panic!("{name} artifact {index} fingerprint must be a string"));
            assert_eq!(
                fingerprint,
                sha256_hex(content),
                "{name} artifact {index}: fingerprint is not the SHA-256 of its content"
            );
            checked += 1;
        }
    }
    assert!(checked >= 1, "expected at least one artifact to verify");
}

#[test]
fn invalid_fixtures_are_rejected_by_the_schema() {
    let validator = capture_validator();
    let mut invalid = 0;
    let mut incomplete = 0;
    let mut incompatible = 0;

    for (name, path) in fixture_files() {
        let is_invalid = name.starts_with("invalid-");
        let is_incomplete = name.starts_with("incomplete-");
        let is_incompatible = name.starts_with("incompatible-");
        if !(is_invalid || is_incomplete || is_incompatible) {
            continue;
        }
        if is_invalid {
            invalid += 1;
        }
        if is_incomplete {
            incomplete += 1;
        }
        if is_incompatible {
            incompatible += 1;
        }

        let value = read_fixture(&path);
        assert!(
            !validator.is_valid(&value),
            "{name} must be rejected by the schema"
        );

        let serde_must_reject = is_incomplete
            || name.starts_with("invalid-extra-field")
            || name.starts_with("invalid-wrong-type")
            || name.starts_with("invalid-unknown-kind");
        if serde_must_reject {
            assert!(
                serde_json::from_value::<CaptureEnvelope>(value).is_err(),
                "{name} must also fail typed deserialization"
            );
        }
    }

    assert!(invalid >= 3, "expected at least three invalid fixtures");
    assert!(incomplete >= 2, "expected at least two incomplete fixtures");
    assert!(
        incompatible >= 1,
        "expected at least one incompatible fixture"
    );
}

#[test]
fn fixture_corpus_covers_the_required_shapes() {
    let names: Vec<String> = fixture_files().into_iter().map(|(name, _)| name).collect();
    assert!(
        names.iter().any(|name| name == "valid-minimal.json"),
        "a minimal valid fixture is required"
    );
    assert!(
        names.iter().any(|name| name == "valid-complete.json"),
        "a complete valid fixture (all four kinds) is required"
    );
    assert!(
        names.iter().any(|name| name.starts_with("valid-large-")),
        "a large valid fixture is required"
    );
}
