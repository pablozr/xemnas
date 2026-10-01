//! Decision Inbox use-case tests: list/pagination, edits, the transition
//! matrix, batch caps and the structural "no batch can accept" invariant.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use application::extract::RelevanceSignal;
use application::inbox::{
    ArtifactView, CandidateEdits, CandidateStatus, DecisionSeed, Inbox, InboxError, InboxFilter,
    InboxQuery, InboxStore, StoredCandidate, ValidatedEdits, MAX_BATCH_IDS,
};

/// A decision the fake created during a confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DecisionMock {
    decision_id: String,
    candidate_id: String,
    question: String,
    choice: String,
    rationale: String,
    version: i64,
    revisions: Vec<i64>,
    evidence: Vec<String>,
}

/// One transition action exercised by the matrix test.
type Action = fn(&Inbox<FakeInbox>, &str) -> Result<(), InboxError>;

/// In-memory store that mirrors the port semantics, including CAS updates.
/// Cloning shares the backing maps, so tests keep a handle for assertions.
#[derive(Default, Clone)]
struct FakeInbox {
    rows: Arc<Mutex<HashMap<String, StoredCandidate>>>,
    artifacts: Arc<Mutex<HashMap<(String, String), ArtifactView>>>,
    decisions: Arc<Mutex<HashMap<String, DecisionMock>>>,
}

impl FakeInbox {
    fn with(rows: Vec<StoredCandidate>) -> Self {
        let store = Self::default();
        {
            let mut map = store.rows.lock().expect("lock");
            for row in rows {
                map.insert(row.id.clone(), row);
            }
        }
        store
    }

    fn with_artifact(self, capture_id: &str, artifact_id: &str, content: &str) -> Self {
        self.artifacts.lock().expect("lock").insert(
            (capture_id.to_string(), artifact_id.to_string()),
            ArtifactView {
                artifact_id: artifact_id.to_string(),
                kind: "diff_hunk".to_string(),
                content: content.to_string(),
                metadata: "{}".to_string(),
            },
        );
        self
    }

    fn status_of(&self, id: &str) -> Option<CandidateStatus> {
        self.rows
            .lock()
            .expect("lock")
            .get(id)
            .map(|row| row.status)
    }

    fn field_of(&self, id: &str, pick: impl Fn(&StoredCandidate) -> String) -> Option<String> {
        self.rows.lock().expect("lock").get(id).map(pick)
    }

    fn decision(&self, decision_id: &str) -> Option<DecisionMock> {
        self.decisions
            .lock()
            .expect("lock")
            .get(decision_id)
            .cloned()
    }

    fn decision_count(&self) -> usize {
        self.decisions.lock().expect("lock").len()
    }
}

impl InboxStore for FakeInbox {
    fn list(&self, query: &InboxQuery) -> Result<Vec<StoredCandidate>, InboxError> {
        let rows = self.rows.lock().expect("lock");
        let mut selected: Vec<StoredCandidate> = rows
            .values()
            .filter(|row| query.statuses.contains(&row.status))
            .filter(|row| {
                query
                    .project_id
                    .as_ref()
                    .map(|project| &row.project_id == project)
                    .unwrap_or(true)
            })
            .filter(|row| match &query.before {
                Some(before) => {
                    row.created_at < before.created_at
                        || (row.created_at == before.created_at && row.id < before.id)
                }
                None => true,
            })
            .cloned()
            .collect();
        selected.sort_by(|left, right| {
            right
                .created_at
                .cmp(&left.created_at)
                .then(right.id.cmp(&left.id))
        });
        selected.truncate(query.limit);
        Ok(selected)
    }

    fn get(&self, id: &str) -> Result<Option<StoredCandidate>, InboxError> {
        Ok(self.rows.lock().expect("lock").get(id).cloned())
    }

    fn artifacts(
        &self,
        capture_id: &str,
        refs: &[String],
    ) -> Result<Vec<ArtifactView>, InboxError> {
        let artifacts = self.artifacts.lock().expect("lock");
        let mut seen = std::collections::HashSet::new();
        Ok(refs
            .iter()
            .filter(|reference| seen.insert((*reference).clone()))
            .filter_map(|reference| {
                artifacts
                    .get(&(capture_id.to_string(), reference.clone()))
                    .cloned()
            })
            .collect())
    }

    fn confirm_one(
        &self,
        id: &str,
        expected: &StoredCandidate,
        edits: Option<&ValidatedEdits>,
        seed: &DecisionSeed,
        updated_at: &str,
    ) -> Result<bool, InboxError> {
        let mut rows = self.rows.lock().expect("lock");
        let Some(row) = rows.get_mut(id) else {
            return Ok(false);
        };
        if row != expected
            || !matches!(
                row.status,
                CandidateStatus::Pending | CandidateStatus::Snoozed
            )
        {
            return Ok(false);
        }
        row.status = if let Some(edits) = edits {
            row.question = edits.question().to_string();
            row.choice = edits.choice().to_string();
            row.rationale = edits.rationale().to_string();
            CandidateStatus::EditedAndAccepted
        } else {
            CandidateStatus::Accepted
        };
        row.updated_at = updated_at.to_string();
        {
            self.decisions.lock().expect("lock").insert(
                seed.decision_id.clone(),
                DecisionMock {
                    decision_id: seed.decision_id.clone(),
                    candidate_id: id.to_string(),
                    question: seed.question.clone(),
                    choice: seed.choice.clone(),
                    rationale: seed.rationale.clone(),
                    version: 1,
                    revisions: vec![1],
                    evidence: seed.evidence_refs.clone(),
                },
            );
        }
        Ok(true)
    }

    fn adjust_one(
        &self,
        id: &str,
        edits: &ValidatedEdits,
        updated_at: &str,
    ) -> Result<bool, InboxError> {
        let mut rows = self.rows.lock().expect("lock");
        let Some(row) = rows.get_mut(id) else {
            return Ok(false);
        };
        if !matches!(
            row.status,
            CandidateStatus::Pending | CandidateStatus::Snoozed
        ) {
            return Ok(false);
        }
        row.question = edits.question().to_string();
        row.choice = edits.choice().to_string();
        row.rationale = edits.rationale().to_string();
        row.updated_at = updated_at.to_string();
        Ok(true)
    }

    fn dismiss_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        self.apply(
            id,
            &[CandidateStatus::Pending, CandidateStatus::Snoozed],
            CandidateStatus::Dismissed,
            None,
            updated_at,
        )
    }

    fn dismiss_batch(&self, ids: &[String], updated_at: &str) -> Result<usize, InboxError> {
        self.apply_batch(
            ids,
            &[CandidateStatus::Pending, CandidateStatus::Snoozed],
            CandidateStatus::Dismissed,
            updated_at,
        )
    }

    fn snooze_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        self.apply(
            id,
            &[CandidateStatus::Pending],
            CandidateStatus::Snoozed,
            None,
            updated_at,
        )
    }

    fn snooze_batch(&self, ids: &[String], updated_at: &str) -> Result<usize, InboxError> {
        self.apply_batch(
            ids,
            &[CandidateStatus::Pending],
            CandidateStatus::Snoozed,
            updated_at,
        )
    }

    fn unsnooze_one(&self, id: &str, updated_at: &str) -> Result<bool, InboxError> {
        self.apply(
            id,
            &[CandidateStatus::Snoozed],
            CandidateStatus::Pending,
            None,
            updated_at,
        )
    }
}

impl FakeInbox {
    fn apply(
        &self,
        id: &str,
        allowed_from: &[CandidateStatus],
        to: CandidateStatus,
        edits: Option<&ValidatedEdits>,
        updated_at: &str,
    ) -> Result<bool, InboxError> {
        let mut rows = self.rows.lock().expect("lock");
        let Some(row) = rows.get_mut(id) else {
            return Ok(false);
        };
        if !allowed_from.contains(&row.status) {
            return Ok(false);
        }
        row.status = to;
        row.updated_at = updated_at.to_string();
        if let Some(edits) = edits {
            row.question = edits.question().to_string();
            row.choice = edits.choice().to_string();
            row.rationale = edits.rationale().to_string();
        }
        Ok(true)
    }

    fn apply_batch(
        &self,
        ids: &[String],
        allowed_from: &[CandidateStatus],
        to: CandidateStatus,
        updated_at: &str,
    ) -> Result<usize, InboxError> {
        let mut rows = self.rows.lock().expect("lock");
        let mut affected = 0;
        for id in ids {
            let Some(row) = rows.get_mut(id) else {
                continue;
            };
            if allowed_from.contains(&row.status) {
                row.status = to;
                row.updated_at = updated_at.to_string();
                affected += 1;
            }
        }
        Ok(affected)
    }
}

fn stored(id: &str, created_at: &str, status: CandidateStatus) -> StoredCandidate {
    StoredCandidate {
        id: id.to_string(),
        project_id: "project-1".to_string(),
        project_location: "C:/synthetic/project".to_string(),
        capture_id: "capture-1".to_string(),
        status,
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        signals: "[\"public_contract\"]".to_string(),
        confidence: 0.7,
        confidence_reason: "x".to_string(),
        evidence_refs: "[]".to_string(),
        diff_summary: "{\"files\":[],\"artifacts\":0}".to_string(),
        adapter: Some("opencode".to_string()),
        session_id: Some("session-1".to_string()),
        observed_at: Some(created_at.to_string()),
        received_at: created_at.to_string(),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
        kind: "decision".to_string(),
        significance: 1.0,
        criteria: "[]".to_string(),
    }
}

fn edits(question: &str, choice: &str, rationale: &str) -> CandidateEdits {
    CandidateEdits {
        question: question.to_string(),
        choice: choice.to_string(),
        rationale: rationale.to_string(),
    }
}

#[test]
fn default_filter_lists_only_pending_and_snoozed() {
    let fake = FakeInbox::with(vec![
        stored("a", "2026-01-05T00:00:00Z", CandidateStatus::Pending),
        stored("b", "2026-01-04T00:00:00Z", CandidateStatus::Snoozed),
        stored("c", "2026-01-03T00:00:00Z", CandidateStatus::Accepted),
        stored("d", "2026-01-02T00:00:00Z", CandidateStatus::Dismissed),
    ]);
    let inbox = Inbox::new(fake);

    let page = inbox.list(&InboxFilter::new()).expect("list");
    let ids: Vec<&str> = page.candidates.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b"]);
    assert!(page.next_cursor.is_none());
    assert_eq!(
        page.candidates[0].signals,
        vec![RelevanceSignal::PublicContract]
    );
    assert_eq!(page.candidates[0].project_location, "C:/synthetic/project");
    assert_eq!(page.candidates[0].session_id.as_deref(), Some("session-1"));
}

#[test]
fn keyset_pagination_has_no_overlap_or_duplicates() {
    let rows = (0..5)
        .map(|index| {
            stored(
                &format!("id-{index}"),
                &format!("2026-01-0{}T00:00:00Z", 9 - index),
                CandidateStatus::Pending,
            )
        })
        .collect();
    let inbox = Inbox::new(FakeInbox::with(rows));

    let mut seen = Vec::new();
    let mut cursor = None;
    for _ in 0..4 {
        let page = inbox
            .list(&InboxFilter {
                limit: 2,
                cursor,
                ..InboxFilter::new()
            })
            .expect("list page");
        seen.extend(page.candidates.iter().map(|row| row.id.clone()));
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }

    assert_eq!(seen.len(), 5, "every candidate appears exactly once");
    let unique: std::collections::HashSet<&String> = seen.iter().collect();
    assert_eq!(unique.len(), 5, "no duplicates across pages");
    assert_eq!(seen, vec!["id-0", "id-1", "id-2", "id-3", "id-4"]);
}

#[test]
fn confirm_without_edits_keeps_the_fields_and_promotes() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Pending,
    )]);
    let inbox = Inbox::new(fake.clone());
    let outcome = inbox.confirm("a", None).expect("confirm");
    assert_eq!(outcome.status, CandidateStatus::Accepted);
    assert_eq!(fake.status_of("a"), Some(CandidateStatus::Accepted));
    assert_eq!(
        fake.field_of("a", |row| row.question.clone()).as_deref(),
        Some("q")
    );

    let decision = fake.decision(&outcome.decision_id).expect("decision");
    assert_eq!(decision.candidate_id, "a");
    assert_eq!(decision.version, 1, "the promotion creates version 1");
    assert_eq!(decision.revisions, vec![1], "v1 snapshot exists");
    assert_eq!(decision.question, "q");
}

#[test]
fn confirm_with_edits_persists_trimmed_fields_and_promotes() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Snoozed,
    )]);
    let inbox = Inbox::new(fake.clone());
    let outcome = inbox
        .confirm(
            "a",
            Some(edits("  nova pergunta  ", " nova escolha ", " nova razão ")),
        )
        .expect("confirm with edits");
    assert_eq!(outcome.status, CandidateStatus::EditedAndAccepted);
    assert_eq!(
        fake.status_of("a"),
        Some(CandidateStatus::EditedAndAccepted)
    );
    assert_eq!(
        fake.field_of("a", |row| row.question.clone()).as_deref(),
        Some("nova pergunta")
    );
    assert_eq!(
        fake.field_of("a", |row| row.choice.clone()).as_deref(),
        Some("nova escolha")
    );

    let decision = fake.decision(&outcome.decision_id).expect("decision");
    assert_eq!(decision.question, "nova pergunta");
    assert_eq!(decision.choice, "nova escolha");
    assert_eq!(decision.rationale, "nova razão");
}

#[test]
fn confirm_is_not_repeatable_and_creates_one_decision() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Pending,
    )]);
    let inbox = Inbox::new(fake.clone());

    inbox.confirm("a", None).expect("first confirm");
    assert_eq!(
        inbox.confirm("a", None).expect_err("second confirm").code(),
        "invalid_state"
    );
    assert_eq!(fake.decision_count(), 1, "confirmation is idempotent");
}

#[test]
fn edits_validation_rejects_empty_whitespace_and_overflow() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Pending,
    )]);
    let inbox = Inbox::new(fake.clone());

    for bad in [
        edits("", "c", "r"),
        edits("   ", "c", "r"),
        edits("q", "", "r"),
        edits("q", "c", "   "),
        edits(&"q".repeat(501), "c", "r"),
        edits("q", &"c".repeat(1_001), "r"),
        edits("q", "c", &"r".repeat(4_001)),
    ] {
        let error = inbox.confirm("a", Some(bad)).expect_err("must reject");
        assert_eq!(error.code(), "invalid_edits");
    }
    assert_eq!(fake.status_of("a"), Some(CandidateStatus::Pending));
}

#[test]
fn validated_edits_can_only_be_built_by_validation() {
    let valid = edits("  pergunta  ", " escolha ", " razão ")
        .validate()
        .expect("valid edits");
    assert_eq!(valid.question(), "pergunta");
    assert_eq!(valid.choice(), "escolha");
    assert_eq!(valid.rationale(), "razão");

    for bad in [
        edits("", "c", "r"),
        edits(" ", "c", "r"),
        edits("q", "", "r"),
        edits("q", "c", " "),
        edits(&"q".repeat(501), "c", "r"),
        edits("q", &"c".repeat(1_001), "r"),
        edits("q", "c", &"r".repeat(4_001)),
    ] {
        assert!(
            bad.validate().is_err(),
            "invalid edits must not produce a ValidatedEdits"
        );
    }
}

#[test]
fn adjust_keeps_the_status_and_updates_the_fields() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Pending,
    )]);
    let inbox = Inbox::new(fake.clone());
    inbox
        .adjust("a", edits("nova", "escolha", "razão"))
        .expect("adjust");
    assert_eq!(fake.status_of("a"), Some(CandidateStatus::Pending));
    assert_eq!(
        fake.field_of("a", |row| row.question.clone()).as_deref(),
        Some("nova")
    );
    assert_ne!(
        fake.field_of("a", |row| row.updated_at.clone()).as_deref(),
        Some("2026-01-01T00:00:00Z"),
        "adjust must touch updated_at"
    );
}

#[test]
fn transition_matrix_allows_and_rejects_the_expected_states() {
    let allowed: [(CandidateStatus, Action); 4] = [
        (CandidateStatus::Pending, |inbox, id| inbox.reject(id)),
        (CandidateStatus::Snoozed, |inbox, id| inbox.reject(id)),
        (CandidateStatus::Pending, |inbox, id| inbox.snooze(id)),
        (CandidateStatus::Snoozed, |inbox, id| inbox.unsnooze(id)),
    ];
    for (status, action) in allowed {
        let store = FakeInbox::with(vec![stored("a", "2026-01-01T00:00:00Z", status)]);
        let inbox = Inbox::new(store);
        action(&inbox, "a").unwrap_or_else(|error| panic!("{status:?}: {error}"));
    }

    for status in [
        CandidateStatus::Accepted,
        CandidateStatus::EditedAndAccepted,
        CandidateStatus::Dismissed,
    ] {
        let store = FakeInbox::with(vec![stored("a", "2026-01-01T00:00:00Z", status)]);
        let inbox = Inbox::new(store);
        assert_eq!(
            inbox.reject("a").expect_err("terminal").code(),
            "invalid_state"
        );
        assert_eq!(
            inbox.snooze("a").expect_err("terminal").code(),
            "invalid_state"
        );
        assert_eq!(
            inbox.unsnooze("a").expect_err("terminal").code(),
            "invalid_state"
        );
        assert_eq!(
            inbox.confirm("a", None).expect_err("terminal").code(),
            "invalid_state"
        );
        assert_eq!(
            inbox
                .adjust("a", edits("q", "c", "r"))
                .expect_err("terminal")
                .code(),
            "invalid_state"
        );
    }

    let store = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Snoozed,
    )]);
    let inbox = Inbox::new(store);
    assert_eq!(
        inbox.snooze("a").expect_err("already snoozed").code(),
        "invalid_state"
    );
    assert_eq!(
        inbox.unsnooze("missing").expect_err("missing").code(),
        "not_found"
    );
    assert_eq!(
        inbox.confirm("missing", None).expect_err("missing").code(),
        "not_found"
    );
}

#[test]
fn batch_caps_and_empty_lists_are_rejected() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Pending,
    )]);
    let inbox = Inbox::new(fake);

    assert_eq!(
        inbox.dismiss_batch(&[]).expect_err("empty").code(),
        "invalid_batch"
    );
    let too_many: Vec<String> = (0..=MAX_BATCH_IDS)
        .map(|index| format!("id-{index}"))
        .collect();
    assert_eq!(
        inbox.snooze_batch(&too_many).expect_err("cap").code(),
        "batch_too_large"
    );
    assert_eq!(
        inbox
            .dismiss_batch(&["missing".to_string()])
            .expect_err("missing")
            .code(),
        "not_found"
    );
}

#[test]
fn batches_affect_the_count_and_never_accept() {
    let rows = (0..4)
        .map(|index| {
            stored(
                &format!("id-{index}"),
                &format!("2026-01-0{}T00:00:00Z", index + 1),
                CandidateStatus::Pending,
            )
        })
        .collect();
    let fake = FakeInbox::with(rows);
    let inbox = Inbox::new(fake.clone());

    let dismissed = inbox
        .dismiss_batch(&["id-0".to_string(), "id-1".to_string()])
        .expect("dismiss batch");
    assert_eq!(dismissed, 2);
    let snoozed = inbox
        .snooze_batch(&["id-2".to_string(), "id-3".to_string()])
        .expect("snooze batch");
    assert_eq!(snoozed, 2);

    for id in ["id-0", "id-1", "id-2", "id-3"] {
        let status = fake.status_of(id).expect("row");
        assert!(
            matches!(
                status,
                CandidateStatus::Dismissed | CandidateStatus::Snoozed
            ),
            "{id} must never be accepted by a batch, got {status:?}"
        );
    }
}

#[test]
fn unsnooze_returns_the_candidate_to_pending() {
    let fake = FakeInbox::with(vec![stored(
        "a",
        "2026-01-01T00:00:00Z",
        CandidateStatus::Snoozed,
    )]);
    let inbox = Inbox::new(fake.clone());
    inbox.unsnooze("a").expect("unsnooze");
    assert_eq!(fake.status_of("a"), Some(CandidateStatus::Pending));
}

#[test]
fn detail_returns_artifacts_in_reference_order() {
    let mut row = stored("a", "2026-01-01T00:00:00Z", CandidateStatus::Pending);
    row.evidence_refs = "[\"b\", \"a\", \"b\"]".to_string();
    let fake = FakeInbox::with(vec![row])
        .with_artifact("capture-1", "a", "content-a")
        .with_artifact("capture-1", "b", "content-b");
    let inbox = Inbox::new(fake);

    let detail = inbox.detail("a").expect("detail");
    let ids: Vec<&str> = detail
        .artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str())
        .collect();
    assert_eq!(ids, vec!["b", "a"], "reference order, duplicates dropped");
    assert_eq!(detail.evidence_refs, vec!["b", "a", "b"]);
    assert_eq!(detail.diff_summary.artifacts, 0);
}
