//! Analysis job: extractor choice by profile, consent gate and setup failures.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use application::analysis::{AnalysisOutcome, AnalyzeCapture, ExtractorFactory};
use application::extract::{
    connection_test_evidence, AssessmentOutcome, AssessmentRecord, AssessmentStore,
    DecisionCandidateRecord, DecisionEvidence, ExtractError, ExtractionStore,
    FakeCandidateExtractor, ProviderSetupError,
};
use application::profile::{
    build_preview, grant_consent, offline_default_profile, AiProfile, AiSettings, ProfileError,
    ProfileKind, ProfileStore, SecretStore,
};

#[derive(Default, Clone)]
struct Store {
    assessments: Arc<Mutex<Vec<AssessmentRecord>>>,
    candidates: Arc<Mutex<Vec<DecisionCandidateRecord>>>,
}

impl Store {
    fn outcomes(&self) -> Vec<(AssessmentOutcome, Option<String>, String)> {
        self.assessments
            .lock()
            .expect("lock")
            .iter()
            .map(|row| (row.outcome, row.error_code.clone(), row.profile_id.clone()))
            .collect()
    }
}

impl ExtractionStore for Store {
    fn load_evidence(&self, _capture_id: &str) -> Result<Option<DecisionEvidence>, ExtractError> {
        Ok(Some(connection_test_evidence()))
    }

    fn insert_candidates(
        &self,
        records: &[DecisionCandidateRecord],
    ) -> Result<usize, ExtractError> {
        self.candidates
            .lock()
            .expect("lock")
            .extend_from_slice(records);
        Ok(records.len())
    }
}

impl AssessmentStore for Store {
    fn record_assessment(&self, row: &AssessmentRecord) -> Result<(), ExtractError> {
        self.assessments.lock().expect("lock").push(row.clone());
        Ok(())
    }
}

struct Profiles(Result<Option<AiProfile>, ProfileError>);

impl ProfileStore for Profiles {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        self.0.clone()
    }
    fn save(&self, _profile: &AiProfile) -> Result<(), ProfileError> {
        Ok(())
    }
}

#[derive(Default)]
struct Secrets(RefCell<HashMap<String, String>>, bool);

impl SecretStore for Secrets {
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
        self.0
            .borrow_mut()
            .insert(account.to_string(), secret.to_string());
        Ok(())
    }
    fn get_secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
        if self.1 {
            return Err(ProfileError::Io("keystore".to_string()));
        }
        Ok(self.0.borrow().get(account).cloned())
    }
    fn delete_secret(&self, _account: &str) -> Result<(), ProfileError> {
        Ok(())
    }
}

/// Returns the fake extractor as the "external" one, or fails to build it.
struct Factory {
    fails: bool,
}

impl ExtractorFactory for Factory {
    type Extractor = FakeCandidateExtractor;

    fn external(
        &self,
        _profile: &AiProfile,
        secret: String,
    ) -> Result<FakeCandidateExtractor, ExtractError> {
        assert_eq!(
            secret, "sk-synthetic",
            "the stored secret reaches the factory"
        );
        if self.fails {
            return Err(ExtractError::Extractor("config".to_string()));
        }
        Ok(FakeCandidateExtractor)
    }
}

fn external_profile(consented: bool) -> AiProfile {
    let profile = AiProfile {
        id: "profile-1".to_string(),
        kind: ProfileKind::OpenAiCompatible,
        model: "gpt-test".to_string(),
        endpoint: Some("https://api.example.test/v1".to_string()),
        max_input_chars: 4_096,
        external_calls_enabled: true,
        consent: None,
    };
    if !consented {
        return profile;
    }
    grant_consent(
        &profile,
        &build_preview(&profile),
        "2026-01-01T00:00:00Z",
        true,
    )
    .expect("grant consent")
}

fn analyze(
    profile: Result<Option<AiProfile>, ProfileError>,
    secret: Option<&str>,
    keystore_broken: bool,
    factory_fails: bool,
) -> (Result<AnalysisOutcome, ExtractError>, Store) {
    let secrets = Secrets(RefCell::new(HashMap::new()), keystore_broken);
    if let Some(secret) = secret {
        secrets
            .set_secret("profile-1", secret)
            .expect("store secret");
    }
    let store = Store::default();
    let outcome = {
        let analysis = AnalyzeCapture::new(
            store.clone(),
            AiSettings::new(Profiles(profile), secrets),
            Factory {
                fails: factory_fails,
            },
        );
        analysis.run("capture-1", Some("job-1".to_string()))
    };
    (outcome, store)
}

#[test]
fn offline_profile_runs_the_fake_extractor() {
    let (outcome, store) = analyze(Ok(Some(offline_default_profile())), None, false, false);
    let Ok(AnalysisOutcome::Extracted(report)) = outcome else {
        panic!("expected an extraction, got {outcome:?}");
    };
    assert_eq!(report.inserted, 1);
    assert_eq!(store.outcomes()[0].0, AssessmentOutcome::Ok);
}

#[test]
fn external_profile_without_consent_is_skipped() {
    let (outcome, store) = analyze(Ok(Some(external_profile(false))), None, false, false);
    assert_eq!(outcome, Ok(AnalysisOutcome::Skipped));
    assert!(!AnalysisOutcome::Skipped.fails_job());
    assert_eq!(
        store.outcomes(),
        vec![(
            AssessmentOutcome::Skipped,
            Some("consent".to_string()),
            "profile-1".to_string()
        )]
    );
}

#[test]
fn consented_profile_uses_the_factory_with_the_stored_secret() {
    let (outcome, store) = analyze(
        Ok(Some(external_profile(true))),
        Some("sk-synthetic"),
        false,
        false,
    );
    assert!(matches!(outcome, Ok(AnalysisOutcome::Extracted(_))));
    assert_eq!(store.candidates.lock().expect("lock").len(), 1);
}

#[test]
fn setup_failures_record_provenance_and_fail_the_job() {
    let cases = [
        (
            analyze(Ok(Some(external_profile(true))), None, false, false),
            ProviderSetupError::MissingSecret,
            "secret",
        ),
        (
            analyze(Ok(Some(external_profile(true))), None, true, false),
            ProviderSetupError::Keystore,
            "keystore",
        ),
        (
            analyze(
                Ok(Some(external_profile(true))),
                Some("sk-synthetic"),
                false,
                true,
            ),
            ProviderSetupError::InvalidConfig,
            "provider_config",
        ),
    ];
    for ((outcome, store), error, code) in cases {
        let outcome = outcome.expect("recorded");
        assert_eq!(outcome, AnalysisOutcome::SetupFailed(error));
        assert!(outcome.fails_job());
        assert_eq!(store.outcomes()[0].0, AssessmentOutcome::Failed);
        assert_eq!(store.outcomes()[0].1.as_deref(), Some(code));
    }
}

#[test]
fn unreadable_profile_records_a_fixed_unavailable_context() {
    let (outcome, store) = analyze(
        Err(ProfileError::Io("SECRET-MARKER".to_string())),
        None,
        false,
        false,
    );
    assert_eq!(
        outcome,
        Ok(AnalysisOutcome::SetupFailed(
            ProviderSetupError::ProfileUnavailable
        ))
    );
    let rows = store.outcomes();
    assert_eq!(rows[0].1.as_deref(), Some("profile"));
    assert_eq!(rows[0].2, "unavailable");
}
