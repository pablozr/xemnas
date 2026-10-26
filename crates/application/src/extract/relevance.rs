//! Cheap, deterministic relevance filter (MVP-SPEC §6).

use std::collections::BTreeSet;

use serde_json::json;

use super::triage::{TriageInput, MIN_SPEECH_ALNUM, TRIAGE_ASSISTANT_CHARS, TRIAGE_USER_CHARS};
use super::{DecisionEvidence, EvidenceArtifact};

/// Metadata source the adapter gives to the answer of an `AskUserQuestion`.
pub const ASK_USER_QUESTION_SOURCE: &str = "ask_user_question";

/// A durable engineering choice worth proposing for human confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelevanceSignal {
    /// Touches a public contract, schema or persistence format.
    PublicContract,
    /// Touches security, privacy or compliance.
    SecurityPrivacy,
    /// Adds a durable external dependency.
    DependencyAdded,
    /// Changes more than one top-level component.
    CrossesBoundaries,
    /// Hard or expensive to revert.
    HardToRevert,
    /// Explicitly rejects a plausible alternative.
    RejectsAlternative,
    /// Conditions work that comes later.
    ConditionsFutureWork,
    /// Material blast radius if wrong.
    MaterialBlastRadius,
    /// Compares alternatives.
    AlternativesCompared,
    /// States an explicit trade-off.
    ExplicitTradeoff,
    /// Expresses disagreement or uncertainty.
    DisagreementUncertainty,
    /// Mentions a relevant cost.
    RelevantCost,
    /// Claims validity over months.
    ValidityMonths,
    /// Concerns maintenance or onboarding.
    MaintenanceOnboarding,
    /// Delegates the choice to an agent.
    DelegatedToAgent,
    /// Rests on an unproven assumption.
    UnprovenAssumption,
    /// The user changes the course of the work: drops, postpones, prioritizes or
    /// switches it. Comes from the turn triage.
    ChangesCourse,
    /// The user picks one of the options put in front of them: an answer to a
    /// question the assistant asked (structure) or a choice in free words (the
    /// turn triage).
    UserPicksOption,
}

impl RelevanceSignal {
    /// Returns the persisted literal for this signal.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PublicContract => "public_contract",
            Self::SecurityPrivacy => "security_privacy",
            Self::DependencyAdded => "dependency_added",
            Self::CrossesBoundaries => "crosses_boundaries",
            Self::HardToRevert => "hard_to_revert",
            Self::RejectsAlternative => "rejects_alternative",
            Self::ConditionsFutureWork => "conditions_future_work",
            Self::MaterialBlastRadius => "material_blast_radius",
            Self::AlternativesCompared => "alternatives_compared",
            Self::ExplicitTradeoff => "explicit_tradeoff",
            Self::DisagreementUncertainty => "disagreement_uncertainty",
            Self::RelevantCost => "relevant_cost",
            Self::ValidityMonths => "validity_months",
            Self::MaintenanceOnboarding => "maintenance_onboarding",
            Self::DelegatedToAgent => "delegated_to_agent",
            Self::UnprovenAssumption => "unproven_assumption",
            Self::ChangesCourse => "changes_course",
            Self::UserPicksOption => "user_picks_option",
        }
    }

    /// Returns `true` when a single occurrence already makes the capture
    /// relevant (MVP-SPEC §6).
    pub fn is_strong(&self) -> bool {
        matches!(
            self,
            Self::PublicContract
                | Self::SecurityPrivacy
                | Self::DependencyAdded
                | Self::CrossesBoundaries
                | Self::HardToRevert
                | Self::RejectsAlternative
                | Self::ConditionsFutureWork
                | Self::MaterialBlastRadius
                | Self::ChangesCourse
                | Self::UserPicksOption
        )
    }
}

/// The project's own documents in the capture: `(path, doc_kind)`.
fn documents(evidence: &DecisionEvidence) -> Vec<(String, String)> {
    evidence
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == crate::documents::DOCUMENT_ARTIFACT)
        .map(|artifact| {
            let metadata: serde_json::Value =
                serde_json::from_str(&artifact.metadata).unwrap_or_default();
            let field = |name: &str| {
                metadata
                    .get(name)
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            (field("file"), field("doc_kind"))
        })
        .collect()
}

/// The signals the structure of the capture states by itself, whatever the
/// words around it say: DDL or security in an added production line (tests and
/// comments do not count), a dependency, an edit to a normative document, or a
/// choice recorded through the tool that asks the user.
fn structural_signals(evidence: &DecisionEvidence) -> BTreeSet<RelevanceSignal> {
    let mut found = BTreeSet::new();
    if normative_document_edited(evidence) {
        found.insert(RelevanceSignal::ConditionsFutureWork);
    }
    if user_picks_option(evidence) {
        found.insert(RelevanceSignal::UserPicksOption);
    }
    for artifact in &evidence.artifacts {
        if artifact.kind != "diff_hunk" {
            continue;
        }
        if artifact_has_dependency_addition(artifact) {
            found.insert(RelevanceSignal::DependencyAdded);
        }
        let mut current_is_test = false;
        for line in artifact.content.lines() {
            if let Some(path) = diff_header_path(line) {
                current_is_test = is_test_path(path);
                continue;
            }
            let Some(rest) = added_line(line) else {
                continue;
            };
            if current_is_test {
                continue;
            }
            let content = rest.trim();
            if content.is_empty() || is_comment_prefix(content) {
                continue;
            }
            let lower = content.to_ascii_lowercase();
            if contains_any_phrase(&lower, STRUCTURAL_DDL_PHRASES) {
                found.insert(RelevanceSignal::PublicContract);
            }
            if contains_any_token(&lower, SECURITY_PRIVACY_TOKENS) {
                found.insert(RelevanceSignal::SecurityPrivacy);
            }
        }
    }
    found
}

/// The first path of a `diff --git` header line, without the `a/` prefix.
fn diff_header_path(line: &str) -> Option<&str> {
    let token = line
        .strip_prefix("diff --git ")?
        .split_whitespace()
        .next()
        .unwrap_or("");
    Some(token.strip_prefix("a/").unwrap_or(token))
}

/// The text of an added diff line (not a `+++` header).
fn added_line(line: &str) -> Option<&str> {
    line.trim_start()
        .strip_prefix('+')
        .filter(|rest| !rest.starts_with("++"))
}

/// Returns `true` when the artifact adds a dependency in a manifest.
fn artifact_has_dependency_addition(artifact: &EvidenceArtifact) -> bool {
    let mut current_manifest = false;
    let mut current_is_test = false;
    for line in artifact.content.lines() {
        if let Some(path) = diff_header_path(line) {
            let lower = path.to_ascii_lowercase();
            current_is_test = is_test_path(path);
            current_manifest = lower.ends_with("cargo.toml") || lower.ends_with("package.json");
            continue;
        }
        if !current_manifest || current_is_test {
            continue;
        }
        let Some(rest) = added_line(line) else {
            continue;
        };
        let addition = rest.trim();
        if addition.is_empty() || is_comment_prefix(addition) {
            continue;
        }
        if addition.contains('=') || addition.contains(':') {
            return true;
        }
    }
    false
}

/// Returns `true` for a path inside a test location.
fn is_test_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let file = lower.rsplit('/').next().unwrap_or(&lower);
    lower
        .split('/')
        .any(|segment| matches!(segment, "tests" | "test" | "__tests__"))
        || file.contains("_test.")
        || file.contains(".test.")
        || file.contains(".spec.")
}

/// Returns `true` for the common comment prefixes.
fn is_comment_prefix(content: &str) -> bool {
    content.starts_with("//")
        || content.starts_with('#')
        || content.starts_with("/*")
        || content.starts_with('*')
        || content.starts_with("<!--")
        || content.starts_with("--")
}

/// What the relevance step decides for a capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Relevance {
    /// Structure alone sends the capture to the extractor with these signals.
    Pass(Vec<RelevanceSignal>),
    /// The user spoke and nothing structural happened: the turn triage decides.
    Triage(TriageInput),
    /// Nothing durable to look at.
    Empty,
}

/// Decides how a capture goes on: straight to the extractor, to the turn
/// triage, or nowhere.
pub fn assess_relevance(evidence: &DecisionEvidence) -> Relevance {
    let signals = filter_relevant(evidence);
    if !signals.is_empty() {
        return Relevance::Pass(signals);
    }
    // A written document is read whole or not at all; the triage is for speech.
    if !documents(evidence).is_empty() {
        return Relevance::Empty;
    }
    triage_input(evidence).map_or(Relevance::Empty, Relevance::Triage)
}

/// What the turn triage reads, or `None` when the user said nothing worth a
/// call: the speech of the turn (without the harness's reminders and the
/// project context delivered to the user) and the end of the assistant's text.
pub fn triage_input(evidence: &DecisionEvidence) -> Option<TriageInput> {
    let speech: Vec<&str> = evidence
        .artifacts
        .iter()
        .filter(|artifact| is_user_speech(artifact))
        .map(|artifact| crate::injection::strip_leading_reminders(&artifact.content))
        .collect();
    let speech: Vec<String> = speech
        .into_iter()
        .map(|text| {
            crate::injection::strip_context_blocks(text)
                .trim()
                .to_string()
        })
        .filter(|text| text.chars().filter(|c| c.is_alphanumeric()).count() >= MIN_SPEECH_ALNUM)
        .collect();
    if speech.is_empty() {
        return None;
    }
    let report: Vec<&str> = evidence
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == "assistant_text")
        .map(|artifact| artifact.content.as_str())
        .collect();
    let report = crate::external::protected_text(&report.join("\n"));
    let skip = report
        .chars()
        .count()
        .saturating_sub(TRIAGE_ASSISTANT_CHARS);
    Some(TriageInput {
        user: crate::external::limited_text(&speech.join("\n---\n"), TRIAGE_USER_CHARS),
        assistant_tail: report.chars().skip(skip).collect(),
    })
}

/// Returns the signals of a capture that goes straight to the extractor, or an
/// empty list. Structure decides (see [`structural_signals`]); the label lists
/// below only add the signals the extractor and the validation work with. A
/// document is the exception: it passes with one strong label or two moderate
/// ones.
pub fn filter_relevant(evidence: &DecisionEvidence) -> Vec<RelevanceSignal> {
    let mut detected = structural_signals(evidence);
    let documents = documents(evidence);
    if detected.is_empty() && documents.is_empty() {
        return Vec::new();
    }
    let structural = !detected.is_empty();
    let text = aggregate_text(evidence);

    // Documentation states intent meant to last; ADRs and specifications
    // record choices that condition the work after them.
    for (_, kind) in documents {
        detected.insert(RelevanceSignal::ValidityMonths);
        if matches!(kind.as_str(), "adr" | "spec") {
            detected.insert(RelevanceSignal::ConditionsFutureWork);
        }
    }

    LABELS_BY_PHRASE
        .iter()
        .filter(|(_, phrases)| contains_any_phrase(&text, phrases))
        .for_each(|(signal, _)| {
            detected.insert(*signal);
        });
    if contains_any_token(&text, PUBLIC_CONTRACT_TOKENS) {
        detected.insert(RelevanceSignal::PublicContract);
    }
    if contains_any_token(&text, SECURITY_PRIVACY_TOKENS) {
        detected.insert(RelevanceSignal::SecurityPrivacy);
    }
    if has_dependency_addition(&text) {
        detected.insert(RelevanceSignal::DependencyAdded);
    }
    if top_level_dirs(evidence).len() >= 2 {
        detected.insert(RelevanceSignal::CrossesBoundaries);
    }

    let strong = detected.iter().filter(|signal| signal.is_strong()).count();
    let moderate = detected.len() - strong;
    if !structural && strong == 0 && moderate < 2 {
        return Vec::new();
    }
    detected.into_iter().collect()
}

/// Whether the user chose among options: the tool that asks them ran or the
/// adapter recorded their answer. Structure only; a choice made in free words
/// is left to the turn triage, not to phrase lists. A choice is
/// where the decision lives, so, like a course change, the diff does not veto it.
fn user_picks_option(evidence: &DecisionEvidence) -> bool {
    evidence
        .artifacts
        .iter()
        .any(|artifact| match artifact.kind.as_str() {
            "tool_summary" => is_ask_user_question(artifact),
            "user_text" => answers_a_question(artifact),
            _ => false,
        })
}

/// A finished `AskUserQuestion` call; the tool name is in the content (or, for
/// older captures, the metadata).
fn is_ask_user_question(artifact: &EvidenceArtifact) -> bool {
    [&artifact.content, &artifact.metadata]
        .into_iter()
        .filter(|source| source.contains("AskUserQuestion"))
        .any(|source| {
            let value: serde_json::Value = serde_json::from_str(source).unwrap_or_default();
            value["tool"] == "AskUserQuestion" && value["status"] == "ok"
        })
}

/// The adapter writes the answer to a question as `user_text` with this source.
fn answers_a_question(artifact: &EvidenceArtifact) -> bool {
    artifact.metadata.contains(ASK_USER_QUESTION_SOURCE)
        && serde_json::from_str::<serde_json::Value>(&artifact.metadata)
            .is_ok_and(|value| value["source"] == ASK_USER_QUESTION_SOURCE)
}

/// Whether the diff adds a line to a normative document: an ADR, a
/// specification, `AGENTS.md` or `CLAUDE.md`, outside an archive folder.
fn normative_document_edited(evidence: &DecisionEvidence) -> bool {
    evidence
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == "diff_hunk")
        .any(|artifact| {
            let mut normative = false;
            for line in artifact.content.lines() {
                if let Some(path) = diff_header_path(line) {
                    normative = is_normative_path(path);
                    continue;
                }
                if normative && added_line(line).is_some_and(|added| !added.trim().is_empty()) {
                    return true;
                }
            }
            false
        })
}

fn is_normative_path(path: &str) -> bool {
    use crate::documents::{classify, is_archived_path, is_documentation_path, DocumentKind};
    if !is_documentation_path(path) || is_archived_path(path) {
        return false;
    }
    let file = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    matches!(file.as_str(), "agents.md" | "claude.md")
        || matches!(classify(path), DocumentKind::Adr | DocumentKind::Spec)
}

/// Whether the artifact is something the user said: not a message the harness
/// wrote (task notification, command echo, `<system-reminder>`) and not a bare
/// `/command`, which names an action and states no choice.
fn is_user_speech(artifact: &EvidenceArtifact) -> bool {
    artifact.kind == "user_text"
        && !crate::injection::is_harness_prompt(&artifact.content)
        && !is_bare_command(&artifact.content)
}

/// `/compact`, `/clear`: a slash and one word, nothing else.
fn is_bare_command(content: &str) -> bool {
    content.trim().strip_prefix('/').is_some_and(|word| {
        !word.is_empty()
            && word
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
    })
}

/// Lowercased concatenation of kinds, contents and metadata, in artifact order.
fn aggregate_text(evidence: &DecisionEvidence) -> String {
    let mut text = String::new();
    for artifact in &evidence.artifacts {
        text.push_str(&artifact.kind.to_lowercase());
        text.push('\n');
        text.push_str(&artifact.content.to_lowercase());
        text.push('\n');
        text.push_str(&artifact.metadata.to_lowercase());
        text.push('\n');
    }
    text
}

/// JSON summary of the diff-shaped artifacts, never their raw content.
pub(super) fn diff_summary(evidence: &DecisionEvidence) -> String {
    let files: Vec<String> = diff_files(evidence).into_iter().collect();
    json!({ "files": files, "artifacts": evidence.artifacts.len() }).to_string()
}

/// Files changed in the capture, sorted.
pub(super) fn diff_file_list(evidence: &DecisionEvidence) -> Vec<String> {
    diff_files(evidence).into_iter().collect()
}

/// Sorted unique file paths found in `diff --git` headers, plus, for a
/// document, its own path and the code paths it cites (so what it decides
/// ties to the components of the map).
fn diff_files(evidence: &DecisionEvidence) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
    for (path, _) in documents(evidence) {
        if !path.is_empty() {
            files.insert(path);
        }
    }
    for artifact in &evidence.artifacts {
        if artifact.kind == crate::documents::DOCUMENT_ARTIFACT {
            files.extend(crate::injection::mentioned_paths(&artifact.content, ""));
        }
    }
    for artifact in &evidence.artifacts {
        for line in artifact.content.lines() {
            let Some(rest) = line.strip_prefix("diff --git ") else {
                continue;
            };
            for token in rest.split_whitespace() {
                let path = token
                    .strip_prefix("a/")
                    .or_else(|| token.strip_prefix("b/"))
                    .unwrap_or(token);
                if !path.is_empty() {
                    files.insert(path.to_string());
                }
            }
        }
    }
    files
}

/// Top-level path segments present in diff headers.
fn top_level_dirs(evidence: &DecisionEvidence) -> BTreeSet<String> {
    let mut dirs = BTreeSet::new();
    for file in diff_files(evidence) {
        if let Some(segment) = file.split('/').next() {
            if !segment.is_empty() {
                dirs.insert(segment.to_string());
            }
        }
    }
    dirs
}

/// Returns `true` when a manifest diff adds a dependency line.
fn has_dependency_addition(text: &str) -> bool {
    if !(text.contains("cargo.toml") || text.contains("package.json")) {
        return false;
    }
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        match trimmed.strip_prefix('+') {
            Some(rest) if rest.starts_with("++") => false,
            Some(addition) => {
                let addition = addition.trim();
                !addition.is_empty()
                    && !is_comment_prefix(addition)
                    && (addition.contains('=') || addition.contains(':'))
            }
            None => false,
        }
    })
}

/// Case-insensitive whole-word match for ASCII tokens.
fn contains_token(text: &str, token: &str) -> bool {
    let bytes = text.as_bytes();
    let mut start = 0;
    while let Some(position) = text[start..].find(token) {
        let index = start + position;
        let before_ok = index == 0 || !is_word_byte(bytes[index - 1]);
        let end = index + token.len();
        let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        start = index + 1;
    }
    false
}

fn contains_any_token(text: &str, tokens: &[&str]) -> bool {
    tokens.iter().any(|token| contains_token(text, token))
}

fn contains_any_phrase(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| text.contains(phrase))
}

fn is_word_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// DDL/migration phrases counted only on a non-comment `+` line of a
/// production `diff_hunk`; a `.sql` path alone is never enough.
const STRUCTURAL_DDL_PHRASES: &[&str] = &[
    "create table",
    "alter table",
    "add column",
    "drop table",
    "create index",
    "migration",
    "endpoint",
];

const PUBLIC_CONTRACT_TOKENS: &[&str] = &[
    "migration",
    "migrations",
    "migracao",
    "schema",
    "endpoint",
    "api",
    "table",
    "tabela",
    "ddl",
    "contract",
    "contrato",
    "persistence",
    "persistencia",
];

const SECURITY_PRIVACY_TOKENS: &[&str] = &[
    "security",
    "seguranca",
    "auth",
    "token",
    "credential",
    "secret",
    "privacy",
    "privacidade",
    "permission",
    "permissao",
    "compliance",
    "encrypt",
    "encryption",
    "lgpd",
    "gdpr",
];

/// Labels the extractor and the validation work with, added to a capture that
/// already passed by structure. They never decide whether it passes.
const LABELS_BY_PHRASE: &[(RelevanceSignal, &[&str])] = &[
    (RelevanceSignal::HardToRevert, HARD_TO_REVERT),
    (RelevanceSignal::RejectsAlternative, REJECTS_ALTERNATIVE),
    (
        RelevanceSignal::ConditionsFutureWork,
        CONDITIONS_FUTURE_WORK,
    ),
    (RelevanceSignal::MaterialBlastRadius, MATERIAL_BLAST_RADIUS),
    (RelevanceSignal::AlternativesCompared, ALTERNATIVES_COMPARED),
    (RelevanceSignal::ExplicitTradeoff, EXPLICIT_TRADEOFF),
    (
        RelevanceSignal::DisagreementUncertainty,
        DISAGREEMENT_UNCERTAINTY,
    ),
    (RelevanceSignal::RelevantCost, RELEVANT_COST),
    (RelevanceSignal::ValidityMonths, VALIDITY_MONTHS),
    (
        RelevanceSignal::MaintenanceOnboarding,
        MAINTENANCE_ONBOARDING,
    ),
    (RelevanceSignal::DelegatedToAgent, DELEGATED_TO_AGENT),
    (RelevanceSignal::UnprovenAssumption, UNPROVEN_ASSUMPTION),
];

const HARD_TO_REVERT: &[&str] = &[
    "hard to reverse",
    "hard to revert",
    "irreversible",
    "irreversivel",
    "breaking change",
    "once deployed",
    "destructive",
    "drop table",
];

const REJECTS_ALTERNATIVE: &[&str] = &[
    "rather than",
    "instead of",
    "em vez de",
    "rejected the",
    "rejeitamos",
    "optamos por",
    "chose not",
];

const CONDITIONS_FUTURE_WORK: &[&str] = &[
    "future",
    "futuro",
    "follow-up",
    "follow up",
    "next step",
    "will allow",
    "prepares",
];

const MATERIAL_BLAST_RADIUS: &[&str] = &[
    "all users",
    "every request",
    "global",
    "todos os",
    "shared across",
    "core path",
    "blast radius",
];

const ALTERNATIVES_COMPARED: &[&str] = &[
    "alternative",
    "alternativa",
    "options",
    "option",
    "candidate",
];

const EXPLICIT_TRADEOFF: &[&str] = &[
    "trade-off",
    "tradeoff",
    "trade off",
    "versus",
    " vs ",
    "downside",
    "compromise",
];

const DISAGREEMENT_UNCERTAINTY: &[&str] = &[
    "unclear",
    "uncertain",
    "not sure",
    "talvez",
    "incerto",
    "ambiguous",
];

const RELEVANT_COST: &[&str] = &[
    "cost",
    "custo",
    "latency",
    "latencia",
    "overhead",
    "budget",
    "throughput",
];

const VALIDITY_MONTHS: &[&str] = &[
    "long-term",
    "long term",
    "months",
    "years",
    "durable",
    "durave",
    "permanent",
];

const MAINTENANCE_ONBOARDING: &[&str] = &[
    "maintain",
    "manutencao",
    "onboarding",
    "readability",
    "documentation",
];

const DELEGATED_TO_AGENT: &[&str] = &["agent", "agente", "llm", "copilot"];

const UNPROVEN_ASSUMPTION: &[&str] = &[
    "assume",
    "assumption",
    "premissa",
    "presume",
    "without evidence",
    "unverified",
];

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_USER: &str = "atualiza e vamos deioxar o v2 do opencode de lado po0r enquanto e seguir pra adaptacao pro claude code";
    const REAL_ASSISTANT: &str = "Corrigi os tres testes do plugin e desfiz o bun add que tinha escrito no diretorio home; a suite passa de novo.";

    fn artifact(id: &str, kind: &str, content: &str) -> EvidenceArtifact {
        EvidenceArtifact {
            artifact_id: id.to_string(),
            kind: kind.to_string(),
            content: content.to_string(),
            metadata: "{}".to_string(),
        }
    }

    fn evidence(artifacts: Vec<EvidenceArtifact>) -> DecisionEvidence {
        DecisionEvidence {
            capture_id: "capture-1".to_string(),
            project_id: "project-1".to_string(),
            adapter: Some("claude-code".to_string()),
            session_id: Some("session-1".to_string()),
            observed_at: Some("2026-01-01T00:00:00Z".to_string()),
            artifacts,
        }
    }

    fn said(text: &str) -> Relevance {
        assess_relevance(&evidence(vec![artifact("a0", "user_text", text)]))
    }

    fn test_diff() -> EvidenceArtifact {
        artifact(
            "diff",
            "diff_hunk",
            "diff --git a/tests/a_test.rs b/tests/a_test.rs\n@@ -1,2 +1,3 @@\n+    let value = 1;\n",
        )
    }

    #[test]
    fn the_users_words_with_a_test_only_diff_go_to_the_triage() {
        let relevance = assess_relevance(&evidence(vec![
            artifact("a0", "user_text", REAL_USER),
            artifact("a1", "assistant_text", REAL_ASSISTANT),
            artifact(
                "a2",
                "diff_hunk",
                "diff --git a/packages/plugin/test/artifact.test.ts b/packages/plugin/test/artifact.test.ts\n@@ -1,2 +1,3 @@\n+    let value = 1;\n",
            ),
            artifact("a3", "tool_summary", "{\"status\":\"ok\",\"tool\":\"Bash\"}"),
        ]));
        let Relevance::Triage(input) = relevance else {
            panic!("{relevance:?}");
        };
        assert_eq!(input.user, REAL_USER);
        assert!(input.assistant_tail.contains("suite passa"));
    }

    #[test]
    fn only_the_users_words_reach_the_triage() {
        let relevance =
            assess_relevance(&evidence(vec![artifact("a1", "assistant_text", REAL_USER)]));
        assert_eq!(relevance, Relevance::Empty);
    }

    #[test]
    fn trivial_requests_never_pass_but_are_left_to_the_triage() {
        for text in [
            "roda os testes",
            "pode ser",
            "obrigado",
            "faca a limpeza vc mesmo",
            "atualiza o README",
        ] {
            let user = artifact("a0", "user_text", text);
            assert!(matches!(said(text), Relevance::Triage(_)), "{text}");
            let with_diff = evidence(vec![user, test_diff()]);
            assert!(filter_relevant(&with_diff).is_empty(), "{text} with a diff");
            assert!(matches!(assess_relevance(&with_diff), Relevance::Triage(_)));
        }
    }

    #[test]
    fn harness_messages_and_bare_commands_are_not_the_users_words() {
        for text in [
            "<task-notification><summary>deixei o v2 de lado a partir de agora</summary></task-notification>",
            "<command-name>/compact</command-name> a partir de agora",
            "<system-reminder>a partir de agora</system-reminder>",
            "/compact",
            "/superpowers:plan",
        ] {
            assert_eq!(said(text), Relevance::Empty, "{text}");
        }
    }

    #[test]
    fn a_reminder_at_the_start_is_not_sent_to_the_triage() {
        let relevance = said("<system-reminder>x</system-reminder>\na partir de agora usa redis");
        let Relevance::Triage(input) = relevance else {
            panic!("{relevance:?}");
        };
        assert_eq!(input.user, "a partir de agora usa redis");
    }

    #[test]
    fn the_context_delivered_to_the_user_is_not_sent_to_the_triage() {
        let text = "vamos de redis\n<xemnas-context>D:abc segredo</xemnas-context>";
        let Relevance::Triage(input) = said(text) else {
            panic!("not a triage");
        };
        assert!(!input.user.contains("xemnas-context"), "{}", input.user);
    }

    #[test]
    fn a_speech_without_words_is_not_worth_a_call() {
        for text in ["ok", "2", "  ?! "] {
            assert_eq!(said(text), Relevance::Empty, "{text}");
        }
    }

    #[test]
    fn the_triage_reads_a_bounded_speech_and_report() {
        let long = "a".repeat(3_000);
        let relevance = assess_relevance(&evidence(vec![
            artifact("a0", "user_text", &long),
            artifact("a1", "assistant_text", &"b".repeat(2_000)),
        ]));
        let Relevance::Triage(input) = relevance else {
            panic!("{relevance:?}");
        };
        assert_eq!(input.user.chars().count(), TRIAGE_USER_CHARS);
        assert_eq!(input.assistant_tail.chars().count(), TRIAGE_ASSISTANT_CHARS);
    }

    fn tool(status: &str) -> EvidenceArtifact {
        artifact(
            "t0",
            "tool_summary",
            &format!("{{\"tool\":\"AskUserQuestion\",\"status\":\"{status}\"}}"),
        )
    }

    #[test]
    fn a_finished_question_to_the_user_is_a_choice_even_beside_a_lint_report() {
        let assistant = artifact("a1", "assistant_text", "`pnpm lint`: sem correcoes");
        let ok = filter_relevant(&evidence(vec![assistant.clone(), tool("ok"), test_diff()]));
        assert_eq!(ok, vec![RelevanceSignal::UserPicksOption]);
        let failed = filter_relevant(&evidence(vec![assistant, tool("error"), test_diff()]));
        assert!(failed.is_empty());
    }

    #[test]
    fn the_recorded_answer_to_a_question_is_a_choice() {
        let mut answer = artifact("a0", "user_text", "question: x\nanswer: Redis");
        answer.metadata = "{\"source\":\"ask_user_question\"}".to_string();
        assert!(filter_relevant(&evidence(vec![answer, test_diff()]))
            .contains(&RelevanceSignal::UserPicksOption));
    }

    fn edit_of(path: &str) -> Vec<EvidenceArtifact> {
        vec![
            artifact("a0", "assistant_text", "`pnpm lint`: sem correcoes"),
            artifact(
                "d0",
                "diff_hunk",
                &format!(
                    "diff --git a/{path} b/{path}\n@@ -1,2 +1,3 @@\n+Decidimos usar SQLite.\n"
                ),
            ),
        ]
    }

    #[test]
    fn editing_a_normative_document_passes_beside_a_lint_report() {
        for path in [
            "docs/adr/0014-x.md",
            "specs/capture.md",
            "AGENTS.md",
            "sub/CLAUDE.md",
        ] {
            let signals = filter_relevant(&evidence(edit_of(path)));
            assert!(
                signals.contains(&RelevanceSignal::ConditionsFutureWork),
                "{path}"
            );
        }
        for path in ["docs/archive/adr/0001.md", "docs/guia.md", "src/lib.rs"] {
            assert!(
                filter_relevant(&evidence(edit_of(path))).is_empty(),
                "{path}"
            );
        }
    }
}
