//! Search terms of adopted decisions over SQLite: written in batches by the
//! job, indexed, kept across a revision and used by the Context Pack to
//! find a decision whose own words the task does not use.

mod support;

use std::cell::{Cell, RefCell};

use application::analysis::ExtractorFactory;
use application::context::{ContextPacks, ContextProvider, ContextRequest};
use application::decisions::{DecisionEdits, Decisions};
use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use application::overview::{JsonValue, StructuredModel};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};
use application::search_terms::{SearchTermFinder, SearchTermStore, BATCH};

struct Profiles(RefCell<AiProfile>);

impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(Some(self.0.borrow().clone()))
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.borrow_mut() = profile.clone();
        Ok(())
    }
}

struct NoSecrets;

impl SecretStore for NoSecrets {
    fn set_secret(&self, _: &str, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
    fn get_secret(&self, _: &str) -> Result<Option<String>, ProfileError> {
        Ok(None)
    }
    fn delete_secret(&self, _: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

fn consented() -> AiProfile {
    let profile = AiProfile {
        kind: ProfileKind::OpenAiCompatible,
        model: "local".into(),
        endpoint: Some("http://127.0.0.1:9/v1".into()),
        ..offline_default_profile()
    };
    grant_consent(
        &profile,
        &build_preview(&profile),
        "2026-01-01T00:00:00Z",
        true,
    )
    .expect("consent")
}

/// Answers every decision of the batch with the same terms and counts calls.
struct Indexer<'a> {
    terms: &'a [&'a str],
    calls: &'a Cell<usize>,
}

impl CandidateExtractor for Indexer<'_> {
    fn extract(
        &self,
        _: &DecisionEvidence,
        _: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        Ok(Vec::new())
    }
}

impl StructuredModel for Indexer<'_> {
    fn complete(
        &self,
        _: &str,
        user: &str,
        schema_name: &str,
        _: &JsonValue,
    ) -> Result<String, ExtractError> {
        assert_eq!(schema_name, "decision_search_terms");
        self.calls.set(self.calls.get() + 1);
        let decisions: Vec<_> = (1..=user.matches("Question: ").count())
            .map(|id| serde_json::json!({"id": id.to_string(), "terms": self.terms}))
            .collect();
        Ok(serde_json::json!({ "decisions": decisions }).to_string())
    }
}

struct Factory<'a> {
    terms: &'a [&'a str],
    calls: &'a Cell<usize>,
}

impl<'a> ExtractorFactory for Factory<'a> {
    type Extractor = Indexer<'a>;
    fn external(&self, _: &AiProfile, _: String) -> Result<Indexer<'a>, ExtractError> {
        Ok(Indexer {
            terms: self.terms,
            calls: self.calls,
        })
    }
}

fn request(task: &str) -> ContextRequest {
    ContextRequest {
        project_id: "p1".into(),
        task: task.into(),
        as_of: None,
        budget_chars: Some(2_000),
        files: Vec::new(),
    }
}

fn found(store: &storage_sqlite::SqliteStore, task: &str) -> Vec<String> {
    ContextPacks::new(store.clone())
        .build_pack(request(task))
        .expect("pack")
        .decisions
        .into_iter()
        .map(|decision| decision.decision_id)
        .collect()
}

#[test]
fn terms_find_a_decision_the_task_shares_no_word_with_and_survive_a_revision() {
    let test = support::open("search-terms", &["p1"]);
    let schema = support::decision(
        &test.store,
        "p1",
        "schema",
        "Como evoluir o esquema do banco?",
        "Migrações só para frente",
    );
    let task = "Adicionar uma coluna nova na tabela";
    assert!(found(&test.store, task).is_empty(), "no word in common yet");

    let calls = Cell::new(0);
    let finder = SearchTermFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory {
            terms: &["coluna", "tabela", "schema migration", "esquema"],
            calls: &calls,
        },
    );
    assert_eq!(finder.run("p1").expect("terms"), 1);
    assert_eq!(found(&test.store, task), vec![schema.clone()]);

    assert_eq!(finder.run("p1").expect("nothing left"), 0);
    assert_eq!(calls.get(), 1, "a decision is asked once");

    Decisions::new(test.store.clone())
        .revise(
            &schema,
            DecisionEdits {
                choice: Some("Migrações numeradas, só para frente".into()),
                ..DecisionEdits::default()
            },
        )
        .expect("revise");
    assert_eq!(
        found(&test.store, task),
        vec![schema],
        "a revision keeps the decision's terms in the index"
    );
}

#[test]
fn a_large_backlog_is_written_in_batches() {
    let test = support::open("search-terms-batches", &["p1"]);
    for index in 0..=BATCH {
        support::decision(
            &test.store,
            "p1",
            &format!("d{index}"),
            &format!("Pergunta {index}?"),
            "Escolha",
        );
    }
    let calls = Cell::new(0);
    let finder = SearchTermFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(consented())), NoSecrets),
        Factory {
            terms: &["termo"],
            calls: &calls,
        },
    );
    assert_eq!(finder.run("p1").expect("first batch"), BATCH);
    assert_eq!(finder.run("p1").expect("the rest"), 1);
    assert_eq!(calls.get(), 2);
    assert!(test
        .store
        .decisions_without_terms("p1", 10)
        .expect("left")
        .is_empty());
}

#[test]
fn without_an_enabled_provider_nothing_is_written() {
    let test = support::open("search-terms-offline", &["p1"]);
    support::decision(&test.store, "p1", "d", "Pergunta?", "Escolha");
    let calls = Cell::new(0);
    let finder = SearchTermFinder::new(
        test.store.clone(),
        AiSettings::new(Profiles(RefCell::new(offline_default_profile())), NoSecrets),
        Factory {
            terms: &["termo"],
            calls: &calls,
        },
    );
    assert_eq!(finder.run("p1").expect("offline"), 0);
    assert_eq!(calls.get(), 0);
    assert_eq!(
        test.store
            .decisions_without_terms("p1", 10)
            .expect("still pending")
            .len(),
        1
    );
}
