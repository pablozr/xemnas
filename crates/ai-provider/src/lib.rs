//! AI provider adapters: OpenAI-compatible endpoints (API key or local
//! model), the ChatGPT plan and the OpenCode Zen/Go gateway (ADR-0004).

#![warn(missing_docs)]

pub mod catalog;
pub mod chatgpt;
pub mod opencode;

pub use catalog::HttpModelCatalog;
pub use chatgpt::{ChatGptExtractor, ChatGptSession, MANAGE_USAGE_URL, PLAN_SCOPE};
pub use opencode::{opencode_wire, OpenCodeExtractor, Wire};

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
pub(crate) const MAX_RESPONSE_BYTES: usize = 512 * 1024;

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
pub(crate) const SYSTEM_PROMPT: &str = "You read one turn of a coding session (the user's \
message, the assistant's answer, tool summaries and code diffs) and extract the durable \
engineering decisions it contains, for a human to confirm later.\n\
A decision is a choice that will still matter in months: architecture, data model or \
persistence, public contracts and APIs, dependencies, security and privacy, conventions the \
team must follow, or an explicit trade-off between alternatives. Not decisions: routine \
fixes, renames, formatting, tests alone, cosmetic UI tweaks, or restating what code does.\n\
Rules: propose at most 3 decisions, only the clearest; return an empty list when there is \
none, which is common and correct. Write every text field in the language of the \
conversation. question: the problem, as a short question (\"Onde guardar os segredos?\"). \
choice: what was chosen, in one sentence. rationale: why, including the rejected \
alternative or trade-off when stated. confidence: 0 to 1, how sure you are that this is a \
durable decision actually taken (not merely discussed); confidence_reason: one sentence. \
evidence_refs: copy the ids exactly as written after \"### artifact\" for the artifacts \
that support the decision. diff_summary: files the decision touched and the number of \
artifacts (the app recomputes both).\n\
Reply with a single JSON object only, no prose, matching exactly: \
{\"proposals\":[{\"question\":string,\"choice\":string,\"rationale\":string,\
\"confidence\":number,\"confidence_reason\":string,\"evidence_refs\":[string],\
\"diff_summary\":{\"files\":[string],\"artifacts\":number}}]}. No extra fields.";

/// Bounded retry policy for transient provider failures.
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
pub(crate) enum Attempt {
    /// A validated batch came back.
    Success(Vec<CandidateProposal>),
    /// A transient failure worth retrying.
    Transient,
    /// A failure that must not be retried.
    Fatal(ExtractError),
}

/// Runs the Settings → IA "teste com resposta estruturada" (MVP-SPEC §8).
pub fn test_connection(
    profile: &AiProfile,
    secret: String,
) -> Result<ConnectionTestReport, ExtractError> {
    let extractor = OpenAiCompatibleExtractor::new(profile, secret)?;
    run_connection_test(&extractor)
}

/// Builds the extractor for any external profile kind; the ChatGPT plan
/// shares the process-wide session so token refreshes never race.
#[derive(Debug, Clone)]
pub struct ProviderFactory {
    chatgpt: Arc<ChatGptSession>,
}

impl ProviderFactory {
    /// Wraps the shared ChatGPT session.
    pub fn new(chatgpt: Arc<ChatGptSession>) -> Self {
        Self { chatgpt }
    }
}

/// One of the external extractors.
#[derive(Debug)]
pub enum ProviderExtractor {
    /// API key or local model.
    OpenAiCompatible(OpenAiCompatibleExtractor),
    /// The user's ChatGPT plan.
    ChatGpt(ChatGptExtractor),
    /// OpenCode Zen or Go with the user's key.
    OpenCode(OpenCodeExtractor),
}

impl CandidateExtractor for ProviderExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        match self {
            Self::OpenAiCompatible(extractor) => extractor.extract(input, signals),
            Self::ChatGpt(extractor) => extractor.extract(input, signals),
            Self::OpenCode(extractor) => extractor.extract(input, signals),
        }
    }
}

impl ExtractorFactory for ProviderFactory {
    type Extractor = ProviderExtractor;

    fn external(
        &self,
        profile: &AiProfile,
        secret: String,
    ) -> Result<ProviderExtractor, ExtractError> {
        match profile.kind {
            ProfileKind::OpenAiCompatible => OpenAiCompatibleExtractor::new(profile, secret)
                .map(ProviderExtractor::OpenAiCompatible),
            ProfileKind::ChatGptPlan => {
                ChatGptExtractor::new(self.chatgpt.clone(), profile).map(ProviderExtractor::ChatGpt)
            }
            ProfileKind::OpenCode => {
                OpenCodeExtractor::new(profile, secret).map(ProviderExtractor::OpenCode)
            }
            ProfileKind::Fake => Err(ExtractError::Extractor(
                "a heurística local não usa provedor externo".to_string(),
            )),
        }
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
    pub fn new(profile: &AiProfile, secret: String) -> Result<Self, ExtractError> {
        if profile.kind != ProfileKind::OpenAiCompatible {
            return Err(ExtractError::Extractor(
                "o perfil não é compatível com OpenAI".to_string(),
            ));
        }
        profile
            .validate()
            .map_err(|_| ExtractError::Extractor("perfil de IA inválido".to_string()))?;
        // A local model on loopback runs without a key (ADR-0004).
        if secret.trim().is_empty() && profile.credential_required() {
            return Err(ExtractError::Extractor(
                "a chave do provedor não está configurada".to_string(),
            ));
        }
        let client = reqwest::blocking::Client::builder()
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
        match parse_model_output(&content, signals) {
            Ok(proposals) => Attempt::Success(proposals),
            Err(error) => Attempt::Fatal(error),
        }
    }
}

/// Parses and bounds the model's JSON, the same way for every provider.
pub(crate) fn parse_model_output(
    content: &str,
    signals: &[RelevanceSignal],
) -> Result<Vec<CandidateProposal>, ExtractError> {
    match serde_json::from_str::<ModelEnvelope>(content) {
        Ok(model) => model.into_proposals(signals),
        Err(_) => Err(ExtractError::Extractor(
            "conteúdo do provedor inválido".to_string(),
        )),
    }
}

/// Strict JSON Schema of the model output, used as structured output by the
/// Responses API (ChatGPT plan) and OpenCode. Mirrors [`ModelEnvelope`].
pub(crate) fn output_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["proposals"],
        "properties": {
            "proposals": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": [
                        "question", "choice", "rationale", "confidence",
                        "confidence_reason", "evidence_refs", "diff_summary"
                    ],
                    "properties": {
                        "question": { "type": "string" },
                        "choice": { "type": "string" },
                        "rationale": { "type": "string" },
                        "confidence": { "type": "number" },
                        "confidence_reason": { "type": "string" },
                        "evidence_refs": { "type": "array", "items": { "type": "string" } },
                        "diff_summary": {
                            "type": "object",
                            "additionalProperties": false,
                            "required": ["files", "artifacts"],
                            "properties": {
                                "files": { "type": "array", "items": { "type": "string" } },
                                "artifacts": { "type": "integer" }
                            }
                        }
                    }
                }
            }
        }
    })
}

/// Maps a transport failure to a fixed, sanitized category.
pub(crate) fn sanitized_transport_error(error: reqwest::Error) -> ExtractError {
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
pub(crate) fn build_user_content(
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
