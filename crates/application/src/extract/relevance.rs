//! Cheap, deterministic relevance filter (MVP-SPEC §6).

use std::collections::BTreeSet;

use serde_json::json;

use super::{DecisionEvidence, EvidenceArtifact};

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
        )
    }
}

/// Returns `true` when the capture is one of the §6 exclusions.
fn is_trivially_excluded(evidence: &DecisionEvidence, text: &str) -> bool {
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
    for artifact in &evidence.artifacts {
        if artifact.kind != "diff_hunk" {
            continue;
        }
        if artifact_has_dependency_addition(artifact) {
            return true;
        }
        let mut current_is_test = false;
        for line in artifact.content.lines() {
            if let Some(rest) = line.strip_prefix("diff --git ") {
                let token = rest.split_whitespace().next().unwrap_or("");
                let path = token.strip_prefix("a/").unwrap_or(token);
                current_is_test = is_test_path(path);
                continue;
            }
            let trimmed = line.trim_start();
            let Some(rest) = trimmed.strip_prefix('+') else {
                continue;
            };
            if current_is_test || rest.starts_with("++") {
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

/// Returns `true` when the artifact adds a dependency in a manifest.
fn artifact_has_dependency_addition(artifact: &EvidenceArtifact) -> bool {
    let mut current_manifest = false;
    let mut current_is_test = false;
    for line in artifact.content.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            let token = rest.split_whitespace().next().unwrap_or("");
            let path = token.strip_prefix("a/").unwrap_or(token);
            let lower = path.to_ascii_lowercase();
            current_is_test = is_test_path(path);
            current_manifest = lower.ends_with("cargo.toml") || lower.ends_with("package.json");
            continue;
        }
        if !current_manifest || current_is_test {
            continue;
        }
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('+') else {
            continue;
        };
        if rest.starts_with("++") {
            continue;
        }
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

    if is_trivially_excluded(evidence, &text) {
        return Vec::new();
    }

    let mut detected = BTreeSet::new();

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

    let strong = detected.iter().filter(|signal| signal.is_strong()).count();
    let moderate = detected.len() - strong;
    if strong == 0 && moderate < 2 {
        return Vec::new();
    }
    detected.into_iter().collect()
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

/// Sorted unique file paths found in `diff --git` headers.
fn diff_files(evidence: &DecisionEvidence) -> BTreeSet<String> {
    let mut files = BTreeSet::new();
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
