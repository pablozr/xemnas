//! Opt-in, process-local sample data for evaluating the native workspace.
//! Never opens the user's database, starts workers, or contacts a provider.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::profile::{AiProfile, AiSettings, ProfileError, ProfileStore, SecretStore};
use application::projects::{ProjectRecord, ProjectRepository};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use storage_sqlite::SqliteStore;

/// Files the sample decisions touched, so the project map has something to
/// propose: two of them fall in the seeded `storage-sqlite` component.
const DEMO_FILES: [&str; 5] = [
    "crates/storage-sqlite/src/inbox.rs",
    "crates/ai-provider/src/lib.rs",
    "crates/application/src/jobs.rs",
    "adapters/opencode/src/index.ts",
    "crates/storage-sqlite/src/decisions.rs",
];

pub(crate) fn store() -> Result<SqliteStore, Box<dyn std::error::Error>> {
    let store = SqliteStore::open(":memory:")?;
    let long_evidence = std::env::args().any(|arg| arg == "--long-evidence");
    let samples = [
        ("Como garantir uma única decisão por captura?", "Confirmar candidatos em uma transação com validação de versão.", "A validação impede decisões duplicadas quando duas revisões acontecem ao mesmo tempo.", "transaction.begin();\nvalidate_version(candidate);\npersist_decision(candidate);\ntransaction.commit();"),
        ("Onde armazenar credenciais do provedor?", "Guardar a chave no cofre de credenciais do sistema.", "A chave fica fora do arquivo de configuração e dos logs.", "let secret = keystore.get(profile_id)?;\nprovider.configure(secret);"),
        ("Como recuperar jobs interrompidos?", "Reenfileirar apenas tarefas idempotentes após uma interrupção.", "Operações que não podem ser repetidas exigem tratamento explícito.", "if job.idempotent {\n    queue.requeue(job);\n}"),
        ("Quando reenviar uma captura?", "Repetir apenas falhas transitórias, preservando a chave de idempotência.", "O reenvio não pode gerar uma segunda captura da mesma mensagem.", "retry.with_backoff();\nrequest.idempotency_key(original_key);"),
        ("Como versionar decisões revisadas?", "Adicionar uma revisão e conservar o histórico anterior.", "O histórico permite entender a evolução da decisão.", "history.append(previous_revision);\ndecision.version += 1;"),
    ];
    for (project, location, count) in [
        ("demo-kpi", "C:/Projects/kpi-front", 1),
        ("demo-xemnas", "C:/Projects/xemnas", samples.len()),
    ] {
        ProjectRepository::insert(
            &store,
            &ProjectRecord::new(
                project.into(),
                location.into(),
                "2026-09-29T10:00:00Z".into(),
            ),
        )?;
        for (index, (question, choice, rationale, source)) in samples.iter().take(count).enumerate()
        {
            let capture = format!("{project}-capture-{index}");
            let timestamp = format!("2026-09-29T{:02}:00:00Z", 15 - index);
            let artifacts = [
                (
                    "source",
                    "diff_hunk",
                    if long_evidence {
                        (0..1500)
                            .map(|line| {
                                format!(
                                    "// linha {line:04}: {}\n",
                                    if line == 0 {
                                        "conteudo_extenso_".repeat(50)
                                    } else {
                                        "trecho ficticio para verificar rolagem".into()
                                    }
                                )
                            })
                            .collect()
                    } else {
                        (*source).to_owned()
                    },
                ),
                (
                    "context",
                    "user_text",
                    format!("Prévia fictícia: {rationale}"),
                ),
            ]
            .into_iter()
            .enumerate()
            .map(|(position, (id, kind, content))| CaptureArtifactRecord {
                capture_id: capture.clone(),
                artifact_id: id.into(),
                kind: kind.into(),
                content,
                metadata: if id == "source" {
                    format!(
                        r#"{{"file":"src/inbox.rs","language":"rust","start_line":{}}}"#,
                        24 + index * 10
                    )
                } else {
                    r#"{"file":"captura.txt"}"#.into()
                },
                fingerprint: format!("{:064x}", index * 2 + position + 1),
            })
            .collect();
            store.insert_capture(&CaptureWrite {
                receipt: CaptureReceiptRecord {
                    capture_id: capture.clone(),
                    idempotency_key: capture.clone(),
                    canonical_path: location.into(),
                    received_at: timestamp.clone(),
                    artifact_count: 2,
                },
                artifacts,
                job: JobRecord {
                    id: format!("job-{capture}"),
                    kind: ANALYZE_CAPTURE_KIND.into(),
                    payload: capture.clone(),
                    state: JobState::Completed,
                    idempotent: true,
                    attempts: 1,
                    last_error: None,
                    created_at: timestamp.clone(),
                    updated_at: timestamp.clone(),
                },
                checkpoint: CaptureCheckpointRecord {
                    adapter: "demo".into(),
                    adapter_version: "0.1.0".into(),
                    session_id: format!("session-{capture}"),
                    message_id: capture.clone(),
                    capture_id: capture.clone(),
                    observed_at: timestamp.clone(),
                    updated_at: timestamp.clone(),
                },
            })?;
            let mut candidate = DecisionCandidateRecord {
                id: format!("candidate-{capture}"),
                project_id: project.into(),
                capture_id: capture,
                status: "pending".into(),
                question: (*question).into(),
                choice: (*choice).into(),
                rationale: (*rationale).into(),
                signals: "[\"public_contract\"]".into(),
                confidence: 0.84,
                confidence_reason: "Exemplo fictício baseado na evidência exibida.".into(),
                evidence_refs: "[\"source\",\"context\"]".into(),
                diff_summary: format!(
                    "{{\"files\":[\"{}\"],\"artifacts\":2}}",
                    DEMO_FILES[index % DEMO_FILES.len()]
                ),
                dedup_hash: format!("demo-{project}-{index}"),
                created_at: timestamp.clone(),
                updated_at: timestamp,
                kind: "decision".to_string(),
                significance: 1.0,
                criteria: "[]".to_string(),
            };
            store.insert_candidates(&[candidate.clone()])?;
            candidate.id = format!("confirmed-{}", candidate.id);
            candidate.dedup_hash = format!("confirmed-{}", candidate.dedup_hash);
            store.insert_candidates(&[candidate.clone()])?;
            let promoted =
                application::inbox::Inbox::new(store.clone()).confirm(&candidate.id, None)?;
            let decisions = application::decisions::Decisions::new(store.clone());
            decisions.revise(
                &promoted.decision_id,
                application::decisions::DecisionEdits {
                    assumptions: Some(vec![
                        "A captura preserva o conteúdo redigido da fonte.".into()
                    ]),
                    scope: Some(vec!["Aplicação desktop e camada de persistência.".into()]),
                    consequences: Some(vec![
                        "A escolha permanece consultável mesmo após outras revisões.".into(),
                    ]),
                    reconsider_when: Some(vec![
                        "Os requisitos de concorrência ou retenção mudarem.".into(),
                    ]),
                    ..Default::default()
                },
            )?;
            if index == 0 {
                decisions.revise(&promoted.decision_id, application::decisions::DecisionEdits {
                    rationale: Some(format!("{rationale}\n\nCada alteração conserva a versão anterior e as fontes usadas para tomar a decisão.")),
                    ..Default::default()
                })?;
            }
        }
    }
    seed_map(&store)?;
    Ok(store)
}

/// A project map for the demo: components with parts, technologies, rules,
/// confirmed links, open suggestions and one stored Visão, so every Mapa and
/// Visão surface has something real to draw.
fn seed_map(store: &SqliteStore) -> Result<(), Box<dyn std::error::Error>> {
    use application::claims::{Claims, NewClaim};
    use application::decisions::{DecisionFilter, Decisions};
    use application::graph::{KnowledgeGraph, LinkRequest, NewEntity};
    use application::injection::short_ref;
    use application::overview::{
        Citation, OverviewFlow, OverviewParagraph, OverviewStep, OverviewStore, ProjectOverview,
    };
    use domain::claims::ClaimKind;
    use domain::entities::{EdgeKind, EntityKind, NodeKind};

    const PROJECT: &str = "demo-xemnas";
    let graph = KnowledgeGraph::new(store.clone());
    let mut ids = HashMap::new();
    for (kind, name, pattern, description) in [
        (
            EntityKind::Component,
            "storage-sqlite",
            "crates/storage-sqlite/**",
            "Banco local: decisões, capturas e o mapa.",
        ),
        (
            EntityKind::Component,
            "application",
            "crates/application/**",
            "Casos de uso e portas.",
        ),
        (
            EntityKind::Component,
            "jobs",
            "crates/application/src/jobs.rs",
            "Fila local de análises, com recuperação.",
        ),
        (
            EntityKind::Component,
            "inbox",
            "crates/storage-sqlite/src/inbox.rs",
            "Candidatos aguardando revisão.",
        ),
        (
            EntityKind::Component,
            "ai-provider",
            "crates/ai-provider/**",
            "Chamadas ao provedor de IA com consentimento.",
        ),
        (EntityKind::Technology, "SQLite", "", "Banco embutido."),
        (
            EntityKind::Technology,
            "Cofre do Windows",
            "",
            "Credenciais do sistema.",
        ),
        (EntityKind::Technology, "Rust", "", "Linguagem do app."),
    ] {
        let entity = graph.create_entity(NewEntity {
            project_id: PROJECT.into(),
            kind: Some(kind),
            name: name.into(),
            description: description.into(),
            patterns: if pattern.is_empty() {
                Vec::new()
            } else {
                vec![pattern.into()]
            },
            ..NewEntity::default()
        })?;
        ids.insert(name, entity.entity_id);
    }
    for (part, whole) in [("jobs", "application"), ("inbox", "storage-sqlite")] {
        graph.link(LinkRequest {
            kind: EdgeKind::PartOf,
            source_kind: NodeKind::Entity,
            source_id: ids[part].clone(),
            entity_id: ids[whole].clone(),
        })?;
    }

    let decisions = Decisions::new(store.clone())
        .list(&DecisionFilter {
            project_id: Some(PROJECT.into()),
            ..DecisionFilter::default()
        })?
        .decisions;
    let decision = |start: &str| {
        decisions
            .iter()
            .find(|row| row.question.starts_with(start))
            .map(|row| row.decision_id.clone())
            .ok_or_else(|| format!("demo decision {start}"))
    };
    let unique = decision("Como garantir")?;
    let secrets = decision("Onde armazenar")?;
    let recover = decision("Como recuperar")?;
    let resend = decision("Quando reenviar")?;
    let revise = decision("Como versionar")?;
    for (kind, source, target) in [
        (EdgeKind::Affects, &unique, "storage-sqlite"),
        (EdgeKind::Uses, &unique, "SQLite"),
        (EdgeKind::Affects, &secrets, "ai-provider"),
        (EdgeKind::Uses, &secrets, "Cofre do Windows"),
        (EdgeKind::Affects, &recover, "jobs"),
        (EdgeKind::Affects, &revise, "storage-sqlite"),
    ] {
        graph.link(LinkRequest {
            kind,
            source_kind: NodeKind::Decision,
            source_id: source.clone(),
            entity_id: ids[target].clone(),
        })?;
    }

    let claims = Claims::new(store.clone());
    let mut rules = Vec::new();
    let mut statements = HashMap::new();
    for (kind, statement, target) in [
        (
            ClaimKind::Constraint,
            "Nenhuma credencial sai do cofre do sistema.",
            "ai-provider",
        ),
        (
            ClaimKind::Convention,
            "Toda escrita de decisão acontece numa transação.",
            "storage-sqlite",
        ),
    ] {
        let claim = claims.create(NewClaim {
            project_id: PROJECT.into(),
            kind,
            statement: statement.into(),
            valid_from: Some("2026-09-01".into()),
            valid_until: None,
            source_decision_id: None,
        })?;
        graph.link(LinkRequest {
            kind: EdgeKind::AppliesTo,
            source_kind: NodeKind::Claim,
            source_id: claim.claim_id.clone(),
            entity_id: ids[target].clone(),
        })?;
        statements.insert(claim.claim_id.clone(), statement.to_owned());
        rules.push(claim.claim_id);
    }
    // The remaining decision stays a suggestion for the Sugestões view.
    graph.refresh_suggestions(PROJECT)?;
    let _ = resend;

    // Sample documentation, as if read from the project folder.
    use application::documents::{document_ref, DocumentKind, DocumentStore, ProjectDocument};
    let documents = [
        (
            "README.md",
            DocumentKind::Readme,
            "xemnas",
            vec!["Instalação", "Como funciona", "Privacidade"],
            "Memória de decisões de engenharia para agentes de código, local e revisada.",
        ),
        (
            "docs/arquitetura/adr/0005-grafo-de-entidades.md",
            DocumentKind::Adr,
            "ADR-0005: Grafo de entidades",
            vec!["Decisão", "Consequências", "Alternativas rejeitadas"],
            "Componentes e tecnologias ligam as decisões às partes do código.",
        ),
        (
            "docs/arquitetura/adr/0006-tipo-e-relevancia-dos-candidatos.md",
            DocumentKind::Adr,
            "ADR-0006: Tipo e relevância dos candidatos",
            vec!["Decisão", "Consequências"],
            "Cada proposta traz tipo, relevância e os critérios que a justificam.",
        ),
        (
            "docs/specs/captura.md",
            DocumentKind::Spec,
            "Especificação da captura",
            vec!["Envelope", "Idempotência", "Limites"],
            "Como uma resposta do agente chega à API local e vira captura.",
        ),
        (
            "docs/design/VISUAL-IDENTITY.md",
            DocumentKind::Guide,
            "Quiet Glass",
            vec!["Hierarquia e material", "Mapa", "Gate de entrega visual"],
            "A regra visual do app desktop.",
        ),
    ]
    .into_iter()
    .map(|(path, kind, title, headings, excerpt)| ProjectDocument {
        project_id: PROJECT.into(),
        path: path.into(),
        kind,
        title: title.into(),
        headings: headings.into_iter().map(str::to_owned).collect(),
        excerpt: excerpt.into(),
        bytes: 4096,
        fingerprint: document_ref(path),
        indexed_at: "2026-09-29T18:00:00Z".into(),
    })
    .collect::<Vec<_>>();
    store.replace_documents(PROJECT, &documents)?;
    let doc = |path: &str| Citation {
        kind: "document".into(),
        id: path.into(),
        label: format!("F:{}", document_ref(path)),
        title: documents
            .iter()
            .find(|document| document.path == path)
            .map(|document| document.title.clone())
            .unwrap_or_default(),
    };

    let cite = |id: &String| Citation {
        kind: "decision".into(),
        id: id.clone(),
        label: format!("D:{}", short_ref(id)),
        title: decisions
            .iter()
            .find(|row| &row.decision_id == id)
            .map(|row| row.question.clone())
            .unwrap_or_default(),
    };
    let rule = |id: &String| Citation {
        kind: "claim".into(),
        id: id.clone(),
        label: format!("R:{}", short_ref(id)),
        title: statements.get(id).cloned().unwrap_or_default(),
    };
    let step = |title: &str, text: &str, entity: &str, citations: Vec<Citation>| OverviewStep {
        title: title.into(),
        text: text.into(),
        entity_id: ids.get(entity).cloned(),
        entity_name: ids.get(entity).map(|_| entity.to_owned()),
        citations,
    };
    store.save_overview(&ProjectOverview {
        project_id: PROJECT.into(),
        generated_at: "2026-09-29T18:00:00Z".into(),
        decisions: decisions.len(),
        rules: rules.len(),
        documents: documents.len(),
        summary: vec![
            OverviewParagraph {
                text: "O xemnas guarda localmente as decisões de engenharia tiradas das \
                       conversas com o agente: cada captura vira candidatos, que só entram \
                       no projeto depois de revisados."
                    .into(),
                citations: vec![cite(&unique), cite(&revise), doc("README.md")],
            },
            OverviewParagraph {
                text: "A análise roda numa fila local que sobrevive a interrupções, e as \
                       chamadas ao provedor de IA usam credenciais que nunca saem do cofre \
                       do sistema."
                    .into(),
                citations: vec![cite(&recover), cite(&secrets), rule(&rules[0])],
            },
        ],
        flows: vec![
            OverviewFlow {
                title: "Da conversa à decisão".into(),
                description: "Como uma resposta do agente vira uma decisão confirmada.".into(),
                steps: vec![
                    step(
                        "Captura enfileirada",
                        "A captura chega pela API local e entra na fila com chave de \
                         idempotência.",
                        "jobs",
                        vec![cite(&recover)],
                    ),
                    step(
                        "Análise pelo provedor",
                        "O provedor configurado propõe candidatos a partir da evidência.",
                        "ai-provider",
                        vec![cite(&secrets)],
                    ),
                    step(
                        "Revisão",
                        "Os candidatos esperam na Revisão até serem confirmados ou \
                         rejeitados.",
                        "inbox",
                        vec![cite(&unique)],
                    ),
                    step(
                        "Gravação",
                        "A confirmação grava a decisão numa transação, com validação de \
                         versão.",
                        "storage-sqlite",
                        vec![cite(&unique), rule(&rules[1])],
                    ),
                ],
            },
            OverviewFlow {
                title: "Revisar uma decisão".into(),
                description: "Como uma decisão muda sem perder o histórico.".into(),
                steps: vec![
                    step(
                        "Nova revisão",
                        "Editar cria uma revisão e conserva a anterior.",
                        "storage-sqlite",
                        vec![cite(&revise)],
                    ),
                    step(
                        "Consulta",
                        "O histórico continua consultável em Decisões.",
                        "application",
                        vec![cite(&revise)],
                    ),
                ],
            },
            OverviewFlow {
                title: "Recuperar jobs".into(),
                description: "O que acontece quando o app fecha no meio de uma análise.".into(),
                steps: vec![
                    step(
                        "Interrupção",
                        "Jobs em andamento voltam para a fila ao reabrir.",
                        "jobs",
                        vec![cite(&recover)],
                    ),
                    step(
                        "Reenvio seguro",
                        "Só tarefas idempotentes são repetidas.",
                        "jobs",
                        vec![cite(&recover)],
                    ),
                ],
            },
        ],
    })?;
    Ok(())
}

/// In-memory AI settings for the demo: never touches the profile file or the
/// OS key vault, and starts from the offline default.
/// Sample model lists for the demo's model picker; nothing is fetched.
pub(crate) struct SampleCatalog;

impl application::providers::ModelCatalog for SampleCatalog {
    fn list(
        &self,
        profile: &AiProfile,
        _secret: Option<String>,
    ) -> Result<Vec<application::providers::ModelInfo>, application::providers::ProviderError> {
        use application::profile::ProfileKind;
        let sample: &[(&str, &str)] = match profile.kind {
            ProfileKind::Fake => &[],
            ProfileKind::OpenAiCompatible => &[
                ("llama3.2", "llama3.2"),
                ("qwen3:8b", "qwen3:8b"),
                ("gemma3:12b", "gemma3:12b"),
            ],
            ProfileKind::ChatGptPlan => &[("gpt-demo", "GPT (demonstração)")],
            ProfileKind::OpenCode => &[
                ("kimi-k3", "kimi-k3"),
                ("deepseek-v4-pro", "deepseek-v4-pro"),
                ("big-pickle", "big-pickle"),
            ],
        };
        Ok(sample
            .iter()
            .map(|(id, label)| application::providers::ModelInfo {
                id: (*id).to_owned(),
                label: (*label).to_owned(),
            })
            .collect())
    }
}

pub(crate) fn ai_settings() -> AiSettings<MemoryProfile, MemorySecrets> {
    AiSettings::new(MemoryProfile::default(), MemorySecrets::default())
}

#[derive(Default, Clone)]
pub(crate) struct MemoryProfile(Arc<Mutex<Option<AiProfile>>>);

impl ProfileStore for MemoryProfile {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(self.0.lock().map_err(poisoned)?.clone())
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        profile.validate()?;
        *self.0.lock().map_err(poisoned)? = Some(profile.clone());
        Ok(())
    }
}

#[derive(Default, Clone)]
pub(crate) struct MemorySecrets(Arc<Mutex<HashMap<String, String>>>);

impl SecretStore for MemorySecrets {
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
        self.0
            .lock()
            .map_err(poisoned)?
            .insert(account.to_owned(), secret.to_owned());
        Ok(())
    }
    fn get_secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
        Ok(self.0.lock().map_err(poisoned)?.get(account).cloned())
    }
    fn delete_secret(&self, account: &str) -> Result<(), ProfileError> {
        self.0.lock().map_err(poisoned)?.remove(account);
        Ok(())
    }
}

fn poisoned<T>(_: std::sync::PoisonError<T>) -> ProfileError {
    ProfileError::Io("demo settings lock poisoned".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use application::inbox::{Inbox, InboxFilter};

    #[test]
    fn demo_is_ephemeral_and_project_scoped_with_readable_sources() {
        let store = store().expect("demo store");
        let inbox = Inbox::new(store.clone());
        for (project, count) in [("demo-xemnas", 5), ("demo-kpi", 1)] {
            let page = inbox
                .list(&InboxFilter {
                    project_id: Some(project.into()),
                    ..InboxFilter::default()
                })
                .expect("list");
            assert_eq!(page.candidates.len(), count);
            assert!(page.candidates.iter().all(|row| row.project_id == project));
            let detail = inbox.detail(&page.candidates[0].id).expect("detail");
            assert_eq!(detail.artifacts.len(), 2);
            assert!(detail
                .artifacts
                .iter()
                .all(|artifact| !artifact.content.is_empty()));
        }
        let graph = application::graph::KnowledgeGraph::new(store.clone());
        let map = graph.project_map("demo-xemnas", None).expect("map");
        assert_eq!(map.entities.len(), 8, "components and technologies");
        assert!(
            map.entities.iter().any(|row| row.decisions == 2),
            "confirmed links"
        );
        let report = graph
            .refresh_suggestions("demo-xemnas")
            .expect("suggestions");
        assert!(
            !report.components.is_empty(),
            "the other sample files propose components"
        );
        assert!(
            ProjectRepository::list(&SqliteStore::open(":memory:").expect("fresh store"))
                .expect("projects")
                .is_empty()
        );
    }
}
