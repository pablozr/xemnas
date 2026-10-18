//! Labelled corpus of the relevance filter: what must reach the extractor and what
//! must stay Empty; plus the filter latency.

use std::path::Path;
use std::time::Instant;

use application::extract::{filter_relevant, DecisionEvidence, EvidenceArtifact};
use integration_contracts::capture::CaptureEnvelope;

/// Floor for the share of relevant captures that reach the extractor.
const RELEVANCE_RECALL_FLOOR: f64 = 1.0;
/// Ceiling for trivial captures that reach the extractor (wasted AI calls); zero, so the gate is exact.
const RELEVANCE_FALSE_PASSES_CEILING: usize = 0;
/// Ceiling for filtering 2,000 realistic captures.
const RELEVANCE_LATENCY_CEILING_MS: u128 = 800;

/// Fixtures that must reach the extractor.
const FIXTURES_RELEVANT: &[&str] = &[
    "durable-migration.json",
    "durable-dependency.json",
    "durable-migration-with-formatting-note.json",
    "durable-course-change.json",
    "notification-with-migration.json",
    "real-option-pick.json",
    "real-ask-user-question.json",
    "ask-user-answer.json",
];
/// Fixtures that must stay Empty.
const FIXTURES_TRIVIAL: &[&str] = &[
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

struct Case {
    name: &'static str,
    user: &'static str,
    assistant: &'static str,
    diff: &'static [&'static str],
    relevant: bool,
}

const fn case(
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
const NEXT_MESSAGE: char = '\u{1e}';

const JEVGUARD_USER: &str = "atualiza e vamos deioxar o v2 do opencode de lado po0r enquanto e seguir pra adaptacao pro claude code";

const CASES: &[Case] = &[
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

fn artifact(index: usize, kind: &str, content: String, metadata: String) -> EvidenceArtifact {
    EvidenceArtifact {
        artifact_id: format!("a{index}"),
        kind: kind.to_string(),
        content,
        metadata,
    }
}

fn evidence_of(case: &Case) -> DecisionEvidence {
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

fn fixture(name: &str) -> DecisionEvidence {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("extraction")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
    let envelope: CaptureEnvelope = serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
    DecisionEvidence {
        capture_id: envelope.capture_id.clone(),
        project_id: "project-1".to_string(),
        adapter: Some(envelope.source.adapter.clone()),
        session_id: Some(envelope.source.session_id.clone()),
        observed_at: Some(envelope.observed_at.clone()),
        artifacts: envelope
            .artifacts
            .iter()
            .map(|artifact| EvidenceArtifact {
                artifact_id: artifact.artifact_id.clone(),
                kind: artifact.kind.as_str().to_string(),
                content: artifact.content.clone(),
                metadata: serde_json::to_string(&artifact.metadata)
                    .unwrap_or_else(|_| "{}".to_string()),
            })
            .collect(),
    }
}

/// Every labelled capture: `(name, evidence, must reach the extractor)`.
fn corpus() -> Vec<(String, DecisionEvidence, bool)> {
    let mut all: Vec<(String, DecisionEvidence, bool)> = CASES
        .iter()
        .map(|case| (case.name.to_string(), evidence_of(case), case.relevant))
        .collect();
    for name in FIXTURES_RELEVANT {
        all.push(((*name).to_string(), fixture(name), true));
    }
    for name in FIXTURES_TRIVIAL {
        all.push(((*name).to_string(), fixture(name), false));
    }
    all
}

#[test]
fn relevance_quality_gate() {
    let (mut positives, mut negatives, mut passed_positives, mut false_passes) = (0, 0, 0, 0);
    for (name, evidence, relevant) in corpus() {
        let passed = !filter_relevant(&evidence).is_empty();
        if passed != relevant {
            println!("relevance differs: {name} passed={passed} expected={relevant}");
        }
        if relevant {
            positives += 1;
            passed_positives += usize::from(passed);
        } else {
            negatives += 1;
            false_passes += usize::from(passed);
        }
    }
    let recall = passed_positives as f64 / positives as f64;
    println!(
        "gate relevance recall={recall:.3} ({passed_positives}/{positives}) \
         false_passes={false_passes}/{negatives}"
    );
    assert!(
        recall >= RELEVANCE_RECALL_FLOOR,
        "recall {recall:.3} fell below {RELEVANCE_RECALL_FLOOR}"
    );
    // A ceiling of zero: any pass of a trivial capture is a wasted AI call.
    assert_eq!(
        false_passes, RELEVANCE_FALSE_PASSES_CEILING,
        "{false_passes} trivial captures reached the extractor"
    );
}

#[test]
fn relevance_filter_latency() {
    const NEUTRAL: &str = "Ajustei o carregamento da lista e conferi o resultado na tela. ";
    let captures: Vec<DecisionEvidence> = (0..2_000usize)
        .map(|index| {
            let neutral = format!(
                "preciso que voce olhe a lista {index} com calma e me diga o que viu, \
                 confere o texto da tela e ajusta o espacamento do cabecalho do painel {index}, \
                 depois revisa o ajuste da lista e confere o texto da tela de novo antes de \
                 me responder com um resumo curto do que mudou"
            );
            let user = if index % 2 == 0 {
                format!("{neutral}. a partir de agora o modulo {index} fica de lado")
            } else {
                neutral
            };
            let mut artifacts = vec![
                artifact(0, "user_text", user, "{}".to_string()),
                artifact(1, "assistant_text", NEUTRAL.repeat(64), "{}".to_string()),
                artifact(
                    2,
                    "diff_hunk",
                    format!(
                        "diff --git a/src/m{index}.rs b/src/m{index}.rs\n@@ -1,2 +1,3 @@\n{}",
                        "+    let value = compute(input);\n".repeat(60)
                    ),
                    format!("{{\"file\":\"src/m{index}.rs\"}}"),
                ),
            ];
            for tool in 0..5 {
                artifacts.push(artifact(
                    3 + tool,
                    "tool_summary",
                    String::new(),
                    "{\"status\":\"ok\",\"tool\":\"Bash\"}".to_string(),
                ));
            }
            DecisionEvidence {
                capture_id: format!("capture-{index}"),
                project_id: "project-1".to_string(),
                adapter: Some("claude-code".to_string()),
                session_id: Some("session-1".to_string()),
                observed_at: Some("2026-01-01T00:00:00Z".to_string()),
                artifacts,
            }
        })
        .collect();

    let start = Instant::now();
    let passed = captures
        .iter()
        .filter(|evidence| !filter_relevant(evidence).is_empty())
        .count();
    let elapsed_ms = start.elapsed().as_millis();
    println!("latency relevance_filter captures=2000 elapsed_ms={elapsed_ms} passed={passed}");
    assert!(
        elapsed_ms <= RELEVANCE_LATENCY_CEILING_MS,
        "filtering took {elapsed_ms} ms"
    );
}
