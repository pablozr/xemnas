//! Approval by bands: the candidates a person would almost surely accept wait
//! a day and are then accepted for them; everything else stays in the queue.
//!
//! Off by default and never a switch to flip on faith. A person who approves
//! many AI proposals in a row approves more and looks less (the share
//! accepted rose 14.5 points over a reviewer's experience in a study of
//! 11,429 reviews of agent pull requests, while inline comments fell 22%),
//! and an automatic mode has the same flaw if it leans on an unchecked
//! number. So the mode opens only when the extractor's confidence has been
//! measured against what the person really accepted (`calibration`), and it
//! closes by itself when a blind check disagrees:
//!
//! * **Band A, held.** A pending decision with a high, calibrated confidence,
//!   a verifiable source, few files and nothing like it already recorded is
//!   *held*: it stays in the queue, marked with the moment it will be
//!   accepted (24 hours later). The person can accept or reject it earlier;
//!   only silence lets it through, and nothing reaches the agents before
//!   that.
//! * **Blind audit.** One in ten of those is *not* held: it goes to the queue
//!   like any other, without a mark, and what the person decides is compared
//!   with what the policy would have done. Rejecting a held one counts as a
//!   disagreement too.
//! * **Breaker.** When at least ten checks exist and fewer than nine in ten
//!   agree, the mode returns to manual and says why.
//! * **Never automatic.** Rules (they change what an agent does), anything
//!   that resembles a recorded question, thin evidence, many files.
//!
//! The policy is a pure function over a stored candidate; the use case reads
//! and writes through [`ApprovalStore`] and accepts through the same
//! adoption path a person's Confirmar takes, so a held candidate becomes a
//! decision tied to the map exactly as if it had been confirmed by hand.

use std::collections::BTreeSet;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::adoption::AdoptionApi;
use crate::calibration::{calibrate, Calibration, Outcome, Sample};
use crate::clock::{add_hours, now_rfc3339};
use crate::extract::{CandidateKind, MIN_SIGNIFICANCE};
use crate::inbox::{
    CandidateStatus, Cursor, InboxQuery, InboxStore, StoredCandidate, MAX_PAGE_LIMIT,
};

/// Hours a held candidate waits before it is accepted.
pub const HOLD_HOURS: i64 = 24;
/// One in this many eligible candidates is left for a blind check.
pub const AUDIT_ONE_IN: u64 = 10;
/// Checks needed before the breaker can trip.
pub const MIN_AUDITS: usize = 10;
/// Share of checks that must agree.
pub const MIN_AGREEMENT: f64 = 0.9;
/// Most files a candidate may touch and still be eligible.
pub const MAX_FILES: usize = 3;
/// Lowest confidence of the band (the lower edge of the top bin).
pub const BAND_FLOOR: f64 = 0.85;
/// Share of the top confidence bin that must have been kept.
pub const BAND_KEPT: f64 = 0.9;
/// Decisions the top bin needs before it vouches for the band.
pub const BAND_MIN_DECIDED: i64 = 10;
/// Word overlap (Jaccard) from which a question resembles another.
pub const SIMILAR_AT: f64 = 0.6;
/// Candidates read at most when looking for what resembles one.
const COMPARE_LIMIT: usize = 500;

/// How approval works.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// A person decides everything (the default).
    #[default]
    Manual,
    /// Band A candidates are held and then accepted.
    Automatic,
}

impl Mode {
    /// The literal persisted in the settings table.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Automatic => "automatic",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "manual" => Some(Self::Manual),
            "automatic" => Some(Self::Automatic),
            _ => None,
        }
    }
}

/// What the policy did with a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// Waiting to be accepted unless the person acts first.
    Held,
    /// Left for a blind check.
    Audit,
}

impl Lane {
    /// The literal persisted in the `lane` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Audit => "audit",
        }
    }

    /// Parses a persisted literal.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "held" => Some(Self::Held),
            "audit" => Some(Self::Audit),
            _ => None,
        }
    }
}

/// The policy's record about one candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The candidate.
    pub candidate_id: String,
    /// Its project.
    pub project_id: String,
    /// What the policy did.
    pub lane: Lane,
    /// RFC 3339 moment the policy decided.
    pub created_at: String,
    /// Held: when it is accepted if still pending.
    pub due_at: Option<String>,
    /// Held: when the policy accepted it.
    pub resolved_at: Option<String>,
    /// The decision the acceptance created.
    pub decision_id: Option<String>,
}

/// One line of the ledger of what was accepted or is waiting to be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRow {
    /// The candidate.
    pub candidate_id: String,
    /// Its question.
    pub question: String,
    /// RFC 3339 moment it is accepted if still pending.
    pub due_at: Option<String>,
    /// RFC 3339 moment the policy accepted it, when it did.
    pub accepted_at: Option<String>,
    /// The decision created, when accepted.
    pub decision_id: Option<String>,
}

/// How the checks fared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Health {
    /// Checks a person has answered (blind audits and held ones decided by
    /// hand).
    pub checked: usize,
    /// Of those, the ones that agreed with the policy.
    pub agreed: usize,
}

impl Health {
    /// Share that agreed, when any check exists.
    pub fn agreement(&self) -> Option<f64> {
        (self.checked > 0).then(|| self.agreed as f64 / self.checked as f64)
    }

    /// Whether the checks disagree enough to close the automatic mode.
    pub fn tripped(&self) -> bool {
        self.checked >= MIN_AUDITS && self.agreement().is_some_and(|share| share < MIN_AGREEMENT)
    }
}

/// Why the automatic mode cannot be on right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    /// The extractor's confidence has not been shown to predict acceptance.
    NotCalibrated,
    /// The top confidence band has too few decisions or too few kept.
    BandUnproven,
    /// Blind checks disagreed with the policy.
    Tripped,
}

/// Where the automatic mode stands, for the settings page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Status {
    /// The saved mode.
    pub automatic: bool,
    /// Decided candidates measured.
    pub decided: i64,
    /// Whether the calibration allows the mode.
    pub predicts: bool,
    /// Whether the top band vouches for itself.
    pub band_proven: bool,
    /// How the checks fared.
    pub health: Health,
    /// Candidates held right now.
    pub held: usize,
}

impl Status {
    /// Why the mode cannot be turned on, if it cannot.
    pub fn blocked(&self) -> Option<Blocked> {
        if self.health.tripped() {
            Some(Blocked::Tripped)
        } else if !self.predicts {
            Some(Blocked::NotCalibrated)
        } else if !self.band_proven {
            Some(Blocked::BandUnproven)
        } else {
            None
        }
    }
}

/// What one pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunReport {
    /// Candidates newly held.
    pub held: usize,
    /// Candidates newly left for a blind check.
    pub audited: usize,
    /// Held candidates accepted by the policy.
    pub accepted: usize,
    /// The breaker closed the mode on this pass.
    pub tripped: bool,
}

/// Failure modes of the approval use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalError {
    /// A storage query failed; the message is diagnostic only.
    Storage(String),
    /// The automatic mode cannot be turned on yet.
    Blocked(Blocked),
    /// Accepting a held candidate failed.
    Adoption(String),
}

impl std::fmt::Display for ApprovalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(message) => write!(formatter, "falha no armazenamento: {message}"),
            Self::Blocked(_) => write!(
                formatter,
                "a aprovação automática ainda não pode ser ligada"
            ),
            Self::Adoption(message) => write!(formatter, "não foi possível aceitar: {message}"),
        }
    }
}

impl std::error::Error for ApprovalError {}

impl From<crate::inbox::InboxError> for ApprovalError {
    fn from(error: crate::inbox::InboxError) -> Self {
        Self::Storage(error.to_string())
    }
}

/// Persistence the approval policy needs besides the inbox.
pub trait ApprovalStore {
    /// The saved mode (manual when nothing was saved).
    fn approval_mode(&self) -> Result<Mode, ApprovalError>;

    /// Saves the mode.
    fn set_approval_mode(&self, mode: Mode, at: &str) -> Result<(), ApprovalError>;

    /// The policy's record about a candidate, if it has one.
    fn auto_entry(&self, candidate_id: &str) -> Result<Option<Entry>, ApprovalError>;

    /// Records what the policy did with a candidate.
    fn insert_auto_entry(&self, entry: &Entry) -> Result<(), ApprovalError>;

    /// Held entries of a project not yet resolved whose moment has come.
    fn due_auto_entries(&self, project_id: &str, now: &str) -> Result<Vec<Entry>, ApprovalError>;

    /// Marks a held entry as resolved (accepted by the policy, or settled by
    /// the person first when `decision_id` is `None`).
    fn resolve_auto_entry(
        &self,
        candidate_id: &str,
        decision_id: Option<&str>,
        at: &str,
    ) -> Result<(), ApprovalError>;

    /// The ledger: held and accepted entries of a project, newest first.
    fn ledger(&self, project_id: &str, limit: usize) -> Result<Vec<LedgerRow>, ApprovalError>;

    /// Every entry of a project with the candidate's present status and
    /// whether the policy itself accepted it (`resolved_at` set).
    fn auto_outcomes(
        &self,
        project_id: Option<&str>,
    ) -> Result<Vec<(Lane, CandidateStatus, bool)>, ApprovalError>;
}

/// Object-safe entry point for the desktop.
pub trait ApprovalsApi: Send + Sync {
    /// Where the mode stands.
    fn status(&self) -> Result<Status, ApprovalError>;
    /// Turns the mode on or off; on only when nothing blocks it.
    fn set_mode(&self, mode: Mode) -> Result<(), ApprovalError>;
    /// One pass for a project: hold, audit and accept what is due.
    fn run(&self, project_id: &str) -> Result<RunReport, ApprovalError>;
    /// What is held or was accepted by the policy.
    fn ledger(&self, project_id: &str) -> Result<Vec<LedgerRow>, ApprovalError>;
}

/// The approval use case.
#[derive(Clone)]
pub struct Approvals<S> {
    store: S,
    adoption: Arc<dyn AdoptionApi>,
}

/// The words of a question, lowercase, without short noise words.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
        .filter(|word| word.chars().count() > 2)
        .collect()
}

/// Overlap of two word sets, 0 to 1.
pub fn similarity(a: &str, b: &str) -> f64 {
    let (a, b) = (words(a), words(b));
    let union = a.union(&b).count();
    if union == 0 {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / union as f64
}

/// Deterministic one-in-`AUDIT_ONE_IN` choice from the candidate id (FNV-1a).
pub fn is_audited(candidate_id: &str) -> bool {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in candidate_id.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash.is_multiple_of(AUDIT_ONE_IN)
}

/// Whether the top confidence band has earned the trust the policy needs.
pub fn band_proven(calibration: &Calibration) -> bool {
    calibration.verdict.allows_automation()
        && calibration.bins.last().is_some_and(|bin| {
            bin.total >= BAND_MIN_DECIDED
                && bin.kept_share().is_some_and(|share| share >= BAND_KEPT)
        })
}

/// Whether a stored candidate is in band A, given the calibration and the
/// questions already recorded in its project.
pub fn eligible(
    candidate: &StoredCandidate,
    calibration: &Calibration,
    recorded_questions: &[String],
) -> bool {
    if !band_proven(calibration)
        || candidate.status != CandidateStatus::Pending
        || candidate.kind != CandidateKind::Decision.as_str()
        || candidate.confidence < BAND_FLOOR
        || candidate.significance < MIN_SIGNIFICANCE
    {
        return false;
    }
    let sources = serde_json::from_str::<Vec<serde_json::Value>>(&candidate.evidence_refs)
        .map(|refs| refs.len())
        .unwrap_or(0);
    let files = serde_json::from_str::<serde_json::Value>(&candidate.diff_summary)
        .ok()
        .and_then(|summary| {
            summary
                .get("files")
                .and_then(|files| files.as_array().map(Vec::len))
        })
        .unwrap_or(0);
    sources >= 1
        && files <= MAX_FILES
        && !recorded_questions
            .iter()
            .any(|question| similarity(question, &candidate.question) >= SIMILAR_AT)
}

impl<S> Approvals<S>
where
    S: ApprovalStore + InboxStore + Clone,
{
    /// Wraps the store and the adoption path used to accept.
    pub fn new(store: S, adoption: Arc<dyn AdoptionApi>) -> Self {
        Self { store, adoption }
    }

    /// Every decided candidate, as calibration samples.
    fn samples(&self) -> Result<Vec<Sample>, ApprovalError> {
        let mut samples = Vec::new();
        let mut query = InboxQuery {
            min_significance: None,
            project_id: None,
            statuses: vec![
                CandidateStatus::Accepted,
                CandidateStatus::EditedAndAccepted,
                CandidateStatus::Dismissed,
            ],
            limit: MAX_PAGE_LIMIT,
            before: None,
        };
        loop {
            let rows = self.store.list(&query)?;
            samples.extend(rows.iter().map(|row| Sample {
                confidence: row.confidence,
                significance: row.significance,
                outcome: match row.status {
                    CandidateStatus::Accepted => Outcome::Accepted,
                    CandidateStatus::EditedAndAccepted => Outcome::Edited,
                    _ => Outcome::Dismissed,
                },
            }));
            match rows.last() {
                Some(last) if rows.len() == query.limit => {
                    query.before = Some(Cursor {
                        created_at: last.created_at.clone(),
                        id: last.id.clone(),
                    });
                }
                _ => return Ok(samples),
            }
        }
    }

    /// How the confidence relates to what was decided.
    pub fn calibration(&self) -> Result<Calibration, ApprovalError> {
        Ok(calibrate(&self.samples()?))
    }

    /// How the checks fared across the projects.
    pub fn health(&self) -> Result<Health, ApprovalError> {
        let mut health = Health::default();
        for (lane, status, by_policy) in self.store.auto_outcomes(None)? {
            let kept = matches!(
                status,
                CandidateStatus::Accepted | CandidateStatus::EditedAndAccepted
            );
            let answered = match lane {
                // A blind audit counts once a person decided it.
                Lane::Audit => kept || status == CandidateStatus::Dismissed,
                // A held one counts when the person decided before the
                // policy did; the policy's own acceptance proves nothing.
                Lane::Held => !by_policy && (kept || status == CandidateStatus::Dismissed),
            };
            if answered {
                health.checked += 1;
                health.agreed += usize::from(kept);
            }
        }
        Ok(health)
    }

    /// Candidates of a project, by status.
    fn candidates(
        &self,
        project_id: &str,
        statuses: Vec<CandidateStatus>,
        limit: usize,
    ) -> Result<Vec<StoredCandidate>, ApprovalError> {
        let mut found = Vec::new();
        let mut query = InboxQuery {
            min_significance: None,
            project_id: Some(project_id.to_owned()),
            statuses,
            limit: MAX_PAGE_LIMIT,
            before: None,
        };
        while found.len() < limit {
            let rows = self.store.list(&query)?;
            let full = rows.len() == query.limit;
            if let Some(last) = rows.last() {
                query.before = Some(Cursor {
                    created_at: last.created_at.clone(),
                    id: last.id.clone(),
                });
            }
            found.extend(rows);
            if !full {
                break;
            }
        }
        found.truncate(limit);
        Ok(found)
    }

    /// One pass at the given moment (RFC 3339), so tests do not wait a day.
    pub fn run_at(&self, project_id: &str, now: &str) -> Result<RunReport, ApprovalError> {
        let mut report = RunReport::default();
        if self.store.approval_mode()? != Mode::Automatic {
            return Ok(report);
        }
        if self.health()?.tripped() {
            self.store.set_approval_mode(Mode::Manual, now)?;
            report.tripped = true;
            return Ok(report);
        }
        let calibration = self.calibration()?;
        if !band_proven(&calibration) {
            return Ok(report);
        }

        // Hold or audit what is eligible and not yet recorded.
        let pending = self.candidates(project_id, vec![CandidateStatus::Pending], COMPARE_LIMIT)?;
        let recorded: Vec<(String, String)> = self
            .candidates(
                project_id,
                vec![
                    CandidateStatus::Accepted,
                    CandidateStatus::EditedAndAccepted,
                    CandidateStatus::Dismissed,
                    CandidateStatus::Snoozed,
                ],
                COMPARE_LIMIT,
            )?
            .into_iter()
            .map(|row| (row.id, row.question))
            .collect();
        for candidate in &pending {
            if self.store.auto_entry(&candidate.id)?.is_some() {
                continue;
            }
            // What it must not resemble: what was decided and what else waits.
            let others: Vec<String> = recorded
                .iter()
                .map(|(_, question)| question.clone())
                .chain(
                    pending
                        .iter()
                        .filter(|other| other.id != candidate.id)
                        .map(|other| other.question.clone()),
                )
                .collect();
            if !eligible(candidate, &calibration, &others) {
                continue;
            }
            let lane = if is_audited(&candidate.id) {
                Lane::Audit
            } else {
                Lane::Held
            };
            let due_at = (lane == Lane::Held)
                .then(|| add_hours(now, HOLD_HOURS))
                .flatten();
            self.store.insert_auto_entry(&Entry {
                candidate_id: candidate.id.clone(),
                project_id: project_id.to_owned(),
                lane,
                created_at: now.to_owned(),
                due_at,
                resolved_at: None,
                decision_id: None,
            })?;
            match lane {
                Lane::Held => report.held += 1,
                Lane::Audit => report.audited += 1,
            }
        }

        // Accept what has waited out its day and is still pending.
        for entry in self.store.due_auto_entries(project_id, now)? {
            let still_pending = self
                .store
                .get(&entry.candidate_id)?
                .is_some_and(|row| row.status == CandidateStatus::Pending);
            if !still_pending {
                self.store
                    .resolve_auto_entry(&entry.candidate_id, None, now)?;
                continue;
            }
            let links = self
                .adoption
                .preview(&entry.candidate_id)
                .map(|preview| preview.links)
                .unwrap_or_default();
            let outcome = self
                .adoption
                .adopt(&entry.candidate_id, None, &links, &[])
                .map_err(|error| ApprovalError::Adoption(error.to_string()))?;
            self.store
                .resolve_auto_entry(&entry.candidate_id, Some(&outcome.id), now)?;
            report.accepted += 1;
        }
        Ok(report)
    }
}

impl<S> ApprovalsApi for Approvals<S>
where
    S: ApprovalStore + InboxStore + Clone + Send + Sync,
{
    fn status(&self) -> Result<Status, ApprovalError> {
        let calibration = self.calibration()?;
        let held = self
            .store
            .auto_outcomes(None)?
            .iter()
            .filter(|(lane, status, by_policy)| {
                *lane == Lane::Held && !*by_policy && *status == CandidateStatus::Pending
            })
            .count();
        Ok(Status {
            automatic: self.store.approval_mode()? == Mode::Automatic,
            decided: calibration.decided,
            predicts: calibration.verdict.allows_automation(),
            band_proven: band_proven(&calibration),
            health: self.health()?,
            held,
        })
    }

    fn set_mode(&self, mode: Mode) -> Result<(), ApprovalError> {
        if mode == Mode::Automatic {
            if let Some(blocked) = self.status()?.blocked() {
                return Err(ApprovalError::Blocked(blocked));
            }
        }
        self.store.set_approval_mode(mode, &now_rfc3339())
    }

    fn run(&self, project_id: &str) -> Result<RunReport, ApprovalError> {
        self.run_at(project_id, &now_rfc3339())
    }

    fn ledger(&self, project_id: &str) -> Result<Vec<LedgerRow>, ApprovalError> {
        self.store.ledger(project_id, 50)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::{Bin, Verdict};

    fn proven() -> Calibration {
        Calibration {
            decided: 80,
            bins: vec![
                Bin::default(),
                Bin::default(),
                Bin::default(),
                Bin {
                    from: 0.85,
                    to: 1.0,
                    total: 30,
                    accepted: 28,
                    edited: 0,
                    dismissed: 2,
                },
            ],
            confidence_separation: Some(0.82),
            verdict: Verdict::Predicts,
            ..Calibration::default()
        }
    }

    fn candidate(question: &str) -> StoredCandidate {
        StoredCandidate {
            kind: "decision".into(),
            significance: 0.9,
            criteria: "[]".into(),
            id: "c1".into(),
            project_id: "p".into(),
            project_location: "C:/p".into(),
            capture_id: "cap".into(),
            status: CandidateStatus::Pending,
            question: question.into(),
            choice: "x".into(),
            rationale: "y".into(),
            signals: "[]".into(),
            confidence: 0.9,
            confidence_reason: "z".into(),
            evidence_refs: "[\"art-1\"]".into(),
            diff_summary: "{\"files\":[\"a.rs\"],\"artifacts\":1}".into(),
            adapter: None,
            session_id: None,
            observed_at: None,
            received_at: "2026-01-01T00:00:00Z".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn a_confident_sourced_novel_decision_is_in_the_band() {
        let recorded = vec!["Qual fila usar para os jobs?".to_owned()];
        assert!(eligible(
            &candidate("Como versionar as decisões revisadas?"),
            &proven(),
            &recorded
        ));
    }

    #[test]
    fn each_condition_keeps_a_candidate_out() {
        let calibration = proven();
        let none: Vec<String> = Vec::new();
        let base = candidate("Como versionar as decisões revisadas?");
        assert!(eligible(&base, &calibration, &none));

        let mut low = base.clone();
        low.confidence = 0.84;
        assert!(!eligible(&low, &calibration, &none), "below the band");
        let mut rule = base.clone();
        rule.kind = "rule".into();
        assert!(
            !eligible(&rule, &calibration, &none),
            "rules are never automatic"
        );
        let mut unsourced = base.clone();
        unsourced.evidence_refs = "[]".into();
        assert!(!eligible(&unsourced, &calibration, &none), "no source");
        let mut wide = base.clone();
        wide.diff_summary = "{\"files\":[\"a\",\"b\",\"c\",\"d\"]}".into();
        assert!(!eligible(&wide, &calibration, &none), "too many files");
        let mut snoozed = base.clone();
        snoozed.status = CandidateStatus::Snoozed;
        assert!(!eligible(&snoozed, &calibration, &none), "only pending");
        let twin = vec!["Como versionar as decisões revisadas?".to_owned()];
        assert!(!eligible(&base, &calibration, &twin), "already recorded");
        assert!(
            !eligible(&base, &Calibration::default(), &none),
            "no calibration, no automation"
        );
    }

    #[test]
    fn similarity_is_word_overlap_and_ignores_case_and_short_words() {
        assert_eq!(
            similarity("Onde guardar a chave?", "onde GUARDAR a chave"),
            1.0
        );
        assert!(similarity("Qual banco usar?", "Como versionar decisões?") < 0.1);
        assert_eq!(similarity("", ""), 0.0);
    }

    #[test]
    fn about_one_in_ten_ids_is_audited_and_the_choice_is_stable() {
        let audited = (0..2000)
            .filter(|n| is_audited(&format!("candidate-{n}")))
            .count();
        assert!((130..270).contains(&audited), "{audited} of 2000");
        assert_eq!(is_audited("candidate-7"), is_audited("candidate-7"));
    }

    #[test]
    fn the_breaker_needs_enough_checks_and_a_real_disagreement() {
        let few = Health {
            checked: 9,
            agreed: 0,
        };
        assert!(!few.tripped(), "nine checks prove nothing");
        let good = Health {
            checked: 20,
            agreed: 19,
        };
        assert!(!good.tripped());
        let bad = Health {
            checked: 20,
            agreed: 17,
        };
        assert!(bad.tripped());
        assert_eq!(Health::default().agreement(), None);
    }

    #[test]
    fn the_band_needs_a_predicting_confidence_and_a_proven_top_bin() {
        assert!(band_proven(&proven()));
        let mut thin = proven();
        thin.bins[3].total = 5;
        assert!(!band_proven(&thin));
        let mut shaky = proven();
        shaky.bins[3].accepted = 20;
        shaky.bins[3].dismissed = 10;
        assert!(!band_proven(&shaky));
        let mut weak = proven();
        weak.verdict = Verdict::Weak;
        assert!(!band_proven(&weak));
    }

    #[test]
    fn status_says_why_the_mode_is_blocked() {
        let base = Status {
            automatic: false,
            decided: 10,
            predicts: false,
            band_proven: false,
            health: Health::default(),
            held: 0,
        };
        assert_eq!(base.blocked(), Some(Blocked::NotCalibrated));
        let calibrated = Status {
            predicts: true,
            ..base.clone()
        };
        assert_eq!(calibrated.blocked(), Some(Blocked::BandUnproven));
        let ready = Status {
            band_proven: true,
            ..calibrated.clone()
        };
        assert_eq!(ready.blocked(), None);
        let tripped = Status {
            health: Health {
                checked: 12,
                agreed: 8,
            },
            ..ready
        };
        assert_eq!(tripped.blocked(), Some(Blocked::Tripped));
    }
}
