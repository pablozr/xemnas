//! Opt-in, process-local sample data for evaluating the native workspace.
//! Never opens the user's database, starts workers, or contacts a provider.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

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
                    session_id: format!("session-{capture}"),
                    message_id: capture.clone(),
                    capture_id: capture.clone(),
                    observed_at: timestamp.clone(),
                    updated_at: timestamp.clone(),
                },
            })?;
            store.insert_candidates(&[DecisionCandidateRecord {
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
                diff_summary: "{\"files\":[\"src/inbox.rs\"],\"artifacts\":2}".into(),
                dedup_hash: format!("demo-{project}-{index}"),
                created_at: timestamp.clone(),
                updated_at: timestamp,
            }])?;
        }
    }
    Ok(store)
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
        assert!(
            ProjectRepository::list(&SqliteStore::open(":memory:").expect("fresh store"))
                .expect("projects")
                .is_empty()
        );
    }
}
