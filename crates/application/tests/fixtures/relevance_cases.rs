//! The relevance evaluation set: labelled captures (inline and fixtures) shared by the
//! relevance gate, the triage plumbing and the live triage evaluation.
#![allow(dead_code)]

use std::path::Path;

use application::extract::{DecisionEvidence, EvidenceArtifact};

/// Fixtures that must reach the extractor.
pub const FIXTURES_RELEVANT: &[&str] = &[
    "durable-migration.json",
    "durable-dependency.json",
    "durable-migration-with-formatting-note.json",
    "durable-course-change.json",
    "notification-with-migration.json",
    "real-ask-user-question.json",
    "ask-user-answer.json",
    "real-option-pick.json",
];
/// Fixtures that must stay Empty.
pub const FIXTURES_TRIVIAL: &[&str] = &[
    "trivial-formatting.json",
    "trivial-test-explained.json",
    "trivial-incidental-terms.json",
    "trivial-test-only-token.json",
    "trivial-two-directories.json",
    "trivial-sql-comment-only.json",
    "trivial-sql-whitespace.json",
    "trivial-prose-strong-text.json",
    "trivial-manifest-comment.json",
    "real-merge-push.json",
    "real-compact.json",
    "real-task-notification.json",
    "real-task-notification-short.json",
    "real-build-and-next.json",
    "real-release-tag.json",
    "real-review-findings-applied.json",
];

pub struct Case {
    pub name: &'static str,
    pub user: &'static str,
    pub assistant: &'static str,
    pub diff: &'static [&'static str],
    pub relevant: bool,
}

pub const fn case(
    name: &'static str,
    user: &'static str,
    assistant: &'static str,
    diff: &'static [&'static str],
    relevant: bool,
) -> Case {
    Case {
        name,
        user,
        assistant,
        diff,
        relevant,
    }
}

/// Separates user messages inside `Case::user`; each becomes its own `user_text`.
pub const NEXT_MESSAGE: char = '\u{1e}';

const JEVGUARD_USER: &str = "atualiza e vamos deioxar o v2 do opencode de lado po0r enquanto e seguir pra adaptacao pro claude code";

pub const CASES: &[Case] = &[
    // Must reach the extractor.
    case(
        "jevguard-real",
        JEVGUARD_USER,
        "Corrigi os tres testes do plugin e desfiz o bun add que tinha escrito no diretorio home; a suite passa de novo.",
        &["packages/plugin/test/artifact.test.ts"],
        true,
    ),
    case(
        "pt-esquece",
        "esquece o redis por enquanto, vamos focar no sqlite local",
        "Ok, tirei o cliente do redis do rascunho.",
        &["src/cache.rs"],
        true,
    ),
    case(
        "pt-a-partir",
        "a partir de agora todo endpoint novo usa paginacao por cursor",
        "",
        &[],
        true,
    ),
    case(
        "pt-nao-vamos-mais",
        "não vamos mais dar suporte ao node 16, sobe o mínimo pro 20",
        "",
        &["tests/engine_test.rs"],
        true,
    ),
    case(
        "pt-prioriza",
        "prioriza o instalador do windows, o do mac fica pra depois",
        "",
        &[],
        true,
    ),
    case(
        "pt-adia",
        "adia a migracao do banco pro mes que vem, agora so o hotfix",
        "",
        &["src/login.rs"],
        true,
    ),
    case(
        "pt-typos-hints",
        "deixa pra dps o suporte a linux e foca no windows por enquanto",
        "",
        &[],
        true,
    ),
    case(
        "pt-para-de-usar",
        "para de usar o mockito, troca pra testes com banco real",
        "",
        &["tests/repo_test.rs"],
        true,
    ),
    case(
        "pt-abandonar",
        "vamos abandonar o electron e seguir com o gpui",
        "",
        &[],
        true,
    ),
    case(
        "en-from-now-on",
        "From now on we use pnpm workspaces for every package.",
        "",
        &["pnpm-workspace.yaml"],
        true,
    ),
    case(
        "en-park",
        "let\u{2019}s park the plugin API and switch to the CLI for now",
        "",
        &["test/cli.test.ts"],
        true,
    ),
    case(
        "en-on-hold",
        "Put the sync feature on hold; prioritize the importer.",
        "",
        &[],
        true,
    ),
    case(
        "en-no-longer",
        "We no longer support the legacy config format, drop it.",
        "",
        &[],
        true,
    ),
    case(
        "en-deprioritize",
        "deprioritize dark mode, it's not worth it this quarter",
        "",
        &[],
        true,
    ),
    case(
        "pt-engaveta",
        "engaveta o modo escuro, nao e prioridade agora",
        "",
        &[],
        true,
    ),
    case(
        "en-pin-dependencies",
        "going forward we pin every dependency to an exact version",
        "",
        &["Cargo.toml"],
        true,
    ),
    case(
        "en-go-with",
        "let's go with sqlite instead of postgres for the local store",
        "",
        &[],
        true,
    ),
    case(
        "en-drop-support",
        "drop support for node 16, the minimum is 20 now",
        "",
        &["tests/engine_test.rs"],
        true,
    ),
    case(
        "pt-optamos",
        "optamos por manter o monorepo, nao vamos separar os pacotes",
        "",
        &[],
        true,
    ),
    case(
        "pt-rejeitamos",
        "rejeitamos o redis, fica no cache local mesmo",
        "",
        &["src/cache.rs"],
        true,
    ),
    // Must stay Empty.
    case(
        "roda-testes",
        "roda os testes",
        "Rodei: 214 testes passaram.",
        &[],
        false,
    ),
    // A bare confirmation has no content of its own: the decision lives in the
    // previous turn. Known gap.
    case(
        "pode-ser",
        "pode ser",
        "Levei as tres correcoes de teste e a versao 0.2.3 para a branch; o planejador ja esta montando o plano do limite de custo do diff.",
        &[],
        false,
    ),
    case("obrigado", "obrigado", "De nada!", &[], false),
    case(
        "limpeza",
        "faca a limpeza vc mesmo",
        "Removi os arquivos temporarios do build.",
        &["scripts/clean.sh"],
        false,
    ),
    case(
        "readme",
        "atualiza o README",
        "Atualizei a secao de instalacao.",
        &["README.md"],
        false,
    ),
    case(
        "uma-pista-seguir",
        "pode seguir para o proximo passo do plano",
        "",
        &["src/lib.rs"],
        false,
    ),
    case(
        "uma-pista-por-enquanto",
        "por enquanto roda so os testes unitarios",
        "",
        &[],
        false,
    ),
    case(
        "lado-a-lado",
        "coloca os dois cards lado a lado no painel",
        "",
        &["web/panel.tsx"],
        false,
    ),
    case(
        "switch-branch",
        "switch to the main branch and run the tests",
        "",
        &[],
        false,
    ),
    case(
        "no-longer-fails",
        "the test no longer fails after the fix, thanks",
        "",
        &["tests/parser_test.rs"],
        false,
    ),
    case("adianta", "adianta rodar o clippy de novo?", "", &[], false),
    case(
        "for-now-typo",
        "for now just fix the typo in the docs",
        "",
        &["docs/guide.md"],
        false,
    ),
    case(
        "foca-teste",
        "foca em fazer o teste passar",
        "",
        &["tests/api_test.rs"],
        false,
    ),
    // Only the user's words decide.
    case(
        "so-o-assistente",
        "e agora?",
        "Sugiro deixar o v2 do opencode de lado por enquanto e seguir para a adaptacao do claude code.",
        &[],
        false,
    ),
    // Operational verbs alone are hints, not a course change.
    case(
        "prioriza-testes",
        "prioriza os testes que falharam",
        "Rodei primeiro os tres que falharam.",
        &["tests/api_test.rs"],
        false,
    ),
    // Hints from different messages do not add up.
    case(
        "dicas-em-mensagens-separadas",
        "foca no teste que falhou\u{1e}troca pra cargo nextest",
        "",
        &["tests/api_test.rs"],
        false,
    ),
    case(
        "abandon-loop",
        "abandon the retry loop in this function",
        "Removed the loop; the call now fails fast.",
        &["src/retry.rs"],
        false,
    ),
    // A message the harness wrote is not the user's word.
    case(
        "notificacao-com-frase-de-rumo",
        "<task-notification><summary>deixei o v2 de lado a partir de agora</summary></task-notification>",
        "",
        &["tests/a_test.rs"],
        false,
    ),
    // Editing a normative document counts even when the report says "lint".
    case(
        "adr-editado-com-relatorio-de-lint",
        "atualiza o adr",
        "Atualizei o ADR. `pnpm lint`: sem correcoes.",
        &["docs/adr/0014-x.md"],
        true,
    ),
    case(
        "adr-arquivado-com-relatorio-de-lint",
        "atualiza o adr",
        "Atualizei o ADR. `pnpm lint`: sem correcoes.",
        &["docs/archive/adr/0014-x.md"],
        false,
    ),
];

pub fn artifact(index: usize, kind: &str, content: String, metadata: String) -> EvidenceArtifact {
    EvidenceArtifact {
        artifact_id: format!("a{index}"),
        kind: kind.to_string(),
        content,
        metadata,
    }
}

pub fn evidence_of(case: &Case) -> DecisionEvidence {
    let mut artifacts = Vec::new();
    for message in case.user.split(NEXT_MESSAGE).filter(|m| !m.is_empty()) {
        artifacts.push(artifact(
            artifacts.len(),
            "user_text",
            message.to_string(),
            "{}".to_string(),
        ));
    }
    if !case.assistant.is_empty() {
        artifacts.push(artifact(
            artifacts.len(),
            "assistant_text",
            case.assistant.to_string(),
            "{}".to_string(),
        ));
    }
    for file in case.diff {
        artifacts.push(artifact(
            artifacts.len(),
            "diff_hunk",
            format!("diff --git a/{file} b/{file}\n@@ -1,2 +1,3 @@\n+    let value = 1;\n"),
            format!("{{\"file\":\"{file}\"}}"),
        ));
    }
    artifacts.push(artifact(
        artifacts.len(),
        "tool_summary",
        String::new(),
        "{\"status\":\"ok\",\"tool\":\"Bash\"}".to_string(),
    ));
    DecisionEvidence {
        capture_id: format!("capture-{}", case.name),
        project_id: "project-1".to_string(),
        adapter: Some("claude-code".to_string()),
        session_id: Some("session-1".to_string()),
        observed_at: Some("2026-01-01T00:00:00Z".to_string()),
        artifacts,
    }
}

/// A fixture capture under `dir`, read as plain JSON.
pub fn fixture(dir: &Path, name: &str) -> DecisionEvidence {
    let path = dir.join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let value: serde_json::Value = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    let text_of =
        |value: &serde_json::Value, key: &str| value[key].as_str().unwrap_or_default().to_string();
    DecisionEvidence {
        capture_id: text_of(&value, "capture_id"),
        project_id: "project-1".to_string(),
        adapter: Some(text_of(&value["source"], "adapter")),
        session_id: Some(text_of(&value["source"], "session_id")),
        observed_at: Some(text_of(&value, "observed_at")),
        artifacts: value["artifacts"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|artifact| EvidenceArtifact {
                artifact_id: text_of(artifact, "artifact_id"),
                kind: text_of(artifact, "kind"),
                content: text_of(artifact, "content"),
                metadata: artifact["metadata"].to_string(),
            })
            .collect(),
    }
}

/// Every labelled capture of `dir`: `(name, evidence, the user's words decide)`.
pub fn corpus(dir: &Path) -> Vec<(String, DecisionEvidence, bool)> {
    let mut all: Vec<(String, DecisionEvidence, bool)> = CASES
        .iter()
        .map(|case| (case.name.to_string(), evidence_of(case), case.relevant))
        .collect();
    for name in FIXTURES_RELEVANT {
        all.push(((*name).to_string(), fixture(dir, name), true));
    }
    for name in FIXTURES_TRIVIAL {
        all.push(((*name).to_string(), fixture(dir, name), false));
    }
    all
}
