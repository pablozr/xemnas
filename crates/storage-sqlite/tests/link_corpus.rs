//! Quality gate of the AI link proposer: precision and recall of the pipeline
//! (production prompt answers -> production validation) on a labeled corpus.
//!
//! The model answers come from a fixture generated once by the live test in
//! `ai-provider/tests/link_suggestions_live.rs` and are never edited by hand;
//! the validation runs for real, so a change of the checks is measured
//! without calling the model. The scorer itself is covered by deterministic
//! tests that need no fixture.

use std::collections::BTreeSet;

use application::link_suggestions::{candidate_components, parse_links};

#[path = "fixtures/link_corpus.rs"]
mod corpus;

/// Floors of the first live run: raise them in the commit that improves the
/// numbers (docs/arquitetura/qualidade-do-nucleo.md).
const PRECISION_FLOOR: f64 = 0.85;
const RECALL_FLOOR: f64 = 0.5;

#[derive(Debug, Default, PartialEq)]
struct Score {
    true_links: usize,
    false_links: usize,
    missed: usize,
    /// Negative decisions that got any link.
    linked_negatives: usize,
}

impl Score {
    fn precision(&self) -> f64 {
        let proposed = self.true_links + self.false_links;
        if proposed == 0 {
            1.0
        } else {
            self.true_links as f64 / proposed as f64
        }
    }

    fn recall(&self) -> f64 {
        let wanted = self.true_links + self.missed;
        self.true_links as f64 / wanted as f64
    }
}

/// Scores the pipeline over the corpus with `answer(key)` as the model.
fn score(answer: impl Fn(&str) -> String) -> Score {
    let entities = corpus::entities();
    let components = candidate_components(&entities);
    let mut score = Score::default();
    for decision in corpus::DECISIONS {
        let links = parse_links(&answer(decision.0), &corpus::subject(decision), &components);
        let predicted: BTreeSet<String> = links
            .iter()
            .map(|link| link.entity_id.trim_start_matches("e-").to_string())
            .collect();
        let expected: BTreeSet<String> = decision.4.iter().map(|name| name.to_string()).collect();
        score.true_links += predicted.intersection(&expected).count();
        score.false_links += predicted.difference(&expected).count();
        score.missed += expected.difference(&predicted).count();
        if expected.is_empty() && !predicted.is_empty() {
            score.linked_negatives += 1;
        }
    }
    score
}

/// The id the proposer is told for a component of the corpus.
fn position(name: &str) -> String {
    let entities = corpus::entities();
    let components = candidate_components(&entities);
    let index = components
        .iter()
        .position(|component| component.name == name)
        .expect("a corpus component");
    format!("c{}", index + 1)
}

#[test]
fn the_corpus_is_labeled_and_balanced() {
    assert_eq!(corpus::COMPONENTS.len(), 6);
    assert_eq!(corpus::DECISIONS.len(), 20);
    let names: BTreeSet<&str> = corpus::COMPONENTS.iter().map(|c| c.0).collect();
    let mut keys = BTreeSet::new();
    for decision in corpus::DECISIONS {
        assert!(keys.insert(decision.0), "duplicate key {}", decision.0);
        assert!(decision.4.iter().all(|name| names.contains(name)));
    }
    let negatives = corpus::DECISIONS.iter().filter(|d| d.4.is_empty()).count();
    assert!(
        negatives >= 6,
        "negatives that share vocabulary: {negatives}"
    );
}

#[test]
fn the_scorer_rewards_the_labels_and_punishes_invented_links() {
    // A model that copies the choice of each decision for its labeled
    // components is perfect.
    let perfect = score(|key| {
        let decision = corpus::DECISIONS.iter().find(|d| d.0 == key).expect("key");
        let links: Vec<_> = decision
            .4
            .iter()
            .map(|name| {
                serde_json::json!({
                    "component_id": position(name), "quote": decision.2, "reason": "r"
                })
            })
            .collect();
        serde_json::json!({ "links": links }).to_string()
    });
    assert_eq!((perfect.precision(), perfect.recall()), (1.0, 1.0));
    assert_eq!(perfect.linked_negatives, 0);

    // A model that links every decision to the core with a real quote is
    // fooled by the negatives; one that invents quotes gets nothing through.
    let eager = score(|key| {
        let decision = corpus::DECISIONS.iter().find(|d| d.0 == key).expect("key");
        serde_json::json!({"links": [{
            "component_id": position("core"), "quote": decision.2, "reason": "r"
        }]})
        .to_string()
    });
    assert!(eager.precision() < 0.5, "{eager:?}");
    assert_eq!(eager.linked_negatives, 8);
    let invented = score(|_| {
        serde_json::json!({"links": [{
            "component_id": position("core"), "quote": "uma frase que nenhuma decisão tem",
            "reason": "r"
        }]})
        .to_string()
    });
    assert_eq!((invented.true_links, invented.false_links), (0, 0));
    assert_eq!(invented.recall(), 0.0);
}

#[test]
#[ignore = "needs fixtures/link_corpus_answers.json, generated once by the live test in \
            ai-provider/tests/link_suggestions_live.rs (requires the user's go-ahead); \
            remove this attribute when the fixture is committed"]
fn link_quality_gate() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/link_corpus_answers.json"
    );
    let fixture: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("answers fixture"))
            .expect("answers fixture is JSON");
    let answers = fixture["answers"].as_object().expect("answers");
    assert_eq!(
        answers.len(),
        corpus::DECISIONS.len(),
        "one answer per decision"
    );

    let started = std::time::Instant::now();
    let result = score(|key| answers[key].as_str().expect("raw answer").to_string());
    let elapsed = started.elapsed();
    println!(
        "gate link proposals: precision {:.2} (floor {PRECISION_FLOOR}) recall {:.2} \
         (floor {RECALL_FLOOR}) true {} false {} missed {} negatives linked {}/{} model {}",
        result.precision(),
        result.recall(),
        result.true_links,
        result.false_links,
        result.missed,
        result.linked_negatives,
        corpus::DECISIONS.iter().filter(|d| d.4.is_empty()).count(),
        fixture["model"].as_str().unwrap_or("?")
    );
    println!("latency link validation of the corpus: {elapsed:?}");
    assert!(result.precision() >= PRECISION_FLOOR, "{result:?}");
    assert!(result.recall() >= RECALL_FLOOR, "{result:?}");
}
