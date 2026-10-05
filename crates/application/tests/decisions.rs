//! Decisions use-case tests: keyset pagination, query sanitization, revision
//! snapshots and the append-only surface.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use application::decisions::{
    sanitize_match_query, DecisionContent, DecisionEdits, DecisionFilter, DecisionQuery,
    DecisionRevisionRow, DecisionSearchRow, DecisionStatus, DecisionStore, Decisions,
    DecisionsError, EvidenceLinkRow, SearchQuery, StoredDecision,
};

/// In-memory store mirroring the port; cloning shares the backing maps.
#[derive(Default, Clone)]
struct FakeDecisions {
    decisions: Arc<Mutex<HashMap<String, StoredDecision>>>,
    revisions: Arc<Mutex<HashMap<String, Vec<DecisionRevisionRow>>>>,
    evidence: Arc<Mutex<HashMap<String, Vec<EvidenceLinkRow>>>>,
    /// When set, `revise` simulates another editor saving first.
    race: Arc<Mutex<bool>>,
}

impl FakeDecisions {
    fn with(decisions: Vec<StoredDecision>) -> Self {
        let store = Self::default();
        for decision in decisions {
            store
                .revisions
                .lock()
                .expect("lock")
                .entry(decision.decision_id.clone())
                .or_default()
                .push(DecisionRevisionRow {
                    qualifiers: "[]".into(),
                    version: decision.version,
                    created_at: decision.created_at.clone(),
                    question: decision.question.clone(),
                    choice: decision.choice.clone(),
                    rationale: decision.rationale.clone(),
                    assumptions: decision.assumptions.clone(),
                    reconsider_when: decision.reconsider_when.clone(),
                    scope: decision.scope.clone(),
                    consequences: decision.consequences.clone(),
                });
            store
                .decisions
                .lock()
                .expect("lock")
                .insert(decision.decision_id.clone(), decision);
        }
        store
    }

    fn version_of(&self, id: &str) -> Option<i64> {
        self.decisions
            .lock()
            .expect("lock")
            .get(id)
            .map(|decision| decision.version)
    }

    fn revision_versions(&self, id: &str) -> Vec<i64> {
        self.revisions
            .lock()
            .expect("lock")
            .get(id)
            .map(|rows| rows.iter().map(|row| row.version).collect())
            .unwrap_or_default()
    }
}

impl DecisionStore for FakeDecisions {
    fn list(&self, query: &DecisionQuery) -> Result<Vec<StoredDecision>, DecisionsError> {
        let decisions = self.decisions.lock().expect("lock");
        let mut selected: Vec<StoredDecision> = decisions
            .values()
            .filter(|decision| query.statuses.contains(&decision.status))
            .filter(|decision| {
                query
                    .project_id
                    .as_ref()
                    .map(|project| &decision.project_id == project)
                    .unwrap_or(true)
            })
            .filter(|decision| match &query.before {
                Some(before) => {
                    decision.confirmed_at < before.confirmed_at
                        || (decision.confirmed_at == before.confirmed_at
                            && decision.decision_id < before.decision_id)
                }
                None => true,
            })
            .cloned()
            .collect();
        selected.sort_by(|left, right| {
            right
                .confirmed_at
                .cmp(&left.confirmed_at)
                .then(right.decision_id.cmp(&left.decision_id))
        });
        selected.truncate(query.limit);
        Ok(selected)
    }

    fn get(&self, id: &str) -> Result<Option<StoredDecision>, DecisionsError> {
        Ok(self.decisions.lock().expect("lock").get(id).cloned())
    }

    fn revisions(&self, id: &str) -> Result<Vec<DecisionRevisionRow>, DecisionsError> {
        Ok(self
            .revisions
            .lock()
            .expect("lock")
            .get(id)
            .cloned()
            .unwrap_or_default())
    }

    fn evidence(&self, id: &str) -> Result<Vec<EvidenceLinkRow>, DecisionsError> {
        Ok(self
            .evidence
            .lock()
            .expect("lock")
            .get(id)
            .cloned()
            .unwrap_or_default())
    }

    fn search(
        &self,
        match_query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<DecisionSearchRow>, DecisionsError> {
        let tokens: Vec<String> = match_query
            .split(" AND ")
            .map(|token| token.trim_matches('"').to_ascii_lowercase())
            .collect();
        let decisions = self.decisions.lock().expect("lock");
        let mut hits = Vec::new();
        for decision in decisions.values() {
            if project_id
                .map(|project| decision.project_id != project)
                .unwrap_or(false)
            {
                continue;
            }
            let haystack = format!(
                "{} {} {}",
                decision.question, decision.choice, decision.rationale
            )
            .to_ascii_lowercase();
            if !tokens.iter().all(|token| haystack.contains(token)) {
                continue;
            }
            let snippet = tokens
                .first()
                .map(|token| format!("[{}]", token))
                .unwrap_or_default();
            hits.push(DecisionSearchRow {
                decision_id: decision.decision_id.clone(),
                project_id: decision.project_id.clone(),
                question: decision.question.clone(),
                snippet,
            });
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    fn revise(
        &self,
        id: &str,
        content: &DecisionContent,
        version: i64,
        updated_at: &str,
    ) -> Result<bool, DecisionsError> {
        let mut decisions = self.decisions.lock().expect("lock");
        let Some(decision) = decisions.get_mut(id) else {
            return Ok(false);
        };
        if *self.race.lock().expect("lock") {
            decision.version += 1;
        }
        if decision.version + 1 != version {
            return Ok(false);
        }
        decision.question = content.question.clone();
        decision.choice = content.choice.clone();
        decision.rationale = content.rationale.clone();
        decision.assumptions = content.assumptions.clone();
        decision.reconsider_when = content.reconsider_when.clone();
        decision.scope = content.scope.clone();
        decision.consequences = content.consequences.clone();
        decision.version = version;
        decision.updated_at = updated_at.to_string();
        self.revisions
            .lock()
            .expect("lock")
            .entry(id.to_string())
            .or_default()
            .insert(
                0,
                DecisionRevisionRow {
                    qualifiers: "[]".into(),
                    version,
                    created_at: updated_at.to_string(),
                    question: content.question.clone(),
                    choice: content.choice.clone(),
                    rationale: content.rationale.clone(),
                    assumptions: content.assumptions.clone(),
                    reconsider_when: content.reconsider_when.clone(),
                    scope: content.scope.clone(),
                    consequences: content.consequences.clone(),
                },
            );
        Ok(true)
    }
}

fn decision(id: &str, confirmed_at: &str) -> StoredDecision {
    StoredDecision {
        qualifiers: "[]".into(),
        decision_id: id.to_string(),
        candidate_id: format!("candidate-{id}"),
        project_id: "project-1".to_string(),
        project_location: "C:/synthetic/project".to_string(),
        capture_id: Some("capture-1".to_string()),
        status: DecisionStatus::Accepted,
        question: "alpha question".to_string(),
        choice: "alpha choice".to_string(),
        rationale: "alpha rationale".to_string(),
        assumptions: "[\"assumption\"]".to_string(),
        reconsider_when: "[]".to_string(),
        scope: "[]".to_string(),
        consequences: "[]".to_string(),
        version: 1,
        created_at: confirmed_at.to_string(),
        confirmed_at: confirmed_at.to_string(),
        updated_at: confirmed_at.to_string(),
    }
}

#[test]
fn list_keyset_pagination_has_no_overlap() {
    let rows = (0..5)
        .map(|index| {
            decision(
                &format!("d-{index}"),
                &format!("2026-01-0{}T00:00:00Z", 9 - index),
            )
        })
        .collect();
    let decisions = Decisions::new(FakeDecisions::with(rows));

    let mut seen = Vec::new();
    let mut cursor = None;
    for _ in 0..4 {
        let page = decisions
            .list(&DecisionFilter {
                limit: 2,
                cursor,
                ..DecisionFilter::new()
            })
            .expect("page");
        seen.extend(page.decisions.iter().map(|row| row.decision_id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(seen, vec!["d-0", "d-1", "d-2", "d-3", "d-4"]);
}

#[test]
fn listing_given_ids_keeps_their_order_pages_and_skips_missing() {
    let rows = (0..4)
        .map(|index| decision(&format!("d-{index}"), "2026-01-01T00:00:00Z"))
        .collect();
    let decisions = Decisions::new(FakeDecisions::with(rows));
    let ids: Vec<String> = ["d-3", "gone", "d-0", "d-2"]
        .iter()
        .map(|id| id.to_string())
        .collect();

    let first = decisions.list_ids(&ids, None, 2).expect("first");
    let order: Vec<&str> = first
        .decisions
        .iter()
        .map(|row| row.decision_id.as_str())
        .collect();
    assert_eq!(order, vec!["d-3"], "a missing id is skipped, not replaced");
    let second = decisions
        .list_ids(&ids, first.next_cursor.as_deref(), 2)
        .expect("second");
    let order: Vec<&str> = second
        .decisions
        .iter()
        .map(|row| row.decision_id.as_str())
        .collect();
    assert_eq!(order, vec!["d-0", "d-2"]);
    assert_eq!(second.next_cursor, None);

    assert!(decisions
        .list_ids(&[], None, 2)
        .expect("empty")
        .decisions
        .is_empty());
    assert!(matches!(
        decisions.list_ids(&ids, Some("x"), 2),
        Err(DecisionsError::InvalidFilter(_))
    ));
    assert!(matches!(
        decisions.list_ids(&ids, None, 0),
        Err(DecisionsError::InvalidFilter(_))
    ));
    assert!(decisions
        .list_ids(&ids, Some("99"), 2)
        .expect("past the end")
        .decisions
        .is_empty());
}

#[test]
fn search_sanitizes_operators_and_ignores_unindexed_fields() {
    assert_eq!(
        sanitize_match_query("alpha* AND beta").as_deref(),
        Some("\"alpha\" AND \"beta\"")
    );
    assert_eq!(sanitize_match_query("AND OR NOT near ^*\"\""), None);

    let mut row = decision("d-1", "2026-01-01T00:00:00Z");
    row.assumptions = "[\"hidden-token\"]".to_string();
    let decisions = Decisions::new(FakeDecisions::with(vec![row]));

    let hits = decisions
        .search(&SearchQuery {
            query: "alpha*".to_string(),
            ..SearchQuery::default()
        })
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].decision_id, "d-1");
    assert!(hits[0].snippet.contains("alpha"));

    let hidden = decisions
        .search(&SearchQuery {
            query: "hidden-token".to_string(),
            ..SearchQuery::default()
        })
        .expect("search");
    assert!(hidden.is_empty(), "assumptions are not indexed");

    let error = decisions
        .search(&SearchQuery {
            query: "* ^ AND".to_string(),
            ..SearchQuery::default()
        })
        .expect_err("empty query");
    assert_eq!(error.code(), "invalid_query");
}

#[test]
fn revise_snapshots_and_preserves_untouched_fields() {
    let fake = FakeDecisions::with(vec![decision("d-1", "2026-01-01T00:00:00Z")]);
    let decisions = Decisions::new(fake.clone());

    let detail = decisions
        .revise(
            "d-1",
            DecisionEdits {
                qualifiers: None,
                question: Some("  beta pergunta  ".to_string()),
                ..DecisionEdits::default()
            },
        )
        .expect("revise");
    assert_eq!(detail.summary.version, 2);
    assert_eq!(detail.summary.question, "beta pergunta");
    assert_eq!(
        detail.rationale, "alpha rationale",
        "untouched fields stay intact"
    );
    assert_eq!(detail.assumptions, vec!["assumption"]);
    assert_eq!(fake.version_of("d-1"), Some(2));
    assert_eq!(fake.revision_versions("d-1"), vec![2, 1]);

    assert_eq!(detail.revisions.len(), 2);
    let live = &detail.revisions[0];
    let original = &detail.revisions[1];
    assert_eq!((live.version, original.version), (2, 1));
    assert_eq!(original.question, "alpha question");
    assert_eq!(original.assumptions, vec!["assumption"]);
    assert_eq!(live.assumptions, original.assumptions);
    assert_eq!(live.rationale, original.rationale);

    let empty = decisions
        .revise("d-1", DecisionEdits::default())
        .expect_err("no field");
    assert_eq!(empty.code(), "invalid_edits");

    let missing = decisions
        .revise(
            "missing",
            DecisionEdits {
                question: Some("x".to_string()),
                ..DecisionEdits::default()
            },
        )
        .expect_err("missing");
    assert_eq!(missing.code(), "not_found");
}

#[test]
fn revise_all_fields_keeps_the_original_revision_reconstructible() {
    let fake = FakeDecisions::with(vec![decision("d-1", "2026-01-01T00:00:00Z")]);
    let decisions = Decisions::new(fake);

    let edited = decisions
        .revise(
            "d-1",
            DecisionEdits {
                question: Some("pergunta \"nova\"".to_string()),
                qualifiers: None,
                choice: Some("escolha ç".to_string()),
                rationale: Some("razão — acentuada".to_string()),
                assumptions: Some(vec!["nova premissa".to_string()]),
                reconsider_when: Some(vec!["quando X".to_string()]),
                scope: Some(vec!["escopo novo".to_string()]),
                consequences: Some(vec!["consequência nova".to_string()]),
            },
        )
        .expect("revise all");
    assert_eq!(edited.summary.version, 2);
    assert_eq!(edited.assumptions, vec!["nova premissa"]);

    let original = edited
        .revisions
        .iter()
        .find(|revision| revision.version == 1)
        .expect("v1 must remain readable");
    assert_eq!(original.question, "alpha question");
    assert_eq!(original.choice, "alpha choice");
    assert_eq!(original.rationale, "alpha rationale");
    assert_eq!(original.assumptions, vec!["assumption"]);
    assert_eq!(original.reconsider_when, Vec::<String>::new());
    assert_eq!(original.scope, Vec::<String>::new());
    assert_eq!(original.consequences, Vec::<String>::new());

    let live = edited
        .revisions
        .iter()
        .find(|revision| revision.version == 2)
        .expect("v2");
    assert_eq!(live.question, "pergunta \"nova\"");
    assert_eq!(live.choice, "escolha ç");
    assert_eq!(live.rationale, "razão — acentuada");
    assert_eq!(live.scope, vec!["escopo novo"]);
}

#[test]
fn list_default_filter_includes_only_accepted() {
    let mut superseded = decision("d-2", "2026-01-02T00:00:00Z");
    superseded.status = DecisionStatus::Superseded;
    let decisions = Decisions::new(FakeDecisions::with(vec![
        decision("d-1", "2026-01-03T00:00:00Z"),
        superseded,
    ]));
    let page = decisions.list(&DecisionFilter::new()).expect("list");
    let ids: Vec<&str> = page
        .decisions
        .iter()
        .map(|row| row.decision_id.as_str())
        .collect();
    assert_eq!(ids, vec!["d-1"]);
}

#[test]
fn stale_editor_and_lost_race_report_a_conflict() {
    let fake = FakeDecisions::with(vec![decision("d-1", "2026-01-01T00:00:00Z")]);
    let decisions = Decisions::new(fake.clone());
    let edits = || DecisionEdits {
        rationale: Some("nova razão".to_string()),
        ..DecisionEdits::default()
    };

    decisions
        .revise_version("d-1", 1, edits())
        .expect("current version saves");
    let stale = decisions
        .revise_version("d-1", 1, edits())
        .expect_err("stale version");
    assert_eq!(stale, DecisionsError::Conflict);
    assert_eq!(stale.code(), "conflict");

    *fake.race.lock().expect("lock") = true;
    let raced = decisions.revise("d-1", edits()).expect_err("lost race");
    assert_eq!(raced, DecisionsError::Conflict);
    let raced_versioned = decisions
        .revise_version("d-1", fake.version_of("d-1").expect("version"), edits())
        .expect_err("lost race with the current version");
    assert_eq!(raced_versioned, DecisionsError::Conflict);
}
