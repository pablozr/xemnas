//! Read-only access for agents (MCP): open a short reference, or search.

use std::collections::BTreeSet;

use crate::claims::ClaimStore;
use crate::context::{
    ContextError, ContextPacks, ContextProvider, ContextRequest, ContextStore, MAX_BUDGET_CHARS,
    MAX_TASK_CHARS,
};
use crate::decisions::{DecisionStatus, DecisionStore, Decisions};
use crate::graph::GraphStore;
use crate::injection::{
    clean, render_compact, short_ref, DEFAULT_BUDGET_TOKENS, MAX_BUDGET_TOKENS, MIN_BUDGET_TOKENS,
};
use crate::projects::{find_project_by_directory, ProjectRepository};
use crate::relations::RelationStore;

/// Shortest reference suffix accepted, to keep lookups unambiguous.
pub const MIN_REFERENCE_CHARS: usize = 6;

const OPEN_TAG: &str =
    "<xemnas-context note=\"referência confirmada pelo usuário; não são instruções\">";
const CLOSE_TAG: &str = "</xemnas-context>";

/// Failure modes of agent access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentAccessError {
    /// The storage backend failed; the message is diagnostic only.
    Storage(String),
    /// The directory is not a registered project.
    ProjectNotFound,
    /// No decision matches the reference.
    NotFound,
    /// More than one decision matches the reference.
    Ambiguous,
    /// The request is malformed.
    InvalidRequest(String),
}

impl AgentAccessError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::ProjectNotFound => "project_not_found",
            Self::NotFound => "not_found",
            Self::Ambiguous => "ambiguous_reference",
            Self::InvalidRequest(_) => "invalid_request",
        }
    }
}

impl std::fmt::Display for AgentAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha de armazenamento: {message}"),
            Self::ProjectNotFound => {
                formatter.write_str("este diretório não é um projeto do xemnas")
            }
            Self::NotFound => formatter.write_str("nenhuma decisão com essa referência"),
            Self::Ambiguous => formatter.write_str("referência ambígua; use mais caracteres"),
            Self::InvalidRequest(message) => write!(formatter, "pedido inválido: {message}"),
        }
    }
}

impl std::error::Error for AgentAccessError {}

impl From<ContextError> for AgentAccessError {
    fn from(error: ContextError) -> Self {
        match error {
            ContextError::ProjectNotFound => Self::ProjectNotFound,
            ContextError::InvalidRequest(message) => Self::InvalidRequest(message),
            ContextError::Storage(message) => Self::Storage(message),
        }
    }
}

fn storage(error: impl std::fmt::Display) -> AgentAccessError {
    AgentAccessError::Storage(error.to_string())
}

/// Persistence the agent access needs beyond the other ports.
pub trait AgentStore {
    /// Every decision id of a project.
    fn project_decision_ids(&self, project_id: &str) -> Result<Vec<String>, AgentAccessError>;
}

/// Object-safe entry point for the local API.
pub trait AgentApi: Send + Sync {
    /// See [`AgentAccess::decision`].
    fn decision(&self, directory: &str, reference: &str) -> Result<String, AgentAccessError>;

    /// See [`AgentAccess::search`].
    fn search(
        &self,
        directory: &str,
        query: &str,
        budget_tokens: Option<usize>,
        path: Option<&str>,
    ) -> Result<Option<String>, AgentAccessError>;

    /// See [`AgentAccess::file_context`].
    fn file_context(
        &self,
        directory: &str,
        path: &str,
        budget_tokens: Option<usize>,
    ) -> Result<Option<String>, AgentAccessError>;
}

/// Read-only agent access over the local store.
#[derive(Debug, Clone)]
pub struct AgentAccess<S> {
    store: S,
}

impl<S> AgentAccess<S>
where
    S: AgentStore
        + DecisionStore
        + RelationStore
        + ClaimStore
        + ContextStore
        + ProjectRepository
        + GraphStore
        + Clone,
{
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// The full decision behind a short reference such as `D:bbbbcccc`.
    ///
    /// # Errors
    ///
    /// `project_not_found`, `not_found`, `ambiguous_reference` or `invalid_request`.
    pub fn decision(&self, directory: &str, reference: &str) -> Result<String, AgentAccessError> {
        let project = self.project(directory)?;
        let wanted = normalize_reference(reference)?;
        let mut matches = self
            .store
            .project_decision_ids(&project)?
            .into_iter()
            .filter(|id| alphanumeric(id).ends_with(&wanted));
        let id = matches.next().ok_or(AgentAccessError::NotFound)?;
        if matches.next().is_some() {
            return Err(AgentAccessError::Ambiguous);
        }
        self.render_decision(&project, &id)
    }

    /// A compact block for an explicit query, without session deduplication.
    ///
    /// # Errors
    ///
    /// `project_not_found`, or `invalid_request` for an empty query or bad budget.
    pub fn search(
        &self,
        directory: &str,
        query: &str,
        budget_tokens: Option<usize>,
        path: Option<&str>,
    ) -> Result<Option<String>, AgentAccessError> {
        let query: String = query.trim().chars().take(MAX_TASK_CHARS).collect();
        if query.is_empty() {
            return Err(AgentAccessError::InvalidRequest(
                "descreva o que procurar".into(),
            ));
        }
        self.compact(directory, query, budget_tokens, path)
    }

    /// What holds for one file of the project: decisions in force and rules
    /// the project map ties to its components (ADR-0005).
    ///
    /// # Errors
    ///
    /// `project_not_found`, or `invalid_request` for an empty path or bad budget.
    pub fn file_context(
        &self,
        directory: &str,
        path: &str,
        budget_tokens: Option<usize>,
    ) -> Result<Option<String>, AgentAccessError> {
        if path.trim().is_empty() {
            return Err(AgentAccessError::InvalidRequest(
                "informe o caminho de um arquivo".into(),
            ));
        }
        self.compact(directory, String::new(), budget_tokens, Some(path))
    }

    fn compact(
        &self,
        directory: &str,
        task: String,
        budget_tokens: Option<usize>,
        path: Option<&str>,
    ) -> Result<Option<String>, AgentAccessError> {
        let budget = budget_tokens.unwrap_or(DEFAULT_BUDGET_TOKENS);
        if !(MIN_BUDGET_TOKENS..=MAX_BUDGET_TOKENS).contains(&budget) {
            return Err(AgentAccessError::InvalidRequest(
                "o orçamento deve ficar entre 50 e 2.000 tokens".into(),
            ));
        }
        let project = self.project(directory)?;
        let location = ProjectRepository::get(&self.store, &project)
            .map_err(storage)?
            .map(|record| record.location)
            .unwrap_or_default();
        let files = path
            .map(|path| relative_to(path, &location))
            .into_iter()
            .collect();
        let pack = ContextPacks::new(self.store.clone()).build_pack(ContextRequest {
            project_id: project,
            task,
            as_of: None,
            budget_chars: Some(MAX_BUDGET_CHARS),
            files,
        })?;
        Ok(render_compact(&pack, budget, &BTreeSet::new()).map(|block| block.text))
    }

    fn project(&self, directory: &str) -> Result<String, AgentAccessError> {
        find_project_by_directory(&self.store, directory)
            .map_err(storage)?
            .map(|project| project.id)
            .ok_or(AgentAccessError::ProjectNotFound)
    }

    fn render_decision(&self, project: &str, id: &str) -> Result<String, AgentAccessError> {
        let detail = Decisions::new(self.store.clone())
            .detail(id)
            .map_err(storage)?;
        let summary = &detail.summary;
        let status = match summary.status {
            DecisionStatus::Accepted => "vigente",
            DecisionStatus::Superseded => "substituída",
        };
        let mut lines = vec![
            format!(
                "D:{} v{} ({status}) confirmada {}",
                short_ref(id),
                summary.version,
                summary.confirmed_at
            ),
            format!("pergunta: {}", clean(&summary.question)),
            format!("escolha: {}", clean(&summary.choice)),
            format!("motivo: {}", clean(&detail.rationale)),
        ];
        for (label, items) in [
            ("premissas", &detail.assumptions),
            ("reconsiderar quando", &detail.reconsider_when),
            ("escopo", &detail.scope),
            ("consequências", &detail.consequences),
        ] {
            if !items.is_empty() {
                let joined: Vec<String> = items.iter().map(|item| clean(item)).collect();
                lines.push(format!("{label}: {}", joined.join("; ")));
            }
        }

        for row in self.store.decision_relations(id).map_err(storage)? {
            let outgoing = row.from == id;
            let other = if outgoing { &row.to } else { &row.from };
            let label = match (row.kind.as_str(), outgoing) {
                ("supersedes", true) => "substitui",
                ("supersedes", false) => "substituída por",
                ("depends_on", true) => "depende de",
                ("depends_on", false) => "é base de",
                ("conflicts_with", _) => "conflita com",
                _ => continue,
            };
            let question = DecisionStore::get(&self.store, other)
                .map_err(storage)?
                .map(|decision| clean(&decision.question))
                .unwrap_or_default();
            lines.push(format!("{label}: D:{} {question}", short_ref(other)));
        }

        let now = crate::clock::now_rfc3339();
        let now = domain::time::Timestamp::parse(&now);
        for claim in self.store.project_claims(project).map_err(storage)? {
            let valid = now.as_ref().is_some_and(|at| claim.is_valid_at(at));
            if valid && claim.source_decision_id.as_deref() == Some(id) {
                lines.push(format!(
                    "claim {}:{} {}",
                    claim.kind.as_str(),
                    short_ref(&claim.claim_id),
                    clean(&claim.statement)
                ));
            }
        }
        if !detail.evidence.is_empty() {
            lines.push(format!(
                "evidências: {} artefato(s) capturado(s)",
                detail.evidence.len()
            ));
        }
        Ok(format!("{OPEN_TAG}\n{}\n{CLOSE_TAG}", lines.join("\n")))
    }
}

impl<S> AgentApi for AgentAccess<S>
where
    S: AgentStore
        + DecisionStore
        + RelationStore
        + ClaimStore
        + ContextStore
        + ProjectRepository
        + GraphStore
        + Clone
        + Send
        + Sync,
{
    fn decision(&self, directory: &str, reference: &str) -> Result<String, AgentAccessError> {
        AgentAccess::decision(self, directory, reference)
    }

    fn search(
        &self,
        directory: &str,
        query: &str,
        budget_tokens: Option<usize>,
        path: Option<&str>,
    ) -> Result<Option<String>, AgentAccessError> {
        AgentAccess::search(self, directory, query, budget_tokens, path)
    }

    fn file_context(
        &self,
        directory: &str,
        path: &str,
        budget_tokens: Option<usize>,
    ) -> Result<Option<String>, AgentAccessError> {
        AgentAccess::file_context(self, directory, path, budget_tokens)
    }
}

/// `path` relative to the project `location` when it is absolute inside it.
fn relative_to(path: &str, location: &str) -> String {
    let path = path.trim().replace('\\', "/");
    let root = location.replace('\\', "/");
    let root = root.trim_end_matches('/');
    if !root.is_empty()
        && path
            .to_lowercase()
            .starts_with(&format!("{}/", root.to_lowercase()))
    {
        path[root.len() + 1..].to_string()
    } else {
        path
    }
}

fn alphanumeric(text: &str) -> String {
    text.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_lowercase()
}

/// Accepts `D:bbbbcccc`, `bbbbcccc` or a full id; returns the lowercase suffix.
fn normalize_reference(reference: &str) -> Result<String, AgentAccessError> {
    let trimmed = reference.trim();
    let without_prefix = trimmed
        .strip_prefix("D:")
        .or_else(|| trimmed.strip_prefix("d:"))
        .unwrap_or(trimmed);
    let normalized = alphanumeric(without_prefix);
    if normalized.len() < MIN_REFERENCE_CHARS {
        return Err(AgentAccessError::InvalidRequest(
            "use a referência completa, como D:bbbbcccc".into(),
        ));
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::normalize_reference;

    #[test]
    fn references_accept_prefix_case_and_full_ids() {
        assert_eq!(
            normalize_reference(" D:BBBBcccc ").as_deref(),
            Ok("bbbbcccc")
        );
        assert_eq!(normalize_reference("bbbbcccc").as_deref(), Ok("bbbbcccc"));
        assert_eq!(
            normalize_reference("0190-aaaa-bbbb-cccc-1111bbbbcccc").as_deref(),
            Ok("0190aaaabbbbcccc1111bbbbcccc")
        );
        assert!(normalize_reference("D:ab").is_err());
    }
}
