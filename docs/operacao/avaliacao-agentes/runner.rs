use std::{path::PathBuf, sync::Arc};
use application::{artifact_fingerprint, canonicalize_location, run_extraction, ArtifactKind,
    CandidateEdits, CaptureApi, CaptureEnvelope, CaptureIngest, CaptureSource,
    FakeCandidateExtractor, Inbox, InboxFilter, ProjectRef, Projects, RunContext, SourceArtifact};
use application::serde_json::{self, json, Map};
use storage_sqlite::SqliteStore;

fn seed(store: &SqliteStore, location: &str, tag: &str, question: &str, choice: &str,
        reason: &str) -> String {
    let diff = "diff --git a/src/schema.rs b/src/schema.rs\n+CREATE TABLE queue (id TEXT);\n";
    let note = format!("Decidimos: {choice}. Motivo: {reason}");
    let envelope = CaptureEnvelope {
        schema_version: 1,
        capture_id: format!("controlled-{tag}"),
        idempotency_key: format!("controlled-key-{tag}"),
        source: CaptureSource { adapter: "controlled-experiment".into(),
            adapter_version: "1".into(), session_id: tag.into(), message_id: tag.into() },
        project: ProjectRef { canonical_path: location.into() },
        observed_at: "2026-10-03T12:00:00Z".into(),
        artifacts: vec![SourceArtifact {
            artifact_id: format!("diff-{tag}"), kind: ArtifactKind::DiffHunk,
            content: diff.into(), metadata: Map::new(), fingerprint: artifact_fingerprint(diff)
        }, SourceArtifact {
            artifact_id: format!("note-{tag}"), kind: ArtifactKind::UserText,
            content: note.clone(), metadata: Map::new(), fingerprint: artifact_fingerprint(&note)
        }],
    };
    let receipt = CaptureIngest::new(store.clone()).ingest(&envelope, &envelope.idempotency_key)
        .expect("controlled ingest");
    assert!(!receipt.replayed);
    let replay = CaptureIngest::new(store.clone()).ingest(&envelope, &envelope.idempotency_key)
        .expect("replay");
    assert!(replay.replayed);
    run_extraction(store, &FakeCandidateExtractor, &receipt.receipt.capture_id,
        &RunContext::for_tests()).expect("offline extraction");
    let inbox = Inbox::new(store.clone());
    let candidate = inbox.list(&InboxFilter::new()).expect("inbox").candidates.into_iter()
        .find(|c| c.status == application::CandidateStatus::Pending).expect("pending");
    println!("{}", json!({"stage":"extraction", "tag":tag,
        "question":candidate.question, "candidate_id":candidate.id}));
    let adopted = inbox.confirm(&candidate.id, Some(CandidateEdits {
        question: question.into(), choice: choice.into(), rationale: reason.into()
    })).expect("controlled confirmation; NOT human UI review");
    adopted.decision_id
}

fn main() {
    let root = PathBuf::from(std::env::args().nth(1).expect("experiment directory"));
    let project = root.join("scenario");
    let paths = application::AppPaths::under(root.join("data"), None);
    std::fs::create_dir_all(&paths.runtime_dir).expect("runtime");
    let store = SqliteStore::open(&paths.database).expect("SQLite");
    let location = canonicalize_location(&project.to_string_lossy()).expect("canonical");

    if std::env::args().nth(2).as_deref() != Some("reopen") {
        Projects::new(store.clone()).register(&project).expect("register");
        let old = seed(&store, &location, "old-persistence", "Qual persistência usar no RelayDesk?",
            "Usar PostgreSQL com servidor separado para a fila local",
            "A equipe inicialmente planejava compartilhar a fila entre máquinas.");
        let current = seed(&store, &location, "current-persistence",
            "Qual persistência usar no RelayDesk?",
            "Usar SQLite embutido para a persistência local, sem serviço de banco separado",
            concat!("O escopo mudou para desktop individual que precisa operar offline em campo; ",
                "não depender de rede nem instalar um daemon. PostgreSQL foi abandonado junto ",
                "com a premissa de fila compartilhada."));
        application::relations::DecisionRelations::new(store.clone()).supersede(&current, &old)
            .expect("supersession");
        let search = seed(&store, &location, "search", "Qual mecanismo de busca usar no RelayDesk?",
            "Usar FTS5 do SQLite para busca lexical dos registros nesta primeira versão",
            concat!("Evitar baixar modelos e consumir memória num laptop de 8 GB. ",
                "A busca lexical não garante encontrar sinônimos; ",
                "busca semântica ainda não foi aprovada."));
        std::fs::write(root.join("oracle.json"), serde_json::to_string_pretty(&json!({
            "old":old,"current":current,"search":search,"retention":null,
            "confirmation":"controlled, scripted, not human UI review",
            "extraction":"real offline fake; semantic choices supplied through CandidateEdits"
        })).unwrap()).expect("oracle");
    }
    let api: Arc<dyn CaptureApi> = Arc::new(CaptureIngest::new(store.clone()));
    let mut config = local_api::ApiServerConfig::new(api, paths.runtime_dir);
    config.services.agent = Some(Arc::new(application::agent_access::AgentAccess::new(store)));
    let server = local_api::ApiServer::start(config).expect("headless real local API");
    println!("{}", json!({"stage":"ready", "address":server.address().to_string(),
        "project":location,"database":paths.database}));
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).expect("wait for shutdown");
    server.shutdown();
}
