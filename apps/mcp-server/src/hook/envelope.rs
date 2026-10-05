//! Capture Envelope of one Claude Code turn.
//!
//! Mirrors `buildEnvelope` in `adapters/opencode/src/envelope.ts`, with the same
//! limits as its `config.ts`. Every content is stripped of injected context
//! blocks, redacted and truncated on a UTF-8 boundary before it is kept.

use application::{
    artifact_fingerprint, ArtifactKind, CaptureEnvelope, CaptureSource, ProjectRef, SourceArtifact,
};
use serde_json::{json, Map, Value};

use super::transcript::Turn;

/// Maximum UTF-8 bytes of one artifact content.
pub const MAX_ARTIFACT_BYTES: usize = 64 * 1024;

/// Maximum artifacts per envelope.
pub const MAX_ARTIFACTS: usize = 64;

/// Maximum UTF-8 bytes of diff content per envelope.
pub const MAX_DIFF_BYTES: usize = 32 * 1024;

/// Envelope for `turn`, or `None` when nothing survives cleaning (an envelope
/// carries at least one artifact).
pub fn build_envelope(
    session_id: &str,
    cwd: &str,
    turn: &Turn,
    observed_at: String,
) -> Option<CaptureEnvelope> {
    let mut artifacts: Vec<SourceArtifact> = Vec::new();
    let mut keep = |kind: ArtifactKind, content: String, metadata: Map<String, Value>| {
        if content.trim().is_empty() || artifacts.len() >= MAX_ARTIFACTS {
            return;
        }
        artifacts.push(SourceArtifact {
            artifact_id: uuid::Uuid::now_v7().to_string(),
            kind,
            fingerprint: artifact_fingerprint(&content),
            content,
            metadata,
        });
    };

    keep(
        ArtifactKind::UserText,
        cleaned(&turn.user_text, MAX_ARTIFACT_BYTES),
        Map::new(),
    );
    keep(
        ArtifactKind::AssistantText,
        cleaned(&turn.assistant_texts.join("\n"), MAX_ARTIFACT_BYTES),
        Map::new(),
    );

    // One block per edited file, in first-edit order; files outside the
    // project are dropped, like `reducePatches` does.
    let mut files: Vec<(String, String)> = Vec::new();
    for edit in &turn.edits {
        let Some(file) = relative_inside(&edit.path, cwd) else {
            continue;
        };
        match files.iter_mut().find(|(known, _)| *known == file) {
            Some((_, hunks)) => hunks.push_str(&edit.hunks),
            None => files.push((file, edit.hunks.clone())),
        }
    }
    let mut remaining = MAX_DIFF_BYTES;
    for (file, hunks) in &files {
        if remaining == 0 {
            break;
        }
        let block = format!("diff --git a/{file} b/{file}\n{hunks}");
        let block = cleaned(&block, remaining.min(MAX_ARTIFACT_BYTES));
        remaining = remaining.saturating_sub(block.len());
        let mut metadata = Map::new();
        metadata.insert("file".to_string(), json!(file));
        keep(ArtifactKind::DiffHunk, block, metadata);
    }

    for tool in &turn.tools {
        let status = if tool.is_error { "error" } else { "ok" };
        let mut summary = Map::new();
        summary.insert("tool".to_string(), json!(tool.name));
        summary.insert("status".to_string(), json!(status));
        let content = Value::Object(summary.clone()).to_string();
        keep(
            ArtifactKind::ToolSummary,
            cleaned(&content, MAX_ARTIFACT_BYTES),
            summary,
        );
    }
    if artifacts.is_empty() {
        return None;
    }

    // Raw (pre-redaction) diff, so the key does not depend on the budget.
    let raw_diff = files
        .iter()
        .map(|(file, hunks)| format!("{file}\u{0}{hunks}"))
        .collect::<Vec<_>>()
        .join("\u{1}");
    Some(CaptureEnvelope {
        schema_version: 1,
        capture_id: uuid::Uuid::now_v7().to_string(),
        idempotency_key: format!(
            "claude-code:{session_id}:{}:{}",
            turn.prompt_uuid,
            artifact_fingerprint(&raw_diff)
        ),
        source: CaptureSource {
            adapter: "claude-code".to_string(),
            adapter_version: env!("CARGO_PKG_VERSION").to_string(),
            session_id: session_id.to_string(),
            message_id: turn.prompt_uuid.clone(),
        },
        project: ProjectRef {
            canonical_path: cwd.to_string(),
        },
        observed_at,
        artifacts,
    })
}

/// Strips injected context, redacts secrets and truncates to `max_bytes`.
fn cleaned(text: &str, max_bytes: usize) -> String {
    let text = application::injection::strip_context_blocks(text);
    let text = application::redact::redact_secrets(&text);
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

/// `path` relative to `cwd` with forward slashes, or `None` when it is outside.
/// A relative `path` is taken as inside `cwd`. Windows paths compare ignoring case.
pub fn relative_inside(path: &str, cwd: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    let cwd = cwd.replace('\\', "/");
    let cwd = cwd.trim_end_matches('/');
    let absolute = path.starts_with('/') || path.as_bytes().get(1) == Some(&b':');
    let relative = if absolute {
        let prefix = path.get(..cwd.len())?;
        let same = if cfg!(windows) {
            prefix.eq_ignore_ascii_case(cwd)
        } else {
            prefix == cwd
        };
        path.get(cwd.len()..)?
            .strip_prefix('/')
            .filter(|_| same)?
            .to_string()
    } else {
        path
    };
    let relative = relative.trim_start_matches("./");
    let escapes = relative.split('/').any(|part| part == "..");
    (!relative.is_empty() && !escapes).then(|| relative.to_string())
}

/// Current UTC time as RFC 3339 (`application::clock` is private).
pub fn now_rfc3339() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = (seconds / 86_400) as i64;
    let of_day = seconds % 86_400;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3_600,
        (of_day % 3_600) / 60,
        of_day % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hook::transcript::{FileEdit, ToolCall};

    const CWD: &str = "C:/proj";

    fn turn() -> Turn {
        Turn {
            prompt_uuid: "p1".to_string(),
            user_text: "faça isso".to_string(),
            assistant_texts: vec!["feito".to_string(), "mais".to_string()],
            tools: vec![
                ToolCall {
                    name: "Edit".to_string(),
                    id: "t1".to_string(),
                    is_error: false,
                },
                ToolCall {
                    name: "Bash".to_string(),
                    id: "t2".to_string(),
                    is_error: true,
                },
            ],
            edits: vec![
                FileEdit {
                    path: "C:\\proj\\src\\a.rs".to_string(),
                    hunks: "@@ -1,1 +1,1 @@\n-a\n+b\n".to_string(),
                },
                FileEdit {
                    path: "C:\\outro\\x.rs".to_string(),
                    hunks: "@@ -1,1 +1,1 @@\n-x\n+y\n".to_string(),
                },
            ],
            ..Turn::default()
        }
    }

    fn build(turn: &Turn) -> Option<CaptureEnvelope> {
        build_envelope("s1", CWD, turn, now_rfc3339())
    }

    fn kinds(envelope: &CaptureEnvelope) -> Vec<&'static str> {
        envelope
            .artifacts
            .iter()
            .map(|artifact| artifact.kind.as_str())
            .collect()
    }

    #[test]
    fn builds_every_artifact_kind_and_skips_files_outside_the_project() {
        let envelope = build(&turn()).expect("envelope");
        assert_eq!(
            kinds(&envelope),
            [
                "user_text",
                "assistant_text",
                "diff_hunk",
                "tool_summary",
                "tool_summary"
            ]
        );
        assert_eq!(envelope.artifacts[1].content, "feito\nmais");
        let diff = &envelope.artifacts[2];
        assert_eq!(
            diff.content,
            "diff --git a/src/a.rs b/src/a.rs\n@@ -1,1 +1,1 @@\n-a\n+b\n"
        );
        assert_eq!(diff.metadata["file"], json!("src/a.rs"));
        assert_eq!(envelope.artifacts[3].metadata["status"], json!("ok"));
        assert_eq!(envelope.artifacts[4].metadata["status"], json!("error"));
        assert_eq!(
            envelope.artifacts[4].content,
            r#"{"status":"error","tool":"Bash"}"#
        );
        assert_eq!(envelope.source.adapter, "claude-code");
        assert_eq!(envelope.source.message_id, "p1");
        assert!(envelope.idempotency_key.starts_with("claude-code:s1:p1:"));
    }

    #[test]
    fn serializes_into_the_capture_contract() {
        let envelope = build(&turn()).expect("envelope");
        let value = serde_json::to_value(&envelope).expect("json");
        let back: CaptureEnvelope = serde_json::from_value(value).expect("deny_unknown_fields");
        assert_eq!(back, envelope);

        let v7 = |id: &str| id.len() == 36 && id.as_bytes()[14] == b'7';
        assert!(v7(&envelope.capture_id));
        for artifact in &envelope.artifacts {
            assert!(v7(&artifact.artifact_id));
            assert_eq!(
                artifact.fingerprint,
                artifact_fingerprint(&artifact.content)
            );
        }
        let at = &envelope.observed_at;
        assert!(
            at.len() == 20 && at.ends_with('Z') && &at[10..11] == "T",
            "{at}"
        );
    }

    #[test]
    fn redacts_secrets_and_strips_context_blocks() {
        let mut turn = turn();
        turn.user_text =
            "chave sk-abcdEFGH1234 e <xemnas-context>segredo interno</xemnas-context> ok"
                .to_string();
        let envelope = build(&turn).expect("envelope");
        let text = &envelope.artifacts[0].content;
        assert!(!text.contains("sk-abcd"), "{text}");
        assert!(!text.contains("segredo interno"), "{text}");
    }

    #[test]
    fn the_key_ignores_text_but_follows_the_diff() {
        let first = build(&turn()).expect("envelope");
        let mut changed = turn();
        changed.user_text = "outro texto".to_string();
        assert_eq!(
            build(&changed).expect("envelope").idempotency_key,
            first.idempotency_key
        );
        changed.edits[0].hunks.push_str("+c\n");
        assert_ne!(
            build(&changed).expect("envelope").idempotency_key,
            first.idempotency_key
        );
    }

    #[test]
    fn an_empty_turn_has_no_envelope() {
        let empty = Turn {
            prompt_uuid: "p".to_string(),
            user_text: "  ".to_string(),
            ..Turn::default()
        };
        assert!(build(&empty).is_none());
    }

    #[test]
    fn limits_bound_artifacts_bytes_and_diff() {
        let mut big = turn();
        big.user_text = "é".repeat(MAX_ARTIFACT_BYTES);
        big.tools = (0..100)
            .map(|index| ToolCall {
                name: format!("T{index}"),
                id: index.to_string(),
                is_error: false,
            })
            .collect();
        big.edits = (0..3)
            .map(|index| FileEdit {
                path: format!("C:/proj/f{index}.rs"),
                hunks: format!("+{}\n", "x".repeat(20_000)),
            })
            .collect();
        let envelope = build(&big).expect("envelope");

        assert_eq!(envelope.artifacts.len(), MAX_ARTIFACTS);
        let user = &envelope.artifacts[0].content;
        assert!(user.len() <= MAX_ARTIFACT_BYTES && user.chars().all(|c| c == 'é'));
        let diff_bytes: usize = envelope
            .artifacts
            .iter()
            .filter(|artifact| artifact.kind == ArtifactKind::DiffHunk)
            .map(|artifact| artifact.content.len())
            .sum();
        assert!(diff_bytes <= MAX_DIFF_BYTES, "{diff_bytes}");
        assert!(diff_bytes > 20_000);
    }

    #[test]
    fn relative_paths() {
        assert_eq!(
            relative_inside("C:\\proj\\a\\b.rs", "C:/proj"),
            Some("a/b.rs".to_string())
        );
        assert_eq!(
            relative_inside("src/a.rs", "C:/proj/"),
            Some("src/a.rs".to_string())
        );
        assert_eq!(relative_inside("C:/project/a.rs", "C:/proj"), None);
        assert_eq!(relative_inside("C:/proj/../x.rs", "C:/proj"), None);
        assert_eq!(relative_inside("D:/x.rs", "C:/proj"), None);
    }
}
