//! Decision Inbox use case: review, edit and decide on Decision Candidates.

use serde::{Deserialize, Serialize};

use crate::clock::now_rfc3339;
use crate::extract::{CandidateKind, RelevanceSignal};

/// Maximum ids accepted by one batch operation.
pub const MAX_BATCH_IDS: usize = 100;

/// Maximum candidates returned by one page.
pub const MAX_PAGE_LIMIT: usize = 100;

/// Page size used when the caller does not pick one.
pub const DEFAULT_PAGE_LIMIT: usize = 50;

/// Maximum characters accepted in an edited question.
pub const MAX_QUESTION_CHARS: usize = 500;

/// Maximum characters accepted in an edited choice.
pub const MAX_CHOICE_CHARS: usize = 1_000;

/// Maximum characters accepted in an edited rationale.
pub const MAX_RATIONALE_CHARS: usize = 4_000;

/// Lifecycle status of a Decision Candidate (migration 0005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateStatus {
    /// Extracted, awaiting human review.
    Pending,
    /// Confirmed without edits.
    Accepted,
    /// Confirmed with edits.
    EditedAndAccepted,
    /// Rejected.
    Dismissed,
    /// Deferred for later review.
    Snoozed,
}

impl CandidateStatus {
    /// Returns the literal persisted in the `status` column.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Accepted => "accepted",
            Self::EditedAndAccepted => "edited_and_accepted",
            Self::Dismissed => "dismissed",
            Self::Snoozed => "snoozed",
        }
    }

    /// Parses a persisted status literal, or `None` when unknown.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "accepted" => Some(Self::Accepted),
            "edited_and_accepted" => Some(Self::EditedAndAccepted),
            "dismissed" => Some(Self::Dismissed),
            "snoozed" => Some(Self::Snoozed),
            _ => None,
        }
    }

    /// Returns `true` for the terminal statuses a batch must never reach.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Accepted | Self::EditedAndAccepted | Self::Dismissed
        )
    }
}

/// Failure modes of the Inbox use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// No candidate with the requested id exists.
    NotFound,
    /// The candidate is not in a status that allows the requested action.
    InvalidState,
    /// The submitted edits failed validation (empty or too long).
    InvalidEdits(String),
    /// The submitted filter is malformed (bad cursor or limit).
    InvalidFilter(String),
    /// A batch was empty.
    InvalidBatch(String),
    /// A batch exceeded [`MAX_BATCH_IDS`].
    BatchTooLarge,
    /// A stored column could not be parsed back into its typed value.
    InvalidData(String),
}

impl InboxError {
    /// Returns a short, stable code safe to surface or log.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::NotFound => "not_found",
            Self::InvalidState => "invalid_state",
            Self::InvalidEdits(_) => "invalid_edits",
            Self::InvalidFilter(_) => "invalid_filter",
            Self::InvalidBatch(_) => "invalid_batch",
            Self::BatchTooLarge => "batch_too_large",
            Self::InvalidData(_) => "invalid_data",
        }
    }
}

impl std::fmt::Display for InboxError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::NotFound => formatter.write_str("candidato não encontrado"),
            Self::InvalidState => formatter.write_str("estado do candidato não permite a ação"),
            Self::InvalidEdits(message) => write!(formatter, "edição inválida: {message}"),
            Self::InvalidFilter(message) => write!(formatter, "filtro inválido: {message}"),
            Self::InvalidBatch(message) => write!(formatter, "lote inválido: {message}"),
            Self::BatchTooLarge => formatter.write_str("lote com ids demais"),
            Self::InvalidData(message) => {
                write!(formatter, "dados do candidato inválidos: {message}")
            }
        }
    }
}

impl std::error::Error for InboxError {}

/// Filter for the virtualized inbox list.
#[derive(Debug, Clone, PartialEq)]
pub struct InboxFilter {
    /// Restrict to one project, or every project when `None`.
    pub project_id: Option<String>,
    /// Statuses to include; defaults to `[Pending, Snoozed]`.
    pub statuses: Vec<CandidateStatus>,
    /// Requested page size, capped at [`MAX_PAGE_LIMIT`].
    pub limit: usize,
    /// Opaque keyset cursor from a previous [`InboxPage::next_cursor`].
    pub cursor: Option<String>,
    /// Only candidates at least this significant; every one when `None`.
    pub min_significance: Option<f64>,
}

impl Default for InboxFilter {
    fn default() -> Self {
        Self {
            project_id: None,
            statuses: vec![CandidateStatus::Pending, CandidateStatus::Snoozed],
            limit: DEFAULT_PAGE_LIMIT,
            cursor: None,
            min_significance: None,
        }
    }
}

impl InboxFilter {
    /// Builds the default filter (pending and snoozed, default page size).
    pub fn new() -> Self {
        Self::default()
    }
}

/// Editable substantive fields of a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEdits {
    /// Explicit qualifications; artifact-less entries are reviewer declarations.
    pub qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
    /// Edited decision question.
    pub question: String,
    /// Edited proposed choice.
    pub choice: String,
    /// Edited rationale.
    pub rationale: String,
}

impl CandidateEdits {
    /// Validates and trims the edits, returning the canonical value to persist.
    pub fn validate(&self) -> Result<ValidatedEdits, InboxError> {
        crate::qualifiers::validate_qualifiers(&self.qualifiers)
            .map_err(InboxError::InvalidEdits)?;
        let question = self.question.trim();
        let choice = self.choice.trim();
        let rationale = self.rationale.trim();
        if question.is_empty() {
            return Err(InboxError::InvalidEdits("pergunta vazia".to_string()));
        }
        if choice.is_empty() {
            return Err(InboxError::InvalidEdits("escolha vazia".to_string()));
        }
        if rationale.is_empty() {
            return Err(InboxError::InvalidEdits("justificativa vazia".to_string()));
        }
        if question.chars().count() > MAX_QUESTION_CHARS {
            return Err(InboxError::InvalidEdits(
                "pergunta longa demais".to_string(),
            ));
        }
        if choice.chars().count() > MAX_CHOICE_CHARS {
            return Err(InboxError::InvalidEdits("escolha longa demais".to_string()));
        }
        if rationale.chars().count() > MAX_RATIONALE_CHARS {
            return Err(InboxError::InvalidEdits(
                "justificativa longa demais".to_string(),
            ));
        }
        Ok(ValidatedEdits {
            qualifiers: self
                .qualifiers
                .iter()
                .cloned()
                .map(|mut item| {
                    // Human edits are declarations, not extractor-verified artifact citations.
                    item.artifact_id = None;
                    item
                })
                .collect(),
            question: question.to_string(),
            choice: choice.to_string(),
            rationale: rationale.to_string(),
        })
    }
}

/// Edits already validated by [`CandidateEdits::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedEdits {
    qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
    question: String,
    choice: String,
    rationale: String,
}

impl ValidatedEdits {
    /// Serialized qualifications for persistence adapters.
    pub fn qualifiers_json(&self) -> String {
        serde_json::to_string(&self.qualifiers).expect("qualification serialization is infallible")
    }
    /// Canonical explicit qualifications.
    pub fn qualifiers(&self) -> &[crate::qualifiers::KnowledgeQualifier] {
        &self.qualifiers
    }
    /// Trimmed, non-empty decision question.
    pub fn question(&self) -> &str {
        &self.question
    }

    /// Trimmed, non-empty proposed choice.
    pub fn choice(&self) -> &str {
        &self.choice
    }

    /// Trimmed, non-empty rationale.
    pub fn rationale(&self) -> &str {
        &self.rationale
    }
}

/// Diff summary shape persisted by extraction (`{files, artifacts}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffSummary {
    /// File paths touched by the capture.
    pub files: Vec<String>,
    /// Number of artifacts the capture carried.
    pub artifacts: usize,
}

/// One source artifact referenced by a candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactView {
    /// Stable artifact identifier.
    pub artifact_id: String,
    /// Persisted `snake_case` artifact kind.
    pub kind: String,
    /// Already-redacted content.
    pub content: String,
    /// Serialized metadata object.
    pub metadata: String,
}

/// List-row projection of a candidate (§7.5 item).
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateSummary {
    /// Decision or project rule.
    pub kind: CandidateKind,
    /// How much it matters, in `[0.0, 1.0]`.
    pub significance: f64,
    /// Candidate identifier.
    pub id: String,
    /// Project identifier.
    pub project_id: String,
    /// Project canonical location (list label).
    pub project_location: String,
    /// Capture the candidate came from.
    pub capture_id: String,
    /// Current status.
    pub status: CandidateStatus,
    /// Decision question.
    pub question: String,
    /// Proposed choice.
    pub choice: String,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Explanation of the confidence.
    pub confidence_reason: String,
    /// Relevance signals that made the capture relevant.
    pub signals: Vec<RelevanceSignal>,
    /// Adapter, when a checkpoint still points at this capture.
    pub adapter: Option<String>,
    /// Adapter session, when a checkpoint still points at this capture.
    pub session_id: Option<String>,
    /// Observation time; falls back to the receipt time.
    pub observed_at: Option<String>,
    /// RFC 3339 receipt time.
    pub received_at: String,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// RFC 3339 last-update time.
    pub updated_at: String,
}

/// Full candidate detail for the diff/sources disclosure.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateDetail {
    /// Explicit qualifications; an empty list means not informed.
    pub qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
    /// The list-row fields.
    pub summary: CandidateSummary,
    /// Significance criteria the extractor ticked.
    pub criteria: Vec<String>,
    /// Inferred rationale (editable via [`Inbox::adjust`]).
    pub rationale: String,
    /// Artifact ids the candidate cites, in stored order.
    pub evidence_refs: Vec<String>,
    /// Parsed diff summary.
    pub diff_summary: DiffSummary,
    /// Referenced artifacts, in `evidence_refs` order.
    pub artifacts: Vec<ArtifactView>,
}

/// One page of candidates plus the cursor for the next page.
#[derive(Debug, Clone, PartialEq)]
pub struct InboxPage {
    /// The page rows.
    pub candidates: Vec<CandidateSummary>,
    /// Opaque cursor for the next page, or `None` at the end.
    pub next_cursor: Option<String>,
}

/// Keyset position `(created_at, id)` for pagination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// `created_at` of the last row of the previous page.
    pub created_at: String,
    /// `id` of the last row of the previous page.
    pub id: String,
}

/// Port-level query handed to [`InboxStore::list`].
#[derive(Debug, Clone, PartialEq)]
pub struct InboxQuery {
    /// Only candidates at least this significant.
    pub min_significance: Option<f64>,
    /// Restrict to one project, or every project when `None`.
    pub project_id: Option<String>,
    /// Statuses to include.
    pub statuses: Vec<CandidateStatus>,
    /// Maximum rows to return.
    pub limit: usize,
    /// Return rows ordered strictly after this position.
    pub before: Option<Cursor>,
}

/// A raw candidate row with its joined provenance fields.
#[derive(Debug, Clone, PartialEq)]
pub struct StoredCandidate {
    /// Serialized explicit qualification array.
    pub qualifiers: String,
    /// `decision` or `rule`.
    pub kind: String,
    /// Significance in `[0.0, 1.0]`.
    pub significance: f64,
    /// Serialized significance criteria array.
    pub criteria: String,
    /// Candidate identifier.
    pub id: String,
    /// Project identifier.
    pub project_id: String,
    /// Project canonical location.
    pub project_location: String,
    /// Capture the candidate came from.
    pub capture_id: String,
    /// Current status.
    pub status: CandidateStatus,
    /// Decision question.
    pub question: String,
    /// Proposed choice.
    pub choice: String,
    /// Inferred rationale.
    pub rationale: String,
    /// Serialized signals array.
    pub signals: String,
    /// Confidence in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Explanation of the confidence.
    pub confidence_reason: String,
    /// Serialized `evidence_refs` array.
    pub evidence_refs: String,
    /// Serialized diff summary.
    pub diff_summary: String,
    /// Adapter, when a checkpoint still points at this capture.
    pub adapter: Option<String>,
    /// Adapter session, when a checkpoint still points at this capture.
    pub session_id: Option<String>,
    /// Observation time; falls back to the receipt time.
    pub observed_at: Option<String>,
    /// RFC 3339 receipt time.
    pub received_at: String,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// RFC 3339 last-update time.
    pub updated_at: String,
}

/// Values the confirmation writes into the new Engineering Decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSeed {
    /// Qualifications copied intact from the reviewed candidate.
    pub qualifiers: String,
    /// Confirm as a project rule (a claim) instead of a decision.
    pub as_rule: bool,
    /// Identifier of the new decision (UUID v7, generated by the use case).
    pub decision_id: String,
    /// Project the decision belongs to.
    pub project_id: String,
    /// Capture the candidate came from, when recorded.
    pub capture_id: Option<String>,
    /// Final question (edited or original).
    pub question: String,
    /// Final choice (edited or original).
    pub choice: String,
    /// Final rationale (edited or original).
    pub rationale: String,
    /// Artifact ids in reference order.
    pub evidence_refs: Vec<String>,
}

/// Result of confirming one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmOutcome {
    /// Whether it became a project rule; `decision_id` then holds the claim id.
    pub rule: bool,
    /// Status the candidate reached.
    pub status: CandidateStatus,
    /// Identifier of the Engineering Decision created by the confirmation.
    pub decision_id: String,
}

/// Persistence port the Inbox use case needs.
pub trait InboxStore {
    /// The components the extractor said a candidate applies to, each with
    /// its quote. Empty for stores without the data.
    fn candidate_components(
        &self,
        _candidate_id: &str,
    ) -> Result<Vec<crate::extract::CandidateComponent>, InboxError> {
        Ok(Vec::new())
    }
    /// Checks presentation eligibility independent of the visible page.
    /// Production stores should implement a bounded per-id query.
    fn eligible_in_review(
        &self,
        project_id: &str,
        candidate_id: &str,
        query: &InboxQuery,
    ) -> Result<bool, InboxError> {
        let mut query = query.clone();
        query.project_id = Some(project_id.to_owned());
        query.before = None;
        query.limit = MAX_PAGE_LIMIT;
        loop {
            let rows = self.list(&query)?;
            if rows.iter().any(|row| row.id == candidate_id) {
                return Ok(true);
            }
            if rows.len() < query.limit {
                return Ok(false);
            }
            let last = &rows[rows.len() - 1];
            query.before = Some(Cursor {
                created_at: last.created_at.clone(),
                id: last.id.clone(),
            });
        }
    }
    /// Counts what changed in a project since `since` (RFC 3339). Stores
    /// without the data answer with nothing changed.
    fn briefing(
        &self,
        _project_id: &str,
        since: &str,
    ) -> Result<crate::briefing::Briefing, InboxError> {
        Ok(crate::briefing::Briefing {
            since: since.to_owned(),
            ..Default::default()
        })
    }

    /// Counts the whole filtered queue, independent of pagination. Stores can
    /// override this paginated fallback with an aggregate query.
    fn count(
        &self,
        project_id: Option<&str>,
        statuses: &[CandidateStatus],
        min_significance: Option<f64>,
    ) -> Result<usize, InboxError> {
        let mut query = InboxQuery {
            min_significance,
            project_id: project_id.map(str::to_owned),
            statuses: statuses.to_vec(),
            limit: MAX_PAGE_LIMIT,
            before: None,
        };
        let mut total = 0;
        loop {
            let rows = self.list(&query)?;
            total += rows.len();
            if rows.len() < query.limit {
                return Ok(total);
            }
            let last = &rows[rows.len() - 1];
            query.before = Some(Cursor {
                created_at: last.created_at.clone(),
                id: last.id.clone(),
            });
        }
    }

    /// Returns candidates matching `query`, ordered `created_at DESC, id DESC`.
    fn list(&self, query: &InboxQuery) -> Result<Vec<StoredCandidate>, InboxError>;

    /// Returns one candidate with its joined provenance fields.
    fn get(&self, id: &str) -> Result<Option<StoredCandidate>, InboxError>;

    /// Returns the referenced artifacts, in `refs` order.
    fn artifacts(&self, capture_id: &str, refs: &[String])
        -> Result<Vec<ArtifactView>, InboxError>;

    /// Confirms one candidate from `pending|snoozed`, promoting it to an
    /// Engineering Decision in the same transaction.
    fn confirm_one(
        &self,
        id: &str,
        expected: &StoredCandidate,
        edits: Option<&ValidatedEdits>,
        seed: &DecisionSeed,
        updated_at: &str,
    ) -> Result<bool, InboxError>;

    /// Saves validated edits in place, keeping `pending|snoozed`.
    fn adjust_one(
        &self,
        id: &str,
        edits: &ValidatedEdits,
        updated_at: &str,
    ) -> Result<bool, InboxError>;

    /// Rejects one candidate from `pending|snoozed`.
    fn dismiss_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError>;

    /// Rejects many candidates from `pending|snoozed`; returns the affected count.
    fn dismiss_batch(&self, ids: &[String], updated_at: &str) -> Result<usize, InboxError>;

    /// Defers one candidate from `pending`.
    fn snooze_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError>;

    /// Defers many candidates from `pending`; returns the affected count.
    fn snooze_batch(&self, ids: &[String], updated_at: &str) -> Result<usize, InboxError>;

    /// Returns one candidate from `snoozed` to `pending`.
    fn unsnooze_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError>;

    /// Returns one candidate from `dismissed` to `pending` (undoing a
    /// rejection). Stores that cannot answer `false`.
    fn reopen_one(&self, _id: &str, _updated_at: &str) -> Result<bool, InboxError> {
        Ok(false)
    }
}

/// Decision Inbox use case over an [`InboxStore`].
#[derive(Debug, Clone)]
pub struct Inbox<S> {
    store: S,
}

impl<S: InboxStore> Inbox<S> {
    /// Checks whether an id is a current review representative for this project
    /// and filter. Page limit/cursor do not affect eligibility. The caller must
    /// explicitly include its low-relevance threshold and status policy.
    pub fn eligible_in_review(
        &self,
        project_id: &str,
        candidate_id: &str,
        filter: &InboxFilter,
    ) -> Result<bool, InboxError> {
        if filter.statuses.is_empty() {
            return Err(InboxError::InvalidFilter("nenhum estado informado".into()));
        }
        if filter
            .project_id
            .as_deref()
            .is_some_and(|id| id != project_id)
        {
            return Ok(false);
        }
        self.store.eligible_in_review(
            project_id,
            candidate_id,
            &InboxQuery {
                project_id: Some(project_id.to_owned()),
                statuses: filter.statuses.clone(),
                min_significance: filter.min_significance,
                limit: 1,
                before: None,
            },
        )
    }
    /// What changed in a project since `since` (the previous visit).
    pub fn briefing(
        &self,
        project_id: &str,
        since: &str,
    ) -> Result<crate::briefing::Briefing, InboxError> {
        self.store.briefing(project_id, since)
    }

    /// Counts every matching candidate; page size and cursor do not limit it.
    pub fn count(&self, filter: &InboxFilter) -> Result<usize, InboxError> {
        if filter.statuses.is_empty() {
            return Err(InboxError::InvalidFilter("nenhum estado informado".into()));
        }
        self.store.count(
            filter.project_id.as_deref(),
            &filter.statuses,
            filter.min_significance,
        )
    }
    /// Wraps a store with the Inbox use case.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Lists candidates for the virtualized inbox.
    pub fn list(&self, filter: &InboxFilter) -> Result<InboxPage, InboxError> {
        if filter.limit == 0 {
            return Err(InboxError::InvalidFilter("limite zero".to_string()));
        }
        if filter.statuses.is_empty() {
            return Err(InboxError::InvalidFilter(
                "nenhum estado informado".to_string(),
            ));
        }
        let limit = filter.limit.min(MAX_PAGE_LIMIT);
        let before = match filter.cursor.as_deref() {
            Some(cursor) => Some(parse_cursor(cursor)?),
            None => None,
        };
        let query = InboxQuery {
            min_significance: filter.min_significance,
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
                .map(|row| format!("{}|{}", row.created_at, row.id))
        } else {
            None
        };
        let candidates = rows
            .iter()
            .map(summary_from_row)
            .collect::<Result<_, _>>()?;
        Ok(InboxPage {
            candidates,
            next_cursor,
        })
    }

    /// Loads the full detail of one candidate.
    pub fn detail(&self, id: &str) -> Result<CandidateDetail, InboxError> {
        let row = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        let summary = summary_from_row(&row)?;
        let refs = parse_string_array(&row.evidence_refs, "evidence_refs")?;
        let diff_summary: DiffSummary = serde_json::from_str(&row.diff_summary)
            .map_err(|_| InboxError::InvalidData("diff_summary inválido".to_string()))?;
        let artifacts = self.store.artifacts(&row.capture_id, &refs)?;
        Ok(CandidateDetail {
            qualifiers: crate::qualifiers::decode(&row.qualifiers)
                .map_err(InboxError::InvalidEdits)?,
            criteria: parse_string_array(&row.criteria, "criteria").unwrap_or_default(),
            summary,
            rationale: row.rationale,
            evidence_refs: refs,
            diff_summary,
            artifacts,
        })
    }

    /// Confirms one candidate, optionally applying edits.
    pub fn confirm(
        &self,
        id: &str,
        edits: Option<CandidateEdits>,
    ) -> Result<ConfirmOutcome, InboxError> {
        self.confirm_revision(id, None, edits)
    }

    /// Confirms only the exact detail previously displayed by the caller.
    ///
    /// A changed status, qualification, source, or substantive field fails closed.
    /// The persistence adapter additionally compares this snapshot transactionally.
    pub fn confirm_reviewed(
        &self,
        reviewed: &CandidateDetail,
        edits: Option<CandidateEdits>,
    ) -> Result<ConfirmOutcome, InboxError> {
        self.confirm_revision(&reviewed.summary.id, Some(reviewed), edits)
    }

    fn confirm_revision(
        &self,
        id: &str,
        reviewed: Option<&CandidateDetail>,
        edits: Option<CandidateEdits>,
    ) -> Result<ConfirmOutcome, InboxError> {
        let (status, mut validated) = match edits {
            Some(edits) => (CandidateStatus::EditedAndAccepted, Some(edits.validate()?)),
            None => (CandidateStatus::Accepted, None),
        };
        let current = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        if let Some(reviewed) = reviewed {
            let qualifiers =
                crate::qualifiers::decode(&current.qualifiers).map_err(InboxError::InvalidData)?;
            let refs = parse_string_array(&current.evidence_refs, "evidence_refs")?;
            if summary_from_row(&current)? != reviewed.summary
                || current.rationale != reviewed.rationale
                || qualifiers != reviewed.qualifiers
                || refs != reviewed.evidence_refs
            {
                return Err(InboxError::InvalidState);
            }
        }
        if let Some(edits) = &mut validated {
            let original =
                crate::qualifiers::decode(&current.qualifiers).map_err(InboxError::InvalidEdits)?;
            for item in &mut edits.qualifiers {
                if let Some(previous) = original
                    .iter()
                    .find(|previous| previous.kind == item.kind && previous.text == item.text)
                {
                    item.artifact_id = previous.artifact_id.clone();
                }
            }
        }
        let allowed = [CandidateStatus::Pending, CandidateStatus::Snoozed];
        if !allowed.contains(&current.status) {
            return Err(InboxError::InvalidState);
        }
        let (question, choice, rationale) = match &validated {
            Some(edits) => (
                edits.question().to_string(),
                edits.choice().to_string(),
                edits.rationale().to_string(),
            ),
            None => (
                current.question.clone(),
                current.choice.clone(),
                current.rationale.clone(),
            ),
        };
        let seed = DecisionSeed {
            qualifiers: match &validated {
                Some(edits) => serde_json::to_string(edits.qualifiers())
                    .map_err(|e| InboxError::InvalidEdits(e.to_string()))?,
                None => current.qualifiers.clone(),
            },
            as_rule: current.kind == CandidateKind::Rule.as_str(),
            decision_id: uuid::Uuid::now_v7().to_string(),
            project_id: current.project_id.clone(),
            capture_id: Some(current.capture_id.clone()),
            question,
            choice,
            rationale,
            evidence_refs: parse_string_array(&current.evidence_refs, "evidence_refs")?,
        };
        let applied =
            self.store
                .confirm_one(id, &current, validated.as_ref(), &seed, &now_rfc3339())?;
        if !applied {
            return Err(InboxError::InvalidState);
        }
        Ok(ConfirmOutcome {
            rule: seed.as_rule,
            status,
            decision_id: seed.decision_id,
        })
    }

    /// Saves edits while keeping the candidate in the review queue.
    pub fn adjust(&self, id: &str, edits: CandidateEdits) -> Result<(), InboxError> {
        let mut edits = edits.validate()?;
        let current = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        let original =
            crate::qualifiers::decode(&current.qualifiers).map_err(InboxError::InvalidEdits)?;
        for item in &mut edits.qualifiers {
            if let Some(previous) = original
                .iter()
                .find(|previous| previous.kind == item.kind && previous.text == item.text)
            {
                item.artifact_id = previous.artifact_id.clone();
            }
        }
        let allowed = [CandidateStatus::Pending, CandidateStatus::Snoozed];
        if !allowed.contains(&current.status) {
            return Err(InboxError::InvalidState);
        }
        if !self.store.adjust_one(id, &edits, &now_rfc3339())? {
            return Err(InboxError::InvalidState);
        }
        Ok(())
    }

    /// Rejects one candidate.
    pub fn reject(&self, id: &str) -> Result<(), InboxError> {
        let current = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        if !matches!(
            current.status,
            CandidateStatus::Pending | CandidateStatus::Snoozed
        ) {
            return Err(InboxError::InvalidState);
        }
        let now = now_rfc3339();
        if !self.store.dismiss_one(id, &now)? {
            return Err(InboxError::InvalidState);
        }
        Ok(())
    }

    /// Rejects many candidates; returns the affected count.
    pub fn dismiss_batch(&self, ids: &[String]) -> Result<usize, InboxError> {
        Self::validate_batch(ids)?;
        let affected = self.store.dismiss_batch(ids, &now_rfc3339())?;
        self.after_batch(ids, affected)
    }

    /// Defers one pending candidate.
    pub fn snooze(&self, id: &str) -> Result<(), InboxError> {
        let current = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        if current.status != CandidateStatus::Pending {
            return Err(InboxError::InvalidState);
        }
        let now = now_rfc3339();
        if !self.store.snooze_one(id, &now)? {
            return Err(InboxError::InvalidState);
        }
        Ok(())
    }

    /// Defers many pending candidates; returns the affected count.
    pub fn snooze_batch(&self, ids: &[String]) -> Result<usize, InboxError> {
        Self::validate_batch(ids)?;
        let affected = self.store.snooze_batch(ids, &now_rfc3339())?;
        self.after_batch(ids, affected)
    }

    /// Undoes a rejection: a dismissed candidate returns to the pending queue.
    pub fn reopen(&self, id: &str) -> Result<(), InboxError> {
        let current = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        if current.status != CandidateStatus::Dismissed {
            return Err(InboxError::InvalidState);
        }
        if !self.store.reopen_one(id, &now_rfc3339())? {
            return Err(InboxError::InvalidState);
        }
        Ok(())
    }

    /// Returns a snoozed candidate to the pending queue.
    pub fn unsnooze(&self, id: &str) -> Result<(), InboxError> {
        let current = self.store.get(id)?.ok_or(InboxError::NotFound)?;
        if current.status != CandidateStatus::Snoozed {
            return Err(InboxError::InvalidState);
        }
        let now = now_rfc3339();
        if !self.store.unsnooze_one(id, &now)? {
            return Err(InboxError::InvalidState);
        }
        Ok(())
    }

    /// Rejects an empty or oversized batch before touching the store.
    fn validate_batch(ids: &[String]) -> Result<(), InboxError> {
        if ids.is_empty() {
            return Err(InboxError::InvalidBatch("lista vazia".to_string()));
        }
        if ids.len() > MAX_BATCH_IDS {
            return Err(InboxError::BatchTooLarge);
        }
        Ok(())
    }

    /// Turns a zero-affected batch into the right stable error.
    fn after_batch(&self, ids: &[String], affected: usize) -> Result<usize, InboxError> {
        if affected == 0 {
            let any_exists = ids
                .iter()
                .any(|id| matches!(self.store.get(id), Ok(Some(_))));
            return Err(if any_exists {
                InboxError::InvalidState
            } else {
                InboxError::NotFound
            });
        }
        Ok(affected)
    }
}

impl<S: InboxStore + crate::review_exception::ReviewExceptionStore> Inbox<S> {
    /// Discloses retained occurrences without replacing selected source detail.
    pub fn group_detail(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<crate::review_exception::ReviewGroup, InboxError> {
        self.store.review_group(id, offset, limit)
    }

    /// Reports potential review opportunities, never measured human actions.
    pub fn review_metrics(
        &self,
        project_id: &str,
    ) -> Result<crate::review_exception::ReviewMetrics, InboxError> {
        self.store.review_metrics(project_id)
    }
}

/// Parses `"<created_at>|<id>"` into a cursor.
fn parse_cursor(cursor: &str) -> Result<Cursor, InboxError> {
    match cursor.split_once('|') {
        Some((created_at, id)) if !created_at.is_empty() && !id.is_empty() => Ok(Cursor {
            created_at: created_at.to_string(),
            id: id.to_string(),
        }),
        _ => Err(InboxError::InvalidFilter("cursor inválido".to_string())),
    }
}

/// Maps a stored row into its list summary.
fn summary_from_row(row: &StoredCandidate) -> Result<CandidateSummary, InboxError> {
    let labels = parse_string_array(&row.signals, "signals")?;
    let signals = labels
        .iter()
        .filter_map(|label| signal_from_str(label))
        .collect();
    Ok(CandidateSummary {
        kind: CandidateKind::parse(&row.kind),
        significance: row.significance,
        id: row.id.clone(),
        project_id: row.project_id.clone(),
        project_location: row.project_location.clone(),
        capture_id: row.capture_id.clone(),
        status: row.status,
        question: row.question.clone(),
        choice: row.choice.clone(),
        confidence: row.confidence,
        confidence_reason: row.confidence_reason.clone(),
        signals,
        adapter: row.adapter.clone(),
        session_id: row.session_id.clone(),
        observed_at: row.observed_at.clone(),
        received_at: row.received_at.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
    })
}

/// Parses a JSON array of strings from a stored column.
fn parse_string_array(raw: &str, column: &str) -> Result<Vec<String>, InboxError> {
    serde_json::from_str(raw).map_err(|_| InboxError::InvalidData(format!("{column} inválido")))
}

/// Maps a persisted signal literal; unknown literals are skipped.
fn signal_from_str(label: &str) -> Option<RelevanceSignal> {
    match label {
        "public_contract" => Some(RelevanceSignal::PublicContract),
        "security_privacy" => Some(RelevanceSignal::SecurityPrivacy),
        "dependency_added" => Some(RelevanceSignal::DependencyAdded),
        "crosses_boundaries" => Some(RelevanceSignal::CrossesBoundaries),
        "hard_to_revert" => Some(RelevanceSignal::HardToRevert),
        "rejects_alternative" => Some(RelevanceSignal::RejectsAlternative),
        "conditions_future_work" => Some(RelevanceSignal::ConditionsFutureWork),
        "material_blast_radius" => Some(RelevanceSignal::MaterialBlastRadius),
        "alternatives_compared" => Some(RelevanceSignal::AlternativesCompared),
        "explicit_tradeoff" => Some(RelevanceSignal::ExplicitTradeoff),
        "disagreement_uncertainty" => Some(RelevanceSignal::DisagreementUncertainty),
        "relevant_cost" => Some(RelevanceSignal::RelevantCost),
        "validity_months" => Some(RelevanceSignal::ValidityMonths),
        "maintenance_onboarding" => Some(RelevanceSignal::MaintenanceOnboarding),
        "delegated_to_agent" => Some(RelevanceSignal::DelegatedToAgent),
        "unproven_assumption" => Some(RelevanceSignal::UnprovenAssumption),
        _ => None,
    }
}
