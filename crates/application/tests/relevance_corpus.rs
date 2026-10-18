//! Gate of the relevance step: what must reach the extractor by structure, what the
//! turn triage may be asked, and what must stay Empty; plus the step's latency.
//! The labelled set is `fixtures/relevance_cases.rs`.

#[path = "fixtures/relevance_cases.rs"]
mod cases;

use std::path::Path;
use std::time::Instant;

use application::extract::{assess_relevance, DecisionEvidence, Relevance};
use cases::{artifact, corpus};

/// Floor for the share of decisions that reach the extractor or the triage.
const RELEVANCE_RECALL_FLOOR: f64 = 1.0;
/// Ceiling for trivial captures that reach the extractor (wasted AI calls); zero, so the gate is exact.
const RELEVANCE_FALSE_PASSES_CEILING: usize = 0;
/// Ceiling for captures sent to the turn triage in this set (one small AI call each).
const RELEVANCE_TRIAGE_CALLS_CEILING: usize = 50;
/// Ceiling for the characters one triage call sends (instructions plus content).
const TRIAGE_INPUT_CHARS_CEILING: usize = 2_800;
/// Ceiling for assessing 2,000 realistic captures.
const RELEVANCE_LATENCY_CEILING_MS: u128 = 800;

/// Captures the harness wrote or a bare command: never worth a triage call.
const NEVER_TRIAGED: &[&str] = &[
    "real-compact.json",
    "real-task-notification.json",
    "real-task-notification-short.json",
    "notificacao-com-frase-de-rumo",
];

fn fixtures_dir() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("extraction")
}

#[test]
fn relevance_quality_gate() {
    let (mut positives, mut negatives, mut reached, mut false_passes) = (0, 0, 0, 0);
    let (mut triage_calls, mut chars_max, mut chars_sum, mut harness_triaged) = (0, 0, 0, 0);
    for (name, evidence, relevant) in corpus(&fixtures_dir()) {
        let relevance = assess_relevance(&evidence);
        let (passes, triaged) = match &relevance {
            Relevance::Pass(_) => (true, false),
            Relevance::Triage(_) => (false, true),
            Relevance::Empty => (false, false),
        };
        if let Relevance::Triage(input) = &relevance {
            let chars = application::extract::triage_input_chars(input);
            triage_calls += 1;
            chars_max = chars_max.max(chars);
            chars_sum += chars;
        }
        if NEVER_TRIAGED.contains(&name.as_str()) && triaged {
            harness_triaged += 1;
            println!("relevance triaged a harness capture: {name}");
        }
        if relevant {
            positives += 1;
            reached += usize::from(passes || triaged);
            if !(passes || triaged) {
                println!("relevance lost a decision: {name}");
            }
        } else {
            negatives += 1;
            false_passes += usize::from(passes);
            if passes {
                println!("relevance false pass: {name}");
            }
        }
    }
    let total = positives + negatives;
    let recall = reached as f64 / positives as f64;
    println!(
        "gate relevance recall={recall:.3} ({reached}/{positives}) \
         false_passes={false_passes}/{negatives} harness_triaged={harness_triaged} \
         triage_calls={triage_calls}/{total} triage_chars_max={chars_max} \
         triage_chars_sum={chars_sum}"
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
    assert_eq!(harness_triaged, 0, "a harness capture reached the triage");
    assert!(
        triage_calls <= RELEVANCE_TRIAGE_CALLS_CEILING,
        "{triage_calls} captures asked the triage"
    );
    assert!(
        chars_max <= TRIAGE_INPUT_CHARS_CEILING,
        "a triage call sends {chars_max} characters"
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
        .filter(|evidence| !matches!(assess_relevance(evidence), Relevance::Empty))
        .count();
    let elapsed_ms = start.elapsed().as_millis();
    println!("latency relevance_assess captures=2000 elapsed_ms={elapsed_ms} passed={passed}");
    assert!(
        elapsed_ms <= RELEVANCE_LATENCY_CEILING_MS,
        "assessing took {elapsed_ms} ms"
    );
}
