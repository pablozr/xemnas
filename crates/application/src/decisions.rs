//! Engineering Decisions: accepted candidates promoted to versioned, searchable
//! decisions with provenance. Nothing is ever hard-deleted: a revision keeps the
//! previous snapshot in `decision_revisions`, and the history stays readable.
//!
//! The promotion itself happens inside [`crate::inbox::InboxStore::confirm_one`]
//! so the candidate status change, the decision row, its first snapshot, its
//! evidence links and the search index commit in one transaction. This module
//! owns the read/revise/search surface of an already-promoted decision.
//!
//! `superseded` is modelled for a later ticket (§7.6 line 441); no MVP action
//! sets it, so a decision only ever reaches `accepted` today.

use crate::clock::now_rfc3339;
use crate::inbox::{
    DEFAULT_PAGE_LIMIT, MAX_CHOICE_CHARS, MAX_PAGE_LIMIT, MAX_QUESTION_CHARS, MAX_RATIONALE_CHARS,
};

/// Maximum items accepted in a decision string array.
pub const MAX_ARRAY_ITEMS: usize = 50;

/// Maximum characters accepted in one array item.
pub const MAX_ARRAY_ITEM_CHARS: usize = 1_000;

/// Lifecycle status of an Engineering Decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DecisionStatus {
    /// The decision is current.
    Accepted,
    /// The decision was replaced by a newer one (model only; no MVP action).
    Superseded,
}

impl DecisionStatus {
    /// Returns the literal persisted in the `status` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
        }
    }

    /// Parses a persisted status literal, or `None` when unknown.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "accepted" => Some(Self::Accepted),
            "superseded" => Some(Self::Superseded),
            _ => None,
        }
    }
}

/// Failure modes of the Decisions use case.
///
/// Every variant carries a stable [`DecisionsError::code`]; messages never
/// include decision content, host paths or query text (PRIV-001).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionsError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// No decision with the requested id exists.
    NotFound,
    /// The submitted revision failed validation.
    InvalidEdits(String),
    /// The search query was empty after sanitization.
    InvalidQuery(String),
    /// The submitted filter is malformed (bad cursor or limit).
    InvalidFilter(String),
    /// A stored column could not be parsed back into its typed value.
    InvalidData(String),
}

impl DecisionsError {
    /// Returns a short, stable code safe to surface or log.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::NotFound => "not_found",
            Self::InvalidEdits(_) => "invalid_edits",
            Self::InvalidQuery(_) => "invalid_query",
            Self::InvalidFilter(_) => "invalid_filter",
            Self::InvalidData(_) => "invalid_data",
        }
    }
}

impl std::fmt::Display for DecisionsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::NotFound => formatter.write_str("decisão não encontrada"),
            Self::InvalidEdits(message) => write!(formatter, "revisão inválida: {message}"),
            Self::InvalidQuery(message) => write!(formatter, "consulta inválida: {message}"),
            Self::InvalidFilter(message) => write!(formatter, "filtro inválido: {message}"),
            Self::InvalidData(message) => {
                write!(formatter, "dados da decisão inválidos: {message}")
            }
        }
    }
}

impl std::error::Error for DecisionsError {}

/// Filter for the Decisions list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionFilter {
    /// Restrict to one project, or every project when `None`.
    pub project_id: Option<String>,
    /// Statuses to include; defaults to `[Accepted]`.
    pub statuses: Vec<DecisionStatus>,
    /// Requested page size, capped at [`MAX_PAGE_LIMIT`].
    pub limit: usize,
    /// Opaque keyset cursor from a previous [`DecisionPage::next_cursor`].
    pub cursor: Option<String>,
}

impl Default for DecisionFilter {
    fn default() -> Self {
        Self {
            project_id: None,
            statuses: vec![DecisionStatus::Accepted],
            limit: DEFAULT_PAGE_LIMIT,
            cursor: None,
        }
    }
}

impl DecisionFilter {
    /// Builds the default filter (accepted decisions, default page size).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Search request for the Decisions screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    /// Free-text query; sanitized before it reaches FTS5.
    pub query: String,
    /// Restrict to one project, or every project when `None`.
    pub project_id: Option<String>,
    /// Maximum hits, capped at [`MAX_PAGE_LIMIT`].
    pub limit: usize,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            project_id: None,
            limit: DEFAULT_PAGE_LIMIT,
        }
    }
}

/// Partial revision; at least one field must be present.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DecisionEdits {
    /// New decision question.
    pub question: Option<String>,
    /// New proposed choice.
    pub choice: Option<String>,
    /// New rationale.
    pub rationale: Option<String>,
    /// New assumptions list.
    pub assumptions: Option<Vec<String>>,
    /// New reconsider-when list.
    pub reconsider_when: Option<Vec<String>>,
    /// New scope list.
    pub scope: Option<Vec<String>>,
    /// New consequences list.
    pub consequences: Option<Vec<String>>,
}

impl DecisionEdits {
    /// Returns `true` when no field was provided.
    pub fn is_empty(&self) -> bool {
        self.question.is_none()
            && self.choice.is_none()
            && self.rationale.is_none()
            && self.assumptions.is_none()
            && self.reconsider_when.is_none()
            && self.scope.is_none()
            && self.consequences.is_none()
    }

    /// Validates and trims the provided fields (at least one required).
    pub fn validate(&self) -> Result<DecisionEdits, DecisionsError> {
        if self.is_empty() {
            return Err(DecisionsError::InvalidEdits(
                "nenhum campo informado".to_string(),
            ));
        }
        Ok(DecisionEdits {
            question: self
                .question
                .as_deref()
                .map(|value| validate_text(value, MAX_QUESTION_CHARS, "pergunta"))
                .transpose()?,
            choice: self
                .choice
                .as_deref()
                .map(|value| validate_text(value, MAX_CHOICE_CHARS, "escolha"))
                .transpose()?,
            rationale: self
                .rationale
                .as_deref()
                .map(|value| validate_text(value, MAX_RATIONALE_CHARS, "justificativa"))
                .transpose()?,
            assumptions: self
                .assumptions
                .as_ref()
                .map(|items| validate_array(items))
                .transpose()?,
            reconsider_when: self
                .reconsider_when
                .as_ref()
                .map(|items| validate_array(items))
                .transpose()?,
            scope: self
                .scope
                .as_ref()
                .map(|items| validate_array(items))
                .transpose()?,
            consequences: self
                .consequences
                .as_ref()
                .map(|items| validate_array(items))
                .transpose()?,
        })
    }
}

/// List-row projection of a decision.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionSummary {
    /// Decision identifier.
    pub decision_id: String,
    /// Candidate the decision came from.
    pub candidate_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Project canonical location (list label).
    pub project_location: String,
    /// Capture the decision came from, when recorded.
    pub capture_id: Option<String>,
    /// Current status.
    pub status: DecisionStatus,
    /// Decision question.
    pub question: String,
    /// Proposed choice.
    pub choice: String,
    /// Current version.
    pub version: i64,
    /// RFC 3339 confirmation time.
    pub confirmed_at: String,
    /// RFC 3339 last-update time.
    pub updated_at: String,
}

/// One full revision in the history.
///
/// Every version is stored in full, so an earlier version is reconstructible
/// after any number of revisions (nothing is hard-deleted).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRevision {
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

/// Provenance of a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionProvenance {
    /// Candidate the decision came from.
    pub candidate_id: String,
    /// Capture the decision came from, when recorded.
    pub capture_id: Option<String>,
    /// Project canonical location.
    pub project_location: String,
}

/// One evidence link of a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceLinkView {
    /// Referenced artifact id.
    pub artifact_id: String,
    /// Artifact kind, when the capture artifact still exists.
    pub kind: Option<String>,
    /// Position preserving the candidate's reference order.
    pub position: i64,
}

/// Full decision detail for the disclosure panels.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionDetail {
    /// The list-row fields.
    pub summary: DecisionSummary,
    /// Rationale (editable via [`Decisions::revise`]).
    pub rationale: String,
    /// Assumptions list.
    pub assumptions: Vec<String>,
    /// Reconsider-when list.
    pub reconsider_when: Vec<String>,
    /// Scope list.
    pub scope: Vec<String>,
    /// Consequences list.
    pub consequences: Vec<String>,
    /// Provenance of the decision.
    pub provenance: DecisionProvenance,
    /// Evidence links in stored order.
    pub evidence: Vec<EvidenceLinkView>,
    /// Full revision history, newest first (every version embedded).
    pub revisions: Vec<DecisionRevision>,
}

/// One page of decisions plus the cursor for the next page.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionPage {
    /// The page rows.
    pub decisions: Vec<DecisionSummary>,
    /// Opaque cursor for the next page, or `None` at the end.
    pub next_cursor: Option<String>,
}

/// One full-text search hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSearchHit {
    /// Decision identifier.
    pub decision_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Decision question (for display).
    pub question: String,
    /// FTS snippet with `[` `]` markers.
    pub snippet: String,
}

/// Keyset position `(confirmed_at, decision_id)` for pagination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionCursor {
    /// `confirmed_at` of the last row of the previous page.
    pub confirmed_at: String,
    /// `decision_id` of the last row of the previous page.
    pub decision_id: String,
}

/// Port-level query handed to [`DecisionStore::list`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionQuery {
    /// Restrict to one project, or every project when `None`.
    pub project_id: Option<String>,
    /// Statuses to include.
    pub statuses: Vec<DecisionStatus>,
    /// Maximum rows to return.
    pub limit: usize,
    /// Return rows ordered strictly after this position.
    pub before: Option<DecisionCursor>,
}

/// A stored decision row with its joined project location.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredDecision {
    /// Decision identifier.
    pub decision_id: String,
    /// Candidate the decision came from.
    pub candidate_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Project canonical location.
    pub project_location: String,
    /// Capture the decision came from.
    pub capture_id: Option<String>,
    /// Current status.
    pub status: DecisionStatus,
    /// Decision question.
    pub question: String,
    /// Proposed choice.
    pub choice: String,
    /// Rationale.
    pub rationale: String,
    /// Serialized assumptions array.
    pub assumptions: String,
    /// Serialized reconsider-when array.
    pub reconsider_when: String,
    /// Serialized scope array.
    pub scope: String,
    /// Serialized consequences array.
    pub consequences: String,
    /// Current version.
    pub version: i64,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// RFC 3339 confirmation time.
    pub confirmed_at: String,
    /// RFC 3339 last-update time.
    pub updated_at: String,
}

/// One revision row in the history, with its full content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRevisionRow {
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
    /// Serialized assumptions array.
    pub assumptions: String,
    /// Serialized reconsider-when array.
    pub reconsider_when: String,
    /// Serialized scope array.
    pub scope: String,
    /// Serialized consequences array.
    pub consequences: String,
}

/// One evidence-link row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceLinkRow {
    /// Referenced artifact id.
    pub artifact_id: String,
    /// Artifact kind, when known.
    pub kind: Option<String>,
    /// Position preserving the reference order.
    pub position: i64,
}

/// One raw FTS hit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSearchRow {
    /// Decision identifier.
    pub decision_id: String,
    /// Project identifier.
    pub project_id: String,
    /// Decision question.
    pub question: String,
    /// FTS snippet.
    pub snippet: String,
}

/// Full content written when a revision is snapshotted.
///
/// Array fields carry serialized JSON arrays, exactly as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionContent {
    /// Decision question.
    pub question: String,
    /// Proposed choice.
    pub choice: String,
    /// Rationale.
    pub rationale: String,
    /// Serialized assumptions array.
    pub assumptions: String,
    /// Serialized reconsider-when array.
    pub reconsider_when: String,
    /// Serialized scope array.
    pub scope: String,
    /// Serialized consequences array.
    pub consequences: String,
}

/// Persistence port the Decisions use case needs.
///
/// There is deliberately no delete method anywhere: a decision and its history
/// are append-only.
pub trait DecisionStore {
    /// Returns decisions matching `query`, ordered `confirmed_at DESC, id DESC`.
    fn list(&self, query: &DecisionQuery) -> Result<Vec<StoredDecision>, DecisionsError>;

    /// Returns one decision with its joined project location.
    fn get(&self, id: &str) -> Result<Option<StoredDecision>, DecisionsError>;

    /// Returns the revision history, newest first, with its full content.
    fn revisions(&self, id: &str) -> Result<Vec<DecisionRevisionRow>, DecisionsError>;

    /// Returns the evidence links in position order.
    fn evidence(&self, id: &str) -> Result<Vec<EvidenceLinkRow>, DecisionsError>;

    /// Runs a sanitized FTS5 `MATCH`, optionally filtered by project.
    fn search(
        &self,
        match_query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DecisionSearchRow>, DecisionsError>;

    /// Snapshots `content` as `version` and updates the live row and the index.
    ///
    /// Returns `false` when no decision matched (concurrent change).
    fn revise(
        &self,
        id: &str,
        content: &DecisionContent,
        version: i64,
        updated_at: &str,
    ) -> Result<bool, DecisionsError>;
}

/// Decisions use case over a [`DecisionStore`].
#[derive(Debug, Clone)]
pub struct Decisions<S> {
    store: S,
}

/// A recorded evidence link and its redacted source, when still available.
#[derive(Debug, Clone)]
pub struct DecisionSource {
    /// Ordered provenance link; retained even if the source is unavailable.
    pub link: EvidenceLinkView,
    /// Redacted capture content, never read from the working tree.
    pub artifact: Option<crate::inbox::ArtifactView>,
}

impl<S: DecisionStore + crate::inbox::InboxStore> Decisions<S> {
    /// Resolves only the decision's linked artifacts within its original capture.
    pub fn sources(&self, detail: &DecisionDetail) -> Result<Vec<DecisionSource>, DecisionsError> {
        let refs = detail
            .evidence
            .iter()
            .map(|link| link.artifact_id.clone())
            .collect::<Vec<_>>();
        let artifacts = match detail.provenance.capture_id.as_deref() {
            Some(capture) => self
                .store
                .artifacts(capture, &refs)
                .map_err(|_| DecisionsError::Storage("fonte indisponível".into()))?,
            None => Vec::new(),
        };
        Ok(detail
            .evidence
            .iter()
            .map(|link| DecisionSource {
                link: link.clone(),
                artifact: artifacts
                    .iter()
                    .find(|artifact| artifact.artifact_id == link.artifact_id)
                    .cloned(),
            })
            .collect())
    }
}

impl<S: DecisionStore> Decisions<S> {
    /// Wraps a store with the Decisions use case.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Lists decisions for the Decisions screen.
    pub fn list(&self, filter: &DecisionFilter) -> Result<DecisionPage, DecisionsError> {
        if filter.limit == 0 {
            return Err(DecisionsError::InvalidFilter("limite zero".to_string()));
        }
        if filter.statuses.is_empty() {
            return Err(DecisionsError::InvalidFilter(
                "nenhum estado informado".to_string(),
            ));
        }
        let limit = filter.limit.min(MAX_PAGE_LIMIT);
        let before = match filter.cursor.as_deref() {
            Some(cursor) => Some(parse_cursor(cursor)?),
            None => None,
        };
        let query = DecisionQuery {
            project_id: filter.project_id.clone(),
            statuses: filter.statuses.clone(),
            limit: limit + 1,
            before,
        };
        let mut rows = self.store.list(&query)?;
        let has_more = rows.len() > limit;
        if has_more {
            rows.truncate(limit);
        }
        let next_cursor = if has_more {
            rows.last()
                .map(|row| format!("{}|{}", row.confirmed_at, row.decision_id))
        } else {
            None
        };
        let decisions = rows.iter().map(summary_from_row).collect();
        Ok(DecisionPage {
            decisions,
            next_cursor,
        })
    }

    /// Loads the full detail of one decision.
    pub fn detail(&self, id: &str) -> Result<DecisionDetail, DecisionsError> {
        let row = self.store.get(id)?.ok_or(DecisionsError::NotFound)?;
        let summary = summary_from_row(&row);
        let evidence = self
            .store
            .evidence(id)?
            .into_iter()
            .map(|link| EvidenceLinkView {
                artifact_id: link.artifact_id,
                kind: link.kind,
                position: link.position,
            })
            .collect();
        let revisions = self
            .store
            .revisions(id)?
            .into_iter()
            .map(revision_from_row)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(DecisionDetail {
            provenance: DecisionProvenance {
                candidate_id: row.candidate_id.clone(),
                capture_id: row.capture_id.clone(),
                project_location: row.project_location.clone(),
            },
            summary,
            rationale: row.rationale,
            assumptions: parse_array(&row.assumptions, "assumptions")?,
            reconsider_when: parse_array(&row.reconsider_when, "reconsider_when")?,
            scope: parse_array(&row.scope, "scope")?,
            consequences: parse_array(&row.consequences, "consequences")?,
            evidence,
            revisions,
        })
    }

    /// Searches decisions by question, choice and rationale.
    pub fn search(&self, query: &SearchQuery) -> Result<Vec<DecisionSearchHit>, DecisionsError> {
        if query.limit == 0 {
            return Err(DecisionsError::InvalidFilter("limite zero".to_string()));
        }
        let match_query = sanitize_match_query(&query.query)
            .ok_or_else(|| DecisionsError::InvalidQuery("consulta vazia".to_string()))?;
        let limit = query.limit.min(MAX_PAGE_LIMIT);
        let rows = self
            .store
            .search(&match_query, query.project_id.as_deref(), limit)?;
        Ok(rows
            .into_iter()
            .map(|row| DecisionSearchHit {
                decision_id: row.decision_id,
                project_id: row.project_id,
                question: row.question,
                snippet: row.snippet,
            })
            .collect())
    }

    /// Revises a decision: snapshots the new version and updates the live row.
    pub fn revise(&self, id: &str, edits: DecisionEdits) -> Result<DecisionDetail, DecisionsError> {
        let edits = edits.validate()?;
        let row = self.store.get(id)?.ok_or(DecisionsError::NotFound)?;
        let content = DecisionContent {
            question: edits.question.unwrap_or(row.question),
            choice: edits.choice.unwrap_or(row.choice),
            rationale: edits.rationale.unwrap_or(row.rationale),
            assumptions: merge_array(edits.assumptions, &row.assumptions, "assumptions")?,
            reconsider_when: merge_array(
                edits.reconsider_when,
                &row.reconsider_when,
                "reconsider_when",
            )?,
            scope: merge_array(edits.scope, &row.scope, "scope")?,
            consequences: merge_array(edits.consequences, &row.consequences, "consequences")?,
        };
        let version = row.version + 1;
        if !self.store.revise(id, &content, version, &now_rfc3339())? {
            return Err(DecisionsError::NotFound);
        }
        self.detail(id)
    }
}

/// Sanitizes a free-text query into an FTS5 `MATCH` expression.
///
/// Keeps only word characters and `-`, drops FTS operators and boolean keywords,
/// then quotes every term and joins them with `AND`. Returns `None` when nothing
/// usable remains.
pub fn sanitize_match_query(raw: &str) -> Option<String> {
    let mut tokens = Vec::new();
    for token in raw.split_whitespace() {
        let cleaned: String = token
            .chars()
            .filter(|character| {
                character.is_alphanumeric() || *character == '-' || *character == '_'
            })
            .collect();
        if cleaned.is_empty() {
            continue;
        }
        let lower = cleaned.to_ascii_lowercase();
        if matches!(lower.as_str(), "and" | "or" | "not" | "near") {
            continue;
        }
        tokens.push(cleaned);
    }
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|token| format!("\"{token}\""))
            .collect::<Vec<_>>()
            .join(" AND "),
    )
}

/// Trims and validates a required text field.
fn validate_text(value: &str, max: usize, label: &str) -> Result<String, DecisionsError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(DecisionsError::InvalidEdits(format!("{label} vazia")));
    }
    if trimmed.chars().count() > max {
        return Err(DecisionsError::InvalidEdits(format!(
            "{label} longa demais"
        )));
    }
    Ok(trimmed.to_string())
}

/// Validates one string array.
fn validate_array(items: &[String]) -> Result<Vec<String>, DecisionsError> {
    if items.len() > MAX_ARRAY_ITEMS {
        return Err(DecisionsError::InvalidEdits(
            "lista com itens demais".to_string(),
        ));
    }
    let mut cleaned = Vec::with_capacity(items.len());
    for item in items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            return Err(DecisionsError::InvalidEdits(
                "item de lista vazio".to_string(),
            ));
        }
        if trimmed.chars().count() > MAX_ARRAY_ITEM_CHARS {
            return Err(DecisionsError::InvalidEdits(
                "item de lista longo demais".to_string(),
            ));
        }
        cleaned.push(trimmed.to_string());
    }
    Ok(cleaned)
}

/// Parses a stored JSON array.
fn parse_array(raw: &str, column: &str) -> Result<Vec<String>, DecisionsError> {
    serde_json::from_str(raw).map_err(|_| DecisionsError::InvalidData(format!("{column} inválido")))
}

/// Maps a stored revision row into its full typed value.
fn revision_from_row(row: DecisionRevisionRow) -> Result<DecisionRevision, DecisionsError> {
    Ok(DecisionRevision {
        version: row.version,
        created_at: row.created_at,
        question: row.question,
        choice: row.choice,
        rationale: row.rationale,
        assumptions: parse_array(&row.assumptions, "assumptions")?,
        reconsider_when: parse_array(&row.reconsider_when, "reconsider_when")?,
        scope: parse_array(&row.scope, "scope")?,
        consequences: parse_array(&row.consequences, "consequences")?,
    })
}

/// Merges an optional array into the stored JSON array.
fn merge_array(
    provided: Option<Vec<String>>,
    stored: &str,
    column: &str,
) -> Result<String, DecisionsError> {
    let items = match provided {
        Some(items) => items,
        None => parse_array(stored, column)?,
    };
    serde_json::to_string(&items)
        .map_err(|_| DecisionsError::InvalidData(format!("{column} inválido")))
}

/// Maps a stored row into its list summary.
fn summary_from_row(row: &StoredDecision) -> DecisionSummary {
    DecisionSummary {
        decision_id: row.decision_id.clone(),
        candidate_id: row.candidate_id.clone(),
        project_id: row.project_id.clone(),
        project_location: row.project_location.clone(),
        capture_id: row.capture_id.clone(),
        status: row.status,
        question: row.question.clone(),
        choice: row.choice.clone(),
        version: row.version,
        confirmed_at: row.confirmed_at.clone(),
        updated_at: row.updated_at.clone(),
    }
}

/// Parses `"<confirmed_at>|<decision_id>"` into a cursor.
fn parse_cursor(cursor: &str) -> Result<DecisionCursor, DecisionsError> {
    match cursor.split_once('|') {
        Some((confirmed_at, decision_id))
            if !confirmed_at.is_empty() && !decision_id.is_empty() =>
        {
            Ok(DecisionCursor {
                confirmed_at: confirmed_at.to_string(),
                decision_id: decision_id.to_string(),
            })
        }
        _ => Err(DecisionsError::InvalidFilter("cursor inválido".to_string())),
    }
}
