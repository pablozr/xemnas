//! Cheap, deterministic relevance filter (MVP-SPEC §6).

use std::collections::BTreeSet;

use serde_json::json;

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
    /// The user changes the course of the work: drops, postpones, prioritizes or switches it.
    ChangesCourse,
    /// The user picks one of the options put in front of them: an answer to a
    /// question the assistant asked, or "o 1 funciona, mas o 2 nao".
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

/// Returns `true` when the capture is one of the §6 exclusions.
fn is_trivially_excluded(evidence: &DecisionEvidence, text: &str) -> bool {
    // A written document is read whole; the extractor decides what in it
    // was decided.
    if !documents(evidence).is_empty() {
        return false;
    }
    if has_structural_strong(evidence) {
        return false;
    }
    if !has_diff_hunk(evidence) {
        // A decision can be recorded in the conversation before (or without)
        // any code change (MVP-SPEC AD-08). Without a diff, only explicit
        // choice language lets the capture through: bare mentions of
        // contract or security nouns are facts, not choices.
        return !contains_any_phrase(text, CHOICE_LANGUAGE)
            || TRIVIAL_MARKERS.iter().any(|marker| text.contains(marker));
    }
    if TRIVIAL_MARKERS.iter().any(|marker| text.contains(marker)) {
        return true;
    }
    let files = diff_files(evidence);
    if !files.is_empty() && files.iter().all(|path| is_test_path(path)) {
        return true;
    }
    is_comment_only_diff(evidence)
}

/// Returns `true` when the evidence carries at least one `diff_hunk` artifact.
fn has_diff_hunk(evidence: &DecisionEvidence) -> bool {
    evidence
        .artifacts
        .iter()
        .any(|artifact| artifact.kind == "diff_hunk")
}

/// Returns `true` when a production `diff_hunk` carries a strong structural
/// signal.
fn has_structural_strong(evidence: &DecisionEvidence) -> bool {
    if normative_document_edited(evidence) {
        return true;
    }
    for artifact in &evidence.artifacts {
        if artifact.kind != "diff_hunk" {
            continue;
        }
        if artifact_has_dependency_addition(artifact) {
            return true;
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
                return true;
            }
            if contains_any_token(&lower, SECURITY_PRIVACY_TOKENS) {
                return true;
            }
        }
    }
    false
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

/// Returns `true` when every added/removed diff line is a comment.
fn is_comment_only_diff(evidence: &DecisionEvidence) -> bool {
    let mut saw_comment = false;
    for artifact in &evidence.artifacts {
        for line in artifact.content.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("+++") || trimmed.starts_with("---") {
                continue;
            }
            let Some(rest) = trimmed
                .strip_prefix('+')
                .or_else(|| trimmed.strip_prefix('-'))
            else {
                continue;
            };
            let content = rest.trim_start();
            if content.is_empty() {
                continue;
            }
            if is_comment_prefix(content) {
                saw_comment = true;
            } else {
                return false;
            }
        }
    }
    saw_comment
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

/// Returns the relevance signals that pass the §6 filter, or an empty list.
pub fn filter_relevant(evidence: &DecisionEvidence) -> Vec<RelevanceSignal> {
    let text = aggregate_text(evidence);
    let changes_course = user_changes_course(evidence);
    let picks_option = user_picks_option(evidence);

    if is_trivially_excluded(evidence, &text) {
        // The user's words decide; what the turn touched does not veto them.
        let mut kept = Vec::new();
        if changes_course {
            kept.push(RelevanceSignal::ChangesCourse);
        }
        if picks_option {
            kept.push(RelevanceSignal::UserPicksOption);
        }
        return kept;
    }

    let mut detected = BTreeSet::new();

    // Documentation states intent meant to last; ADRs and specifications
    // record choices that condition the work after them.
    for (_, kind) in documents(evidence) {
        detected.insert(RelevanceSignal::ValidityMonths);
        if matches!(kind.as_str(), "adr" | "spec") {
            detected.insert(RelevanceSignal::ConditionsFutureWork);
        }
    }

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
    if contains_any_phrase(&text, HARD_TO_REVERT) {
        detected.insert(RelevanceSignal::HardToRevert);
    }
    if contains_any_phrase(&text, REJECTS_ALTERNATIVE) {
        detected.insert(RelevanceSignal::RejectsAlternative);
    }
    if contains_any_phrase(&text, CONDITIONS_FUTURE_WORK) {
        detected.insert(RelevanceSignal::ConditionsFutureWork);
    }
    if contains_any_phrase(&text, MATERIAL_BLAST_RADIUS) {
        detected.insert(RelevanceSignal::MaterialBlastRadius);
    }
    if contains_any_phrase(&text, ALTERNATIVES_COMPARED) {
        detected.insert(RelevanceSignal::AlternativesCompared);
    }
    if contains_any_phrase(&text, EXPLICIT_TRADEOFF) {
        detected.insert(RelevanceSignal::ExplicitTradeoff);
    }
    if contains_any_phrase(&text, DISAGREEMENT_UNCERTAINTY) {
        detected.insert(RelevanceSignal::DisagreementUncertainty);
    }
    if contains_any_phrase(&text, RELEVANT_COST) {
        detected.insert(RelevanceSignal::RelevantCost);
    }
    if contains_any_phrase(&text, VALIDITY_MONTHS) {
        detected.insert(RelevanceSignal::ValidityMonths);
    }
    if contains_any_phrase(&text, MAINTENANCE_ONBOARDING) {
        detected.insert(RelevanceSignal::MaintenanceOnboarding);
    }
    if contains_any_phrase(&text, DELEGATED_TO_AGENT) {
        detected.insert(RelevanceSignal::DelegatedToAgent);
    }
    if contains_any_phrase(&text, UNPROVEN_ASSUMPTION) {
        detected.insert(RelevanceSignal::UnprovenAssumption);
    }

    if changes_course {
        detected.insert(RelevanceSignal::ChangesCourse);
    }
    if picks_option {
        detected.insert(RelevanceSignal::UserPicksOption);
    }
    // Editing an ADR, a specification or an agent rules file conditions the
    // work after it, whatever else the turn reported.
    if normative_document_edited(evidence) {
        detected.insert(RelevanceSignal::ConditionsFutureWork);
    }

    let strong = detected.iter().filter(|signal| signal.is_strong()).count();
    let moderate = detected.len() - strong;
    if strong == 0 && moderate < 2 {
        return Vec::new();
    }
    detected.into_iter().collect()
}

/// Returns `true` when the user's own words change the course of the work.
/// Only `user_text` counts, because the decision lives there; what the turn
/// touched (test-only diff, trivial markers) does not veto it. A firm phrase
/// counts from any message; hints must be two distinct ones inside the same
/// message, so "foca no teste X" and, later, "troca pra nextest" do not add up.
fn user_changes_course(evidence: &DecisionEvidence) -> bool {
    evidence
        .artifacts
        .iter()
        .filter(|artifact| is_user_speech(artifact))
        .any(|artifact| {
            let text = plain_words(&artifact.content);
            contains_any_token(&text, COURSE_CHANGE)
                || COURSE_CHANGE_HINTS
                    .iter()
                    .filter(|phrase| contains_token(&text, phrase))
                    .take(2)
                    .count()
                    == 2
        })
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

/// One message folded, with typographic apostrophes made plain and whitespace
/// runs collapsed so phrases match across line breaks inside the message.
fn plain_words(content: &str) -> String {
    let mut text = String::with_capacity(content.len());
    let mut space = false;
    for ch in crate::terms::fold(content).chars() {
        if ch.is_whitespace() {
            space = true;
            continue;
        }
        if space && !text.is_empty() {
            text.push(' ');
        }
        space = false;
        text.push(if ch == '\u{2019}' { '\'' } else { ch });
    }
    text
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

/// Matched as whole words on the user's text, folded to lowercase ASCII.
const COURSE_CHANGE: &[&str] = &[
    "de lado",
    "deixa pra depois",
    "deixa para depois",
    "deixar pra depois",
    "deixar para depois",
    "fica pra depois",
    "fica para depois",
    "nao vamos mais",
    "nao vou mais",
    "nunca mais",
    "vamos parar de",
    "para de usar",
    "parar de usar",
    "desiste de",
    "desistir de",
    "desistimos",
    "abandona",
    "abandonar",
    "abandonamos",
    "adia",
    "adiar",
    "adiamos",
    "adiado",
    "adiada",
    "engaveta",
    "engavetar",
    "a partir de agora",
    "de agora em diante",
    "daqui pra frente",
    "daqui para frente",
    "daqui em diante",
    "from now on",
    "going forward",
    "let's drop",
    "lets drop",
    "let's park",
    "lets park",
    "on hold",
    "postpone",
    "postponed",
    "deprioritize",
    "we no longer",
    "we'll no longer",
    "stop using",
    "let's go with",
    "lets go with",
    "drop support for",
];

/// Common alone in operational requests ("prioriza os testes que falharam");
/// two distinct ones make a course change. No entry may contain another as
/// whole words.
const COURSE_CHANGE_HINTS: &[&str] = &[
    "prioriza",
    "priorizar",
    "priorizamos",
    "prioritize",
    "abandon",
    "por enquanto",
    "por ora",
    "seguir pra",
    "seguir para",
    "segue pra",
    "segue para",
    "focar em",
    "focar no",
    "focar na",
    "foca em",
    "foca no",
    "foca na",
    "esquece o",
    "esquece a",
    "esquece esse",
    "esquece essa",
    "troca pra",
    "troca para",
    "trocar pra",
    "trocar para",
    "muda pra",
    "muda para",
    "mudar pra",
    "mudar para",
    "em vez de",
    "ao inves de",
    "no lugar de",
    "for now",
    "switch to",
    "move on to",
    "focus on",
    "instead of",
    "rather than",
];

const TRIVIAL_MARKERS: &[&str] = &[
    "formatting",
    "format only",
    "reformat",
    "whitespace",
    "lint",
    "renamed",
    "renaming",
    "rename",
    "comment only",
    "comment-only",
    "no behavior change",
    "no behavioural change",
    "sem mudanca de comportamento",
    "sem mudança de comportamento",
];

/// Explicit choice language that lets a conversation-only capture (no diff)
/// reach the signal detection. Matched on lowercased text, accents kept.
const CHOICE_LANGUAGE: &[&str] = &[
    "decid",
    "decisão",
    "decisao",
    "optamos",
    "optei",
    "escolhemos",
    "escolhi",
    "vamos usar",
    "vamos manter",
    "em vez de",
    "ao invés de",
    "ao inves de",
    "rejeitamos",
    "instead of",
    "rather than",
    "we chose",
    "we will use",
    "we'll use",
    "going with",
    "trade-off",
    "tradeoff",
];

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

    fn said(text: &str) -> Vec<RelevanceSignal> {
        filter_relevant(&evidence(vec![artifact("a0", "user_text", text)]))
    }

    fn test_diff() -> EvidenceArtifact {
        artifact(
            "diff",
            "diff_hunk",
            "diff --git a/tests/a_test.rs b/tests/a_test.rs\n@@ -1,2 +1,3 @@\n+    let value = 1;\n",
        )
    }

    #[test]
    fn the_users_course_change_passes_despite_a_test_only_diff() {
        let signals = filter_relevant(&evidence(vec![
            artifact("a0", "user_text", REAL_USER),
            artifact("a1", "assistant_text", REAL_ASSISTANT),
            artifact(
                "a2",
                "diff_hunk",
                "diff --git a/packages/plugin/test/artifact.test.ts b/packages/plugin/test/artifact.test.ts\n@@ -1,2 +1,3 @@\n+    let value = 1;\n",
            ),
            artifact("a3", "tool_summary", "{\"status\":\"ok\",\"tool\":\"Bash\"}"),
        ]));
        assert_eq!(signals, vec![RelevanceSignal::ChangesCourse]);
    }

    #[test]
    fn a_course_change_needs_one_firm_phrase_or_two_hints() {
        assert!(said("pode seguir para o proximo passo").is_empty());
        assert!(said("esquece o redis por enquanto").contains(&RelevanceSignal::ChangesCourse));
        assert!(said("Deixa isso DE LADO").contains(&RelevanceSignal::ChangesCourse));
    }

    #[test]
    fn only_the_users_words_change_the_course() {
        let signals = filter_relevant(&evidence(vec![
            artifact("a0", "user_text", "e agora?"),
            artifact("a1", "assistant_text", REAL_USER),
        ]));
        assert!(signals.is_empty());
    }

    #[test]
    fn phrases_are_whole_words_without_accents() {
        assert!(said("não vamos mais usar X").contains(&RelevanceSignal::ChangesCourse));
        assert!(said("adianta rodar de novo?").is_empty());
        assert!(said("let\u{2019}s park the API").contains(&RelevanceSignal::ChangesCourse));
    }

    #[test]
    fn trivial_requests_stay_empty() {
        for text in [
            "roda os testes",
            "pode ser",
            "obrigado",
            "faca a limpeza vc mesmo",
            "atualiza o README",
        ] {
            let user = artifact("a0", "user_text", text);
            assert!(said(text).is_empty(), "{text}");
            assert!(
                filter_relevant(&evidence(vec![user, test_diff()])).is_empty(),
                "{text} with a test diff"
            );
        }
    }

    #[test]
    fn course_phrases_are_folded_ascii_and_hints_never_nest() {
        for phrase in COURSE_CHANGE.iter().chain(COURSE_CHANGE_HINTS) {
            assert_eq!(
                crate::terms::fold(phrase),
                *phrase,
                "{phrase} is not folded"
            );
            assert!(phrase.is_ascii(), "{phrase} is not ASCII");
        }
        for a in COURSE_CHANGE_HINTS {
            for b in COURSE_CHANGE_HINTS {
                assert!(a == b || !contains_token(a, b), "{a} contains {b}");
            }
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
            assert!(said(text).is_empty(), "{text}");
        }
        let after_reminder = "<system-reminder>x</system-reminder>\na partir de agora usa redis";
        assert!(said(after_reminder).contains(&RelevanceSignal::ChangesCourse));
    }

    fn tool(status: &str) -> EvidenceArtifact {
        artifact(
            "t0",
            "tool_summary",
            &format!("{{\"tool\":\"AskUserQuestion\",\"status\":\"{status}\"}}"),
        )
    }

    #[test]
    fn a_finished_question_to_the_user_is_a_choice_even_with_trivial_markers() {
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
    fn editing_a_normative_document_passes_despite_trivial_markers() {
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

    #[test]
    fn a_hint_repeated_across_messages_counts_once() {
        let evidence = evidence(vec![
            artifact("a0", "user_text", "por enquanto roda os testes"),
            artifact("a1", "user_text", "por enquanto so isso"),
        ]);
        assert!(filter_relevant(&evidence).is_empty());
    }

    #[test]
    fn hints_in_different_messages_do_not_add_up() {
        let apart = evidence(vec![
            artifact("a0", "user_text", "foca no teste que falhou"),
            artifact("a1", "user_text", "troca pra cargo nextest"),
        ]);
        assert!(filter_relevant(&apart).is_empty());
        let together = said("foca no cache e troca pra nextest");
        assert!(together.contains(&RelevanceSignal::ChangesCourse));
    }
}
