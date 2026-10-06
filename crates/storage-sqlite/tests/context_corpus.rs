//! Synthetic retrieval corpus (v3) with a quality gate; not a human evaluation.
//! Entry points: ContextPacks::build_pack and render_compact (not prepare).

#[path = "fixtures/context_corpus.rs"]
mod corpus;
#[path = "fixtures/context_corpus_v4.rs"]
mod corpus_v4;
#[path = "fixtures/context_corpus_v5.rs"]
mod corpus_v5;
mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use application::claims::{Claims, NewClaim};
use application::context::{ContextPack, ContextPacks, ContextProvider, ContextRequest};
use application::decisions::{DecisionEdits, Decisions};
use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
use application::injection::render_compact;
use application::qualifiers::{KnowledgeQualifier, QualifierKind};
use application::relations::DecisionRelations;
use application::search_terms::SearchTermStore;
use domain::claims::ClaimKind;
use domain::entities::{EdgeKind, EntityKind, NodeKind};

struct Fixture {
    test: support::TestStore,
    aliases: BTreeMap<String, String>,
}

impl Fixture {
    fn seed(tag: &str) -> Self {
        let test = support::open(tag, &["p1", "p2"]);
        let mut aliases = BTreeMap::new();
        for (alias, project, question, choice) in corpus::DECISIONS {
            let id = support::decision(&test.store, project, alias, question, choice);
            aliases.insert(alias.to_string(), id);
        }
        Decisions::new(test.store.clone())
            .revise(
                &aliases["override"],
                DecisionEdits {
                    rationale: Some(
                        concat!(
                    "Override não deve zerar contadores nem mudar o comportamento local. ",
                    "A avaliação é sintética e local; não representa aprovação de mantenedores."
                )
                        .into(),
                    ),
                    scope: Some(vec!["avaliação local".into()]),
                    qualifiers: Some(vec![KnowledgeQualifier {
                        kind: QualifierKind::Validation,
                        text:
                            "Avaliação sintética local, sem validação ou autoridade de mantenedores"
                                .into(),
                        artifact_id: None,
                    }]),
                    ..DecisionEdits::default()
                },
            )
            .expect("qualify local Override evaluation");
        DecisionRelations::new(test.store.clone())
            .supersede(&aliases["storage"], &aliases["superseded"])
            .expect("supersede synthetic decision");
        for (alias, kind, statement, until) in CLAIMS {
            let id = Claims::new(test.store.clone())
                .create(NewClaim {
                    source_version: None,
                    qualifiers: Vec::new(),
                    project_id: "p1".into(),
                    kind: *kind,
                    statement: (*statement).into(),
                    valid_from: Some("2020-01-01".into()),
                    valid_until: until.map(str::to_string),
                    source_decision_id: None,
                })
                .expect("seed synthetic claim")
                .claim_id;
            aliases.insert(alias.to_string(), id);
        }
        let graph = KnowledgeGraph::new(test.store.clone());
        for (name, pattern, linked) in LINKS {
            let component = graph
                .create_entity(NewEntity {
                    project_id: "p1".into(),
                    kind: Some(EntityKind::Component),
                    name: (*name).into(),
                    patterns: vec![(*pattern).into()],
                    ..NewEntity::default()
                })
                .expect("seed component")
                .entity_id;
            for alias in *linked {
                graph
                    .link(LinkRequest {
                        kind: EdgeKind::Affects,
                        source_kind: NodeKind::Decision,
                        source_id: aliases[*alias].clone(),
                        entity_id: component.clone(),
                    })
                    .expect("seed file link");
            }
        }
        let ledger = graph
            .create_entity(NewEntity {
                project_id: "p1".into(),
                kind: Some(EntityKind::Component),
                name: "ledger".into(),
                patterns: vec!["crates/ledger/**".into()],
                ..NewEntity::default()
            })
            .expect("seed ledger component")
            .entity_id;
        graph
            .link(LinkRequest {
                kind: EdgeKind::AppliesTo,
                source_kind: NodeKind::Claim,
                source_id: aliases["scoped"].clone(),
                entity_id: ledger,
            })
            .expect("seed scoped rule");
        if WITH_SEARCH_TERMS {
            seed_search_terms(&test.store, &aliases);
        }
        Self { test, aliases }
    }

    fn alias(&self, id: &str) -> String {
        self.aliases
            .iter()
            .find(|(_, value)| value.as_str() == id)
            .expect("every result has a frozen alias")
            .0
            .clone()
    }
}

/// Seeded claims: (alias, kind, statement, valid until).
const CLAIMS: &[(&str, ClaimKind, &str, Option<&str>)] = &[
    (
        "standing",
        ClaimKind::Convention,
        "Mensagens de erro em português",
        None,
    ),
    (
        "rate",
        ClaimKind::Assumption,
        "API cache suporta dez pedidos por segundo",
        None,
    ),
    (
        "expired",
        ClaimKind::Constraint,
        "Suportar fax legado",
        Some("2021-01-01"),
    ),
    // A standing rule tied to the `ledger` component (see `Fixture::seed`).
    (
        "scoped",
        ClaimKind::Constraint,
        "Registrar cada lançamento contábil com o selo do auditor",
        None,
    ),
];

/// Seeded components: (name, file pattern, decisions linked to it).
const LINKS: &[(&str, &str, &[&str])] = &[
    ("storage", "crates/storage/**", &["storage"]),
    ("outbox", "adapters/outbox/**", &["outbox"]),
    // Coarse on purpose: a whole-app component links unrelated UI
    // decisions, which a file-led task must not all receive.
    (
        "desktop-ui",
        "apps/desktop-gpui/**",
        &["virtual-list", "fonts", "accent"],
    ),
];

/// Whether decisions carry the search terms a live model generated for them
/// (`fixtures/context_corpus_terms.json`, see `application::search_terms`).
const WITH_SEARCH_TERMS: bool = true;

fn seed_search_terms(store: &storage_sqlite::SqliteStore, aliases: &BTreeMap<String, String>) {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/context_corpus_terms.json"))
            .expect("terms fixture");
    let model = fixture["model"].as_str().expect("model");
    for (alias, terms) in fixture["terms"].as_object().expect("terms") {
        let terms: Vec<String> = serde_json::from_value(terms.clone()).expect("term list");
        store
            .set_search_terms(&aliases[alias], &terms, model)
            .expect("seed terms");
    }
}

fn request(family: &corpus::Family, task: &str, budget: usize) -> ContextRequest {
    ContextRequest {
        project_id: "p1".into(),
        task: task.into(),
        as_of: None,
        budget_chars: Some(budget),
        files: family.files.iter().map(|s| (*s).into()).collect(),
    }
}

fn selected(fixture: &Fixture, pack: &ContextPack) -> BTreeSet<String> {
    pack.decisions
        .iter()
        .map(|d| fixture.alias(&d.decision_id))
        .chain(pack.claims.iter().map(|c| fixture.alias(&c.claim_id)))
        .collect()
}

#[test]
fn corpus_integrity_budgets_and_isolation() {
    let fixture = Fixture::seed("corpus-integrity");
    let packs = ContextPacks::new(fixture.test.store.clone());
    assert_eq!(corpus::FAMILIES.len(), 33);
    assert_eq!(corpus::FAMILIES.iter().filter(|f| f.positive).count(), 24);
    assert_eq!(
        corpus::FAMILIES
            .iter()
            .filter(|f| f.holdout && f.positive)
            .count(),
        9
    );
    assert_eq!(
        corpus::FAMILIES
            .iter()
            .filter(|f| f.holdout && !f.positive)
            .count(),
        3
    );
    assert_eq!(fixture.aliases.len(), 30);
    let mut queries = BTreeSet::new();
    let mut names = BTreeSet::new();
    for family in corpus::FAMILIES {
        assert!(names.insert(family.name));
        assert_eq!(family.positive, !family.required.is_empty());
        for alias in family
            .required
            .iter()
            .chain(family.partial)
            .chain(corpus::STANDING)
            .chain(corpus::FORBIDDEN)
        {
            assert!(fixture.aliases.contains_key(*alias));
        }
        for task in family.queries {
            assert!(queries.insert(task));
            let pack = packs
                .build_pack(request(family, task, 12_000))
                .expect("pack");
            assert_eq!(pack.project_id, "p1");
            assert!(pack.used_chars <= pack.budget_chars);
            let items = selected(&fixture, &pack);
            assert_eq!(items.len(), pack.decisions.len() + pack.claims.len());
            for alias in corpus::FORBIDDEN {
                assert!(!items.contains(*alias), "{}: {alias}", family.name);
            }
            if let Some(block) = render_compact(&pack, 300, &BTreeSet::new()) {
                assert!(block.tokens <= 300);
                assert_eq!(block.tokens, block.text.chars().count().div_ceil(4));
                assert_eq!(
                    block.items.len() + block.omitted,
                    pack.decisions.len() + pack.claims.len() + pack.omitted
                );
            }
            let small = packs
                .build_pack(request(family, task, 700))
                .expect("small pack");
            assert!(small.used_chars <= 700);
        }
    }
    assert_eq!(queries.len(), corpus::FAMILIES.len() * 3);
}

#[test]
fn v4_and_v5_corpora_are_well_formed() {
    let fixture = Fixture::seed("corpus-v4-integrity");
    let mut names: BTreeSet<&str> = corpus::FAMILIES.iter().map(|f| f.name).collect();
    let mut queries: BTreeSet<&str> = corpus::FAMILIES.iter().flat_map(|f| f.queries).collect();
    let v4 = corpus_v4::FAMILIES_V4.iter().map(|family| (family, true));
    let v5 = corpus_v5::FAMILIES_V5.iter().map(|family| (family, false));
    for (family, holdout) in v4.chain(v5) {
        assert!(names.insert(family.name), "{}", family.name);
        assert_eq!(family.holdout, holdout);
        assert_eq!(family.positive, !family.required.is_empty());
        for task in family.queries {
            assert!(queries.insert(task), "{task}");
        }
        for alias in family.required.iter().chain(family.partial) {
            assert!(fixture.aliases.contains_key(*alias), "{alias}");
            assert!(!corpus::FORBIDDEN.contains(alias), "{alias}");
            assert!(!corpus::STANDING.contains(alias), "{alias}");
        }
    }
    assert_eq!(corpus_v4::FAMILIES_V4.len(), 24);
    assert_eq!(corpus_v5::FAMILIES_V5.len(), 30);
}

#[derive(Default)]
struct Totals {
    tp: usize,
    retrieved: usize,
    expected: usize,
    negative: usize,
    negative_cases: usize,
    contaminated: usize,
}

fn ratio(n: usize, d: usize) -> String {
    if d == 0 {
        "N/A".into()
    } else {
        format!("{:.4}", n as f64 / d as f64)
    }
}

fn score(
    stage: &str,
    family: &corpus::Family,
    variant: usize,
    items: &BTreeSet<String>,
    upstream: &BTreeSet<String>,
    tokens: usize,
    totals: &mut Totals,
) {
    let required: BTreeSet<String> = family.required.iter().map(|s| (*s).into()).collect();
    let standing: BTreeSet<String> = corpus::STANDING.iter().map(|s| (*s).into()).collect();
    let partial: BTreeSet<String> = family.partial.iter().map(|s| (*s).into()).collect();
    let topical: BTreeSet<String> = items.difference(&standing).cloned().collect();
    let tp = topical.intersection(&required).count();
    let missing: BTreeSet<String> = required.difference(items).cloned().collect();
    let oracle_miss = missing.difference(upstream).count();
    let budget_miss = missing.intersection(upstream).count();
    let contamination = if family.positive { 0 } else { topical.len() };
    totals.tp += tp;
    totals.retrieved += topical.len();
    totals.expected += required.len();
    totals.negative += contamination;
    totals.negative_cases += usize::from(!family.positive);
    totals.contaminated += usize::from(!family.positive && contamination > 0);
    println!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\",\"{}\"",
        stage,
        family.name,
        variant,
        if family.holdout {
            "holdout"
        } else {
            "development"
        },
        family.positive,
        tp,
        topical.len(),
        required.len(),
        ratio(tp, topical.len()),
        ratio(tp, required.len()),
        items.intersection(&standing).count(),
        contamination,
        tokens,
        oracle_miss,
        budget_miss,
        items
            .intersection(&partial)
            .cloned()
            .collect::<Vec<_>>()
            .join("|"),
        topical
            .difference(&required)
            .cloned()
            .collect::<Vec<_>>()
            .join("|")
    );
}

#[test]
#[ignore = "synthetic baseline report; run with --nocapture --test-threads=1"]
fn report_context_corpus() {
    let fixture = Fixture::seed("corpus-report");
    let packs = ContextPacks::new(fixture.test.store.clone());
    let mut latency = Vec::new();
    let mut pack_totals = Totals::default();
    let mut compact_totals = Totals::default();
    println!("synthetic=true human_judgments=false calibration=false provider_calls=0");
    println!("entrypoints=build_pack+render_compact prepare_integration=not_exercised");
    println!(
        "human_active_time=not_measured interventions=not_measured tokens=estimated_not_billed"
    );
    println!("precision=strict_required/topical partial_related=reported_not_true_positive");
    println!(concat!(
        "stage,family,variant,split,positive,tp,retrieved,expected,precision,recall,",
        "standing,negative_items,estimated_tokens,oracle_misses,budget_misses,partial_related,noise"
    ));
    for family in corpus::FAMILIES {
        for (variant, task) in family.queries.iter().enumerate() {
            let req = request(family, task, 12_000);
            let pack = packs.build_pack(req.clone()).expect("warmup: one per case");
            let items = selected(&fixture, &pack);
            // Unbudgeted reference distinguishes retrieval misses from pack budget losses.
            let reference = packs
                .build_pack(request(family, task, 50_000))
                .expect("budget attribution reference");
            let upstream = selected(&fixture, &reference);
            score(
                "pack",
                family,
                variant,
                &items,
                &upstream,
                pack.used_chars.div_ceil(4),
                &mut pack_totals,
            );
            let block = render_compact(&pack, 300, &BTreeSet::new());
            let compact_items = block
                .as_ref()
                .map(|b| b.items.iter().map(|i| fixture.alias(&i.id)).collect())
                .unwrap_or_default();
            score(
                "compact300",
                family,
                variant,
                &compact_items,
                &upstream,
                block.as_ref().map_or(0, |b| b.tokens),
                &mut compact_totals,
            );
            for _ in 0..10 {
                let input = req.clone();
                let start = Instant::now();
                let result = packs.build_pack(input);
                let elapsed = start.elapsed();
                result.expect("latency pack");
                latency.push(elapsed.as_micros());
            }
        }
    }
    let samples = latency.len();
    assert_eq!(samples, corpus::FAMILIES.len() * 3 * 10);
    latency.sort_unstable();
    println!(
        "latency_build_pack_only samples={samples} p50_us={} p95_us={}",
        latency[samples / 2],
        latency[samples * 95 / 100]
    );
    for (stage, t) in [("pack", pack_totals), ("compact300", compact_totals)] {
        println!(
            concat!(
                "summary {} precision={}/{}={} recall={}/{}={} negative_items={} ",
                "contaminated_cases={}/{}={} standing_rules=excluded_from_quality"
            ),
            stage,
            t.tp,
            t.retrieved,
            ratio(t.tp, t.retrieved),
            t.tp,
            t.expected,
            ratio(t.tp, t.expected),
            t.negative,
            t.contaminated,
            t.negative_cases,
            ratio(t.contaminated, t.negative_cases)
        );
    }
}

/// Floors of the context selection on corpus v3. The gate fails on a
/// regression; raise a floor whenever an improvement lands, so the next
/// change cannot quietly give it back. Improvements are judged on the
/// holdout families, which no tuning may look at.
///
/// History: baseline 0.23 / 0.76 / 9 contaminated (OR match, no cut). Term
/// coverage (two meaningful words in common) and fuller stopwords: 0.63 /
/// 0.58 / 1. Recall was traded for precision on purpose (a wrong item
/// costs the agent more than a missing one); the PT/EN bridge must win it
/// back. Graph focus (a wide component's items that share nothing with the
/// task go when others do): 0.70 / 0.58 / 1; its motivating case, big-list,
/// is a holdout family, so it no longer counts as independent evidence.
/// PT/EN bridge (plurals, verb endings, a general bilingual glossary):
/// 0.75 / 0.91 / 1, holdout 0.70 / 0.78 / 0; the verb endings were
/// prompted by confirm-race, also a holdout family. Search terms from a
/// live model (`fixtures/context_corpus_terms.json`): counted always, 0.66 /
/// 0.95 / 1, so they are a last resort, used only when no decision speaks
/// of the task in its own words: 0.76 / 0.95 / 1, holdout 0.73 / 0.89 / 0.
/// The last-resort rule was chosen after seeing which families regressed.
/// Synonyms of the task are one concept and a word of the text answers for
/// one concept ("mudar" no longer covers both "trocar" and "mudar"): 0.79 /
/// 0.95 / 1, holdout 0.80 / 0.89 / 0. Relative coverage (when the best
/// lexical match covers 4 concepts or more, the others must match it):
/// 0.83 / 0.95 / 1, holdout 0.92 / 0.89 / 0. A bar from 3 concepts gave
/// 0.88 but recall 0.92 (same holdout), so 4 was kept. Main clause (a
/// match must stand on the task's main clause, as much as the best match
/// does): 0.94 / 0.95 / 0, holdout 0.96 / 0.89 / 0. Demanding two main
/// concepts always gave recall 0.92 (outbox-delivery, whose subordinate
/// clause carries the subject). The corpus's distractor variants are
/// written as subordinate clauses, so this gain is likely optimistic.
///
/// Scoped rules (06/10/2026): four families on a standing rule tied to one
/// component took v3 to 0.9474 / 0.9600 / 0 (33 families); a rule outside the
/// task's components no longer rides along.
const PRECISION_FLOOR: f64 = 0.94;
const RECALL_FLOOR: f64 = 0.96;
const CONTAMINATED_CASES_CEILING: usize = 0;
/// Generous on purpose: this runs on a developer's machine beside other work.
const BUILD_PACK_P95_CEILING_US: u128 = 20_000;

struct Measure {
    totals: Totals,
    development: Totals,
    holdout: Totals,
    p95_us: u128,
}

fn measure(tag: &str, families: &[corpus::Family]) -> Measure {
    let fixture = Fixture::seed(tag);
    let packs = ContextPacks::new(fixture.test.store.clone());
    let mut totals = Totals::default();
    let mut development = Totals::default();
    let mut holdout = Totals::default();
    let mut latency = Vec::new();
    for family in families {
        for (variant, task) in family.queries.iter().enumerate() {
            let req = request(family, task, 12_000);
            let pack = packs.build_pack(req.clone()).expect("pack");
            let items = selected(&fixture, &pack);
            let reference = packs
                .build_pack(request(family, task, 50_000))
                .expect("reference");
            let upstream = selected(&fixture, &reference);
            let tokens = pack.used_chars.div_ceil(4);
            score(
                "gate",
                family,
                variant,
                &items,
                &upstream,
                tokens,
                &mut totals,
            );
            let split = if family.holdout {
                &mut holdout
            } else {
                &mut development
            };
            score("split", family, variant, &items, &upstream, tokens, split);
            for _ in 0..3 {
                let start = Instant::now();
                packs.build_pack(req.clone()).expect("latency pack");
                latency.push(start.elapsed().as_micros());
            }
        }
    }
    latency.sort_unstable();
    let p95_us = latency[latency.len() * 95 / 100];
    println!(
        "gate {tag} precision={} recall={} contaminated={}/{} p95_us={p95_us}",
        ratio(totals.tp, totals.retrieved),
        ratio(totals.tp, totals.expected),
        totals.contaminated,
        totals.negative_cases
    );
    for (name, split) in [("development", &development), ("holdout", &holdout)] {
        println!(
            "gate {tag} {name} precision={} recall={} contaminated={}/{}",
            ratio(split.tp, split.retrieved),
            ratio(split.tp, split.expected),
            split.contaminated,
            split.negative_cases
        );
    }
    Measure {
        totals,
        development,
        holdout,
        p95_us,
    }
}

fn assert_floors(measure: &Measure, precision_floor: f64, recall_floor: f64, ceiling: usize) {
    let totals = &measure.totals;
    let precision = totals.tp as f64 / totals.retrieved.max(1) as f64;
    let recall = totals.tp as f64 / totals.expected.max(1) as f64;
    assert!(
        precision >= precision_floor,
        "context precision fell to {precision:.4} (floor {precision_floor})"
    );
    assert!(
        recall >= recall_floor,
        "context recall fell to {recall:.4} (floor {recall_floor})"
    );
    assert!(
        totals.contaminated <= ceiling,
        "{} negative cases received context (ceiling {ceiling})",
        totals.contaminated
    );
    assert!(
        measure.p95_us <= BUILD_PACK_P95_CEILING_US,
        "build_pack p95 {}us above {BUILD_PACK_P95_CEILING_US}us",
        measure.p95_us
    );
}

#[test]
fn context_quality_gate() {
    let measure = measure("corpus-gate", corpus::FAMILIES);
    assert!(measure.development.expected > 0 && measure.holdout.expected > 0);
    assert_floors(
        &measure,
        PRECISION_FLOOR,
        RECALL_FLOOR,
        CONTAMINATED_CASES_CEILING,
    );
}

/// Floors of the sealed v4 holdout, set at its first measurement and never
/// a target for tuning. First run, with every rule up to the main clause:
/// 0.58 / 0.62 / 4 of 21. The same families at earlier steps: search terms
/// last resort 0.54 / 0.62 / 4, synonym concepts and relative coverage 0.55
/// / 0.62 / 4; without search terms 0.60 / 0.59 / 2. The v3 gains were
/// mostly fitted to v3: on tasks written blind, the rules after the PT/EN
/// bridge move precision by four points and recall not at all.
const V4_PRECISION_FLOOR: f64 = 0.58;
const V4_RECALL_FLOOR: f64 = 0.61;
const V4_CONTAMINATED_CASES_CEILING: usize = 4;

#[test]
fn sealed_v4_quality_gate() {
    let measure = measure("corpus-v4-gate", corpus_v4::FAMILIES_V4);
    assert_floors(
        &measure,
        V4_PRECISION_FLOOR,
        V4_RECALL_FLOOR,
        V4_CONTAMINATED_CASES_CEILING,
    );
}

/// Floors of the v5 calibration corpus, written blind like v4 (30 families,
/// hard same-vocabulary negatives). Tuning may look here; v4 stays sealed.
/// First run, lexical selection as of the main-clause rule: 0.41 / 0.43 /
/// 15 of 24. No embedding variant that kept v3 above 0.90 moved it (the
/// experiment and its vectors were removed; results in
/// `docs/pesquisas/precisao-do-contexto.md`).
const V5_PRECISION_FLOOR: f64 = 0.41;
const V5_RECALL_FLOOR: f64 = 0.43;
const V5_CONTAMINATED_CASES_CEILING: usize = 15;

#[test]
fn calibration_v5_quality_gate() {
    let measure = measure("corpus-v5-gate", corpus_v5::FAMILIES_V5);
    assert_floors(
        &measure,
        V5_PRECISION_FLOOR,
        V5_RECALL_FLOOR,
        V5_CONTAMINATED_CASES_CEILING,
    );
}
