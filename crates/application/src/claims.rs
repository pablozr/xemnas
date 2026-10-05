//! Context Claims use case: create, list by date and retire.

use domain::claims::{ClaimError, ClaimKind, ContextClaim};
use domain::time::Timestamp;

use crate::clock::now_rfc3339;
use crate::decisions::{DecisionStore, DecisionsError};
use crate::projects::{ProjectError, ProjectRepository};

/// Failure modes of the claims use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimsError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// No claim with the requested id exists.
    NotFound,
    /// The project does not exist.
    ProjectNotFound,
    /// The source decision does not exist or belongs to another project.
    InvalidSource,
    /// The claim breaks a domain rule.
    Invalid(ClaimError),
    /// The claim changed since it was read.
    Conflict,
}

impl ClaimsError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::NotFound => "not_found",
            Self::ProjectNotFound => "project_not_found",
            Self::InvalidSource => "invalid_source",
            Self::Invalid(error) => error.code(),
            Self::Conflict => "conflict",
        }
    }
}

impl std::fmt::Display for ClaimsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::NotFound => formatter.write_str("afirmação não encontrada"),
            Self::ProjectNotFound => formatter.write_str("projeto não encontrado"),
            Self::InvalidSource => {
                formatter.write_str("a decisão de origem não existe neste projeto")
            }
            Self::Invalid(error) => write!(formatter, "{error}"),
            Self::Conflict => formatter.write_str("a afirmação mudou; atualize e tente de novo"),
        }
    }
}

impl std::error::Error for ClaimsError {}

impl From<ClaimError> for ClaimsError {
    fn from(error: ClaimError) -> Self {
        Self::Invalid(error)
    }
}

/// A persisted claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRecord {
    /// Exact source revision; unknown for legacy/direct claims.
    pub source_version: Option<i64>,
    /// Scope inherited from the exact source version, independently bounded.
    pub inherited_scope: String,
    /// Explicit qualifications inherited intact from the source version.
    pub qualifiers: String,
    /// Claim identifier (UUID v7).
    pub claim_id: String,
    /// Owning project.
    pub project_id: String,
    /// Claim kind.
    pub kind: ClaimKind,
    /// Trimmed statement.
    pub statement: String,
    /// Start of validity (inclusive).
    pub valid_from: String,
    /// End of validity (exclusive), when set.
    pub valid_until: Option<String>,
    /// Decision the claim was derived from, when any.
    pub source_decision_id: Option<String>,
    /// RFC 3339 creation time.
    pub created_at: String,
    /// RFC 3339 last update.
    pub updated_at: String,
}

impl ClaimRecord {
    /// Whether the claim holds at `at`; unparseable dates never hold.
    pub fn is_valid_at(&self, at: &Timestamp) -> bool {
        self.to_domain().is_some_and(|claim| claim.is_valid_at(at))
    }

    fn to_domain(&self) -> Option<ContextClaim> {
        ContextClaim::new(
            self.kind,
            &self.statement,
            &self.valid_from,
            self.valid_until.as_deref(),
        )
        .ok()
    }
}

/// Input for [`Claims::create`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClaim {
    /// Expected original source version; absent for direct human claims.
    pub source_version: Option<i64>,
    /// Human qualifications; derived claims inherit their source instead.
    pub qualifiers: Vec<crate::qualifiers::KnowledgeQualifier>,
    /// Owning project.
    pub project_id: String,
    /// Claim kind.
    pub kind: ClaimKind,
    /// Statement text.
    pub statement: String,
    /// Start of validity; now when `None`.
    pub valid_from: Option<String>,
    /// End of validity, when known.
    pub valid_until: Option<String>,
    /// Decision the claim comes from, when any.
    pub source_decision_id: Option<String>,
}

/// Persistence port for claims.
pub trait ClaimStore {
    /// Inserts only while the source still has the version read by the caller.
    /// Adapters must compare and insert in the same transaction.
    fn insert_claim_versioned(
        &self,
        record: &ClaimRecord,
        source_version: Option<i64>,
    ) -> Result<(), ClaimsError> {
        if source_version.is_some() {
            return Err(ClaimsError::InvalidSource);
        }
        self.insert_claim(record)
    }
    /// Inserts a claim and indexes its statement.
    fn insert_claim(&self, record: &ClaimRecord) -> Result<(), ClaimsError>;

    /// Returns one claim.
    fn get_claim(&self, claim_id: &str) -> Result<Option<ClaimRecord>, ClaimsError>;

    /// Every claim of a project, ordered by `valid_from`, then id.
    fn project_claims(&self, project_id: &str) -> Result<Vec<ClaimRecord>, ClaimsError>;

    /// Sets `valid_until` only if it still equals `expected_until`.
    fn close_claim(
        &self,
        claim_id: &str,
        valid_until: &str,
        expected_until: Option<&str>,
        updated_at: &str,
    ) -> Result<bool, ClaimsError>;
}

/// Claims use case.
#[derive(Debug, Clone)]
pub struct Claims<S> {
    store: S,
}

impl<S: ClaimStore + DecisionStore + ProjectRepository> Claims<S> {
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Creates a claim after validating it and its source decision.
    ///
    /// # Errors
    ///
    /// Domain rule violations, an unknown project or an invalid source.
    pub fn create(&self, input: NewClaim) -> Result<ClaimRecord, ClaimsError> {
        let now = now_rfc3339();
        let claim = ContextClaim::new(
            input.kind,
            &input.statement,
            input.valid_from.as_deref().unwrap_or(&now),
            input.valid_until.as_deref(),
        )?;
        ProjectRepository::get(&self.store, &input.project_id)
            .map_err(project_error)?
            .ok_or(ClaimsError::ProjectNotFound)?;
        let mut source_version = None;
        let mut inherited_scope = "[]".to_string();
        let qualifiers = if let Some(source) = &input.source_decision_id {
            let decision = DecisionStore::get(&self.store, source)
                .map_err(decision_error)?
                .ok_or(ClaimsError::InvalidSource)?;
            if decision.project_id != input.project_id {
                return Err(ClaimsError::InvalidSource);
            }
            if input
                .source_version
                .is_some_and(|version| version != decision.version)
            {
                return Err(ClaimsError::InvalidSource);
            }
            source_version = Some(decision.version);
            let inherited =
                crate::qualifiers::decode(&decision.qualifiers).map_err(ClaimsError::Storage)?;
            let scope: Vec<String> = serde_json::from_str(&decision.scope)
                .map_err(|e| ClaimsError::Storage(e.to_string()))?;
            inherited_scope =
                serde_json::to_string(&scope).map_err(|e| ClaimsError::Storage(e.to_string()))?;
            crate::qualifiers::validate_qualifiers(&inherited).map_err(ClaimsError::Storage)?;
            serde_json::to_string(&inherited).map_err(|e| ClaimsError::Storage(e.to_string()))?
        } else {
            crate::qualifiers::validate_qualifiers(&input.qualifiers)
                .map_err(ClaimsError::Storage)?;
            serde_json::to_string(&input.qualifiers)
                .map_err(|e| ClaimsError::Storage(e.to_string()))?
        };
        let record = ClaimRecord {
            source_version,
            inherited_scope,
            qualifiers,
            claim_id: uuid::Uuid::now_v7().to_string(),
            project_id: input.project_id,
            kind: claim.kind(),
            statement: claim.statement().to_string(),
            valid_from: claim.valid_from().to_string(),
            valid_until: claim.valid_until().map(ToString::to_string),
            source_decision_id: input.source_decision_id,
            created_at: now.clone(),
            updated_at: now,
        };
        self.store.insert_claim_versioned(&record, source_version)?;
        Ok(record)
    }

    /// Claims of a project; only those valid at `as_of` when given.
    ///
    /// # Errors
    ///
    /// `invalid_date` for a malformed `as_of`, `storage` on query failure.
    pub fn list(
        &self,
        project_id: &str,
        as_of: Option<&str>,
    ) -> Result<Vec<ClaimRecord>, ClaimsError> {
        let as_of = as_of
            .map(|value| Timestamp::parse(value).ok_or(ClaimError::InvalidDate))
            .transpose()?;
        Ok(self
            .store
            .project_claims(project_id)?
            .into_iter()
            .filter(|claim| as_of.as_ref().is_none_or(|at| claim.is_valid_at(at)))
            .collect())
    }

    /// Ends a claim at `at` (now when `None`), keeping it in the history.
    ///
    /// # Errors
    ///
    /// `not_found`, domain rule violations, or `conflict` on a concurrent change.
    pub fn retire(&self, claim_id: &str, at: Option<&str>) -> Result<ClaimRecord, ClaimsError> {
        let record = self
            .store
            .get_claim(claim_id)?
            .ok_or(ClaimsError::NotFound)?;
        let mut claim = record.to_domain().ok_or(ClaimError::InvalidDate)?;
        let now = now_rfc3339();
        claim.retire(at.unwrap_or(&now))?;
        let until = claim
            .valid_until()
            .map(ToString::to_string)
            .ok_or(ClaimError::InvalidDate)?;
        if !self
            .store
            .close_claim(claim_id, &until, record.valid_until.as_deref(), &now)?
        {
            return Err(ClaimsError::Conflict);
        }
        Ok(ClaimRecord {
            valid_until: Some(until),
            updated_at: now,
            ..record
        })
    }
}

fn project_error(error: ProjectError) -> ClaimsError {
    ClaimsError::Storage(error.to_string())
}

fn decision_error(error: DecisionsError) -> ClaimsError {
    ClaimsError::Storage(error.to_string())
}
