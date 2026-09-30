//! Capture analysis job: picks the extractor allowed by the AI profile and runs it.

use crate::extract::{
    fail_provider_setup, record_skipped_assessment, run_extraction, AssessmentStore,
    CandidateExtractor, ExtractError, ExtractionReport, ExtractionStore, FakeCandidateExtractor,
    ProviderSetupError, RunContext,
};
use crate::profile::{
    choose_extractor, AiProfile, AiSettings, ExtractorChoice, ProfileStore, SecretStore,
};

/// Builds the external provider extractor for a consented profile.
pub trait ExtractorFactory {
    /// Extractor produced by this factory.
    type Extractor: CandidateExtractor;

    /// Builds the extractor; an error means the profile configuration is invalid.
    fn external(
        &self,
        profile: &AiProfile,
        secret: String,
    ) -> Result<Self::Extractor, ExtractError>;
}

/// How one analysis ended when no storage error interrupted it.
#[derive(Debug, Clone, PartialEq)]
pub enum AnalysisOutcome {
    /// An extractor ran and its candidates were stored.
    Extracted(ExtractionReport),
    /// External calls lack consent; nothing ran and a `skipped` row was recorded.
    Skipped,
    /// The provider could not start; a `failed` row was recorded.
    SetupFailed(ProviderSetupError),
}

impl AnalysisOutcome {
    /// Whether the job that ran this analysis must be marked as failed.
    pub fn fails_job(&self) -> bool {
        matches!(self, Self::SetupFailed(_))
    }
}

/// Analysis use case run by the `analyze_capture` job.
#[derive(Debug, Clone)]
pub struct AnalyzeCapture<S, P, K, F> {
    store: S,
    settings: AiSettings<P, K>,
    factory: F,
}

impl<S, P, K, F> AnalyzeCapture<S, P, K, F>
where
    S: ExtractionStore + AssessmentStore,
    P: ProfileStore,
    K: SecretStore,
    F: ExtractorFactory,
{
    /// Wraps the store, the AI settings and the external extractor factory.
    pub fn new(store: S, settings: AiSettings<P, K>, factory: F) -> Self {
        Self {
            store,
            settings,
            factory,
        }
    }

    /// Analyzes one capture, reloading the profile so Settings changes apply at once.
    pub fn run(
        &self,
        capture_id: &str,
        job_id: Option<String>,
    ) -> Result<AnalysisOutcome, ExtractError> {
        let Ok(profile) = self.settings.load_or_seed() else {
            return self.setup_failed(
                capture_id,
                &RunContext::unavailable(job_id),
                ProviderSetupError::ProfileUnavailable,
            );
        };
        let context = RunContext::for_profile(&profile, job_id);

        match choose_extractor(Some(&profile)) {
            ExtractorChoice::OfflineFake => {
                self.extract(&FakeCandidateExtractor, capture_id, &context)
            }
            ExtractorChoice::ExternalBlocked => {
                record_skipped_assessment(&self.store, capture_id, &context)?;
                Ok(AnalysisOutcome::Skipped)
            }
            ExtractorChoice::ExternalEnabled => {
                // A local model on loopback and OpenCode run without a stored
                // credential; they receive an empty secret.
                let secret = match self.settings.secret(&profile.credential_account()) {
                    Ok(Some(secret)) => secret,
                    Ok(None) if !profile.credential_required() => String::new(),
                    Ok(None) => {
                        return self.setup_failed(
                            capture_id,
                            &context,
                            ProviderSetupError::MissingSecret,
                        )
                    }
                    Err(_) => {
                        return self.setup_failed(
                            capture_id,
                            &context,
                            ProviderSetupError::Keystore,
                        )
                    }
                };
                match self.factory.external(&profile, secret) {
                    Ok(extractor) => self.extract(&extractor, capture_id, &context),
                    Err(_) => {
                        self.setup_failed(capture_id, &context, ProviderSetupError::InvalidConfig)
                    }
                }
            }
        }
    }

    fn extract<E: CandidateExtractor>(
        &self,
        extractor: &E,
        capture_id: &str,
        context: &RunContext,
    ) -> Result<AnalysisOutcome, ExtractError> {
        run_extraction(&self.store, extractor, capture_id, context).map(AnalysisOutcome::Extracted)
    }

    fn setup_failed(
        &self,
        capture_id: &str,
        context: &RunContext,
        error: ProviderSetupError,
    ) -> Result<AnalysisOutcome, ExtractError> {
        fail_provider_setup(&self.store, capture_id, context, error)?;
        Ok(AnalysisOutcome::SetupFailed(error))
    }
}
