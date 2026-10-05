//! Synthetic retrieval corpus (v3) with a quality gate; not a human evaluation.
//! Entry points: ContextPacks::build_pack and render_compact (not prepare).

#[path = "fixtures/context_corpus.rs"]
mod corpus;
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
        for (alias, project, question, choice) in [
            (
                "override",
                "p1",
                "Como aplicar Override na avaliação local?",
                "Preservar comportamento e contadores locais ao aplicar Override",
            ),
            (
                "cache",
                "p1",
                "Como fazer cache das respostas da API?",
                "Memória",
            ),
            (
                "storage",
                "p1",
                "Qual banco usar para persistência local?",
                "SQLite",
            ),
            (
                "password",
                "p1",
                "Como proteger senhas nos logs?",
                "Nunca registrar senhas",
            ),
            (
                "logging",
                "p1",
                "Como registrar logs da aplicação?",
                "JSON estruturado",
            ),
            (
                "foreign",
                "p2",
                "Como configurar sonar submarino?",
                "Ativar",
            ),
            (
                "superseded",
                "p1",
                "Qual banco de persistência?",
                "Postgres",
            ),
            (
                "outbox",
                "p1",
                "Como o hook entrega o turno ao app?",
                "Gravar um arquivo JSON na pasta outbox quando o app estiver fechado",
            ),
            (
                "transaction",
                "p1",
                "Como confirmar candidatos sem perder edições?",
                "Uma transação com validação de versão",
            ),
            (
                "retry",
                "p1",
                "Quando reenviar uma captura que falhou?",
                "Só falhas transitórias, mantendo a chave de idempotência",
            ),
            (
                "keychain",
                "p1",
                "Onde guardar a chave do provedor de IA?",
                "No cofre de credenciais do sistema operacional",
            ),
            (
                "migrations",
                "p1",
                "Como evoluir o esquema do banco?",
                "Migrações só para frente, numeradas, sem editar as antigas",
            ),
            (
                "redaction",
                "p1",
                "Como evitar que segredos cheguem ao provedor?",
                "Redigir padrões conhecidos de segredo antes de qualquer envio",
            ),
            (
                "budget",
                "p1",
                "Quanto contexto entregar ao agente?",
                "Um bloco de até 300 tokens por tarefa",
            ),
            (
                "virtual-list",
                "p1",
                "Como desenhar listas com milhares de itens?",
                "Lista virtual do GPUI, desenhando só o que está na tela",
            ),
            (
                "timestamps",
                "p1",
                "Em que formato guardar datas?",
                "RFC 3339 em UTC",
            ),
            (
                "release",
                "p1",
                "Como distribuir o app no Windows?",
                "ZIP com scripts de instalação, sem instalador MSI",
            ),
            (
                "mcp",
                "p1",
                "Como o agente consulta decisões sob demanda?",
                "Servidor MCP somente leitura via stdio",
            ),
            (
                "lanes",
                "p1",
                "Como evitar que documentos atrasem as capturas?",
                "Filas separadas por tipo de job, capturas primeiro",
            ),
            (
                "rate-limit",
                "p1",
                "O que fazer quando o provedor responde 429?",
                "Pausar as chamadas pelo Retry-After e devolver o job à fila",
            ),
            (
                "ci",
                "p1",
                "O que o CI precisa verificar?",
                "fmt, clippy com -D warnings e os testes do workspace",
            ),
            (
                "fonts",
                "p1",
                "Que fonte usar nos títulos?",
                "Bricolage Grotesque embutida no app",
            ),
            (
                "accent",
                "p1",
                "Qual cor de destaque usar na interface?",
                "Lavanda só para seleção, foco e ação primária",
            ),
            (
                "cache-invalidation",
                "p1",
                "Quando invalidar o cache semântico das observações?",
                "Quando a fonte muda de hash",
            ),
            (
                "log-retention",
                "p1",
                "Por quanto tempo manter os logs?",
                "Sete dias, sem conteúdo de conversas",
            ),
            (
                "pagination",
                "p1",
                "Como paginar respostas da API local?",
                "Cursor opaco por data de criação e id",
            ),
        ] {
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
        for (alias, kind, statement, until) in [
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
        ] {
            let id = Claims::new(test.store.clone())
                .create(NewClaim {
                    source_version: None,
                    qualifiers: Vec::new(),
                    project_id: "p1".into(),
                    kind,
                    statement: statement.into(),
                    valid_from: Some("2020-01-01".into()),
                    valid_until: until.map(str::to_string),
                    source_decision_id: None,
                })
                .expect("seed synthetic claim")
                .claim_id;
            aliases.insert(alias.to_string(), id);
        }
        let graph = KnowledgeGraph::new(test.store.clone());
        let entity = graph
            .create_entity(NewEntity {
                project_id: "p1".into(),
                kind: Some(EntityKind::Component),
                name: "storage".into(),
                patterns: vec!["crates/storage/**".into()],
                ..NewEntity::default()
            })
            .expect("seed component")
            .entity_id;
        graph
            .link(LinkRequest {
                kind: EdgeKind::Affects,
                source_kind: NodeKind::Decision,
                source_id: aliases["storage"].clone(),
                entity_id: entity,
            })
            .expect("seed file link");
        for (name, pattern, linked) in [
            ("outbox", "adapters/outbox/**", &["outbox"][..]),
            // Coarse on purpose: a whole-app component links unrelated UI
            // decisions, which a file-led task must not all receive.
            (
                "desktop-ui",
                "apps/desktop-gpui/**",
                &["virtual-list", "fonts", "accent"][..],
            ),
        ] {
            let component = graph
                .create_entity(NewEntity {
                    project_id: "p1".into(),
                    kind: Some(EntityKind::Component),
                    name: name.into(),
                    patterns: vec![pattern.into()],
                    ..NewEntity::default()
                })
                .expect("seed component")
                .entity_id;
            for alias in linked {
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
        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},\"{}\"",
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
        "standing,negative_items,estimated_tokens,oracle_misses,budget_misses,partial_related"
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
const PRECISION_FLOOR: f64 = 0.70;
const RECALL_FLOOR: f64 = 0.57;
const CONTAMINATED_CASES_CEILING: usize = 1;
/// Generous on purpose: this runs on a developer's machine beside other work.
const BUILD_PACK_P95_CEILING_US: u128 = 20_000;

#[test]
fn context_quality_gate() {
    let fixture = Fixture::seed("corpus-gate");
    let packs = ContextPacks::new(fixture.test.store.clone());
    let mut totals = Totals::default();
    let mut development = Totals::default();
    let mut holdout = Totals::default();
    let mut latency = Vec::new();
    for family in corpus::FAMILIES {
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
    let p95 = latency[latency.len() * 95 / 100];
    let precision = totals.tp as f64 / totals.retrieved.max(1) as f64;
    let recall = totals.tp as f64 / totals.expected.max(1) as f64;
    println!(
        "gate precision={precision:.4} recall={recall:.4} contaminated={}/{} p95_us={p95}",
        totals.contaminated, totals.negative_cases
    );
    for (name, split) in [("development", &development), ("holdout", &holdout)] {
        println!(
            "gate {name} precision={} recall={} contaminated={}/{}",
            ratio(split.tp, split.retrieved),
            ratio(split.tp, split.expected),
            split.contaminated,
            split.negative_cases
        );
    }
    assert!(
        precision >= PRECISION_FLOOR,
        "context precision fell to {precision:.4} (floor {PRECISION_FLOOR})"
    );
    assert!(
        recall >= RECALL_FLOOR,
        "context recall fell to {recall:.4} (floor {RECALL_FLOOR})"
    );
    assert!(
        totals.contaminated <= CONTAMINATED_CASES_CEILING,
        "{} negative cases received context (ceiling {CONTAMINATED_CASES_CEILING})",
        totals.contaminated
    );
    assert!(
        p95 <= BUILD_PACK_P95_CEILING_US,
        "build_pack p95 {p95}us above {BUILD_PACK_P95_CEILING_US}us"
    );
}
