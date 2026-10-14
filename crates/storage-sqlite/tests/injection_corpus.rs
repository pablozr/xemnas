//! Delivery gate for the automatic injection: runs the real `prepare` over a
//! scripted session that reproduces the JevGuard dogfood (manifest facts
//! dripping into unrelated turns, subagent notifications getting a block) and
//! measures filler blocks, harness prompts, recall and block size.

mod support;

use application::claims::{Claims, NewClaim};
use application::context_settings::{ContextMode, ContextSettings};
use application::injection::{ContextInjection, InjectionRequest};
use application::observations::refresh::{identity, refresh_project};
use application::observations::*;
use application::projects::canonicalize_location;
use domain::claims::ClaimKind;
use rusqlite::Connection;
use std::time::Instant;

const PACKAGE_JSON: &str = r#"{"name":"jevguard","version":"0.0.0","dependencies":{"@jevguard/core":"workspace:*"},"devDependencies":{"vitest":"^3.0.0","@types/node":"^22.0.0"}}"#;

/// Everything the harness delivers as a user message after a background agent ends.
const NOTIFICATION: &str = "<task-notification><task-id>a1</task-id><status>completed</status><summary>cache da API</summary></task-notification>";

/// One scripted turn: the prompt, the transcript's recent files, the pieces a
/// useful block must carry (empty when the turn must get no block) and whether
/// it is a synthetic harness prompt.
struct Turn {
    prompt: &'static str,
    files: &'static [&'static str],
    wants: &'static [&'static str],
    harness: bool,
}

const SCRIPT: &[Turn] = &[
    Turn {
        prompt: "melhorar o cache da API",
        files: &["src/api.ts"],
        wants: &["Como fazer cache da API?", "regra:"],
        harness: false,
    },
    Turn {
        prompt: NOTIFICATION,
        files: &["src/api.ts"],
        wants: &[],
        harness: true,
    },
    Turn {
        prompt: "rodar os testes de novo",
        files: &["src/index.ts"],
        wants: &[],
        harness: false,
    },
    Turn {
        prompt: "ajuste o lint",
        files: &["package.json"],
        wants: &[],
        harness: false,
    },
    Turn {
        prompt: NOTIFICATION,
        files: &[],
        wants: &[],
        harness: true,
    },
    Turn {
        prompt: "atualizar o vitest para 3.1",
        files: &[],
        wants: &["vitest"],
        harness: false,
    },
    Turn {
        prompt: "revise as dependências do package.json",
        files: &[],
        wants: &["O:"],
        harness: false,
    },
];

struct Reader(Vec<u8>);

impl ObservationReader for Reader {
    fn read_source(
        &self,
        request: &SourceReadRequest,
    ) -> Result<SourceReadResult, ObservationError> {
        Ok(if request.project_relative_path == "package.json" {
            SourceReadResult {
                status: CheckStatus::Verified,
                bytes: self.0.clone(),
            }
        } else {
            SourceReadResult {
                status: CheckStatus::Missing,
                bytes: vec![],
            }
        })
    }
}

/// Registers the manifest as a source, then reads it like a real refresh.
fn refresh_manifest(test: &support::TestStore) {
    let generation = test
        .store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "seed".into(),
            requested_at: "now".into(),
        })
        .unwrap()
        .unwrap();
    test.store
        .apply_refresh(&RefreshBatch {
            project_id: "p1".into(),
            expected_generation: generation.generation,
            sources: vec![ObservationSource {
                source_id: identity("p1", "package.json"),
                project_id: "p1".into(),
                project_relative_path: "package.json".into(),
                manifest_kind: ManifestKind::PackageJson,
                sha256: None,
                parser_policy_version: "1".into(),
                last_checked_at: None,
                last_check_status: CheckStatus::Missing,
                semantic_cache: None,
            }],
            observations: vec![],
            coverage: Default::default(),
        })
        .unwrap();
    let generation = test
        .store
        .request_refresh(&RefreshRequest {
            project_id: "p1".into(),
            capture_trigger: "test".into(),
            requested_at: "now".into(),
        })
        .unwrap()
        .unwrap();
    assert_eq!(
        refresh_project(
            &test.store,
            &Reader(PACKAGE_JSON.as_bytes().to_vec()),
            "p1",
            "root",
            generation.generation,
            "test"
        )
        .unwrap(),
        ApplyRefreshResult::Applied
    );
}

fn register_directory(test: &support::TestStore) -> String {
    let directory = test.root.join("workspace");
    std::fs::create_dir_all(&directory).expect("workspace");
    let location = canonicalize_location(&directory.to_string_lossy()).expect("canonical");
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .execute(
            "UPDATE projects SET location = ?1 WHERE id = 'p1'",
            [&location],
        )
        .expect("relocate project");
    directory.to_string_lossy().into_owned()
}

fn rows(test: &support::TestStore) -> i64 {
    Connection::open(test.root.join("app.db"))
        .expect("raw")
        .query_row("SELECT COUNT(*) FROM context_injections", [], |row| {
            row.get(0)
        })
        .expect("count")
}

fn fixture() -> (support::TestStore, String) {
    let test = support::open("inject-corpus", &["p1"]);
    let path = register_directory(&test);
    support::decision(
        &test.store,
        "p1",
        "cache",
        "Como fazer cache da API?",
        "Cache em memória",
    );
    Claims::new(test.store.clone())
        .create(NewClaim {
            source_version: None,
            qualifiers: vec![application::qualifiers::KnowledgeQualifier {
                kind: application::qualifiers::QualifierKind::Scope,
                text: application::context::GLOBAL_SCOPE.into(),
                artifact_id: None,
            }],
            project_id: "p1".to_string(),
            kind: ClaimKind::Convention,
            statement: "Mensagens de erro em português".to_string(),
            valid_from: Some("2020-01-01".to_string()),
            valid_until: None,
            source_decision_id: None,
        })
        .expect("claim");
    ContextSettings::new(test.store.clone())
        .set("p1", ContextMode::Inject, None)
        .expect("inject mode");
    refresh_manifest(&test);
    (test, path)
}

#[derive(Default)]
struct Measure {
    blocks: usize,
    filler: usize,
    harness: usize,
    found: usize,
    wanted: usize,
    tokens: usize,
}

/// Runs the script once in `session`; `latency` collects each `prepare` in microseconds.
fn run_session(
    test: &support::TestStore,
    path: &str,
    session: &str,
    latency: &mut Vec<u128>,
) -> Measure {
    let injection = ContextInjection::new(test.store.clone());
    let mut measure = Measure::default();
    for (number, turn) in SCRIPT.iter().enumerate() {
        let before = rows(test);
        let start = Instant::now();
        let outcome = injection
            .prepare(InjectionRequest {
                canonical_path: path.to_string(),
                session_id: session.to_string(),
                prompt: turn.prompt.to_string(),
                files: turn.files.iter().map(|file| file.to_string()).collect(),
                ..InjectionRequest::default()
            })
            .expect("prepare");
        latency.push(start.elapsed().as_micros());
        let recorded = rows(test) - before;
        let block = outcome.block.as_deref();
        if turn.harness && (block.is_some() || recorded > 0) {
            measure.harness += 1;
        }
        measure.wanted += turn.wants.len();
        let Some(block) = block else {
            continue;
        };
        measure.blocks += 1;
        measure.tokens += outcome.tokens;
        let found = turn
            .wants
            .iter()
            .filter(|want| block.contains(**want))
            .count();
        measure.found += found;
        if found == 0 {
            measure.filler += 1;
            println!(
                "filler turn={} block={}",
                number + 1,
                block.replace('\n', " | ")
            );
        }
    }
    measure
}

fn quality() -> Measure {
    let (test, path) = fixture();
    let mut latency = Vec::new();
    let measure = run_session(&test, &path, "s0", &mut latency);
    println!(
        "gate injection blocks={} filler={} harness={} recall={}/{} avg_tokens={} total_tokens={}",
        measure.blocks,
        measure.filler,
        measure.harness,
        measure.found,
        measure.wanted,
        measure.tokens.checked_div(measure.blocks).unwrap_or(0),
        measure.tokens
    );
    measure
}

fn p95_ms() -> f64 {
    let (test, path) = fixture();
    let mut latency = Vec::new();
    for session in 0..50 {
        run_session(&test, &path, &format!("s{session}"), &mut latency);
    }
    latency.sort_unstable();
    let p50 = latency[latency.len() / 2] as f64 / 1000.0;
    let p95 = latency[latency.len() * 95 / 100] as f64 / 1000.0;
    println!("latency injection_prepare p50={p50:.2}ms p95={p95:.2}ms");
    p95
}

/// Measured on 14/10/2026: 74 tokens on average (79 before plain-text
/// qualifiers). Raise it only with a reason.
const AVG_TOKENS_CEILING: usize = 75;

/// Same slack as the context pack gate.
const PREPARE_P95_CEILING_MS: f64 = 20.0;

#[test]
fn injection_quality_gate() {
    let measure = quality();
    assert_eq!(
        measure.filler, 0,
        "blocks without anything the turn asked for"
    );
    assert_eq!(measure.harness, 0, "synthetic prompts must get nothing");
    assert_eq!(
        measure.found, measure.wanted,
        "a requested item was not delivered"
    );
    let average = measure.tokens.checked_div(measure.blocks).unwrap_or(0);
    assert!(
        average <= AVG_TOKENS_CEILING,
        "average block {average} tokens above {AVG_TOKENS_CEILING}"
    );
}

#[test]
fn injection_latency_gate() {
    let p95 = p95_ms();
    assert!(
        p95 <= PREPARE_P95_CEILING_MS,
        "prepare p95 {p95:.2}ms above {PREPARE_P95_CEILING_MS}ms"
    );
}

const UUID: &str = "01a11834-2a51-71d1-ab30-e3d5dce2a335";

fn qualifier(
    kind: application::qualifiers::QualifierKind,
    text: String,
) -> application::qualifiers::KnowledgeQualifier {
    application::qualifiers::KnowledgeQualifier {
        kind,
        text,
        artifact_id: Some(UUID.to_string()),
    }
}

/// A pack shaped like the JevGuard dogfood: 16 decisions and 53 claims, half
/// of them scoped, each carrying one or two qualifiers with an artifact id.
fn jevguard_pack() -> application::context::ContextPack {
    use application::context::{ContextPack, PackClaim, PackDecision};
    use application::qualifiers::QualifierKind::{Scope, Validation};
    let filler = |label: &str, length: usize| -> String {
        let mut text = format!("{label} ");
        while text.chars().count() < length {
            text.push_str("palavra ");
        }
        text.chars().take(length).collect()
    };
    let qualifiers = |index: usize| {
        let scope_text = if index.is_multiple_of(11) {
            "*".to_string()
        } else {
            filler(&format!("escopo {index}"), 80)
        };
        let mut list = vec![qualifier(Scope, scope_text)];
        if index.is_multiple_of(2) {
            list.push(qualifier(
                Validation,
                filler(&format!("validado {index}"), 60),
            ));
        }
        list
    };
    let scoped = |index: usize| -> Vec<String> {
        if index.is_multiple_of(2) {
            vec!["packages/core".to_string()]
        } else {
            Vec::new()
        }
    };
    let decisions = (0..16)
        .map(|index| PackDecision {
            qualifiers: qualifiers(index),
            scope: scoped(index),
            decision_id: format!("01a11834-0000-7000-8000-{index:012}"),
            version: 1,
            question: filler(&format!("pergunta {index}"), 60),
            choice: filler(&format!("escolha {index}"), 40),
            rationale: filler(&format!("motivo {index}"), 120),
            confirmed_at: "2026-10-01T00:00:00Z".to_string(),
            evidence: vec![UUID.to_string()],
            depends_on: Vec::new(),
            conflicts_with: Vec::new(),
        })
        .collect();
    let claims = (0..53)
        .map(|index| PackClaim {
            source_version: Some(1),
            inherited_scope: scoped(index),
            qualifiers: qualifiers(index),
            claim_id: format!("01a11834-1111-7000-8000-{index:012}"),
            kind: "constraint".to_string(),
            statement: filler(&format!("regra {index}"), 100),
            valid_from: "2026-10-01T00:00:00Z".to_string(),
            valid_until: None,
            source_decision_id: None,
            matched: true,
        })
        .collect();
    ContextPack {
        observations: Vec::new(),
        observation_coverage: Default::default(),
        project_id: "p1".to_string(),
        task: "t".to_string(),
        as_of: "2026-10-14T00:00:00Z".to_string(),
        budget_chars: 8_000,
        used_chars: 0,
        decisions,
        claims,
        omitted: 0,
    }
}

/// Measured on 14/10/2026 over `jevguard_pack`: 2 items at 478 chars each with
/// the JSON qualifiers, 3 items at 350 chars with plain text.
const MIN_COMPACT_ITEMS: usize = 3;
const MAX_CHARS_PER_ITEM: usize = 360;

#[test]
fn compact_density_gate() {
    use application::injection::{render_compact, DEFAULT_BUDGET_TOKENS};
    let pack = jevguard_pack();
    let block = render_compact(&pack, DEFAULT_BUDGET_TOKENS, &Default::default()).expect("block");
    let lines: Vec<&str> = block.text.lines().filter(|l| !l.starts_with('<')).collect();
    let items = block.items.len();
    let chars: usize = lines.iter().map(|line| line.chars().count()).sum();
    let chars_per_item = chars.checked_div(items).unwrap_or(usize::MAX);
    println!(
        "gate compact items={items} chars_per_item={chars_per_item} omitted={}",
        block.omitted
    );
    for line in &lines {
        for forbidden in ["artifact_id", "qualifiers:", UUID] {
            assert!(!line.contains(forbidden), "{forbidden} leaked: {line}");
        }
    }
    // Every delivered item keeps the meaning of its qualifiers.
    for decision in &pack.decisions {
        let prefix = format!(
            "D:{}",
            application::injection::short_ref(&decision.decision_id)
        );
        if let Some(line) = lines.iter().find(|l| l.starts_with(&prefix)) {
            for q in decision.qualifiers.iter().filter(|q| q.text != "*") {
                assert!(line.contains(&q.text), "{prefix} lost {}", q.text);
            }
        }
    }
    for claim in &pack.claims {
        let prefix = format!(
            "regra:{}",
            application::injection::short_ref(&claim.claim_id)
        );
        if let Some(line) = lines.iter().find(|l| l.starts_with(&prefix)) {
            for q in claim.qualifiers.iter().filter(|q| q.text != "*") {
                assert!(line.contains(&q.text), "{prefix} lost {}", q.text);
            }
        }
    }
    assert!(items >= MIN_COMPACT_ITEMS, "only {items} items fit");
    assert!(
        chars_per_item <= MAX_CHARS_PER_ITEM,
        "{chars_per_item} chars per item"
    );
}
