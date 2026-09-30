//! Context Pack export: deterministic Markdown/JSON preview and safe writing.

use application::context::{ContextPack, PackClaim, PackDecision};
use application::export::{preview_pack, write_pack, ExportError, ExportFormat};

fn pack() -> ContextPack {
    ContextPack {
        project_id: "project-1".to_string(),
        task: "adicionar cache na API".to_string(),
        as_of: "2026-09-30T00:00:00Z".to_string(),
        budget_chars: 8_000,
        used_chars: 120,
        decisions: vec![PackDecision {
            decision_id: "d-1".to_string(),
            version: 2,
            question: "Como fazer cache da API?".to_string(),
            choice: "Cache em memória".to_string(),
            rationale: "App local, sem servidor.".to_string(),
            confirmed_at: "2026-09-01T10:00:00Z".to_string(),
            evidence: vec!["art-1".to_string()],
            depends_on: vec!["d-0".to_string()],
            conflicts_with: Vec::new(),
        }],
        claims: vec![PackClaim {
            claim_id: "c-1".to_string(),
            kind: "convention".to_string(),
            statement: "Mensagens de erro em português".to_string(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: None,
            source_decision_id: Some("d-1".to_string()),
            matched: false,
        }],
        omitted: 1,
    }
}

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    let directory =
        std::env::temp_dir().join(format!("xemnas-pack-export-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

#[test]
fn markdown_carries_the_selection_and_every_citation() {
    let document = preview_pack(&pack(), ExportFormat::Markdown).expect("preview");
    let text = &document.content;
    assert!(text.starts_with("# Context Pack: adicionar cache na API\n"));
    assert!(text.contains("data de referência 2026-09-30T00:00:00Z · 120/8000 caracteres · 1 item(ns) fora do orçamento"));
    assert!(text.contains("### Como fazer cache da API?\n\n**Escolha:** Cache em memória"));
    assert!(text.contains("decisão `d-1` v2 · confirmada em 2026-09-01T10:00:00Z · evidências `art-1` · depende de `d-0`"));
    assert!(text.contains("- **Convenção:** Mensagens de erro em português _(válida desde 2026-01-01T00:00:00Z · `c-1` · origem: decisão `d-1`)_"));
    assert_eq!(document.bytes, text.len());
    assert_eq!(
        preview_pack(&pack(), ExportFormat::Markdown).expect("again"),
        document
    );
}

#[test]
fn empty_sections_are_explicit() {
    let empty = ContextPack {
        decisions: Vec::new(),
        claims: Vec::new(),
        omitted: 0,
        ..pack()
    };
    let text = preview_pack(&empty, ExportFormat::Markdown)
        .expect("preview")
        .content;
    assert!(text.contains("_(nenhuma decisão relevante)_"));
    assert!(text.contains("_(nenhuma válida nesta data)_"));
    assert!(!text.contains("fora do orçamento"));
}

#[test]
fn json_round_trips() {
    let document = preview_pack(&pack(), ExportFormat::Json).expect("preview");
    let parsed: ContextPack = serde_json::from_str(&document.content).expect("valid json");
    assert_eq!(parsed, pack());
    assert!(document.content.ends_with('\n'));
}

#[test]
fn writes_exactly_the_preview_and_never_overwrites_by_default() {
    let root = temporary_directory("write");
    let destination = root.join("pack.md");
    let document = preview_pack(&pack(), ExportFormat::Markdown).expect("preview");

    let result = write_pack(&document, &destination, false).expect("write");
    assert_eq!(result.bytes, document.bytes);
    assert_eq!(
        std::fs::read_to_string(&destination).expect("read"),
        document.content
    );

    assert_eq!(
        write_pack(&document, &destination, false),
        Err(ExportError::DestinationExists)
    );
    let json = preview_pack(&pack(), ExportFormat::Json).expect("json");
    write_pack(&json, &destination, true).expect("explicit overwrite");
    assert_eq!(
        std::fs::read_to_string(&destination).expect("read"),
        json.content
    );
    let leftovers = std::fs::read_dir(&root)
        .expect("list")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(leftovers, 0, "no temporary file is left behind");
    let _ = std::fs::remove_dir_all(&root);
}
