//! Approval by bands against the real database: what is held, accepted after
//! its day, audited blind, and what closes the mode.

mod support;

use std::sync::Arc;

use application::adoption::{Adoption, AdoptionApi};
use application::auto_approval::{
    is_audited, ApprovalError, Approvals, ApprovalsApi, Blocked, Mode, HOLD_HOURS,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::inbox::{CandidateStatus, Inbox};
use storage_sqlite::SqliteStore;

const PROJECT: &str = "p1";
const NOW: &str = "2026-03-01T10:00:00Z";
const NEXT_DAY: &str = "2026-03-02T10:00:01Z";

fn approvals(store: &SqliteStore) -> Approvals<SqliteStore> {
    let adoption: Arc<dyn AdoptionApi> = Arc::new(Adoption::new(store.clone()));
    Approvals::new(store.clone(), adoption)
}

fn candidate(id: &str, status: &str, confidence: f64, question: &str) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        id: id.to_string(),
        project_id: PROJECT.to_string(),
        capture_id: format!("capture-{PROJECT}"),
        status: status.to_string(),
        question: question.to_string(),
        choice: "escolha".to_string(),
        rationale: "motivo".to_string(),
        signals: "[\"public_contract\"]".to_string(),
        confidence,
        confidence_reason: "sintético".to_string(),
        evidence_refs: format!("[\"art-{PROJECT}\"]"),
        diff_summary: "{\"files\":[\"src/lib.rs\"],\"artifacts\":1}".to_string(),
        dedup_hash: format!("dedup-{id}"),
        created_at: "2026-02-01T00:00:00Z".to_string(),
        updated_at: "2026-02-01T00:00:00Z".to_string(),
        kind: "decision".to_string(),
        significance: 1.0,
        criteria: "[]".to_string(),
    }
}

/// 80 decided candidates whose confidence orders the outcomes: the top band
/// is kept, the bottom one dismissed. Their questions share no words.
fn seed_calibration(store: &SqliteStore) {
    let mut rows = Vec::new();
    for at in 0..40 {
        rows.push(candidate(
            &format!("hi-{at}"),
            "accepted",
            0.86 + at as f64 * 0.003,
            &format!("topico{at} alto{at} resolvido{at}"),
        ));
        rows.push(candidate(
            &format!("lo-{at}"),
            "dismissed",
            0.30 + at as f64 * 0.004,
            &format!("tema{at} baixo{at} descartado{at}"),
        ));
    }
    store.insert_candidates(&rows).expect("seed decided");
}

/// An id the policy leaves for a blind check, or not.
fn id_with(audited: bool, prefix: &str) -> String {
    (0..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|id| is_audited(id) == audited)
        .expect("an id")
}

#[test]
fn the_automatic_mode_cannot_be_turned_on_without_a_calibration() {
    let test = support::open("auto-blocked", &[PROJECT]);
    let approvals = approvals(&test.store);
    assert_eq!(
        approvals.set_mode(Mode::Automatic),
        Err(ApprovalError::Blocked(Blocked::NotCalibrated))
    );
    let status = approvals.status().expect("status");
    assert!(!status.automatic);
    assert_eq!(status.blocked(), Some(Blocked::NotCalibrated));
    // Manual stays available, and a pass in manual mode does nothing.
    approvals.set_mode(Mode::Manual).expect("manual");
    assert_eq!(
        approvals.run_at(PROJECT, NOW).expect("run"),
        Default::default()
    );
}

#[test]
fn a_calibrated_band_holds_then_accepts_after_the_day_and_a_person_can_act_first() {
    let test = support::open("auto-hold", &[PROJECT]);
    seed_calibration(&test.store);
    let approvals = approvals(&test.store);
    approvals.set_mode(Mode::Automatic).expect("turn on");
    assert!(approvals.status().expect("status").automatic);

    let held = id_with(false, "novel-a");
    let early = id_with(false, "novel-b");
    let weak = id_with(false, "weak");
    test.store
        .insert_candidates(&[
            candidate(&held, "pending", 0.95, "como versionar revisoes passadas"),
            candidate(&early, "pending", 0.93, "onde guardar credenciais externas"),
            candidate(
                &weak,
                "pending",
                0.60,
                "qual formato para exportar relatorios",
            ),
        ])
        .expect("pending");

    let report = approvals.run_at(PROJECT, NOW).expect("first pass");
    assert_eq!((report.held, report.audited, report.accepted), (2, 0, 0));
    let ledger = approvals.ledger(PROJECT).expect("ledger");
    assert_eq!(ledger.len(), 2, "the weak one is not in the band");
    assert!(ledger.iter().all(|row| row.accepted_at.is_none()));
    assert!(ledger
        .iter()
        .all(|row| row.due_at.as_deref() == Some("2026-03-02T10:00:00Z")));
    assert_eq!(HOLD_HOURS, 24);

    // Nothing is decided before the day is out.
    let again = approvals
        .run_at(PROJECT, "2026-03-02T09:00:00Z")
        .expect("again");
    assert_eq!((again.held, again.accepted), (0, 0));
    let inbox = Inbox::new(test.store.clone());
    inbox.reject(&early).expect("the person rejects one early");

    let after = approvals.run_at(PROJECT, NEXT_DAY).expect("after the day");
    assert_eq!(after.accepted, 1, "only the one still pending is accepted");
    let rows = approvals.ledger(PROJECT).expect("ledger");
    let accepted: Vec<_> = rows
        .iter()
        .filter(|row| row.decision_id.is_some())
        .collect();
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].candidate_id, held);
    let status_of = |id: &str| {
        use application::inbox::InboxStore;
        test.store.get(id).expect("get").expect("row").status
    };
    assert_eq!(status_of(&held), CandidateStatus::Accepted);
    assert_eq!(status_of(&early), CandidateStatus::Dismissed);
    assert_eq!(status_of(&weak), CandidateStatus::Pending);
    // Rejecting a held candidate is a disagreement with the policy.
    let health = approvals.status().expect("status").health;
    assert_eq!((health.checked, health.agreed), (1, 0));
}

#[test]
fn a_candidate_that_resembles_a_recorded_question_stays_in_the_queue() {
    let test = support::open("auto-similar", &[PROJECT]);
    seed_calibration(&test.store);
    let approvals = approvals(&test.store);
    approvals.set_mode(Mode::Automatic).expect("turn on");
    let id = id_with(false, "twin");
    test.store
        .insert_candidates(&[candidate(
            &id,
            "pending",
            0.97,
            "topico3 alto3 resolvido3 de novo",
        )])
        .expect("pending");
    let report = approvals.run_at(PROJECT, NOW).expect("run");
    assert_eq!((report.held, report.audited), (0, 0));
}

#[test]
fn one_in_ten_is_left_for_a_blind_check_and_disagreement_closes_the_mode() {
    let test = support::open("auto-breaker", &[PROJECT]);
    seed_calibration(&test.store);
    let approvals = approvals(&test.store);
    approvals.set_mode(Mode::Automatic).expect("turn on");

    // Twelve audited candidates the person then rejects: the policy was wrong.
    let mut rows = Vec::new();
    for at in 0..12 {
        rows.push(candidate(
            &id_with(true, &format!("audit-{at}")),
            "pending",
            0.96,
            &format!("assunto{at} inedito{at} sem{at} parecido{at}"),
        ));
    }
    test.store.insert_candidates(&rows).expect("pending");
    let report = approvals.run_at(PROJECT, NOW).expect("run");
    assert_eq!(
        (report.audited, report.held),
        (12, 0),
        "audited are not held"
    );
    assert!(approvals.ledger(PROJECT).expect("ledger").is_empty());
    let inbox = Inbox::new(test.store.clone());
    for row in &rows {
        inbox.reject(&row.id).expect("reject");
    }
    let health = approvals.status().expect("status").health;
    assert_eq!((health.checked, health.agreed), (12, 0));
    assert!(health.tripped());

    let closing = approvals.run_at(PROJECT, NEXT_DAY).expect("closing pass");
    assert!(closing.tripped);
    assert!(!approvals.status().expect("status").automatic);
    assert_eq!(
        approvals.set_mode(Mode::Automatic),
        Err(ApprovalError::Blocked(Blocked::Tripped)),
        "it stays closed until the checks agree again"
    );
}
