//! Disposable preparation only; no subjects, scores, live API or production edits.
use application::agent_access::AgentAccess;
use application::decisions::{DecisionEdits, Decisions};
use application::export::{Export, ExportFormat};
use application::observations::reader::LocalObservationReader;
use application::observations::refresh::refresh_project_with_metrics;
use application::observations::*;
use application::qualifiers::{KnowledgeQualifier, QualifierKind};
use application::serde_json::{json, Map};
use application::{
    artifact_fingerprint, canonicalize_location, run_extraction, ArtifactKind, CandidateEdits,
    CaptureApi, CaptureEnvelope, CaptureIngest, CaptureSource, FakeCandidateExtractor, Inbox,
    InboxFilter, ProjectRef, Projects, RunContext, SourceArtifact,
};
use std::{path::Path, time::Instant};
use storage_sqlite::SqliteStore;

const ROOT: &str = r"C:\Users\Pablo\AppData\Local\Temp\opencode\xemnas-comparison-20261004";
const PROTOCOL: &str =
    include_str!("../../../docs/operacao/experimento-produtividade-assertividade.md");

fn block(after: &str) -> &'static str {
    let tail = PROTOCOL.split_once(after).unwrap().1;
    let fenced = tail.split_once("```").unwrap().1;
    fenced
        .split_once('\n')
        .unwrap()
        .1
        .split_once("```")
        .unwrap()
        .0
}

fn write(root: &Path, name: &str, bytes: impl AsRef<[u8]>) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

fn seed(store: &SqliteStore, root: &Path, tag: &str, source: &str, choice: &str) -> String {
    const TRIGGER: &str =
        "diff --git a/src/schema.rs b/src/schema.rs\n+CREATE TABLE t (id TEXT);\n";
    let location = canonicalize_location(&root.to_string_lossy()).unwrap();
    let envelope = CaptureEnvelope {
        schema_version: 1,
        capture_id: format!("lab-{tag}"),
        idempotency_key: format!("lab-{tag}"),
        source: CaptureSource {
            adapter: "laboratorial-nao-humana".into(),
            adapter_version: "1".into(),
            session_id: tag.into(),
            message_id: tag.into(),
        },
        project: ProjectRef {
            canonical_path: location,
        },
        observed_at: "2026-10-04T09:55:00Z".into(),
        artifacts: vec![
            SourceArtifact {
                artifact_id: format!("source-{tag}"),
                kind: ArtifactKind::UserText,
                content: source.into(),
                metadata: Map::new(),
                fingerprint: artifact_fingerprint(source),
            },
            SourceArtifact {
                artifact_id: format!("lab-diff-{tag}"),
                kind: ArtifactKind::DiffHunk,
                content: TRIGGER.into(),
                metadata: Map::new(),
                fingerprint: artifact_fingerprint(TRIGGER),
            },
        ],
    };
    let receipt = CaptureIngest::new(store.clone())
        .ingest(&envelope, &envelope.idempotency_key)
        .unwrap();
    run_extraction(
        store,
        &FakeCandidateExtractor,
        &receipt.receipt.capture_id,
        &RunContext::for_tests(),
    )
    .unwrap();
    let inbox = Inbox::new(store.clone());
    let candidate = inbox
        .list(&InboxFilter::new())
        .unwrap()
        .candidates
        .into_iter()
        .find(|c| c.status == application::CandidateStatus::Pending)
        .unwrap();
    let adopted = inbox
        .confirm(
            &candidate.id,
            Some(CandidateEdits {
                qualifiers: Vec::new(),
                question: tag.into(),
                choice: choice.into(),
                rationale: source.into(),
            }),
        )
        .unwrap();
    Decisions::new(store.clone())
        .revise(
            &adopted.decision_id,
            DecisionEdits {
                scope: Some(vec![if tag == "contadores" {
                    "avaliador-local-ripgrep"
                } else {
                    "RelayDesk"
                }
                .into()]),
                qualifiers: Some(vec![KnowledgeQualifier {
                    kind: QualifierKind::Attribution,
                    text: "Adoção laboratorial por script, não revisão humana".into(),
                    artifact_id: None,
                }]),
                ..DecisionEdits::default()
            },
        )
        .unwrap();
    adopted.decision_id
}

#[test]
fn corpus_blocks_are_literal_and_have_no_questions() {
    assert!(block("`fontes/e1-persistencia.txt`").contains("Usar SQLite embutido"));
    assert!(block("`servico/Cargo.toml` inicial").contains("serde = \"1\""));
    assert!(!block("`fontes/e3-contadores.txt`").contains("T3"));
}

#[test]
#[ignore = "creates persistent disposable materials in the explicitly approved root"]
fn prepare_comparative_materials() {
    let start = Instant::now();
    let root = Path::new(ROOT);
    assert!(root.parent().unwrap().is_dir());
    if root.exists() {
        assert!(!root.join("observer/metadata.json").exists());
        assert!(
            !root.join("N").exists(),
            "refuse overwrite subject materials"
        );
        let failed = format!(
            "failed-attempt-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::fs::rename(root.join("observer"), root.join(failed)).unwrap();
    } else {
        std::fs::create_dir(root).unwrap();
    }
    write(
        root,
        "observer/preregistration.json",
        json!({
            "arm_x":"EXPORTSNAPSHOT", "registered_before_subjects":true,
            "deviation":"raw/ADR/Xemnas exported retrieval; not live API protocol",
            "subjects":"not_run", "SAC_cap":3, "human_active_ms":null,
            "product_provider_calls":0, "routing":"default offline, no provider or judge",
            "adoption":"laboratorial, not human", "productivity_claim":false
        })
        .to_string(),
    );
    let common = [
        ("README.md", block("`README.md`:")),
        ("src/storage.rs", block("`src/storage.rs`")),
        (
            "fontes/e1-persistencia.txt",
            block("`fontes/e1-persistencia.txt`"),
        ),
        ("fontes/e2-busca.txt", block("`fontes/e2-busca.txt`")),
        (
            "fontes/e3-contadores.txt",
            block("`fontes/e3-contadores.txt`"),
        ),
        ("fontes/e4-terminal.txt", block("`fontes/e4-terminal.txt`")),
        ("Cargo.toml", block("`Cargo.toml` da raiz comum")),
        ("servico/src/lib.rs", ""),
    ];
    let v1 = block("`servico/Cargo.toml` inicial");
    let v2 = v1.replace("serde = \"1\"", "serde = \"2\"");
    let initial_note = block("`fontes/e5-verificacao.txt` começa");
    let updated_note = block("Acrescentar estes bytes");
    let unknown_note = block("Antes de T6, tornar");
    let work = root.join("observer/reader-project");
    for (name, bytes) in common {
        write(&work, name, bytes);
    }
    let local = root.join("observer/avaliador-local-ripgrep");
    std::fs::create_dir_all(&local).unwrap();
    let store = SqliteStore::open(root.join("observer/lab.sqlite")).unwrap();
    let project = Projects::new(store.clone()).register(&work).unwrap();
    Projects::new(store.clone()).register(&local).unwrap();
    let ids = [
        seed(
            &store,
            &work,
            "persistencia-anterior",
            common[2].1,
            "Usar PostgreSQL com servidor separado para a fila local.",
        ),
        seed(
            &store,
            &work,
            "persistencia",
            common[2].1,
            "Usar SQLite embutido, sem serviço de banco separado.",
        ),
        seed(
            &store,
            &work,
            "busca",
            common[3].1,
            "Usar FTS5 do SQLite para busca lexical dos registros.",
        ),
        seed(
            &store,
            &local,
            "contadores",
            common[4].1,
            "Preservar a delegação cruzada dos contadores Override em relação a Gitignore.",
        ),
    ];
    application::relations::DecisionRelations::new(store.clone())
        .supersede(&ids[1], &ids[0])
        .unwrap();
    let mut hashes = Vec::new();
    let mut historical = Vec::new();
    for (stage, manifest, notes) in [
        ("initial", Some(v1), initial_note.to_string()),
        (
            "updated",
            Some(v2.as_str()),
            format!("{initial_note}{updated_note}"),
        ),
        (
            "unknown",
            None,
            format!("{initial_note}{updated_note}{unknown_note}"),
        ),
    ] {
        if let Some(bytes) = manifest {
            write(&work, "servico/Cargo.toml", bytes);
        } else {
            std::fs::rename(
                work.join("servico/Cargo.toml"),
                root.join("observer/last-manifest.toml"),
            )
            .unwrap();
            std::fs::create_dir(work.join("servico/Cargo.toml")).unwrap();
        }
        let generation = store
            .request_refresh(&RefreshRequest {
                project_id: project.id().to_string(),
                capture_trigger: "comparison-lab".into(),
                requested_at: "2026-10-04T10:05:00Z".into(),
            })
            .unwrap()
            .unwrap();
        let (applied, _) = refresh_project_with_metrics(
            &store,
            &LocalObservationReader,
            project.id().as_str(),
            &work.to_string_lossy(),
            generation.generation,
            "comparison-lab",
        )
        .unwrap();
        assert_eq!(applied, ApplyRefreshResult::Applied);
        let snapshot = store.snapshot(project.id().as_str()).unwrap();
        assert_eq!(snapshot.coverage.unknown, stage == "unknown");
        let serde = snapshot.observations.iter().find(|o|
            matches!(&o.subject, ObservationSubject::Dependency { name } if name == "serde")).unwrap();
        assert_eq!(
            serde.value.version_requirement.as_deref(),
            Some(if stage == "initial" { "1" } else { "2" })
        );
        historical.push(
            json!({"stage":stage,"observations":snapshot.observations,"sources":snapshot.sources}),
        );
        for condition in ["N", "A", "X"] {
            let subject = root.join(condition).join(stage);
            for (name, bytes) in common {
                write(&subject, name, bytes);
                let actual = std::fs::read(subject.join(name)).unwrap();
                assert_eq!(actual, bytes.as_bytes());
                hashes.push(json!({"condition":condition,"stage":stage,"file":name,
                    "sha256":artifact_fingerprint(bytes),"bytes":actual.len()}));
            }
            write(&subject, "fontes/e5-verificacao.txt", &notes);
            if let Some(bytes) = manifest {
                write(&subject, "servico/Cargo.toml", bytes);
            } else {
                std::fs::create_dir(subject.join("servico/Cargo.toml")).unwrap();
            }
            for (name, bytes) in [
                ("fontes/e5-verificacao.txt", notes.as_str()),
                ("servico/Cargo.toml", manifest.unwrap_or("")),
            ] {
                hashes.push(json!({"condition":condition,"stage":stage,"file":name,
                    "sha256":artifact_fingerprint(bytes),"bytes":bytes.len(),
                    "unavailable_directory":manifest.is_none() && name == "servico/Cargo.toml"}));
            }
            if condition == "A" {
                write(&subject, "memoria.md", format!(
                    "# Memória\n## ADR-001 substituído por ADR-002 — E1\n{}\n\
                     ## ADR-002 vigente — E1\n{}\n## ADR-003 vigente, primeira versão — E2\n{}\n\
                     ## ADR-004 local ao avaliador-ripgrep, não RelayDesk — E3\n{}\n\
                     ## O-001 descritiva — servico/Cargo.toml, dependencies.serde, runtime\n\
                     Registro {stage}; histórico de verificações, não norma nem instalação:\n{notes}",
                    common[2].1, common[2].1, common[3].1, common[4].1));
            }
            if condition != "X" {
                continue;
            }
            let access = AgentAccess::new(store.clone());
            let mut index = String::from(
                "# EXPORTSNAPSHOT somente leitura\n\
                Não há endpoint live. Retornos reais offline, adoção laboratorial não humana.\n\
                O texto padrão da API sobre confirmação não significa revisão humana aqui.\n\
                Evidence: fontes/; exportação completa disponível mesmo se busca retornar null.\n",
            );
            for query in [
                "persistencia",
                "busca",
                "contadores",
                "serde",
                "retenção terminal",
            ] {
                for (scope, directory) in [("RelayDesk", &work), ("avaliador-local", &local)] {
                    let result = access
                        .search(&directory.to_string_lossy(), query, Some(2000), None)
                        .unwrap();
                    let filename =
                        format!("lookup/search-{scope}-{}.json", query.replace(' ', "-"));
                    write(
                        &subject,
                        &filename,
                        json!({"api":"AgentAccess.search",
                        "scope":scope,"query":query,"result":result})
                        .to_string(),
                    );
                    index.push_str(&format!("search({query:?}), scope={scope}: {filename}\n"));
                }
            }
            for (i, id) in ids.iter().enumerate() {
                let directory = if i == 3 { &local } else { &work };
                let detail = access.decision(&directory.to_string_lossy(), id).unwrap();
                write(&subject, &format!("lookup/decision-{id}.txt"), detail);
                let export = Export::new(store.clone())
                    .preview(id, ExportFormat::Json)
                    .unwrap();
                let value: application::serde_json::Value =
                    application::serde_json::from_str(&export.content).unwrap();
                assert!(!value["evidence"].as_array().unwrap().is_empty());
                write(
                    &subject,
                    &format!("lookup/export-{id}.json"),
                    export.content,
                );
                index.push_str(&format!(
                    "decision({id:?}): lookup/decision-{id}.txt; lookup/export-{id}.json\n"
                ));
            }
            write(
                &subject,
                "lookup/observations-history.json",
                json!(historical).to_string(),
            );
            let rendered = access
                .file_context(&work.to_string_lossy(), "servico/Cargo.toml", Some(2000))
                .unwrap();
            write(
                &subject,
                "lookup/file-context.json",
                json!({"api":"AgentAccess.file_context",
                "path":"servico/Cargo.toml","result":rendered})
                .to_string(),
            );
            index.push_str(
                "file_context(servico/Cargo.toml): lookup/file-context.json\n\
                ObservationStore.snapshot history/state/hash: lookup/observations-history.json\n",
            );
            write(&subject, "lookup/INDEX.md", index);
        }
    }
    write(
        root,
        "observer/metadata.json",
        json!({"ids":ids,"hashes":hashes,
        "preparation_elapsed_ns":start.elapsed().as_nanos().to_string(),
        "human_active_ms":null,"units":"4 decisions + descriptive serde history",
        "equivalence":"literal E1/E2/E3 in ADR and decision rationale; common E5 notes/history",
        "coverage":"reader initial/updated verified; unknown retained last verified",
        "estimated_bytes":"file byte lengths recorded, not token or billing estimates",
        "subjects":"not_run","SAC_cap":3})
        .to_string(),
    );
    println!("PREPARED {ROOT}; 3 conditions x 3 checkpoints; subjects not run");
}
