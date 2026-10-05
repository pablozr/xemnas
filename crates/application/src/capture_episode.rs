//! Immutable capture coordinates and bounded, descriptive artifact facts.
use crate::captures::{CaptureArtifactRecord, CaptureError};
use crate::redact::redact_secrets;
use serde::{Deserialize, Serialize};

/// Adapter coordinates, never an authenticated author identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureProvenance {
    /// Reported adapter, not verified identity.
    pub adapter: Option<String>,
    /// Reported adapter version.
    pub adapter_version: Option<String>,
    /// Reported session coordinate.
    pub session_id: Option<String>,
    /// Reported message coordinate.
    pub message_id: Option<String>,
    /// Reported source clock.
    pub observed_at: Option<String>,
}

/// Facts describe only the recorded artifact, not the current workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactFact {
    /// Recorded artifact identifier.
    pub artifact_id: String,
    /// Recorded artifact format.
    pub kind: String,
    /// Hash of redacted stored content.
    pub redacted_hash: String,
    /// Safe relative metadata path; only a mention.
    pub file_mention: Option<String>,
    /// Explicit role metadata; never author authentication.
    pub role_metadata: Option<String>,
    /// Snippet syntax counts, unknown for other formats or oversized content.
    pub diff_counts: Option<DiffCounts>,
}

/// Counts of unified-diff syntax in the recorded snippet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffCounts {
    /// Recorded hunk headers.
    pub hunks: usize,
    /// Recorded addition lines, excluding headers.
    pub additions: usize,
    /// Recorded removal lines, excluding headers.
    pub removals: usize,
}

/// Backend detail; missing provenance explicitly means legacy/unknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureEpisode {
    /// Receipt identifier.
    pub capture_id: String,
    /// Receipt acceptance clock.
    pub received_at: String,
    /// Immutable source coordinates, absent for legacy captures.
    pub provenance: Option<CaptureProvenance>,
    /// At most one hundred descriptive artifact facts.
    pub artifacts: Vec<ArtifactFact>,
    /// Artifact or snippet limit was reached.
    pub facts_truncated: bool,
}

/// Project-scoped persistence port. Pages contain at most twenty episodes.
pub trait CaptureEpisodeStore {
    /// Reads a project-scoped page or a single project-scoped capture.
    fn capture_episodes(
        &self,
        project_id: &str,
        capture_id: Option<&str>,
        offset: usize,
    ) -> Result<Vec<CaptureEpisode>, CaptureError>;
}

/// Reject rather than truncate coordinates or metadata into misleading values.
pub fn safe_coordinate(value: &str) -> Option<String> {
    if value.is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return None;
    }
    (redact_secrets(value) == value).then(|| value.to_owned())
}

/// Derives bounded facts from already-redacted persisted artifacts only.
pub fn artifact_fact(artifact: &CaptureArtifactRecord) -> ArtifactFact {
    let metadata = if artifact.metadata.len() <= 16_384 {
        serde_json::from_str::<serde_json::Value>(&artifact.metadata).ok()
    } else {
        None
    };
    let text = |key: &str| {
        metadata
            .as_ref()?
            .get(key)?
            .as_str()
            .and_then(safe_coordinate)
    };
    let file_mention = text("file").or_else(|| text("path")).filter(|path| {
        !path.starts_with(['/', '\\'])
            && !path.contains([':', '\\'])
            && !path.split('/').any(|part| part == ".." || part.is_empty())
            && !path.split('/').any(|part| part.starts_with('.'))
    });
    let diff_counts = if artifact.kind == "diff_hunk" && artifact.content.len() <= 65_536 {
        snippet_counts(&artifact.content)
    } else {
        None
    };
    ArtifactFact {
        artifact_id: artifact.artifact_id.clone(),
        kind: artifact.kind.clone(),
        redacted_hash: artifact.fingerprint.clone(),
        file_mention,
        role_metadata: text("role")
            .filter(|role| matches!(role.as_str(), "user" | "assistant" | "system" | "tool")),
        diff_counts,
    }
}

fn snippet_counts(content: &str) -> Option<DiffCounts> {
    if content.contains('\0') {
        return None;
    }
    let mut counts = DiffCounts {
        hunks: 0,
        additions: 0,
        removals: 0,
    };
    let (mut old, mut new) = (0usize, 0usize);
    for line in content.lines() {
        if old != 0 || new != 0 {
            match line.as_bytes().first()? {
                b'+' if new > 0 => {
                    new -= 1;
                    counts.additions += 1;
                }
                b'-' if old > 0 => {
                    old -= 1;
                    counts.removals += 1;
                }
                b' ' if old > 0 && new > 0 => {
                    old -= 1;
                    new -= 1;
                }
                b'\\' if line == "\\ No newline at end of file" => {}
                _ => return None,
            }
        } else if line.starts_with("@@ ") {
            let mut fields = line.split_whitespace();
            if fields.next()? != "@@" {
                return None;
            }
            old = range_count(fields.next()?, '-')?;
            new = range_count(fields.next()?, '+')?;
            if fields.next()? != "@@" {
                return None;
            }
            counts.hunks += 1;
        } else if !(line.starts_with("--- ")
            || line.starts_with("+++ ")
            || line.starts_with("diff --git ")
            || line.starts_with("index ")
            || line == "\\ No newline at end of file")
        {
            return None;
        }
    }
    (counts.hunks > 0 && old == 0 && new == 0).then_some(counts)
}

fn range_count(field: &str, prefix: char) -> Option<usize> {
    let range = field.strip_prefix(prefix)?;
    let (start, count) = range.split_once(',').unwrap_or((range, "1"));
    start.parse::<usize>().ok()?;
    count.parse().ok()
}
