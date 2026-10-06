//! Inbox persistence tests: the join behind the list, the artifact order in the
//! detail, compare-and-set exclusivity and the default status filter.

use application::captures::{
    CaptureArtifactRecord, CaptureCheckpointRecord, CaptureReceiptRecord, CaptureRepository,
    CaptureWrite,
};
use application::extract::{DecisionCandidateRecord, ExtractionStore};
use application::inbox::{
    CandidateEdits, CandidateStatus, DecisionSeed, Inbox, InboxFilter, InboxStore,
};
use application::jobs::{JobRecord, JobState, ANALYZE_CAPTURE_KIND};
use application::projects::{ProjectRecord, ProjectRepository};
use storage_sqlite::SqliteStore;

const LOCATION: &str = "C:/synthetic/inbox";

#[test]
fn per_id_review_eligibility_ignores_page_but_respects_project_threshold_and_resolution() {
    use application::review_exception::CandidateNature;
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "eligibility", "eligibility-message");
    let first = candidate(
        "eligible-a",
        "eligibility",
        "2026-01-01T00:00:00Z",
        "pending",
    );
    let mut sibling = first.clone();
    sibling.id = "eligible-b".into();
    sibling.dedup_hash = "eligibility-b".into();
    let mut other = first.clone();
    other.id = "newest".into();
    other.dedup_hash = "eligibility-newest".into();
    other.question = "another group".into();
    other.created_at = "2026-02-01T00:00:00Z".into();
    let mut inference = first.clone();
    inference.id = "hidden-inference".into();
    inference.dedup_hash = "eligibility-inference".into();
    inference.question = "inference group".into();
    let mut low = first.clone();
    low.id = "low".into();
    low.dedup_hash = "eligibility-low".into();
    low.question = "low relevance".into();
    low.significance = 0.1;
    store
        .insert_candidates(&[first, sibling, other, inference, low])
        .expect("insert");
    store
        .record_nature("eligibility-inference", CandidateNature::Inference)
        .expect("nature");
    let inbox = Inbox::new(store);
    let mut filter = InboxFilter {
        limit: 1,
        min_significance: Some(0.5),
        ..Default::default()
    };
    assert_eq!(inbox.list(&filter).unwrap().candidates[0].id, "newest");
    filter.cursor = Some("2099-01-01T00:00:00Z|ignored".into());
    assert!(inbox
        .eligible_in_review("project-1", "eligible-b", &filter)
        .unwrap());
    assert!(!inbox
        .eligible_in_review("project-1", "eligible-a", &filter)
        .unwrap());
    assert!(!inbox
        .eligible_in_review("other-project", "eligible-b", &filter)
        .unwrap());
    assert!(!inbox
        .eligible_in_review("project-1", "hidden-inference", &filter)
        .unwrap());
    assert!(!inbox
        .eligible_in_review("project-1", "low", &filter)
        .unwrap());
    filter.min_significance = None;
    assert!(inbox
        .eligible_in_review("project-1", "low", &filter)
        .unwrap());
    inbox
        .confirm("eligible-b", None)
        .expect("confirm representative");
    assert!(!inbox
        .eligible_in_review("project-1", "eligible-a", &filter)
        .unwrap());
}

#[test]
fn group_projection_latency_is_reported_without_performance_threshold_claims() {
    for size in [800, 8000] {
        let root = temporary_directory("projection-benchmark");
        let path = root.join("app.db");
        let store = SqliteStore::open(&path).expect("store");
        seed_project(&store);
        seed_capture(&store, "latency", "latency-message");
        let rows: Vec<_> = (0..size)
            .map(|i| {
                candidate(
                    &format!("latency-{i:05}"),
                    "latency",
                    "2026-01-01T00:00:00Z",
                    "pending",
                )
            })
            .collect();
        store.insert_candidates(&rows).expect("insert");
        let inbox = Inbox::new(store);
        let start = std::time::Instant::now();
        assert_eq!(inbox.count(&InboxFilter::new()).expect("cold count"), 1);
        let cold = start.elapsed();
        let raw = rusqlite::Connection::open(&path).expect("instrumentation");
        raw.execute_batch(
            "CREATE TABLE benchmark_membership_writes(n INTEGER);
            INSERT INTO benchmark_membership_writes VALUES(0);
            CREATE TRIGGER benchmark_track_membership AFTER UPDATE ON review_group_members
            BEGIN UPDATE benchmark_membership_writes SET n=n+1; END;",
        )
        .expect("counter");
        // Instrument persisted membership writes across both connections;
        // unchanged polling must leave this counter and the dirty queue empty.
        let mut samples = Vec::new();
        for _ in 0..30 {
            let start = std::time::Instant::now();
            assert_eq!(inbox.count(&InboxFilter::new()).expect("warm count"), 1);
            assert_eq!(
                inbox
                    .list(&InboxFilter::new())
                    .expect("warm list")
                    .candidates
                    .len(),
                1
            );
            samples.push(start.elapsed());
            let dirty: i64 = raw
                .query_row("SELECT COUNT(*) FROM review_dirty", [], |r| r.get(0))
                .unwrap();
            assert_eq!(dirty, 0);
        }
        samples.sort();
        let writes: i64 = raw
            .query_row("SELECT n FROM benchmark_membership_writes", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(writes, 0, "unchanged polling must not rewrite membership");
        eprintln!(
            "group projection {size}: cold={cold:?}, warm count+list p50={:?}, p95={:?}, samples=30",
            samples[14], samples[28]
        );
        drop(raw);
        drop(inbox);
        std::fs::remove_dir_all(root).expect("cleanup");
    }
}

#[test]
fn eight_hundred_rules_create_one_claim_and_closed_claim_reopens_siblings() {
    use application::Claims;
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "rules", "rules-message");
    let rows: Vec<_> = (0..800)
        .map(|i| {
            let mut row = candidate(
                &format!("rule-{i:04}"),
                "rules",
                "2026-01-01T00:00:00Z",
                "pending",
            );
            row.kind = "rule".into();
            row
        })
        .collect();
    store.insert_candidates(&rows).expect("insert rules");
    let inbox = Inbox::new(store.clone());
    assert_eq!(
        inbox.count(&InboxFilter::new()).expect("one opportunity"),
        1
    );
    let displayed = inbox.detail("rule-0799").expect("displayed");
    let confirmed = inbox
        .confirm_reviewed(&displayed, None)
        .expect("confirm rule");
    let claims = Claims::new(store.clone());
    assert_eq!(claims.list("project-1", None).expect("claims").len(), 1);
    assert_eq!(inbox.count(&InboxFilter::new()).expect("represented"), 0);
    assert_eq!(
        inbox
            .review_metrics("project-1")
            .expect("metrics")
            .pending_occurrences,
        799
    );
    claims
        .retire(&confirmed.decision_id, None)
        .expect("close claim");
    assert_eq!(
        inbox.count(&InboxFilter::new()).expect("reopened safely"),
        1
    );
    assert_eq!(
        InboxStore::get(&store, "rule-0000")
            .unwrap()
            .unwrap()
            .status,
        CandidateStatus::Pending
    );
    assert_eq!(
        InboxStore::get(&store, "rule-0799")
            .unwrap()
            .unwrap()
            .status,
        CandidateStatus::Accepted
    );
}

#[test]
fn exact_description_is_hidden_only_while_snapshot_is_verified_and_clean() {
    use application::observations::*;
    use application::review_exception::{
        NatureDestination, NatureTraceStore, ObservationReference,
    };
    let root = temporary_directory("description-nature");
    let path = root.join("app.db");
    let store = SqliteStore::open(&path).expect("store");
    seed_project(&store);
    seed_capture(&store, "description", "description-message");
    let row = candidate(
        "description-candidate",
        "description",
        "2026-01-01T00:00:00Z",
        "pending",
    );
    store.insert_candidates(&[row]).expect("candidate");
    let record = ObservationRecord {
        observation_id: "fact".into(),
        project_id: "project-1".into(),
        version: 1,
        subject: ObservationSubject::Package {
            name: "demo".into(),
        },
        value: ObservationValue {
            declared_version: Some("1".into()),
            dependency_category: None,
            version_requirement: None,
            target: None,
        },
        path_scope: vec!["package.json".into()],
        provenance: ObservationProvenance {
            source_id: "source".into(),
            source_sha256: "hash".into(),
            parser_policy_version: "policy".into(),
            supporting_sources: vec![],
            field_pointer: "/version".into(),
            capture_trigger: "test".into(),
            commit: None,
        },
        observed_at: "2026-01-01T00:00:00Z".into(),
        status: RecordStatus::Current,
        invalidated_at: None,
        invalidation_reason: None,
    };
    let reference = ObservationReference {
        project_id: record.project_id.clone(),
        observation_id: record.observation_id.clone(),
        version: 1,
        source_id: record.provenance.source_id.clone(),
        source_sha256: record.provenance.source_sha256.clone(),
        parser_policy_version: record.provenance.parser_policy_version.clone(),
        supporting_sources: vec![],
        path_scope: record.path_scope.clone(),
        field_pointer: record.provenance.field_pointer.clone(),
        subject: record.subject.clone(),
        value: record.value.clone(),
    };
    let raw = rusqlite::Connection::open(&path).expect("raw fixture connection");
    raw.execute(
        "UPDATE observation_refresh SET dirty=0 WHERE project_id='project-1'",
        [],
    )
    .expect("refresh");
    raw.execute(
        "INSERT INTO observation_sources(source_id,project_id,project_relative_path,manifest_kind,
        sha256,parser_policy_version,last_check_status) VALUES ('source','project-1','package.json',
        'package_json','hash','policy','verified')",
        [],
    )
    .expect("source");
    raw.execute(
        "INSERT INTO observation_records VALUES ('fact',1,'project-1','source','current',?1)",
        [serde_json::to_string(&record).unwrap()],
    )
    .expect("record");
    raw.execute(
        "UPDATE capture_artifacts SET content=?2,metadata=?1 WHERE capture_id='description'
        AND artifact_id='art-1'",
        [
            format!(
                "{{\"observation_reference\":{}}}",
                serde_json::to_string(&reference).unwrap()
            ),
            reference.canonical_description(),
        ],
    )
    .expect("typed literal evidence");
    raw.execute(
        "UPDATE decision_candidates SET choice=?1 WHERE id='description-candidate'",
        [reference.canonical_description()],
    )
    .expect("canonical assertion");
    store
        .record_nature(
            "dedup-description-candidate",
            application::review_exception::CandidateNature::Description,
        )
        .expect("description nature");
    for (id, nature, choice) in [
        (
            "normative-spoof",
            application::review_exception::CandidateNature::Normative,
            "Never upgrade demo beyond version 1",
        ),
        (
            "inference-spoof",
            application::review_exception::CandidateNature::Inference,
            "demo version 1 probably explains the behavior",
        ),
        (
            "description-prose",
            application::review_exception::CandidateNature::Description,
            "demo uses version 1",
        ),
    ] {
        let mut spoof = candidate(id, "description", "2026-01-01T00:00:00Z", "pending");
        spoof.choice = choice.into();
        let hash = spoof.dedup_hash.clone();
        store.insert_candidates(&[spoof]).expect("spoof fixture");
        store.record_nature(&hash, nature).expect("original nature");
        let outcome = store.bind_observation(id, &reference);
        if nature == application::review_exception::CandidateNature::Description {
            assert_eq!(outcome.unwrap(), NatureDestination::ReviewRequired);
        } else {
            assert_eq!(outcome, Err(application::InboxError::InvalidState));
        }
        let traces = store.nature_traces("project-1", 0, 10).unwrap();
        assert_eq!(
            traces.iter().find(|t| t.candidate_id == id).unwrap().nature,
            nature
        );
    }
    assert_eq!(
        store
            .bind_observation("description-candidate", &reference)
            .expect("bind"),
        NatureDestination::DescriptionVerified
    );
    let inbox = Inbox::new(store.clone());
    assert_eq!(
        inbox.count(&InboxFilter::new()).expect("descriptive only"),
        2
    );
    assert!(!inbox
        .eligible_in_review("project-1", "description-candidate", &InboxFilter::new())
        .unwrap());
    assert_eq!(
        inbox.confirm("description-candidate", None),
        Err(application::InboxError::InvalidState)
    );
    store
        .request_refresh(&RefreshRequest {
            project_id: "project-1".into(),
            capture_trigger: "changed".into(),
            requested_at: "2026-01-02T00:00:00Z".into(),
        })
        .expect("dirty");
    // A second schedule while the descriptive candidate is still marked
    // dirty: the upsert must not turn the triggers' OR IGNORE into an abort.
    store
        .request_refresh(&RefreshRequest {
            project_id: "project-1".into(),
            capture_trigger: "reconciler".into(),
            requested_at: "2026-01-02T00:00:01Z".into(),
        })
        .expect("a second refresh while candidates are marked");
    assert_eq!(
        store
            .nature_traces("project-1", 0, 10)
            .expect("uncertain trace")
            .into_iter()
            .find(|t| t.candidate_id == "description-candidate")
            .unwrap()
            .destination,
        NatureDestination::ReviewRequired
    );
    assert_eq!(
        inbox.count(&InboxFilter::new()).expect("uncertain visible"),
        3
    );
    assert_eq!(
        InboxStore::get(&store, "description-candidate")
            .unwrap()
            .unwrap()
            .status,
        CandidateStatus::Pending
    );
    let decisions: i64 = raw
        .query_row("SELECT COUNT(*) FROM engineering_decisions", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(decisions, 0);
    let mut wrong_project = reference;
    wrong_project.project_id = "another-project".into();
    assert!(store
        .bind_observation("description-candidate", &wrong_project)
        .is_err());
    drop(raw);
    drop(inbox);
    drop(store);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn confirm_reconciles_new_duplicate_without_any_projection_read_for_decisions_and_rules() {
    for rule in [false, true] {
        let store = SqliteStore::open(":memory:").expect("store");
        seed_project(&store);
        seed_capture(&store, "direct", "direct-message");
        let mut first = candidate("direct-first", "direct", "2026-01-01T00:00:00Z", "pending");
        if rule {
            first.kind = "rule".into();
        }
        store.insert_candidates(&[first.clone()]).expect("first");
        let inbox = Inbox::new(store.clone());
        inbox
            .confirm("direct-first", None)
            .expect("first confirmation");
        first.id = "direct-second".into();
        first.dedup_hash = "direct-second-hash".into();
        store.insert_candidates(&[first]).expect("later occurrence");
        assert_eq!(
            inbox.confirm("direct-second", None),
            Err(application::InboxError::InvalidState)
        );
        assert_eq!(inbox.count(&InboxFilter::new()).expect("count"), 0);
        assert_eq!(
            InboxStore::get(&store, "direct-second")
                .unwrap()
                .unwrap()
                .status,
            CandidateStatus::Pending
        );
    }
}

#[test]
fn nature_inference_is_retained_but_not_confirmable_and_edits_restore_unknown_review() {
    use application::review_exception::{CandidateNature, NatureDestination, NatureTraceStore};
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "nature", "nature-message");
    let record = candidate("inference", "nature", "2026-01-01T00:00:00Z", "pending");
    store
        .insert_candidates(std::slice::from_ref(&record))
        .expect("insert");
    store
        .record_nature(&record.dedup_hash, CandidateNature::Inference)
        .expect("classify");
    let inbox = Inbox::new(store.clone());
    assert_eq!(inbox.count(&InboxFilter::new()).expect("hidden"), 0);
    assert_eq!(
        inbox.confirm("inference", None),
        Err(application::InboxError::InvalidState)
    );
    let trace = store.nature_traces("project-1", 0, 10).expect("trace");
    assert_eq!(trace[0].destination, NatureDestination::InferenceStored);
    let detail = inbox.detail("inference").expect("retained evidence");
    inbox
        .adjust(
            "inference",
            CandidateEdits {
                question: detail.summary.question,
                choice: "explicitly revised proposal".into(),
                rationale: detail.rationale,
                qualifiers: detail.qualifiers,
            },
        )
        .expect("adjust");
    assert_eq!(inbox.count(&InboxFilter::new()).expect("unknown review"), 1);
    assert_eq!(
        store.nature_traces("project-1", 0, 10).unwrap()[0].nature,
        CandidateNature::Unknown
    );
}

#[test]
fn parallel_connections_cannot_duplicate_confirmation_or_overwrite_current_membership() {
    use std::sync::{Arc, Barrier};
    let root = temporary_directory("parallel-groups");
    let path = root.join("app.db");
    let first = SqliteStore::open(&path).expect("first connection");
    seed_project(&first);
    seed_capture(&first, "parallel", "parallel-message");
    first
        .insert_candidates(&[
            candidate("parallel-a", "parallel", "2026-01-01T00:00:00Z", "pending"),
            candidate("parallel-b", "parallel", "2026-01-01T00:00:00Z", "pending"),
        ])
        .expect("insert");
    let second = SqliteStore::open(&path).expect("second connection");
    let barrier = Arc::new(Barrier::new(2));
    let worker_barrier = barrier.clone();
    let worker = std::thread::spawn(move || {
        let inbox = Inbox::new(second);
        let snapshot = inbox
            .detail("parallel-b")
            .expect("snapshot before concurrent confirmation");
        worker_barrier.wait();
        let result = inbox.confirm_reviewed(&snapshot, None);
        let count = inbox
            .count(&InboxFilter::new())
            .expect("concurrent projection");
        (result, count)
    });
    let inbox = Inbox::new(first);
    let snapshot = inbox.detail("parallel-a").expect("snapshot");
    barrier.wait();
    let result = inbox.confirm_reviewed(&snapshot, None);
    let (other, count) = worker.join().expect("worker");
    assert_ne!(result.is_ok(), other.is_ok());
    assert_eq!(count, 0);
    assert_eq!(
        inbox.count(&InboxFilter::new()).expect("final projection"),
        0
    );
    drop(inbox);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn eight_hundred_occurrences_are_one_opportunity_not_eight_hundred_decisions() {
    use application::review_exception::ReviewExceptionStore;
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "duplicates", "duplicates-message");
    let records: Vec<_> = (0..800)
        .map(|i| {
            candidate(
                &format!("duplicate-{i:04}"),
                "duplicates",
                "2026-01-01T00:00:00Z",
                "pending",
            )
        })
        .collect();
    assert_eq!(store.insert_candidates(&records).expect("insert"), 800);
    let inbox = Inbox::new(store.clone());
    assert_eq!(inbox.count(&InboxFilter::new()).expect("count"), 1);
    let page = inbox.list(&InboxFilter::new()).expect("page");
    assert_eq!(page.candidates.len(), 1);
    let id = &page.candidates[0].id;
    let group = store.review_group(id, 750, 100).expect("members");
    assert_eq!(group.occurrence_count, 800);
    assert_eq!(group.members.len(), 50);
    assert!(group.members.iter().all(|m| m.evidence_refs.len() == 2));
    let displayed = inbox.detail(id).expect("displayed");
    inbox.confirm_reviewed(&displayed, None).expect("confirm");
    assert_eq!(inbox.count(&InboxFilter::new()).expect("resolved"), 0);
    let metrics = store.review_metrics("project-1").expect("metrics");
    assert_eq!(metrics.pending_occurrences, 799);
    assert_eq!(metrics.potential_review_opportunities, 0);
    let sibling = inbox.detail("duplicate-0000").expect("sibling detail");
    assert_eq!(
        inbox.confirm_reviewed(&sibling, None),
        Err(application::InboxError::InvalidState)
    );
    assert_eq!(
        InboxStore::get(&store, "duplicate-0000")
            .expect("raw")
            .unwrap()
            .status,
        CandidateStatus::Pending
    );
    assert!(store
        .review_group(id, 0, 100)
        .expect("resolved members")
        .members
        .iter()
        .filter(|m| m.candidate_id != *id)
        .all(|m| m.already_represented));
    assert_eq!(
        inbox.confirm_reviewed(&displayed, None),
        Err(application::InboxError::InvalidState)
    );
    assert_eq!(inbox.count(&InboxFilter::new()).expect("retry"), 0);
}

#[test]
fn negation_paths_qualifiers_and_lifecycle_do_not_share_review() {
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "split", "split-message");
    let mut rows: Vec<_> = (0..6)
        .map(|i| {
            candidate(
                &format!("split-{i}"),
                "split",
                "2026-01-01T00:00:00Z",
                "pending",
            )
        })
        .collect();
    rows[1].choice = "not c".into();
    rows[2].diff_summary = "{\"files\":[\"src/b.rs\"],\"artifacts\":2}".into();
    rows[3].qualifiers = "[{\"kind\":\"validation\",\"text\":\"not tested\"}]".into();
    rows[4].status = "snoozed".into();
    rows[5].status = "dismissed".into();
    store.insert_candidates(&rows).expect("insert");
    let inbox = Inbox::new(store);
    assert_eq!(inbox.count(&InboxFilter::new()).expect("count"), 5);
    inbox.reject("split-0").expect("reject");
    assert_eq!(inbox.count(&InboxFilter::new()).expect("after reject"), 4);
}

#[test]
fn cross_capture_ledger_survives_reopen_and_decision_revision_invalidates_it() {
    use application::review_exception::ReviewExceptionStore;
    use application::{DecisionEdits, DecisionFilter, Decisions};
    let root = temporary_directory("group-ledger");
    let path = root.join("app.db");
    let decision_id;
    {
        let store = SqliteStore::open(&path).expect("store");
        seed_project(&store);
        seed_capture(&store, "first", "first-message");
        seed_capture(&store, "second", "second-message");
        store
            .insert_candidates(&[
                candidate("first-member", "first", "2026-01-01T00:00:00Z", "pending"),
                candidate("second-member", "second", "2026-01-02T00:00:00Z", "pending"),
            ])
            .expect("insert");
        let inbox = Inbox::new(store.clone());
        assert_eq!(inbox.count(&InboxFilter::new()).expect("group count"), 1);
        let detail = inbox.detail("second-member").expect("displayed");
        decision_id = inbox
            .confirm_reviewed(&detail, None)
            .expect("confirm")
            .decision_id;
        assert_eq!(
            Decisions::new(store.clone())
                .list(&DecisionFilter::new())
                .expect("decisions")
                .decisions
                .len(),
            1
        );
    }
    {
        let store = SqliteStore::open(&path).expect("reopen");
        let inbox = Inbox::new(store.clone());
        assert_eq!(
            inbox
                .count(&InboxFilter::new())
                .expect("resolved after reopen"),
            0
        );
        store
            .insert_candidates(&[candidate(
                "later-member",
                "first",
                "2026-01-03T00:00:00Z",
                "pending",
            )])
            .expect("later occurrence");
        assert_eq!(
            inbox.count(&InboxFilter::new()).expect("later represented"),
            0
        );
        assert!(store
            .review_group("first-member", 0, 10)
            .expect("group")
            .members
            .iter()
            .any(|m| m.candidate_id == "first-member" && m.already_represented));
        Decisions::new(store.clone())
            .revise(
                &decision_id,
                DecisionEdits {
                    choice: Some("changed scope and choice".into()),
                    ..Default::default()
                },
            )
            .expect("revise");
        assert_eq!(
            inbox
                .count(&InboxFilter::new())
                .expect("version invalidated"),
            1
        );
        assert!(!store
            .review_group("first-member", 0, 10)
            .expect("group")
            .members
            .iter()
            .any(|m| m.already_represented));
    }
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn identity_is_project_local_and_empty_scope_is_capture_local() {
    use application::review_exception::group_fingerprint;
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "identity", "identity-message");
    store
        .insert_candidates(&[candidate(
            "identity-member",
            "identity",
            "2026-01-01T00:00:00Z",
            "pending",
        )])
        .expect("insert");
    let original = InboxStore::get(&store, "identity-member")
        .expect("get")
        .unwrap();
    let mut other = original.clone();
    other.project_id = "another-project".into();
    assert_ne!(
        group_fingerprint(&original).unwrap(),
        group_fingerprint(&other).unwrap()
    );
    other = original.clone();
    other.capture_id = "another-capture".into();
    assert_eq!(
        group_fingerprint(&original).unwrap(),
        group_fingerprint(&other).unwrap()
    );
    let mut unknown = original.clone();
    unknown.diff_summary = "{\"files\":[],\"artifacts\":2}".into();
    other.diff_summary = unknown.diff_summary.clone();
    assert_ne!(
        group_fingerprint(&unknown).unwrap(),
        group_fingerprint(&other).unwrap()
    );
    other = original.clone();
    other.question = "  q\n ".into();
    assert_eq!(
        group_fingerprint(&original).unwrap(),
        group_fingerprint(&other).unwrap()
    );
    other.question = "q?".into();
    assert_ne!(
        group_fingerprint(&original).unwrap(),
        group_fingerprint(&other).unwrap()
    );
}

#[test]
fn reviewed_confirmation_rejects_edits_after_display_without_promoting() {
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "reviewed-capture", "reviewed-message");
    store
        .insert_candidates(&[candidate(
            "reviewed",
            "reviewed-capture",
            "2026-09-29T15:00:00Z",
            "pending",
        )])
        .expect("candidate");
    let inbox = Inbox::new(store);
    let displayed = inbox.detail("reviewed").expect("displayed detail");
    inbox
        .adjust(
            "reviewed",
            CandidateEdits {
                question: displayed.summary.question.clone(),
                choice: "Do not apply outside the reviewed scope".into(),
                rationale: displayed.rationale.clone(),
                qualifiers: displayed.qualifiers.clone(),
            },
        )
        .expect("concurrent edit");
    assert_eq!(
        inbox.confirm_reviewed(&displayed, None),
        Err(application::InboxError::InvalidState)
    );
    let refreshed = inbox.detail("reviewed").expect("refreshed detail");
    assert_eq!(refreshed.summary.status, CandidateStatus::Pending);
    inbox
        .confirm_reviewed(&refreshed, None)
        .expect("explicit confirmation of refreshed detail");
    assert_eq!(
        inbox.confirm_reviewed(&refreshed, None),
        Err(application::InboxError::InvalidState)
    );
}

#[test]
fn queue_count_ignores_page_size_and_cursor_and_tracks_review_actions() {
    let store = SqliteStore::open(":memory:").expect("store");
    seed_project(&store);
    seed_capture(&store, "capture-count", "message-count");
    let rows: Vec<_> = (0..57)
        .map(|index| {
            let mut row = candidate(
                &format!("count-{index:03}"),
                "capture-count",
                "2026-09-29T15:00:00Z",
                "pending",
            );
            row.question = format!("question {index}");
            row
        })
        .collect();
    store.insert_candidates(&rows).expect("candidates");
    let inbox = Inbox::new(store);
    let mut filter = InboxFilter {
        project_id: Some("project-1".into()),
        limit: 2,
        ..InboxFilter::default()
    };
    let first = inbox.list(&filter).expect("page");
    assert_eq!(first.candidates.len(), 2);
    filter.cursor = first.next_cursor;
    assert_eq!(inbox.count(&filter).expect("count"), 57);
    inbox.snooze("count-000").expect("snooze");
    assert_eq!(
        inbox
            .count(&filter)
            .expect("deferred still awaiting review"),
        57
    );
    inbox.reject("count-001").expect("reject");
    inbox.confirm("count-002", None).expect("confirm");
    assert_eq!(inbox.count(&filter).expect("remaining"), 55);
    filter.project_id = Some("another-project".into());
    assert_eq!(inbox.count(&filter).expect("other project"), 0);
}

fn temporary_directory(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let directory = std::env::temp_dir().join(format!(
        "xemnas-inbox-{tag}-{}-{nanos}-{unique}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).expect("create temporary directory");
    directory
}

fn artifact(
    capture_id: &str,
    artifact_id: &str,
    kind: &str,
    content: &str,
    seed: u8,
) -> CaptureArtifactRecord {
    CaptureArtifactRecord {
        capture_id: capture_id.to_string(),
        artifact_id: artifact_id.to_string(),
        kind: kind.to_string(),
        content: content.to_string(),
        metadata: "{}".to_string(),
        fingerprint: format!("{:064x}", seed as u128),
    }
}

/// Seeds one project and one capture; the adapter session is shared, so a later
/// `seed_capture` call moves the checkpoint away from the earlier capture.
fn seed_capture(store: &SqliteStore, capture_id: &str, message_id: &str) {
    let timestamp = "2026-01-01T00:00:00Z".to_string();
    let write = CaptureWrite {
        receipt: CaptureReceiptRecord {
            capture_id: capture_id.to_string(),
            idempotency_key: format!("key-{capture_id}"),
            canonical_path: LOCATION.to_string(),
            received_at: timestamp.clone(),
            artifact_count: 2,
        },
        artifacts: vec![
            artifact(capture_id, "art-1", "diff_hunk", "content-1", 1),
            artifact(capture_id, "art-2", "user_text", "content-2", 2),
        ],
        job: JobRecord {
            id: format!("job-{capture_id}"),
            kind: ANALYZE_CAPTURE_KIND.to_string(),
            payload: capture_id.to_string(),
            state: JobState::Queued,
            idempotent: true,
            attempts: 0,
            last_error: None,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        },
        checkpoint: CaptureCheckpointRecord {
            adapter: "opencode".to_string(),
            adapter_version: "0.1.0".to_string(),
            session_id: "session-1".to_string(),
            message_id: message_id.to_string(),
            capture_id: capture_id.to_string(),
            observed_at: "2026-01-02T00:00:00Z".to_string(),
            updated_at: "2026-01-02T00:00:00Z".to_string(),
        },
    };
    store.insert_capture(&write).expect("seed capture");
}

fn seed_project(store: &SqliteStore) {
    store
        .insert(&ProjectRecord::new(
            "project-1".to_string(),
            LOCATION.to_string(),
            "2026-01-01T00:00:00Z".to_string(),
        ))
        .expect("seed project");
}

fn candidate(
    id: &str,
    capture_id: &str,
    created_at: &str,
    status: &str,
) -> DecisionCandidateRecord {
    DecisionCandidateRecord {
        qualifiers: "[]".into(),
        id: id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: capture_id.to_string(),
        status: status.to_string(),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        signals: "[\"public_contract\"]".to_string(),
        confidence: 0.7,
        confidence_reason: "x".to_string(),
        evidence_refs: "[\"art-2\", \"art-1\"]".to_string(),
        diff_summary: "{\"files\":[\"src/a.rs\"],\"artifacts\":2}".to_string(),
        dedup_hash: format!("dedup-{id}"),
        created_at: created_at.to_string(),
        updated_at: created_at.to_string(),
        kind: "decision".to_string(),
        significance: 1.0,
        criteria: "[]".to_string(),
    }
}

#[test]
fn list_joins_project_and_checkpoint_with_receipt_fallback() {
    let root = temporary_directory("list");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    seed_capture(&store, "capture-2", "message-2");
    store
        .insert_candidates(&[
            {
                let mut row = candidate("cand-1", "capture-1", "2026-01-03T00:00:00Z", "pending");
                row.question = "distinct question".into();
                row
            },
            candidate("cand-2", "capture-2", "2026-01-04T00:00:00Z", "pending"),
        ])
        .expect("insert candidates");

    let page = Inbox::new(store).list(&InboxFilter::new()).expect("list");
    assert_eq!(page.candidates.len(), 2);

    let first = page
        .candidates
        .iter()
        .find(|row| row.id == "cand-2")
        .expect("cand-2");
    assert_eq!(first.project_location, LOCATION);
    assert_eq!(first.adapter.as_deref(), Some("opencode"));
    assert_eq!(first.session_id.as_deref(), Some("session-1"));
    assert_eq!(first.observed_at.as_deref(), Some("2026-01-02T00:00:00Z"));
    assert_eq!(first.received_at, "2026-01-01T00:00:00Z");

    let fallback = page
        .candidates
        .iter()
        .find(|row| row.id == "cand-1")
        .expect("cand-1");
    assert_eq!(fallback.adapter, None);
    assert_eq!(fallback.session_id, None);
    assert_eq!(
        fallback.observed_at.as_deref(),
        Some("2026-01-01T00:00:00Z"),
        "without a checkpoint the observation falls back to the receipt"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn default_list_excludes_terminal_statuses() {
    let root = temporary_directory("default-filter");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[
            candidate("pending", "capture-1", "2026-01-01T00:00:00Z", "pending"),
            candidate("accepted", "capture-1", "2026-01-02T00:00:00Z", "accepted"),
            candidate(
                "dismissed",
                "capture-1",
                "2026-01-03T00:00:00Z",
                "dismissed",
            ),
            candidate("snoozed", "capture-1", "2026-01-04T00:00:00Z", "snoozed"),
        ])
        .expect("insert candidates");

    let page = Inbox::new(store).list(&InboxFilter::new()).expect("list");
    let ids: Vec<&str> = page.candidates.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, vec!["snoozed", "pending"]);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn detail_returns_referenced_artifacts_in_order() {
    let root = temporary_directory("detail");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("insert candidate");

    let detail = Inbox::new(store).detail("cand-1").expect("detail");
    let ids: Vec<&str> = detail
        .artifacts
        .iter()
        .map(|artifact| artifact.artifact_id.as_str())
        .collect();
    assert_eq!(ids, vec!["art-2", "art-1"], "stored reference order");
    assert_eq!(detail.artifacts[0].content, "content-2");
    assert_eq!(detail.diff_summary.files, vec!["src/a.rs"]);
    assert_eq!(detail.diff_summary.artifacts, 2);
    assert_eq!(detail.evidence_refs, vec!["art-2", "art-1"]);

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn compare_and_set_reject_is_exclusive() {
    let root = temporary_directory("cas");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("insert candidate");

    assert!(InboxStore::dismiss_one(&store, "cand-1", "t1").expect("first reject"));
    assert!(!InboxStore::dismiss_one(&store, "cand-1", "t2").expect("second reject"));

    let inbox = Inbox::new(store);
    assert_eq!(
        inbox
            .reject("cand-1")
            .expect_err("already dismissed")
            .code(),
        "invalid_state"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn adjust_advances_updated_at_and_persists_the_edits() {
    let root = temporary_directory("adjust");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("insert candidate");

    let inbox = Inbox::new(store.clone());
    inbox
        .adjust(
            "cand-1",
            application::inbox::CandidateEdits {
                qualifiers: Vec::new(),
                question: "nova pergunta".to_string(),
                choice: "nova escolha".to_string(),
                rationale: "nova justificativa".to_string(),
            },
        )
        .expect("adjust");

    let stored = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    assert_eq!(stored.status, CandidateStatus::Pending);
    assert_eq!(stored.question, "nova pergunta");
    assert_ne!(
        stored.updated_at, stored.created_at,
        "adjust must advance updated_at"
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn semantic_port_fixes_batch_destinations_and_rejects_wrong_sources() {
    let root = temporary_directory("semantic-port");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[
            candidate("cand-1", "capture-1", "2026-01-03T00:00:00Z", "pending"),
            candidate("cand-2", "capture-1", "2026-01-04T00:00:00Z", "pending"),
            candidate(
                "cand-terminal",
                "capture-1",
                "2026-01-05T00:00:00Z",
                "dismissed",
            ),
        ])
        .expect("insert candidates");

    assert_eq!(
        InboxStore::snooze_batch(&store, &["cand-1".to_string()], "t1").expect("snooze batch"),
        1
    );
    assert_eq!(status_of(&store, "cand-1"), CandidateStatus::Snoozed);
    assert_eq!(
        InboxStore::dismiss_batch(&store, &["cand-2".to_string()], "t1").expect("dismiss batch"),
        1
    );
    assert_eq!(status_of(&store, "cand-2"), CandidateStatus::Dismissed);

    assert_eq!(
        InboxStore::dismiss_batch(&store, &["cand-terminal".to_string()], "t2").expect("no-op"),
        0
    );
    assert!(!InboxStore::confirm_one(
        &store,
        "cand-terminal",
        &InboxStore::get(&store, "cand-terminal")
            .expect("get")
            .expect("row"),
        None,
        &seed("d-terminal"),
        "t2"
    )
    .expect("no-op"));

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn confirmation_rejects_an_adjustment_even_with_the_same_timestamp() {
    let root = temporary_directory("stale-confirm");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[candidate(
            "cand-1",
            "capture-1",
            "2026-01-03T00:00:00Z",
            "pending",
        )])
        .expect("candidate");
    let reviewed = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    let edits = CandidateEdits {
        qualifiers: Vec::new(),
        question: "corrected question".into(),
        choice: "corrected choice".into(),
        rationale: "corrected rationale".into(),
    }
    .validate()
    .expect("edits");
    assert!(
        InboxStore::adjust_one(&store, "cand-1", &edits, &reviewed.updated_at).expect("adjust")
    );
    for confirm_edits in [None, Some(&edits)] {
        assert!(!InboxStore::confirm_one(
            &store,
            "cand-1",
            &reviewed,
            confirm_edits,
            &seed("stale-decision"),
            "later"
        )
        .expect("stale confirmation"));
    }
    let current = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    assert_eq!(current.status, CandidateStatus::Pending);
    assert_eq!(current.question, "corrected question");
    assert!(application::decisions::Decisions::new(store.clone())
        .detail("stale-decision")
        .is_err());
    let confirmed = Inbox::new(store.clone())
        .confirm("cand-1", None)
        .expect("reviewed confirmation");
    let decision = application::decisions::Decisions::new(store)
        .detail(&confirmed.decision_id)
        .expect("decision");
    assert_eq!(decision.summary.question, "corrected question");
    let _ = std::fs::remove_dir_all(&root);
}

/// Reads a candidate status through the inbox port.
fn status_of(store: &SqliteStore, id: &str) -> CandidateStatus {
    InboxStore::get(store, id)
        .expect("get")
        .expect("row")
        .status
}

/// Builds a decision seed for a direct port-level confirmation.
fn seed(decision_id: &str) -> DecisionSeed {
    DecisionSeed {
        qualifiers: "[]".into(),
        decision_id: decision_id.to_string(),
        project_id: "project-1".to_string(),
        capture_id: Some("capture-1".to_string()),
        question: "q".to_string(),
        choice: "c".to_string(),
        rationale: "r".to_string(),
        evidence_refs: vec!["art-2".to_string(), "art-1".to_string()],
        as_rule: false,
    }
}

#[test]
fn validated_edits_are_persisted_by_confirm_and_adjust() {
    let root = temporary_directory("validated-edits");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    store
        .insert_candidates(&[
            candidate("cand-1", "capture-1", "2026-01-03T00:00:00Z", "pending"),
            candidate("cand-2", "capture-1", "2026-01-04T00:00:00Z", "pending"),
        ])
        .expect("insert candidates");

    let confirm_edits = CandidateEdits {
        qualifiers: Vec::new(),
        question: " confirma q ".to_string(),
        choice: " confirma c ".to_string(),
        rationale: " confirma r ".to_string(),
    }
    .validate()
    .expect("valid confirm edits");
    assert!(InboxStore::confirm_one(
        &store,
        "cand-1",
        &InboxStore::get(&store, "cand-1")
            .expect("get")
            .expect("row"),
        Some(&confirm_edits),
        &seed("decision-1"),
        "t1"
    )
    .expect("confirm"));
    let stored = InboxStore::get(&store, "cand-1")
        .expect("get")
        .expect("row");
    assert_eq!(stored.status, CandidateStatus::EditedAndAccepted);
    assert_eq!(stored.question, "confirma q");
    assert_eq!(stored.choice, "confirma c");
    assert_eq!(stored.rationale, "confirma r");

    let adjust_edits = CandidateEdits {
        qualifiers: Vec::new(),
        question: "ajusta q".to_string(),
        choice: "ajusta c".to_string(),
        rationale: "ajusta r".to_string(),
    }
    .validate()
    .expect("valid adjust edits");
    assert!(InboxStore::adjust_one(&store, "cand-2", &adjust_edits, "t2").expect("adjust"));
    let stored = InboxStore::get(&store, "cand-2")
        .expect("get")
        .expect("row");
    assert_eq!(stored.status, CandidateStatus::Pending);
    assert_eq!(stored.question, "ajusta q");
    assert_eq!(stored.choice, "ajusta c");

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn low_significance_stays_out_of_the_default_queue_and_rules_become_claims() {
    use application::claims::Claims;
    use application::extract::{CandidateKind, MIN_SIGNIFICANCE};

    let root = temporary_directory("significance");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    let mut low = candidate("low", "capture-1", "2026-01-01T00:00:01Z", "pending");
    low.significance = 0.2;
    low.dedup_hash = "dedup-low".into();
    let mut rule = candidate("rule", "capture-1", "2026-01-01T00:00:02Z", "pending");
    rule.kind = "rule".into();
    rule.significance = 0.8;
    rule.choice = "Só um humano resolve pendências legadas.".into();
    rule.dedup_hash = "dedup-rule".into();
    store.insert_candidates(&[low, rule]).expect("insert");

    let inbox = Inbox::new(store.clone());
    let relevant = InboxFilter {
        min_significance: Some(MIN_SIGNIFICANCE),
        ..InboxFilter::default()
    };
    let page = inbox.list(&relevant).expect("list");
    assert_eq!(page.candidates.len(), 1, "the low one is hidden");
    assert_eq!(page.candidates[0].kind, CandidateKind::Rule);
    assert_eq!(inbox.count(&relevant).expect("count"), 1);
    assert_eq!(
        inbox.count(&InboxFilter::default()).expect("all"),
        2,
        "kept, not deleted"
    );

    let outcome = inbox.confirm("rule", None).expect("confirm rule");
    assert!(outcome.rule);
    let claims = Claims::new(store.clone())
        .list("project-1", None)
        .expect("claims");
    assert_eq!(claims.len(), 1);
    assert_eq!(
        claims[0].statement,
        "Só um humano resolve pendências legadas."
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn background_lists_what_is_recorded_and_the_users_taste() {
    let root = temporary_directory("background");
    let store = SqliteStore::open(root.join("app.db")).expect("open store");
    seed_project(&store);
    seed_capture(&store, "capture-1", "message-1");
    let mut kept = candidate("kept", "capture-1", "2026-01-01T00:00:01Z", "pending");
    kept.question = "Qual banco?".into();
    kept.choice = "SQLite".into();
    kept.dedup_hash = "dedup-kept".into();
    let mut dropped = candidate("dropped", "capture-1", "2026-01-01T00:00:02Z", "pending");
    dropped.question = "Como validar o campo?".into();
    dropped.choice = "Regex".into();
    dropped.dedup_hash = "dedup-dropped".into();
    store.insert_candidates(&[kept, dropped]).expect("insert");
    let inbox = Inbox::new(store.clone());
    inbox.confirm("kept", None).expect("confirm");
    inbox.reject("dropped").expect("reject");

    let background = store.background("project-1", &[]).expect("background");
    assert!(background
        .known
        .iter()
        .any(|line| line == "Qual banco? → SQLite"));
    assert!(background
        .confirmed
        .iter()
        .any(|line| line.starts_with("Qual banco?")));
    assert!(background
        .rejected
        .iter()
        .any(|line| line == "Como validar o campo? → Regex"));
    let _ = std::fs::remove_dir_all(&root);
}
