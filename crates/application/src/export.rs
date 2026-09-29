//! Manual export of one Engineering Decision (MVP-SPEC §680).
//!
//! Export is always an explicit action: the caller supplies the destination the
//! user picked, and this module never derives a path from the Project location.
//! It also never touches the Project directory — it only reads the decision and
//! writes the chosen file.
//!
//! [`Export::preview`] and [`Export::write`] produce the same bytes, so the user
//! sees exactly what will be written. The content has no volatile timestamps:
//! the same decision always renders to the same string.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::decisions::{DecisionDetail, Decisions, DecisionsError, EvidenceLinkView};

/// Placeholder used for an empty list section.
const EMPTY_LIST: &str = "- _(nenhuma)_";

/// Output format of an export.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Human-readable Markdown.
    Markdown,
    /// Machine-readable JSON.
    Json,
}

/// A rendered export, before it is written anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportDocument {
    /// Decision the document describes.
    pub decision_id: String,
    /// Format the content was rendered in.
    pub format: ExportFormat,
    /// Exact bytes that [`Export::write`] will persist.
    pub content: String,
    /// Length of `content` in bytes.
    pub bytes: usize,
}

/// Where an export was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportResult {
    /// Destination path, as supplied by the caller.
    pub path: String,
    /// Bytes written.
    pub bytes: usize,
}

/// Failure modes of the export use case.
///
/// Messages carry no host path and no decision content beyond a fixed literal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// Reading the decision failed.
    Storage(String),
    /// No decision with the requested id exists.
    NotFound,
    /// The requested format is not supported.
    InvalidFormat(String),
    /// The destination already exists and `overwrite` was not set, or it is a
    /// directory.
    DestinationExists,
    /// The destination parent is missing or not a directory.
    DestinationInvalid(String),
    /// Writing the file failed.
    Io(String),
}

impl ExportError {
    /// Returns a short, stable code safe to surface or log.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::NotFound => "not_found",
            Self::InvalidFormat(_) => "invalid_format",
            Self::DestinationExists => "destination_exists",
            Self::DestinationInvalid(_) => "destination_invalid",
            Self::Io(_) => "io",
        }
    }
}

impl std::fmt::Display for ExportError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha ao ler a decisão: {message}"),
            Self::NotFound => formatter.write_str("decisão não encontrada"),
            Self::InvalidFormat(message) => write!(formatter, "formato inválido: {message}"),
            Self::DestinationExists => formatter.write_str("o destino já existe"),
            Self::DestinationInvalid(message) => write!(formatter, "destino inválido: {message}"),
            Self::Io(message) => write!(formatter, "falha ao gravar o arquivo: {message}"),
        }
    }
}

impl std::error::Error for ExportError {}

/// A decision serialized to JSON, with fixed field order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedDecision {
    /// Decision identifier.
    pub decision_id: String,
    /// Candidate the decision came from.
    pub candidate_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Project canonical location.
    pub project_location: String,
    /// Capture the decision came from, when recorded.
    pub capture_id: Option<String>,
    /// Current status literal.
    pub status: String,
    /// Decision question.
    pub question: String,
    /// Proposed choice.
    pub choice: String,
    /// Rationale.
    pub rationale: String,
    /// Assumptions list.
    pub assumptions: Vec<String>,
    /// Reconsider-when list.
    pub reconsider_when: Vec<String>,
    /// Scope list.
    pub scope: Vec<String>,
    /// Consequences list.
    pub consequences: Vec<String>,
    /// Current version.
    pub version: i64,
    /// RFC 3339 confirmation time.
    pub confirmed_at: String,
    /// RFC 3339 last-update time.
    pub updated_at: String,
    /// Evidence links in stored order.
    pub evidence: Vec<ExportedEvidence>,
    /// Full revision history, newest first.
    pub revisions: Vec<ExportedRevision>,
}

/// One evidence link in the JSON export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedEvidence {
    /// Referenced artifact id.
    pub artifact_id: String,
    /// Artifact kind, when known.
    pub kind: Option<String>,
    /// Position preserving the reference order.
    pub position: i64,
}

/// One revision in the JSON export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedRevision {
    /// Revision version.
    pub version: i64,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// Decision question at this version.
    pub question: String,
    /// Proposed choice at this version.
    pub choice: String,
    /// Rationale at this version.
    pub rationale: String,
    /// Assumptions list at this version.
    pub assumptions: Vec<String>,
    /// Reconsider-when list at this version.
    pub reconsider_when: Vec<String>,
    /// Scope list at this version.
    pub scope: Vec<String>,
    /// Consequences list at this version.
    pub consequences: Vec<String>,
}

/// Manual export use case over a [`crate::decisions::DecisionStore`].
#[derive(Debug, Clone)]
pub struct Export<S> {
    decisions: Decisions<S>,
}

impl<S: crate::decisions::DecisionStore> Export<S> {
    /// Wraps a decision store with the export use case.
    pub fn new(store: S) -> Self {
        Self {
            decisions: Decisions::new(store),
        }
    }

    /// Renders the decision without writing anything.
    pub fn preview(
        &self,
        decision_id: &str,
        format: ExportFormat,
    ) -> Result<ExportDocument, ExportError> {
        let detail = self
            .decisions
            .detail(decision_id)
            .map_err(map_decisions_error)?;
        let content = match format {
            ExportFormat::Markdown => render_markdown(&detail),
            ExportFormat::Json => render_json(&detail)?,
        };
        Ok(ExportDocument {
            decision_id: detail.summary.decision_id.clone(),
            format,
            bytes: content.len(),
            content,
        })
    }

    /// Writes a rendered document to the caller-chosen destination.
    ///
    /// The destination comes from the user's file picker; this use case never
    /// derives a path from the Project and never writes inside it.
    pub fn write(
        &self,
        document: &ExportDocument,
        destination: &Path,
        overwrite: bool,
    ) -> Result<ExportResult, ExportError> {
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or_else(|| {
                ExportError::DestinationInvalid("o destino não tem diretório".to_string())
            })?;
        let parent = parent.canonicalize().map_err(|_| {
            ExportError::DestinationInvalid("o diretório de destino não existe".to_string())
        })?;
        if !parent.is_dir() {
            return Err(ExportError::DestinationInvalid(
                "o destino não é um diretório".to_string(),
            ));
        }
        if destination.is_dir() {
            return Err(ExportError::DestinationExists);
        }
        let file_name = destination
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .ok_or_else(|| ExportError::DestinationInvalid("destino sem nome".to_string()))?;

        // The temporary is created exclusively (`create_new`): an existing file
        // with the same name — including a symlink planted by someone else — is
        // never opened, truncated or written.
        let temporary = create_temporary(&parent, &file_name, document.content.as_bytes())?;

        if overwrite {
            // `rename` atomically replaces the destination. On Windows it
            // replaces the link itself, so a destination symlink is swapped out
            // rather than followed.
            if let Err(error) = std::fs::rename(&temporary, destination) {
                let _ = std::fs::remove_file(&temporary);
                return Err(ExportError::Io(error.to_string()));
            }
        } else {
            // `hard_link` fails atomically with `AlreadyExists` if the
            // destination appeared after the preview, closing the TOCTOU window.
            // A filesystem without hard links reports an error; the export must
            // not silently fall back to an unsafe rename.
            match std::fs::hard_link(&temporary, destination) {
                Ok(()) => {
                    let _ = std::fs::remove_file(&temporary);
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(ExportError::DestinationExists);
                }
                Err(error) => {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(ExportError::Io(error.to_string()));
                }
            }
        }
        Ok(ExportResult {
            path: destination.to_string_lossy().into_owned(),
            bytes: document.content.len(),
        })
    }
}

/// Maximum attempts to create an exclusive temporary in case of a name clash.
const MAX_TEMP_ATTEMPTS: u8 = 8;

/// Creates an exclusive temporary sibling and writes `content` into it.
///
/// The name embeds a fresh UUID v7 and the file is opened with `create_new`, so
/// an existing file is never truncated; a clash simply draws another name.
fn create_temporary(
    directory: &Path,
    filename: &str,
    content: &[u8],
) -> Result<PathBuf, ExportError> {
    for _ in 0..MAX_TEMP_ATTEMPTS {
        let temporary = directory.join(format!("{filename}.{}.tmp", uuid::Uuid::now_v7()));
        match std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                if let Err(error) = file.write_all(content) {
                    drop(file);
                    let _ = std::fs::remove_file(&temporary);
                    return Err(ExportError::Io(error.to_string()));
                }
                return Ok(temporary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(ExportError::Io(error.to_string())),
        }
    }
    Err(ExportError::Io(
        "não foi possível criar o arquivo temporário".to_string(),
    ))
}

/// Maps a decisions error into the export error surface.
fn map_decisions_error(error: DecisionsError) -> ExportError {
    match error {
        DecisionsError::NotFound => ExportError::NotFound,
        other => ExportError::Storage(other.to_string()),
    }
}

/// Renders the fixed Markdown layout.
fn render_markdown(detail: &DecisionDetail) -> String {
    let mut sections = Vec::new();
    sections.push(format!("# {}", detail.summary.question));
    sections.push(section("Escolha", &detail.summary.choice));
    sections.push(section("Justificativa", &detail.rationale));
    sections.push(list_section("Premissas", &detail.assumptions));
    sections.push(list_section("Escopo", &detail.scope));
    sections.push(list_section("Consequências", &detail.consequences));
    sections.push(list_section("Reconsiderar quando", &detail.reconsider_when));
    sections.push(section("Proveniência", &provenance_body(detail)));
    sections.push(section("Evidências", &evidence_body(&detail.evidence)));
    sections.push(section("Histórico de revisões", &revisions_body(detail)));
    let mut content = sections.join("\n\n");
    content.push('\n');
    content
}

/// Renders the JSON layout with fixed field order.
fn render_json(detail: &DecisionDetail) -> Result<String, ExportError> {
    let exported = ExportedDecision {
        decision_id: detail.summary.decision_id.clone(),
        candidate_id: detail.summary.candidate_id.clone(),
        project_id: detail.summary.project_id.clone(),
        project_location: detail.provenance.project_location.clone(),
        capture_id: detail.provenance.capture_id.clone(),
        status: detail.summary.status.as_str().to_string(),
        question: detail.summary.question.clone(),
        choice: detail.summary.choice.clone(),
        rationale: detail.rationale.clone(),
        assumptions: detail.assumptions.clone(),
        reconsider_when: detail.reconsider_when.clone(),
        scope: detail.scope.clone(),
        consequences: detail.consequences.clone(),
        version: detail.summary.version,
        confirmed_at: detail.summary.confirmed_at.clone(),
        updated_at: detail.summary.updated_at.clone(),
        evidence: detail
            .evidence
            .iter()
            .map(|link| ExportedEvidence {
                artifact_id: link.artifact_id.clone(),
                kind: link.kind.clone(),
                position: link.position,
            })
            .collect(),
        revisions: detail
            .revisions
            .iter()
            .map(|revision| ExportedRevision {
                version: revision.version,
                created_at: revision.created_at.clone(),
                question: revision.question.clone(),
                choice: revision.choice.clone(),
                rationale: revision.rationale.clone(),
                assumptions: revision.assumptions.clone(),
                reconsider_when: revision.reconsider_when.clone(),
                scope: revision.scope.clone(),
                consequences: revision.consequences.clone(),
            })
            .collect(),
    };
    let mut content = serde_json::to_string_pretty(&exported)
        .map_err(|error| ExportError::InvalidFormat(error.to_string()))?;
    content.push('\n');
    Ok(content)
}

/// A heading with a scalar body.
fn section(title: &str, body: &str) -> String {
    format!("## {title}\n\n{body}")
}

/// A heading with a bullet list body.
fn list_section(title: &str, items: &[String]) -> String {
    let body = if items.is_empty() {
        EMPTY_LIST.to_string()
    } else {
        items
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!("## {title}\n\n{body}")
}

/// The provenance bullet list.
fn provenance_body(detail: &DecisionDetail) -> String {
    let capture = detail
        .provenance
        .capture_id
        .clone()
        .unwrap_or_else(|| "_(nenhuma)_".to_string());
    format!(
        "- Projeto: {}\n- Candidato: {}\n- Captura: {}\n- Confirmada em: {}",
        detail.provenance.project_location,
        detail.provenance.candidate_id,
        capture,
        detail.summary.confirmed_at,
    )
}

/// The evidence bullet list, in stored order.
fn evidence_body(evidence: &[EvidenceLinkView]) -> String {
    if evidence.is_empty() {
        return EMPTY_LIST.to_string();
    }
    evidence
        .iter()
        .map(|link| match &link.kind {
            Some(kind) => format!("- {} ({kind})", link.artifact_id),
            None => format!("- {}", link.artifact_id),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The revision-history bullet list, newest first.
fn revisions_body(detail: &DecisionDetail) -> String {
    if detail.revisions.is_empty() {
        return EMPTY_LIST.to_string();
    }
    detail
        .revisions
        .iter()
        .map(|revision| format!("- v{} — {}", revision.version, revision.created_at))
        .collect::<Vec<_>>()
        .join("\n")
}
