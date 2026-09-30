//! Diagnostics unit tests: document shape, serialization and the job
//! error-code mapping that keeps raw diagnostics out of the export.

use std::cell::RefCell;
use std::collections::HashMap;

use application::diagnostics::{
    job_error_code, AssessmentDiagnosticRow, Diagnostics, DiagnosticsCounts, DiagnosticsError,
    DiagnosticsMetrics, DiagnosticsStore, Distribution, JobDiagnosticRow, LossMetrics,
    NoiseMetrics, ReceiptDiagnosticRow,
};
use application::jobs::INTERRUPTED_NON_IDEMPOTENT;
use application::profile::{
    build_preview, preview_hash, AiProfile, AiSettings, ConsentRecord, ProfileError, ProfileKind,
    ProfileStore, SecretStore,
};

/// A store returning fixed structural rows, one of them hostile.
struct FakeStore;

impl DiagnosticsStore for FakeStore {
    fn migrations_version(&self) -> Result<i64, DiagnosticsError> {
        Ok(8)
    }

    fn counts(&self) -> Result<DiagnosticsCounts, DiagnosticsError> {
        Ok(DiagnosticsCounts {
            projects: 2,
            captures: 3,
            artifacts: 4,
            candidates_by_status: [("pending".to_string(), 1)].into_iter().collect(),
            decisions: 1,
            revisions: 1,
            assessments_by_outcome: [("ok".to_string(), 1)].into_iter().collect(),
            jobs_by_state: [("failed".to_string(), 1)].into_iter().collect(),
        })
    }

    fn recent_jobs(&self, _limit: usize) -> Result<Vec<JobDiagnosticRow>, DiagnosticsError> {
        Ok(vec![JobDiagnosticRow {
            id: "job-1".to_string(),
            kind: "analyze_capture".to_string(),
            state: "failed".to_string(),
            attempts: 1,
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            last_error: Some("SECRET-MARKER-JOB".to_string()),
        }])
    }

    fn recent_receipts(
        &self,
        _limit: usize,
    ) -> Result<Vec<ReceiptDiagnosticRow>, DiagnosticsError> {
        Ok(vec![ReceiptDiagnosticRow {
            capture_id: "capture-1".to_string(),
            artifact_count: 2,
            received_at: "2026-01-01T00:00:00Z".to_string(),
            project_id: Some("project-1".to_string()),
        }])
    }

    fn recent_assessments(
        &self,
        _limit: usize,
    ) -> Result<Vec<AssessmentDiagnosticRow>, DiagnosticsError> {
        Ok(vec![AssessmentDiagnosticRow {
            outcome: "failed".to_string(),
            error_code: Some("extractor".to_string()),
        }])
    }

    fn metrics(&self) -> Result<DiagnosticsMetrics, DiagnosticsError> {
        Ok(DiagnosticsMetrics {
            latency_capture_to_candidate_ms: Distribution {
                samples: 3,
                p50: Some(100),
                p95: Some(300),
            },
            review_time_ms: Distribution {
                samples: 1,
                p50: Some(10_000),
                p95: Some(10_000),
            },
            noise: NoiseMetrics {
                decided_total: 4,
                dismissed_ratio: Some(0.25),
            },
            losses: LossMetrics {
                assessments_failed: 1,
                assessments_skipped: 2,
                jobs_failed: 3,
                outbox_rejected: 0,
            },
            context: Default::default(),
        })
    }
}

/// A store with no samples at all: every metric must be 0/None.
struct EmptyStore;

impl DiagnosticsStore for EmptyStore {
    fn migrations_version(&self) -> Result<i64, DiagnosticsError> {
        Ok(0)
    }
    fn counts(&self) -> Result<DiagnosticsCounts, DiagnosticsError> {
        Ok(DiagnosticsCounts {
            projects: 0,
            captures: 0,
            artifacts: 0,
            candidates_by_status: Default::default(),
            decisions: 0,
            revisions: 0,
            assessments_by_outcome: Default::default(),
            jobs_by_state: Default::default(),
        })
    }
    fn metrics(&self) -> Result<DiagnosticsMetrics, DiagnosticsError> {
        Ok(DiagnosticsMetrics {
            latency_capture_to_candidate_ms: Distribution {
                samples: 0,
                p50: None,
                p95: None,
            },
            review_time_ms: Distribution {
                samples: 0,
                p50: None,
                p95: None,
            },
            noise: NoiseMetrics {
                decided_total: 0,
                dismissed_ratio: None,
            },
            losses: LossMetrics {
                assessments_failed: 0,
                assessments_skipped: 0,
                jobs_failed: 0,
                outbox_rejected: 0,
            },
            context: Default::default(),
        })
    }
    fn recent_jobs(&self, _limit: usize) -> Result<Vec<JobDiagnosticRow>, DiagnosticsError> {
        Ok(Vec::new())
    }
    fn recent_receipts(
        &self,
        _limit: usize,
    ) -> Result<Vec<ReceiptDiagnosticRow>, DiagnosticsError> {
        Ok(Vec::new())
    }
    fn recent_assessments(
        &self,
        _limit: usize,
    ) -> Result<Vec<AssessmentDiagnosticRow>, DiagnosticsError> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct MemoryProfiles(RefCell<Option<AiProfile>>);

impl ProfileStore for MemoryProfiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        Ok(self.0.borrow().clone())
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        *self.0.borrow_mut() = Some(profile.clone());
        Ok(())
    }
}

#[derive(Default)]
struct MemorySecrets(RefCell<HashMap<String, String>>);

impl SecretStore for MemorySecrets {
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
        self.0
            .borrow_mut()
            .insert(account.to_string(), secret.to_string());
        Ok(())
    }
    fn get_secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
        Ok(self.0.borrow().get(account).cloned())
    }
    fn delete_secret(&self, account: &str) -> Result<(), ProfileError> {
        self.0.borrow_mut().remove(account);
        Ok(())
    }
}

/// A consented external profile with a stored secret.
fn settings() -> AiSettings<MemoryProfiles, MemorySecrets> {
    let mut profile = AiProfile {
        id: "profile-1".to_string(),
        kind: ProfileKind::OpenAiCompatible,
        model: "gpt-test".to_string(),
        endpoint: Some("https://api.example.test/v1".to_string()),
        max_input_chars: 4_096,
        external_calls_enabled: true,
        consent: None,
        chatgpt: None,
    };
    profile.consent = Some(ConsentRecord {
        granted_at: "2026-01-01T00:00:00Z".to_string(),
        preview_hash: preview_hash(&build_preview(&profile)),
    });
    let profiles = MemoryProfiles(RefCell::new(Some(profile)));
    let secrets = MemorySecrets(RefCell::new(HashMap::new()));
    secrets
        .set_secret("profile-1", "sk-synthetic")
        .expect("secret");
    AiSettings::new(profiles, secrets)
}

#[test]
fn job_error_code_never_carries_the_raw_message() {
    assert_eq!(job_error_code(None), None);
    assert_eq!(
        job_error_code(Some(INTERRUPTED_NON_IDEMPOTENT)),
        Some("interrupted")
    );
    assert_eq!(job_error_code(Some("SECRET-MARKER-JOB")), Some("failed"));
}

#[test]
fn document_is_structural_and_serializable() {
    let outbox = outbox_with(&[
        ("pending", 1),
        ("accepted", 2),
        ("rejected", 3),
        ("stalled", 4),
    ]);
    let diagnostics = Diagnostics::new(FakeStore, settings(), &outbox);
    let document = diagnostics.export().expect("export");

    assert_eq!(document.schema.app_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(document.schema.migrations_version, 8);
    assert_eq!(document.counts.projects, 2);
    assert_eq!(document.counts.artifacts, 4);
    assert_eq!(document.outbox.pending, 1);
    assert_eq!(document.outbox.stalled, 4);
    assert_eq!(document.ai_profile.kind, "openai-compatible");
    assert_eq!(
        document.ai_profile.provider.as_deref(),
        Some("api.example.test"),
        "only the host is exposed, never the full URL"
    );
    assert!(document.ai_profile.has_secret);
    assert_eq!(document.ai_profile.consent_status, "valid");
    assert_eq!(document.runtime.mode, "external");
    assert_eq!(document.runtime.platform, std::env::consts::OS);
    assert_eq!(
        document.recent_jobs[0].error_code.as_deref(),
        Some("failed")
    );
    assert_eq!(
        document.recent_assessments[0].error_code.as_deref(),
        Some("extractor")
    );
    assert_eq!(
        document.recent_receipts[0].project_id.as_deref(),
        Some("project-1"),
        "receipts expose the project id, never the path"
    );
    assert_eq!(document.metrics.latency_capture_to_candidate_ms.samples, 3);
    assert_eq!(
        document.metrics.latency_capture_to_candidate_ms.p50,
        Some(100)
    );
    assert_eq!(
        document.metrics.latency_capture_to_candidate_ms.p95,
        Some(300)
    );
    assert_eq!(document.metrics.review_time_ms.p95, Some(10_000));
    assert_eq!(document.metrics.noise.decided_total, 4);
    assert_eq!(document.metrics.noise.dismissed_ratio, Some(0.25));
    assert_eq!(document.metrics.losses.jobs_failed, 3);
    assert_eq!(
        document.metrics.losses.outbox_rejected, 3,
        "rejected losses come from the outbox directory"
    );

    let json = serde_json::to_string_pretty(&document).expect("serialize");
    assert!(json.contains("\"migrations_version\": 8"));
    assert!(json.contains("\"jobs_by_state\""));
    assert!(json.contains("\"metrics\""));
    assert!(
        !json.contains("SECRET-MARKER-JOB"),
        "the raw job message must never reach the document"
    );
    assert!(
        !json.contains("api.example.test/v1"),
        "the full endpoint must never reach the document"
    );

    let reparsed: application::diagnostics::DiagnosticsDocument =
        serde_json::from_str(&json).expect("round trip");
    assert_eq!(reparsed, document);
    let _ = std::fs::remove_dir_all(&outbox);
}

#[test]
fn empty_metrics_are_zero_and_none_with_valid_json() {
    let document = Diagnostics::new(EmptyStore, settings(), outbox_with(&[]))
        .export()
        .expect("export");

    assert_eq!(document.metrics.latency_capture_to_candidate_ms.samples, 0);
    assert_eq!(document.metrics.latency_capture_to_candidate_ms.p50, None);
    assert_eq!(document.metrics.latency_capture_to_candidate_ms.p95, None);
    assert_eq!(document.metrics.review_time_ms.samples, 0);
    assert_eq!(document.metrics.review_time_ms.p95, None);
    assert_eq!(document.metrics.noise.decided_total, 0);
    assert_eq!(document.metrics.noise.dismissed_ratio, None);
    assert_eq!(document.metrics.losses.assessments_failed, 0);
    assert_eq!(document.metrics.losses.outbox_rejected, 0);

    let json = serde_json::to_string_pretty(&document).expect("serialize");
    assert!(json.contains("\"metrics\""));
    assert!(json.contains("\"dismissed_ratio\": null"));
    let reparsed: application::diagnostics::DiagnosticsDocument =
        serde_json::from_str(&json).expect("round trip");
    assert_eq!(reparsed, document);
}

/// Creates an outbox root with `count` JSON files in each named bucket.
fn outbox_with(buckets: &[(&str, usize)]) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let root = std::env::temp_dir().join(format!(
        "xemnas-diagnostics-outbox-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&root);
    for (bucket, count) in buckets {
        let directory = root.join(bucket);
        std::fs::create_dir_all(&directory).expect("create bucket");
        for index in 0..*count {
            std::fs::write(directory.join(format!("{index}.json")), "{}").expect("write item");
        }
    }
    root
}
