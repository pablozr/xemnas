//! OpenAI-compatible AI provider adapter.
//!
//! This is the one real provider selected for Gate 3 (MVP-SPEC §12): an
//! HTTP adapter that turns a [`DecisionEvidence`] plus local relevance signals
//! into [`CandidateProposal`]s through an OpenAI-compatible
//! `/chat/completions` endpoint. It is infrastructure, so HTTP lives here and
//! not in `application`/`domain` (ARCH-001).
//!
//! Safety properties enforced here:
//!
//! - **Consent first.** [`OpenAiCompatibleExtractor::extract`] refuses to touch
//!   the network unless the profile has `external_calls_enabled` and a consent
//!   record; the composition root also gates on [`choose_extractor`].
//! - **Strict structured output.** The response is parsed into
//!   `deny_unknown_fields` types; extra fields, oversized fields, oversized
//!   responses and non-2xx statuses all fail. The model's text never becomes a
//!   SQL string, a path or a command.
//! - **Sanitized errors.** Messages carry status codes and failure categories
//!   only — never the response body, a URL with query, or the secret.
//!
//! [`choose_extractor`]: application::profile::choose_extractor

#![warn(missing_docs)]

use std::fmt;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use application::analysis::ExtractorFactory;
use application::extract::{
    run_connection_test, truncate_content, CandidateExtractor, CandidateProposal,
    ConnectionTestReport, DecisionEvidence, ExtractError, RelevanceSignal, MAX_DIFF_SUMMARY_FILES,
};
use application::profile::{consent_status, AiProfile, ProfileError, ProfileKind, SecretStore};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Maximum bytes read from a provider response.
const MAX_RESPONSE_BYTES: usize = 512 * 1024;

/// Maximum bytes of user content sent in one request.
const MAX_INPUT_BYTES: usize = 64 * 1024;

/// Maximum characters accepted in a single model field.
const MAX_FIELD_CHARS: usize = 4_000;

/// Connection timeout for the provider request.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Total timeout for the provider request.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

/// Service name used for the OS keychain entries.
pub const KEYRING_SERVICE: &str = "xemnas.ai-profile";

/// System prompt describing the strict JSON contract expected back.
const SYSTEM_PROMPT: &str = "You extract durable engineering decisions from a capture. \
Reply with a single JSON object only, no prose, matching exactly: \
{\"proposals\":[{\"question\":string,\"choice\":string,\"rationale\":string,\
\"confidence\":number,\"confidence_reason\":string,\"evidence_refs\":[string],\
\"diff_summary\":{\"files\":[string],\"artifacts\":number}}]}. \
Use only evidence_refs from the provided artifact ids. No extra fields.";

/// Bounded retry policy for transient provider failures.
///
/// Only transport errors (connect/timeout), HTTP 429 and 5xx are retried; a
/// parse failure or any other 4xx fails on the first attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum total attempts, including the first.
    pub max_attempts: u32,
    /// Delay before the first retry.
    pub base_delay: Duration,
    /// Upper bound for the exponential backoff.
    pub max_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(4),
        }
    }
}

impl RetryPolicy {
    /// Exponential backoff that follows attempt `attempt` (1-based).
    pub fn delay_for(&self, attempt: u32) -> Duration {
        let exponent = attempt.saturating_sub(1).min(20);
        let factor = 1u32.checked_shl(exponent).unwrap_or(u32::MAX);
        self.base_delay.saturating_mul(factor).min(self.max_delay)
    }
}

/// Result of one provider attempt.
enum Attempt {
    /// A validated batch came back.
    Success(Vec<CandidateProposal>),
    /// A transient failure worth retrying.
    Transient,
    /// A failure that must not be retried.
    Fatal(ExtractError),
}

/// Runs the Settings → IA "teste com resposta estruturada" (MVP-SPEC §8).
///
/// Sends only the fixed synthetic evidence from
/// [`application::extract::connection_test_evidence`] through the same
/// consent gate, request shape, retries and strict parsing as a real
/// extraction, then validates the proposals. Nothing is persisted and no
/// capture content leaves the machine. It blocks on the network: callers on
/// the UI must run it off the UI thread (ASYNC-001).
///
/// # Errors
///
/// [`ExtractError::Extractor`] when consent is missing or stale, the profile
/// is invalid or the provider call fails; [`ExtractError::Validation`] when the
/// answer breaks the candidate contract.
pub fn test_connection(
    profile: &AiProfile,
    secret: String,
) -> Result<ConnectionTestReport, ExtractError> {
    let extractor = OpenAiCompatibleExtractor::new(profile, secret)?;
    run_connection_test(&extractor)
}

/// Builds [`OpenAiCompatibleExtractor`]s for the analysis job.
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenAiCompatibleFactory;

impl ExtractorFactory for OpenAiCompatibleFactory {
    type Extractor = OpenAiCompatibleExtractor;

    fn external(
        &self,
        profile: &AiProfile,
        secret: String,
    ) -> Result<OpenAiCompatibleExtractor, ExtractError> {
        OpenAiCompatibleExtractor::new(profile, secret)
    }
}

/// An OpenAI-compatible candidate extractor.
pub struct OpenAiCompatibleExtractor {
    profile: AiProfile,
    secret: String,
    client: reqwest::blocking::Client,
    retry: RetryPolicy,
    sleep: Arc<dyn Fn(Duration) + Send + Sync>,
}

impl fmt::Debug for OpenAiCompatibleExtractor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiCompatibleExtractor")
            .field("profile_id", &self.profile.id)
            .field("model", &self.profile.model)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatibleExtractor {
    /// Builds the extractor for a validated profile and a non-empty secret.
    ///
    /// No network call happens here. An empty secret is rejected: a local
    /// provider without authentication is a debt for a later ticket.
    pub fn new(profile: &AiProfile, secret: String) -> Result<Self, ExtractError> {
        if profile.kind != ProfileKind::OpenAiCompatible {
            return Err(ExtractError::Extractor(
                "o perfil não é compatível com OpenAI".to_string(),
            ));
        }
        profile
            .validate()
            .map_err(|_| ExtractError::Extractor("perfil de IA inválido".to_string()))?;
        if secret.trim().is_empty() {
            return Err(ExtractError::Extractor(
                "a chave do provedor não está configurada".to_string(),
            ));
        }
        let client = reqwest::blocking::Client::builder()
            // Consent approves this destination only, including its TLS scheme.
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| {
                ExtractError::Extractor("não foi possível preparar o cliente HTTP".to_string())
            })?;
        Ok(Self {
            profile: profile.clone(),
            secret,
            client,
            retry: RetryPolicy::default(),
            sleep: Arc::new(|delay: Duration| std::thread::sleep(delay)),
        })
    }

    /// Replaces the retry policy (builder style).
    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Replaces the sleep function; tests inject a no-op to stay fast.
    pub fn with_sleep(mut self, sleep: Arc<dyn Fn(Duration) + Send + Sync>) -> Self {
        self.sleep = sleep;
        self
    }
}

impl CandidateExtractor for OpenAiCompatibleExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        // Defense in depth: use the same single consent predicate as the
        // composition root, so a stale or forged consent can never call out.
        if let Err(reason) = consent_status(&self.profile) {
            return Err(ExtractError::Extractor(format!(
                "chamadas externas bloqueadas: {reason}"
            )));
        }

        let endpoint = self
            .profile
            .endpoint
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches('/');
        let url = format!("{endpoint}/chat/completions");
        let body = json!({
            "model": self.profile.model,
            "temperature": 0,
            "response_format": { "type": "json_object" },
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": build_user_content(&self.profile, input, signals) }
            ]
        });

        let mut attempt = 1u32;
        loop {
            match self.attempt(&url, &body, signals) {
                Attempt::Success(proposals) => return Ok(proposals),
                Attempt::Fatal(error) => return Err(error),
                Attempt::Transient => {
                    if attempt >= self.retry.max_attempts {
                        return Err(ExtractError::Extractor(format!(
                            "o provedor falhou após {attempt} tentativa(s)"
                        )));
                    }
                    (self.sleep)(self.retry.delay_for(attempt));
                    attempt += 1;
                }
            }
        }
    }
}

impl OpenAiCompatibleExtractor {
    /// Performs one request and classifies the outcome.
    ///
    /// Transient outcomes are connect/timeout transport errors, HTTP 429 and
    /// 5xx; everything else (other transport errors, other 4xx, oversized or
    /// malformed bodies, validation) is fatal on the first attempt.
    fn attempt(&self, url: &str, body: &serde_json::Value, signals: &[RelevanceSignal]) -> Attempt {
        let mut request = self.client.post(url).json(body);
        if !self.secret.is_empty() {
            request = request.bearer_auth(&self.secret);
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(error) => {
                if error.is_timeout() || error.is_connect() {
                    return Attempt::Transient;
                }
                return Attempt::Fatal(sanitized_transport_error(error));
            }
        };

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 429 || status.is_server_error() {
                return Attempt::Transient;
            }
            return Attempt::Fatal(ExtractError::Extractor(format!(
                "o provedor respondeu {}",
                status.as_u16()
            )));
        }

        let mut buffer = Vec::new();
        if response
            .take(MAX_RESPONSE_BYTES as u64 + 1)
            .read_to_end(&mut buffer)
            .is_err()
        {
            return Attempt::Fatal(ExtractError::Extractor(
                "falha ao ler a resposta do provedor".to_string(),
            ));
        }
        if buffer.len() > MAX_RESPONSE_BYTES {
            return Attempt::Fatal(ExtractError::Extractor(
                "a resposta do provedor excedeu o limite".to_string(),
            ));
        }

        let envelope: ChatEnvelope = match serde_json::from_slice(&buffer) {
            Ok(envelope) => envelope,
            Err(_) => {
                return Attempt::Fatal(ExtractError::Extractor(
                    "resposta do provedor inválida".to_string(),
                ))
            }
        };
        let Some(choice) = envelope.choices.into_iter().next() else {
            return Attempt::Fatal(ExtractError::Extractor(
                "resposta do provedor sem escolhas".to_string(),
            ));
        };
        let Some(content) = choice.message.content else {
            return Attempt::Fatal(ExtractError::Extractor(
                "resposta do provedor sem conteúdo".to_string(),
            ));
        };
        match serde_json::from_str::<ModelEnvelope>(&content) {
            Ok(model) => match model.into_proposals(signals) {
                Ok(proposals) => Attempt::Success(proposals),
                Err(error) => Attempt::Fatal(error),
            },
            Err(_) => Attempt::Fatal(ExtractError::Extractor(
                "conteúdo do provedor inválido".to_string(),
            )),
        }
    }
}

/// Maps a transport failure to a fixed, sanitized category.
fn sanitized_transport_error(error: reqwest::Error) -> ExtractError {
    let category = if error.is_timeout() {
        "tempo esgotado"
    } else if error.is_connect() {
        "falha de conexão"
    } else {
        "falha na chamada externa"
    };
    ExtractError::Extractor(category.to_string())
}

/// Builds the user content from signals and bounded artifact content.
fn build_user_content(
    profile: &AiProfile,
    input: &DecisionEvidence,
    signals: &[RelevanceSignal],
) -> String {
    let labels: Vec<&str> = signals.iter().map(RelevanceSignal::as_str).collect();
    let mut text = format!("Relevance signals: {}\n", labels.join(", "));
    for artifact in &input.artifacts {
        let content = truncate_content(&artifact.content, profile.max_input_chars);
        text.push_str(&format!(
            "\n### artifact {} ({})\n{}\n",
            artifact.artifact_id, artifact.kind, content
        ));
        if text.len() >= MAX_INPUT_BYTES {
            break;
        }
    }
    truncate_content(&text, MAX_INPUT_BYTES)
}

/// Minimal shape of an OpenAI-compatible `/chat/completions` response.
///
/// Real providers add fields we do not need (`usage`, `system_fingerprint`,
/// `service_tier`, `refusal`, `tool_calls`), so this outer envelope is
/// intentionally permissive; the strict, `deny_unknown_fields` contract is
/// applied to the JSON string carried inside `choices[0].message.content`.
#[derive(Debug, Deserialize)]
struct ChatEnvelope {
    choices: Vec<ChatChoice>,
}

/// One chat completion choice; only the message is needed.
#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// The assistant message; `content` is required to carry the JSON contract.
#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

/// Strict provider response envelope (the model's own JSON).
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelEnvelope {
    proposals: Vec<ModelProposal>,
}

/// Strict provider proposal; every field is required and extras are rejected.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelProposal {
    question: String,
    choice: String,
    rationale: String,
    confidence: f64,
    confidence_reason: String,
    evidence_refs: Vec<String>,
    diff_summary: ModelDiffSummary,
}

/// Strict diff summary shape expected by `validate_diff_summary`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelDiffSummary {
    files: Vec<String>,
    artifacts: usize,
}

impl ModelEnvelope {
    /// Converts the model output into proposals, enforcing field bounds.
    ///
    /// Relevance signals always come from the local filter: the model does not
    /// decide whether a capture is relevant.
    fn into_proposals(
        self,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let mut proposals = Vec::new();
        for proposal in self.proposals {
            for field in [
                &proposal.question,
                &proposal.choice,
                &proposal.rationale,
                &proposal.confidence_reason,
            ] {
                if field.chars().count() > MAX_FIELD_CHARS {
                    return Err(ExtractError::Extractor(
                        "resposta do provedor com campo longo demais".to_string(),
                    ));
                }
            }
            if proposal.diff_summary.files.len() > MAX_DIFF_SUMMARY_FILES {
                return Err(ExtractError::Extractor(
                    "resposta do provedor com arquivos demais".to_string(),
                ));
            }
            for file in &proposal.diff_summary.files {
                if file.trim().is_empty() || file.chars().count() > MAX_FIELD_CHARS {
                    return Err(ExtractError::Extractor(
                        "resposta do provedor com arquivo inválido".to_string(),
                    ));
                }
            }
            for reference in &proposal.evidence_refs {
                if reference.chars().count() > MAX_FIELD_CHARS {
                    return Err(ExtractError::Extractor(
                        "resposta do provedor com referência longa demais".to_string(),
                    ));
                }
            }
            let diff_summary = serde_json::to_string(&proposal.diff_summary).map_err(|_| {
                ExtractError::Extractor("resposta do provedor inválida".to_string())
            })?;
            proposals.push(CandidateProposal {
                question: proposal.question,
                choice: proposal.choice,
                rationale: proposal.rationale,
                confidence: proposal.confidence,
                confidence_reason: proposal.confidence_reason,
                signals: signals.to_vec(),
                evidence_refs: proposal.evidence_refs,
                diff_summary,
            });
        }
        Ok(proposals)
    }
}

/// OS keychain secret store (Credential Manager on Windows).
#[derive(Debug, Clone)]
pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    /// Creates the store bound to the product's service name.
    pub fn new() -> Self {
        Self {
            service: KEYRING_SERVICE.to_string(),
        }
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixed diagnostic for an OS keychain failure; never echoes the entry data.
fn keyring_error() -> ProfileError {
    ProfileError::Io("falha no armazenamento seguro do sistema".to_string())
}

impl SecretStore for KeyringSecretStore {
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
        let entry = keyring::Entry::new(&self.service, account).map_err(|_| keyring_error())?;
        entry.set_password(secret).map_err(|_| keyring_error())
    }

    fn get_secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
        let entry = keyring::Entry::new(&self.service, account).map_err(|_| keyring_error())?;
        match entry.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(keyring_error()),
        }
    }

    fn delete_secret(&self, account: &str) -> Result<(), ProfileError> {
        let entry = keyring::Entry::new(&self.service, account).map_err(|_| keyring_error())?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(keyring_error()),
        }
    }
}
