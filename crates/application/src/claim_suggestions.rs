//! Context derived from adopted decisions: the assumptions, constraints,
//! conventions and goals a decision states, proposed as rules for a person
//! to confirm, scoped to the parts of the map the decision touches.
//!
//! A model reads the adopted decision and must quote, verbatim, the sentence
//! each item rests on; the app checks the quote, the kind and that the
//! project does not already hold the same rule. Confirming creates the claim
//! with the decision as its source and `applies_to` the decision's entities.
//! When the source decision is later superseded, its claims are flagged for
//! review (never ended on their own): what was assumed may no longer hold.

use std::collections::BTreeSet;

use domain::claims::ClaimKind;
use domain::entities::{EdgeKind, NodeKind};
use serde::Deserialize;
use serde_json::json;

use crate::analysis::ExtractorFactory;
use crate::claims::{ClaimStore, Claims, ClaimsError, NewClaim};
use crate::clock::now_rfc3339;
use crate::decisions::{DecisionStatus, DecisionStore, DecisionsError, StoredDecision};
use crate::graph::{GraphError, GraphStore, KnowledgeGraph, LinkRequest};
use crate::overview::StructuredModel;
use crate::profile::{choose_extractor, AiSettings, ExtractorChoice, ProfileStore, SecretStore};
use crate::projects::ProjectRepository;
use crate::relations::RelationStore;

/// Job kind that derives context from one adopted decision.
pub const CLAIM_JOB_KIND: &str = "derive_claims";
/// Items proposed per decision, at most.
pub const MAX_DERIVED: usize = 3;
/// Longest statement accepted.
const MAX_STATEMENT_CHARS: usize = 300;
/// Shortest quote accepted as evidence.
const MIN_QUOTE_CHARS: usize = 12;

/// A stored suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimSuggestionRecord {
    /// Identifier.
    pub suggestion_id: String,
    /// Project.
    pub project_id: String,
    /// Decision it was derived from.
    pub decision_id: String,
    /// Assumption, constraint, convention or goal.
    pub kind: ClaimKind,
    /// The rule, in one sentence.
    pub statement: String,
    /// Verbatim sentence of the decision it rests on.
    pub quote: String,
    /// RFC 3339 creation time.
    pub created_at: String,
}

/// A pending suggestion with what it would apply to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimSuggestionView {
    /// The stored suggestion.
    pub record: ClaimSuggestionRecord,
    /// Question of the source decision.
    pub decision_question: String,
    /// `(entity id, name)` the decision touches, where the rule would apply.
    pub scope: Vec<(String, String)>,
}

/// Failures of the suggestion use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimSuggestionError {
    /// Storage failed.
    Storage(String),
    /// The suggestion is no longer pending.
    NotFound,
    /// The claim could not be created.
    Claim(String),
    /// The provider failed.
    Provider(String),
}

impl ClaimSuggestionError {
    /// Short, stable code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Storage(_) => "storage",
            Self::NotFound => "not_found",
            Self::Claim(_) => "claim",
            Self::Provider(_) => "provider",
        }
    }
}

impl std::fmt::Display for ClaimSuggestionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(detail) => write!(formatter, "storage: {detail}"),
            Self::NotFound => formatter.write_str("sugestão não encontrada"),
            Self::Claim(detail) => write!(formatter, "regra: {detail}"),
            Self::Provider(detail) => write!(formatter, "provedor: {detail}"),
        }
    }
}

impl std::error::Error for ClaimSuggestionError {}

fn storage(error: impl std::fmt::Display) -> ClaimSuggestionError {
    ClaimSuggestionError::Storage(error.to_string())
}

/// Persistence of suggestions.
pub trait ClaimSuggestionStore {
    /// Stores a suggestion; `false` when the decision already had the same
    /// statement suggested (pending, confirmed or rejected).
    fn insert_claim_suggestion(
        &self,
        record: &ClaimSuggestionRecord,
    ) -> Result<bool, DecisionsError>;

    /// Pending suggestions of a project, oldest first.
    fn pending_claim_suggestions(
        &self,
        project_id: &str,
    ) -> Result<Vec<ClaimSuggestionRecord>, DecisionsError>;

    /// One suggestion while it is pending.
    fn pending_claim_suggestion(
        &self,
        suggestion_id: &str,
    ) -> Result<Option<ClaimSuggestionRecord>, DecisionsError>;

    /// Marks a pending suggestion `confirmed` or `rejected`; whether it was
    /// still pending.
    fn resolve_claim_suggestion(
        &self,
        suggestion_id: &str,
        outcome: &str,
        at: &str,
    ) -> Result<bool, DecisionsError>;
}

/// Reading, confirming and rejecting derived context.
#[derive(Debug, Clone)]
pub struct ClaimSuggestions<S> {
    store: S,
}

impl<S> ClaimSuggestions<S>
where
    S: ClaimSuggestionStore
        + ClaimStore
        + DecisionStore
        + GraphStore
        + RelationStore
        + ProjectRepository
        + Clone,
{
    /// Wraps the store.
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Entities the decision is tied to by live `affects`/`uses` edges.
    fn scope(
        &self,
        project_id: &str,
        decision_id: &str,
    ) -> Result<Vec<(String, String)>, ClaimSuggestionError> {
        let entities = self.store.project_entities(project_id).map_err(storage)?;
        let mut scope = Vec::new();
        for edge in self.store.project_edges(project_id).map_err(storage)? {
            if edge.source_kind == NodeKind::Decision
                && edge.source_id == decision_id
                && edge.invalidated_at.is_none()
                && matches!(edge.kind, EdgeKind::Affects | EdgeKind::Uses)
                && !scope.iter().any(|(id, _)| *id == edge.entity_id)
            {
                if let Some(entity) = entities.iter().find(|entity| {
                    entity.entity_id == edge.entity_id && entity.retired_at.is_none()
                }) {
                    scope.push((entity.entity_id.clone(), entity.name.clone()));
                }
            }
        }
        Ok(scope)
    }

    /// Pending suggestions whose decision is still in force.
    ///
    /// # Errors
    ///
    /// `storage`.
    pub fn pending(
        &self,
        project_id: &str,
    ) -> Result<Vec<ClaimSuggestionView>, ClaimSuggestionError> {
        let mut views = Vec::new();
        for record in self
            .store
            .pending_claim_suggestions(project_id)
            .map_err(storage)?
        {
            let Some(decision) =
                DecisionStore::get(&self.store, &record.decision_id).map_err(storage)?
            else {
                continue;
            };
            if decision.status != DecisionStatus::Accepted {
                continue;
            }
            views.push(ClaimSuggestionView {
                scope: self.scope(project_id, &record.decision_id)?,
                decision_question: decision.question,
                record,
            });
        }
        Ok(views)
    }

    /// Creates the rule from its decision, applies it to the decision's
    /// entities and marks the suggestion confirmed.
    ///
    /// # Errors
    ///
    /// `not_found` when no longer pending, `claim` when the rule is refused.
    pub fn confirm(&self, suggestion_id: &str) -> Result<String, ClaimSuggestionError> {
        let record = self
            .store
            .pending_claim_suggestion(suggestion_id)
            .map_err(storage)?
            .ok_or(ClaimSuggestionError::NotFound)?;
        let claim = Claims::new(self.store.clone())
            .create(NewClaim {
                project_id: record.project_id.clone(),
                kind: record.kind,
                statement: record.statement.clone(),
                valid_from: None,
                valid_until: None,
                source_decision_id: Some(record.decision_id.clone()),
            })
            .map_err(|error: ClaimsError| ClaimSuggestionError::Claim(error.to_string()))?;
        let graph = KnowledgeGraph::new(self.store.clone());
        for (entity_id, _) in self.scope(&record.project_id, &record.decision_id)? {
            match graph.link(LinkRequest {
                kind: EdgeKind::AppliesTo,
                source_kind: NodeKind::Claim,
                source_id: claim.claim_id.clone(),
                entity_id,
            }) {
                Ok(_) | Err(GraphError::DuplicateEdge) => {}
                Err(error) => return Err(storage(error)),
            }
        }
        self.store
            .resolve_claim_suggestion(suggestion_id, "confirmed", &now_rfc3339())
            .map_err(storage)?;
        Ok(claim.claim_id)
    }

    /// Claims valid now whose source decision was superseded.
    ///
    /// # Errors
    ///
    /// `storage`.
    pub fn to_review(&self, project_id: &str) -> Result<Vec<String>, ClaimSuggestionError> {
        claims_to_review(&self.store, project_id)
    }

    /// Marks the suggestion rejected; it is never suggested again.
    ///
    /// # Errors
    ///
    /// `not_found` when no longer pending, `storage`.
    pub fn reject(&self, suggestion_id: &str) -> Result<(), ClaimSuggestionError> {
        if self
            .store
            .resolve_claim_suggestion(suggestion_id, "rejected", &now_rfc3339())
            .map_err(storage)?
        {
            Ok(())
        } else {
            Err(ClaimSuggestionError::NotFound)
        }
    }
}

/// Claims of a project, valid now, whose source decision was superseded:
/// what they assumed may no longer hold, so a person should review them.
///
/// # Errors
///
/// `storage`.
pub fn claims_to_review<S>(store: &S, project_id: &str) -> Result<Vec<String>, ClaimSuggestionError>
where
    S: ClaimStore + DecisionStore,
{
    let now = domain::time::Timestamp::parse(&now_rfc3339());
    let mut ids = Vec::new();
    for claim in store.project_claims(project_id).map_err(storage)? {
        if !now.as_ref().is_some_and(|at| claim.is_valid_at(at)) {
            continue;
        }
        let Some(source) = &claim.source_decision_id else {
            continue;
        };
        if let Some(decision) = store.get(source).map_err(storage)? {
            if decision.status == DecisionStatus::Superseded {
                ids.push(claim.claim_id);
            }
        }
    }
    Ok(ids)
}

/// System prompt of the context extractor.
pub const CLAIM_PROMPT: &str = concat!(
    "You read one engineering decision a software team adopted and \
list the lasting context an agent must respect when working on the parts it affects. kind \
is assumption (believed true but not proven), constraint (a limit the work must respect), \
convention (an agreed way of working) or goal (an outcome the project pursues). Only what \
the decision states or directly implies, never general advice; at most 3; an empty list is \
a correct answer. statement: the rule in one short sentence, in the decision's language. \
quote: one sentence copied verbatim from the decision text that supports it.\n\
Reply with one JSON object only, matching exactly: {\"claims\":[{\"kind\":\
\"assumption|constraint|convention|goal\",\"statement\":string,\"quote\":string}]}.",
    crate::plain_rules!()
);

/// Strict schema of the extractor's answer.
pub fn claim_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["claims"],
        "properties": {
            "claims": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["kind", "statement", "quote"],
                    "properties": {
                        "kind": {
                            "type": "string",
                            "enum": ["assumption", "constraint", "convention", "goal"]
                        },
                        "statement": { "type": "string" },
                        "quote": { "type": "string" }
                    }
                }
            }
        }
    })
}

#[derive(Deserialize)]
struct RawAnswer {
    #[serde(default)]
    claims: Vec<RawClaim>,
}

#[derive(Deserialize)]
struct RawClaim {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    statement: String,
    #[serde(default)]
    quote: String,
}

fn normalized(text: &str) -> String {
    text.split(|character: char| !character.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validated items: a known kind, a short statement, a quote found in the
/// decision, nothing the project already holds.
pub fn parse_claims(
    answer: &str,
    decision: &StoredDecision,
    existing: &[String],
) -> Vec<(ClaimKind, String, String)> {
    let Ok(raw) = serde_json::from_str::<RawAnswer>(answer.trim()) else {
        return Vec::new();
    };
    let text = normalized(&format!(
        "{}\n{}\n{}",
        decision.question, decision.choice, decision.rationale
    ));
    let mut known: BTreeSet<String> = existing
        .iter()
        .map(|statement| normalized(statement))
        .collect();
    let mut out = Vec::new();
    for claim in raw.claims {
        if out.len() >= MAX_DERIVED {
            break;
        }
        let Some(kind) = ClaimKind::parse(&claim.kind) else {
            continue;
        };
        let statement = claim.statement.trim();
        if statement.is_empty() || statement.chars().count() > MAX_STATEMENT_CHARS {
            continue;
        }
        let needle = normalized(&claim.quote);
        if needle.chars().count() < MIN_QUOTE_CHARS || !text.contains(&needle) {
            continue;
        }
        if !known.insert(normalized(statement)) {
            continue;
        }
        out.push((kind, statement.to_string(), claim.quote.trim().to_string()));
    }
    out
}

/// The job: context an adopted decision states, judged by the model.
#[derive(Debug, Clone)]
pub struct ClaimFinder<S, P, K, F> {
    store: S,
    settings: AiSettings<P, K>,
    factory: F,
}

impl<S, P, K, F> ClaimFinder<S, P, K, F>
where
    S: DecisionStore + ClaimStore + ClaimSuggestionStore,
    P: ProfileStore,
    K: SecretStore,
    F: ExtractorFactory,
    F::Extractor: StructuredModel,
{
    /// Wraps the store, the AI settings and the provider factory.
    pub fn new(store: S, settings: AiSettings<P, K>, factory: F) -> Self {
        Self {
            store,
            settings,
            factory,
        }
    }

    /// Asks the model what the decision states and stores what survives the
    /// checks; 0 without an enabled provider.
    ///
    /// # Errors
    ///
    /// `storage`, or `provider` when the call fails.
    pub fn run(&self, decision_id: &str) -> Result<usize, ClaimSuggestionError> {
        let Some(decision) = self
            .store
            .get(decision_id)
            .map_err(storage)?
            .filter(|decision| decision.status == DecisionStatus::Accepted)
        else {
            return Ok(0);
        };
        let profile = self.settings.load_or_seed().map_err(storage)?;
        if choose_extractor(Some(&profile)) != ExtractorChoice::ExternalEnabled {
            return Ok(0);
        }
        let secret = match self.settings.secret(&profile.credential_account()) {
            Ok(Some(secret)) => secret,
            Ok(None) if !profile.credential_required() => String::new(),
            _ => return Ok(0),
        };
        let Ok(model) = self.factory.external(&profile, secret) else {
            return Ok(0);
        };
        let user = format!(
            "Question: {}\nChoice: {}\nWhy: {}\n",
            decision.question,
            decision.choice,
            decision.rationale.chars().take(1500).collect::<String>()
        );
        let answer = model
            .complete(CLAIM_PROMPT, &user, "decision_context", &claim_schema())
            .map_err(|error| ClaimSuggestionError::Provider(error.to_string()))?;
        let existing: Vec<String> = self
            .store
            .project_claims(&decision.project_id)
            .map_err(storage)?
            .into_iter()
            .map(|claim| claim.statement)
            .collect();
        let now = now_rfc3339();
        let mut stored = 0;
        for (kind, statement, quote) in parse_claims(&answer, &decision, &existing) {
            if self
                .store
                .insert_claim_suggestion(&ClaimSuggestionRecord {
                    suggestion_id: uuid::Uuid::now_v7().to_string(),
                    project_id: decision.project_id.clone(),
                    decision_id: decision.decision_id.clone(),
                    kind,
                    statement,
                    quote,
                    created_at: now.clone(),
                })
                .map_err(storage)?
            {
                stored += 1;
            }
        }
        Ok(stored)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision() -> StoredDecision {
        StoredDecision {
            decision_id: "d1".into(),
            candidate_id: "c1".into(),
            project_id: "p".into(),
            project_location: "C:/p".into(),
            capture_id: None,
            status: DecisionStatus::Accepted,
            question: "Onde guardar credenciais?".into(),
            choice: "No cofre do sistema.".into(),
            rationale: "Nenhuma chave pode ficar em arquivo de configuração. Supomos que todo \
                        usuário tem cofre disponível."
                .into(),
            assumptions: "[]".into(),
            reconsider_when: "[]".into(),
            scope: "[]".into(),
            consequences: "[]".into(),
            version: 1,
            created_at: "2026-01-01T00:00:00Z".into(),
            confirmed_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn only_quoted_new_items_survive() {
        let answer = r#"{"claims":[
            {"kind":"constraint","statement":"Chaves nunca ficam em arquivos de configuração.",
             "quote":"Nenhuma chave pode ficar em arquivo de configuração."},
            {"kind":"assumption","statement":"Todo usuário tem um cofre do sistema.",
             "quote":"Supomos que todo usuário tem cofre disponível."},
            {"kind":"goal","statement":"Ser rápido.","quote":"isso não está na decisão"},
            {"kind":"policy","statement":"Tipo inválido.","quote":"No cofre do sistema."},
            {"kind":"constraint","statement":"Já existe.","quote":"No cofre do sistema."}
        ]}"#;
        let derived = parse_claims(answer, &decision(), &["Já existe".to_string()]);
        let kinds: Vec<ClaimKind> = derived.iter().map(|(kind, _, _)| *kind).collect();
        assert_eq!(kinds, vec![ClaimKind::Constraint, ClaimKind::Assumption]);
        assert!(parse_claims("{", &decision(), &[]).is_empty());
    }
}
