//! Synthetic retrieval corpus (v3) with a quality gate; not a human evaluation.
//! Entry points: ContextPacks::build_pack and render_compact (not prepare).

#[path = "fixtures/context_corpus.rs"]
mod corpus;
#[path = "fixtures/context_corpus_v4.rs"]
mod corpus_v4;
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
    assert_eq!(corpus::FAMILIES.len(), 29);
    assert_eq!(corpus::FAMILIES.iter().filter(|f| f.positive).count(), 21);
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
    assert_eq!(fixture.aliases.len(), 29);
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
fn sealed_v4_corpus_is_well_formed() {
    let fixture = Fixture::seed("corpus-v4-integrity");
    let mut names: BTreeSet<&str> = corpus::FAMILIES.iter().map(|f| f.name).collect();
    let mut queries: BTreeSet<&str> = corpus::FAMILIES.iter().flat_map(|f| f.queries).collect();
    for family in corpus_v4::FAMILIES_V4 {
        assert!(names.insert(family.name), "{}", family.name);
        assert!(family.holdout);
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
const PRECISION_FLOOR: f64 = 0.93;
const RECALL_FLOOR: f64 = 0.95;
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

/// Texts the vector fixture is generated from: each decision's question and
/// choice, each claim's statement and every v3 and v4 task. Input of
/// `tools/context-embeddings`.
#[test]
#[ignore = "writes the embedding input; run with XEMNAS_EMBED_TEXTS=<file>"]
fn export_embedding_texts() {
    let path = std::env::var_os("XEMNAS_EMBED_TEXTS").expect("set XEMNAS_EMBED_TEXTS");
    let items: serde_json::Map<String, serde_json::Value> = corpus::DECISIONS
        .iter()
        .map(|(alias, _, question, choice)| (alias.to_string(), format!("{question} {choice}")))
        .chain(
            CLAIMS
                .iter()
                .map(|(alias, _, statement, _)| (alias.to_string(), statement.to_string())),
        )
        .map(|(alias, text)| (alias, text.into()))
        .collect();
    let tasks: BTreeSet<&str> = corpus::FAMILIES
        .iter()
        .chain(corpus_v4::FAMILIES_V4)
        .flat_map(|family| family.queries)
        .collect();
    let json = serde_json::json!({ "items": items, "tasks": tasks });
    std::fs::write(path, serde_json::to_string_pretty(&json).expect("json")).expect("write");
}

/// Embeddings of the corpus texts (`fixtures/context_corpus_vectors_*.json`,
/// generated by `tools/context-embeddings`), for the experiment in
/// `docs/pesquisas/embeddings-no-contexto.md`.
struct Vectors {
    items: BTreeMap<String, Vec<f32>>,
    tasks: BTreeMap<String, Vec<f32>>,
}

impl Vectors {
    fn load(json: &str) -> Self {
        let fixture: serde_json::Value = serde_json::from_str(json).expect("vectors fixture");
        let decode = |hex: &serde_json::Value| -> Vec<f32> {
            let hex = hex.as_str().expect("hex vector");
            (0..hex.len())
                .step_by(2)
                .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).expect("hex") as i8 as f32)
                .collect()
        };
        let table = |field: &str| {
            fixture[field]
                .as_object()
                .expect("vector table")
                .iter()
                .map(|(key, hex)| (key.clone(), decode(hex)))
                .collect()
        };
        Self {
            items: table("items"),
            tasks: table("tasks"),
        }
    }

    fn similarity(&self, task: &str, alias: &str) -> f32 {
        let (a, b) = (&self.tasks[task], &self.items[alias]);
        let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm = |v: &[f32]| v.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot / (norm(a) * norm(b))
    }
}

/// A semantic pass over the lexical selection. `floor`/`fraction`: an item
/// found only by text stays when its similarity reaches the floor and that
/// fraction of the best one. `rescue`: when nothing is left, the most similar
/// item enters if it reaches `rescue` with `margin` over the second.
#[derive(Clone, Copy, Debug)]
struct Semantic {
    floor: f32,
    fraction: f32,
    rescue: Option<(f32, f32)>,
}

fn graph_linked(family: &corpus::Family) -> BTreeSet<String> {
    LINKS
        .iter()
        .filter(|(_, pattern, _)| {
            let prefix = pattern.trim_end_matches("**");
            family.files.iter().any(|file| file.starts_with(prefix))
        })
        .flat_map(|(_, _, linked)| linked.iter().map(|alias| (*alias).to_string()))
        .collect()
}

fn apply_semantic(
    vectors: &Vectors,
    family: &corpus::Family,
    task: &str,
    items: &BTreeSet<String>,
    semantic: Semantic,
) -> BTreeSet<String> {
    let standing: BTreeSet<String> = corpus::STANDING.iter().map(|s| (*s).into()).collect();
    let graph = graph_linked(family);
    let lexical: Vec<&String> = items
        .iter()
        .filter(|alias| !standing.contains(*alias) && !graph.contains(*alias))
        .collect();
    let best = lexical
        .iter()
        .map(|alias| vectors.similarity(task, alias))
        .fold(f32::MIN, f32::max);
    let mut kept: BTreeSet<String> = items
        .iter()
        .filter(|alias| {
            !lexical.contains(alias) || {
                let similarity = vectors.similarity(task, alias);
                similarity >= semantic.floor && similarity >= semantic.fraction * best
            }
        })
        .cloned()
        .collect();

    let Some((rescue, margin)) = semantic.rescue else {
        return kept;
    };
    if kept.difference(&standing).next().is_some() {
        return kept;
    }
    let forbidden: BTreeSet<&str> = corpus::FORBIDDEN.iter().copied().collect();
    let mut ranked: Vec<(f32, &String)> = vectors
        .items
        .keys()
        .filter(|alias| !forbidden.contains(alias.as_str()) && !standing.contains(*alias))
        .map(|alias| (vectors.similarity(task, alias), alias))
        .collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    if ranked[0].0 >= rescue && ranked[0].0 - ranked[1].0 >= margin {
        kept.insert(ranked[0].1.clone());
    }
    kept
}

fn measure_semantic(tag: &str, vectors: &str, families: &[corpus::Family], variants: &[Semantic]) {
    let fixture = Fixture::seed(tag);
    let packs = ContextPacks::new(fixture.test.store.clone());
    let vectors = Vectors::load(vectors);
    let mut cases = Vec::new();
    for family in families {
        for task in family.queries {
            let pack = packs
                .build_pack(request(family, task, 12_000))
                .expect("pack");
            cases.push((family, task, selected(&fixture, &pack)));
        }
    }
    for semantic in variants {
        let mut totals = Totals::default();
        for (family, task, items) in &cases {
            let kept = apply_semantic(&vectors, family, task, items, *semantic);
            score("semantic", family, 0, &kept, items, 0, &mut totals);
        }
        println!(
            "variant {tag} {semantic:?} precision={} recall={} contaminated={}/{}",
            ratio(totals.tp, totals.retrieved),
            ratio(totals.tp, totals.expected),
            totals.contaminated,
            totals.negative_cases
        );
    }
}

/// Variants chosen on v3 alone before v4 was read: the best v3 veto (0.95 /
/// 0.94 / 0), a rescue whose floor and margin no v3 negative reaches, and both.
const POTION: &str = include_str!("fixtures/context_corpus_vectors_potion.json");
const E5: &str = include_str!("fixtures/context_corpus_vectors_e5.json");

const SEMANTIC_CANDIDATES: &[Semantic] = &[
    Semantic {
        floor: -1.0,
        fraction: 0.0,
        rescue: None,
    },
    Semantic {
        floor: 0.0,
        fraction: 0.8,
        rescue: None,
    },
    Semantic {
        floor: -1.0,
        fraction: 0.0,
        rescue: Some((0.5, 0.1)),
    },
    Semantic {
        floor: 0.0,
        fraction: 0.8,
        rescue: Some((0.5, 0.1)),
    },
];

/// Tunes the semantic pass on v3 only; prints one line per variant.
#[test]
#[ignore = "embedding experiment on v3; run with --nocapture"]
fn report_semantic_variants_v3() {
    let mut variants = SEMANTIC_CANDIDATES.to_vec();
    for floor in [0.0, 0.1, 0.15, 0.2, 0.25, 0.3, 0.35] {
        for fraction in [0.0, 0.6, 0.7, 0.8] {
            for rescue in [
                None,
                Some((0.3, 0.0)),
                Some((0.4, 0.0)),
                Some((0.4, 0.05)),
                Some((0.5, 0.05)),
            ] {
                variants.push(Semantic {
                    floor,
                    fraction,
                    rescue,
                });
            }
        }
    }
    measure_semantic("semantic-v3", POTION, corpus::FAMILIES, &variants);
}

/// The v3-chosen variants on the sealed v4, read once, in aggregate.
#[test]
#[ignore = "embedding experiment on sealed v4; run with --nocapture"]
fn report_semantic_candidates_v4() {
    measure_semantic(
        "semantic-v4",
        POTION,
        corpus_v4::FAMILIES_V4,
        SEMANTIC_CANDIDATES,
    );
}

/// Similarity of each v3 task to its required items and to the best other
/// item, to place the floors of the semantic pass.
#[test]
#[ignore = "embedding experiment on v3; run with --nocapture"]
fn report_similarity_v3() {
    let model = std::env::var("XEMNAS_EMBED_MODEL").unwrap_or_default();
    let vectors = Vectors::load(if model == "e5" { E5 } else { POTION });
    let skip: BTreeSet<&str> = corpus::FORBIDDEN
        .iter()
        .chain(corpus::STANDING)
        .copied()
        .collect();
    for family in corpus::FAMILIES {
        for task in family.queries {
            let mut ranked: Vec<(f32, &str)> = vectors
                .items
                .keys()
                .filter(|alias| !skip.contains(alias.as_str()))
                .map(|alias| (vectors.similarity(task, alias), alias.as_str()))
                .collect();
            ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
            let rank = |alias: &str| ranked.iter().position(|(_, a)| *a == alias).unwrap_or(99);
            let required: Vec<String> = family
                .required
                .iter()
                .map(|alias| {
                    format!(
                        "{alias}:{:.2}#{}",
                        vectors.similarity(task, alias),
                        rank(alias)
                    )
                })
                .collect();
            println!(
                "sim {} positive={} top={}:{:.2} second={}:{:.2} required={}",
                family.name,
                family.positive,
                ranked[0].1,
                ranked[0].0,
                ranked[1].1,
                ranked[1].0,
                required.join(",")
            );
        }
    }
}

/// e5 variants chosen on v3 alone before v4 was read: the best v3 veto
/// (0.97 of the best similarity: 0.95 / 0.95 / 0), a rescue on the margin
/// over the second item alone (no v3 negative has more than 0.03), and both.
const E5_CANDIDATES: &[Semantic] = &[
    Semantic {
        floor: -1.0,
        fraction: 0.0,
        rescue: None,
    },
    Semantic {
        floor: -1.0,
        fraction: 0.97,
        rescue: None,
    },
    Semantic {
        floor: -1.0,
        fraction: 0.0,
        rescue: Some((0.0, 0.04)),
    },
    Semantic {
        floor: -1.0,
        fraction: 0.97,
        rescue: Some((0.0, 0.04)),
    },
];

/// The v3-chosen e5 variants on v3, then on the sealed v4, read once.
#[test]
#[ignore = "embedding experiment on sealed v4; run with --nocapture"]
fn report_e5_candidates() {
    measure_semantic("e5-v3", E5, corpus::FAMILIES, E5_CANDIDATES);
    if std::env::var_os("XEMNAS_READ_V4").is_some() {
        measure_semantic("e5-v4", E5, corpus_v4::FAMILIES_V4, E5_CANDIDATES);
    }
}

/// multilingual-e5-small on v3: its cosines sit high (0.7 to 0.9), so the
/// floors are its own.
#[test]
#[ignore = "embedding experiment on v3; run with --nocapture"]
fn report_e5_variants_v3() {
    let mut variants = vec![Semantic {
        floor: -1.0,
        fraction: 0.0,
        rescue: None,
    }];
    for floor in [0.0, 0.75, 0.78, 0.8, 0.82, 0.84] {
        for fraction in [0.0, 0.95, 0.97, 0.98] {
            for rescue in [
                None,
                Some((0.84, 0.0)),
                Some((0.86, 0.0)),
                Some((0.86, 0.02)),
                Some((0.88, 0.02)),
            ] {
                variants.push(Semantic {
                    floor,
                    fraction,
                    rescue,
                });
            }
        }
    }
    measure_semantic("e5-v3", E5, corpus::FAMILIES, &variants);
}
