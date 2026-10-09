//! AI provider adapters: OpenAI-compatible endpoints (API key or local
//! model), the ChatGPT plan, the OpenCode Zen/Go gateway and the local
//! Claude Code CLI (ADR-0004).

#![warn(missing_docs)]

pub mod catalog;
pub mod chatgpt;
pub mod claude_code;
pub mod opencode;

pub use catalog::HttpModelCatalog;
pub use chatgpt::{ChatGptExtractor, ChatGptSession, MANAGE_USAGE_URL, PLAN_SCOPE};
pub use claude_code::{find_claude_code, ClaudeCodeExtractor, LocalClaudeCode};
pub use opencode::{opencode_wire, OpenCodeExtractor, Wire};

use std::fmt;
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use application::analysis::ExtractorFactory;
use application::extract::{
    run_connection_test, truncate_content, CandidateExtractor, CandidateKind, CandidateProposal,
    ConnectionTestReport, DecisionEvidence, ExtractError, ExtractionBackground, RelevanceSignal,
    MAX_DIFF_SUMMARY_FILES, SIGNIFICANCE_CRITERIA,
};
use application::limiter::{ProviderLimiter, Refusal};
use application::profile::{consent_status, AiProfile, ProfileError, ProfileKind, SecretStore};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Maximum bytes read from a provider response.
pub(crate) const MAX_RESPONSE_BYTES: usize = 512 * 1024;

/// Aggregate extraction prompt budget, distinct from the per-artifact profile cap.
/// The assembled prompt is bounded in characters; bytes are used for early stopping.
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
pub(crate) const SYSTEM_PROMPT: &str = concat!(
    "You read one turn of a coding session (the user's \
message, the assistant's answer, tool summaries and code diffs) and propose what is worth \
remembering about the project, for a human to confirm later. Most turns contain nothing: an \
empty list is the common, correct answer.\n\
Set nature to description (a descriptive assertion), inference (an unverified explanation), \
normative (a proposed constraint or choice requiring human review), or unknown. This is \
classification only, never authority or approval. Never turn source user_text into authenticated \
human approval. Do not invent observation identities or verified facts.\n\
Classify each item with kind: \"decision\" = an architectural choice between real \
alternatives (structure, data model, persistence, public contracts, dependencies, security, \
who is responsible for what); \"rule\" = an invariant or constraint the code of one part must \
keep (\"only a human may resolve X\", \"every write goes through a transaction\"); \"detail\" = \
how something was implemented or fixed (a bug fix, a validation, a rename, a test, UI \
polish). Use detail for anything local to one function or file, anything a code review would \
consider routine, and restatements of what the diff does.\n\
Score significance from 0 to 1 with this test; tick every criterion that applies in \
criteria: cross_cutting (affects several parts of the system), data_or_contract (data \
model, persistence format, API or schema), security_or_privacy, external_dependency (new \
library or service), hard_to_reverse, first_of_a_kind (the project never did this before), \
past_problem (it fixes a class of problems that already hurt), constrains_future_work. No \
criterion means significance below 0.3; one criterion around 0.5; two or more 0.7 and up. \
A bug fix is detail unless it establishes a rule for the whole component.\n\
An artifact of kind document is a file of the project's own documentation (ADR, \
specification, README or guide; path and type in its header), not a conversation: propose \
the decisions and rules it states as taken, and nothing for \
plans, open questions, how-to steps, feature descriptions or instructions addressed to \
agents. Rules written for every contributor (\"every migration is forward-only\") are \
rules. Write in the document's language.\n\
Do not propose anything already recorded (listed under \"Already recorded\"), and follow \
the user's taste shown under \"confirmed\" and \"rejected\". At most 3 items (5 for a document), the clearest \
first; merge items that are the same choice.\n\
Write every text field in the language of the conversation. question: the problem, as a \
short question. choice: what was chosen (for a rule, the rule itself), in one sentence. \
rationale: why, including the rejected alternative or trade-off when stated. confidence: 0 \
to 1, how sure you are that it was actually decided (not merely discussed); \
confidence_reason: one sentence. evidence_refs: copy the ids exactly as written after \
\"### artifact\". diff_summary: files touched and number of artifacts (the app recomputes \
both).\n\
When the message lists \"Project map components\", say in components where each item \
applies or is implemented: the part of the project whose code or behavior would change if the \
item changed. name: copy it exactly as listed. quote: copy a stretch of the item's own \
question, choice or rationale that names or clearly points to that part; never a part named \
only to be excluded, avoided, replaced or compared, and never just a word they share. At most \
3, and an empty list when nothing is listed or you are not sure.\n\
Keep explicit attribution, scope and validation restrictions in qualifiers, even when rationale \
is long. Each qualifier text must be a literal excerpt from the cited artifact_id. Do not infer \
missing qualifications; use an empty array when none are stated. \
Reply with a single JSON object only, no prose, with optional nature as specified above: \
{\"proposals\":[{\"kind\":\"decision|rule|detail\",\"question\":string,\"choice\":string,\
\"rationale\":string,\"confidence\":number,\"confidence_reason\":string,\
\"significance\":number,\"criteria\":[string],\"evidence_refs\":[string],\
\"qualifiers\":[{\"kind\":\"attribution|scope|validation\",\"text\":string,\"artifact_id\":string}],\"components\":[{\"name\":string,\"quote\":string}],\"diff_summary\":{\"files\":[string],\"artifacts\":number}}]}. No extra fields.",
    application::plain_rules!()
);

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
pub(crate) enum Attempt<T> {
    /// The model's answer came back.
    Success(T),
    /// A transient failure worth retrying.
    Transient,
    /// The provider asked to slow down (HTTP 429), with its `Retry-After`.
    /// Not retried in the call: the job is requeued and the limiter paused.
    RateLimited(Option<Duration>),
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
    limiter: Option<ProviderLimiter>,
}

impl ProviderFactory {
    /// Wraps the shared ChatGPT session.
    pub fn new(chatgpt: Arc<ChatGptSession>) -> Self {
        Self {
            chatgpt,
            limiter: None,
        }
    }

    /// Routes every call of the extractors this factory builds through the
    /// shared limiter (builder style). Used by the job handlers.
    pub fn with_limiter(mut self, limiter: ProviderLimiter) -> Self {
        self.limiter = Some(limiter);
        self
    }
}

/// Pause of every call after a 429 that gave no `Retry-After`.
const DEFAULT_PAUSE: Duration = Duration::from_secs(20);

/// One of the external extractors, optionally behind the shared limiter.
#[derive(Debug)]
pub struct ProviderExtractor {
    backend: Backend,
    limiter: Option<ProviderLimiter>,
}

/// The concrete external extractor.
#[derive(Debug)]
enum Backend {
    /// API key or local model.
    OpenAiCompatible(OpenAiCompatibleExtractor),
    /// The user's ChatGPT plan.
    ChatGpt(ChatGptExtractor),
    /// OpenCode Zen or Go with the user's key.
    OpenCode(OpenCodeExtractor),
    /// The Claude Code CLI signed in on this machine.
    ClaudeCode(ClaudeCodeExtractor),
}

impl ProviderExtractor {
    /// Runs one provider call under the limiter: waits for a slot, refuses
    /// during a provider pause, and starts a pause after a 429.
    fn limited<T>(
        &self,
        call: impl FnOnce(&Backend) -> Result<T, ExtractError>,
    ) -> Result<T, ExtractError> {
        let Some(limiter) = &self.limiter else {
            return call(&self.backend);
        };
        let _permit = limiter
            .acquire()
            .map_err(|refusal| ExtractError::RateLimited {
                retry_after: Some(match refusal {
                    Refusal::Paused(left) => left,
                    Refusal::Closed => Duration::from_secs(1),
                }),
            })?;
        let result = call(&self.backend);
        if let Err(ExtractError::RateLimited { retry_after }) = &result {
            limiter.pause_for(retry_after.unwrap_or(DEFAULT_PAUSE));
        }
        result
    }
}

impl CandidateExtractor for ProviderExtractor {
    fn extract_authorized(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
        background: &ExtractionBackground,
        authorization: &dyn application::external::Authorization,
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let profile = match &self.backend {
            Backend::OpenAiCompatible(e) => &e.profile,
            Backend::ChatGpt(e) => e.profile(),
            Backend::OpenCode(e) => e.profile(),
            Backend::ClaudeCode(e) => e.profile(),
        };
        let user = build_user_content(profile, input, signals, background);
        let answer = application::overview::StructuredModel::complete_authorized(
            self,
            SYSTEM_PROMPT,
            &user,
            "decision_candidates",
            &output_schema(),
            authorization,
        )?;
        parse_model_output(&answer, signals)
    }
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        self.extract_with(input, signals, &ExtractionBackground::default())
    }

    fn extract_with(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
        background: &ExtractionBackground,
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        self.limited(|backend| match backend {
            Backend::OpenAiCompatible(extractor) => {
                extractor.extract_with(input, signals, background)
            }
            Backend::ChatGpt(extractor) => extractor.extract_with(input, signals, background),
            Backend::OpenCode(extractor) => extractor.extract_with(input, signals, background),
            Backend::ClaudeCode(extractor) => extractor.extract_with(input, signals, background),
        })
    }
}

impl application::overview::StructuredModel for ProviderExtractor {
    fn complete_authorized(
        &self,
        system: &str,
        user: &str,
        name: &str,
        schema: &serde_json::Value,
        authorization: &dyn application::external::Authorization,
    ) -> Result<String, ExtractError> {
        self.limited(|backend| match backend {
            Backend::OpenAiCompatible(e) => {
                e.complete_checked(system, user, name, schema, Some(authorization))
            }
            Backend::ChatGpt(e) => {
                e.complete_checked(system, user, name, schema, Some(authorization))
            }
            Backend::OpenCode(e) => {
                e.complete_checked(system, user, name, schema, Some(authorization))
            }
            Backend::ClaudeCode(e) => {
                e.complete_checked(system, user, name, schema, Some(authorization))
            }
        })
    }
    fn complete(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        self.limited(|backend| match backend {
            Backend::OpenAiCompatible(model) => model.complete(system, user, schema_name, schema),
            Backend::ChatGpt(model) => model.complete(system, user, schema_name, schema),
            Backend::OpenCode(model) => model.complete(system, user, schema_name, schema),
            Backend::ClaudeCode(model) => model.complete(system, user, schema_name, schema),
        })
    }
}

impl ExtractorFactory for ProviderFactory {
    type Extractor = ProviderExtractor;

    fn external(
        &self,
        profile: &AiProfile,
        secret: String,
    ) -> Result<ProviderExtractor, ExtractError> {
        let backend = match profile.kind {
            ProfileKind::OpenAiCompatible => {
                OpenAiCompatibleExtractor::new(profile, secret).map(Backend::OpenAiCompatible)
            }
            ProfileKind::ChatGptPlan => {
                ChatGptExtractor::new(self.chatgpt.clone(), profile).map(Backend::ChatGpt)
            }
            ProfileKind::OpenCode => OpenCodeExtractor::new(profile, secret).map(Backend::OpenCode),
            ProfileKind::ClaudeCode => ClaudeCodeExtractor::new(profile).map(Backend::ClaudeCode),
            ProfileKind::Fake => Err(ExtractError::Extractor(
                "a heurística local não usa provedor externo".to_string(),
            )),
        }?;
        Ok(ProviderExtractor {
            backend,
            limiter: self.limiter.clone(),
        })
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

impl application::overview::StructuredModel for OpenAiCompatibleExtractor {
    fn complete(
        &self,
        system: &str,
        user: &str,
        name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        OpenAiCompatibleExtractor::complete(self, system, user, name, schema)
    }
    fn complete_authorized(
        &self,
        system: &str,
        user: &str,
        name: &str,
        schema: &serde_json::Value,
        authorization: &dyn application::external::Authorization,
    ) -> Result<String, ExtractError> {
        self.complete_checked(system, user, name, schema, Some(authorization))
    }
}

impl CandidateExtractor for OpenAiCompatibleExtractor {
    fn extract(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        self.extract_with(input, signals, &ExtractionBackground::default())
    }

    fn extract_with(
        &self,
        input: &DecisionEvidence,
        signals: &[RelevanceSignal],
        background: &ExtractionBackground,
    ) -> Result<Vec<CandidateProposal>, ExtractError> {
        let user = build_user_content(&self.profile, input, signals, background);
        let text = self.complete(
            SYSTEM_PROMPT,
            &user,
            "decision_candidates",
            &output_schema(),
        )?;
        parse_model_output(&text, signals)
    }
}

impl OpenAiCompatibleExtractor {
    /// Asks the model for one JSON answer (`json_object` mode), with consent,
    /// retries and the response bounds of every call.
    pub fn complete(
        &self,
        system: &str,
        user: &str,
        _schema_name: &str,
        _schema: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        self.complete_checked(system, user, _schema_name, _schema, None)
    }

    pub(crate) fn complete_checked(
        &self,
        system: &str,
        user: &str,
        _schema_name: &str,
        _schema: &serde_json::Value,
        authorization: Option<&dyn application::external::Authorization>,
    ) -> Result<String, ExtractError> {
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
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        });
        let mut attempt = 1u32;
        loop {
            if let Some(authorization) = authorization {
                authorization.check()?;
            }
            match self.attempt(&url, &body) {
                Attempt::Success(text) => return Ok(text),
                Attempt::Fatal(error) => return Err(error),
                Attempt::RateLimited(retry_after) => {
                    return Err(ExtractError::RateLimited { retry_after })
                }
                Attempt::Transient => {
                    if attempt >= self.retry.max_attempts {
                        return Err(ExtractError::Unavailable { attempts: attempt });
                    }
                    (self.sleep)(self.retry.delay_for(attempt));
                    attempt += 1;
                }
            }
        }
    }

    /// Performs one request and classifies the outcome.
    fn attempt(&self, url: &str, body: &serde_json::Value) -> Attempt<String> {
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
            if status.as_u16() == 429 {
                return Attempt::RateLimited(retry_after(response.headers()));
            }
            if status.is_server_error() {
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
        Attempt::Success(content)
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
                        "nature", "kind", "question", "choice", "rationale", "confidence",
                        "confidence_reason", "significance", "criteria", "qualifiers",
                        "evidence_refs", "diff_summary", "components"
                    ],
                    "properties": {
                        "nature": {"type": "string", "enum": ["description", "inference", "normative", "unknown"]},
                        "qualifiers": {
                            "type": "array", "maxItems": application::qualifiers::MAX_QUALIFIERS,
                            "items": {
                                "type": "object", "additionalProperties": false,
                                "required": ["kind", "text", "artifact_id"],
                                "properties": {
                                    "kind": {"type": "string", "enum": ["attribution", "scope", "validation"]},
                                    "text": {"type": "string", "minLength": 1,
                                        "maxLength": application::qualifiers::MAX_QUALIFIER_CHARS},
                                    "artifact_id": {"type": "string", "minLength": 1, "maxLength": 256}
                                }
                            }
                        },
                        "components": {
                            "type": "array",
                            "maxItems": application::extract::MAX_CANDIDATE_COMPONENTS,
                            "items": {
                                "type": "object", "additionalProperties": false,
                                "required": ["name", "quote"],
                                "properties": {
                                    "name": {"type": "string", "minLength": 1, "maxLength": 200},
                                    "quote": {"type": "string", "minLength": 1,
                                        "maxLength": MAX_FIELD_CHARS}
                                }
                            }
                        },
                        "kind": { "type": "string", "enum": ["decision", "rule", "detail"] },
                        "question": { "type": "string" },
                        "choice": { "type": "string" },
                        "rationale": { "type": "string" },
                        "confidence": { "type": "number" },
                        "confidence_reason": { "type": "string" },
                        "significance": { "type": "number" },
                        "criteria": {
                            "type": "array",
                            "items": { "type": "string", "enum": SIGNIFICANCE_CRITERIA }
                        },
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

/// Longest `Retry-After` honoured; a longer one is capped.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(15 * 60);

/// The `Retry-After` header in seconds, capped. An HTTP-date form is not
/// read and falls back to the job backoff.
pub(crate) fn retry_after(headers: &reqwest::header::HeaderMap) -> Option<Duration> {
    let seconds: u64 = headers
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some(Duration::from_secs(seconds).min(MAX_RETRY_AFTER))
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

/// Most lines of background per section sent with a capture.
const MAX_BACKGROUND_LINES: usize = 12;

/// Builds the user content: the names of the map's components (first, so the
/// part that changes least between captures is a stable prefix), what the
/// project already records and what the user accepted or rejected before,
/// then the signals and the bounded artifacts.
pub(crate) fn build_user_content(
    profile: &AiProfile,
    input: &DecisionEvidence,
    signals: &[RelevanceSignal],
    background: &ExtractionBackground,
) -> String {
    let mut text = String::new();
    if !background.components.is_empty() {
        text.push_str("## Project map components (write a name exactly as listed)\n");
        for component in &background.components {
            text.push_str(&format!(
                "- {}\n",
                application::external::limited_text(&component.name, 80)
            ));
        }
        text.push('\n');
    }
    for (title, lines) in [
        (
            "Already recorded in this project (do not propose these again or anything they already cover)",
            &background.known,
        ),
        (
            "The user confirmed items like these before (this is the level worth proposing)",
            &background.confirmed,
        ),
        (
            "The user rejected items like these before (do not propose similar ones)",
            &background.rejected,
        ),
    ] {
        if lines.is_empty() {
            continue;
        }
        text.push_str(&format!("## {title}\n"));
        for line in lines.iter().take(MAX_BACKGROUND_LINES) {
            text.push_str(&format!("- {}\n", application::external::limited_text(line, 300)));
        }
        text.push('\n');
    }
    let labels: Vec<&str> = signals.iter().map(RelevanceSignal::as_str).collect();
    text.push_str(&format!("Relevance signals: {}\n", labels.join(", ")));
    for artifact in &input.artifacts {
        let content =
            application::external::limited_text(&artifact.content, profile.max_input_chars);
        let header = if artifact.kind == "document" {
            let metadata: serde_json::Value =
                serde_json::from_str(&artifact.metadata).unwrap_or_default();
            let field = |name: &str| {
                application::external::protected_text(
                    metadata
                        .get(name)
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default(),
                )
            };
            format!(
                "document: {}, {}, \"{}\"",
                field("file"),
                field("doc_kind"),
                field("title")
            )
        } else {
            artifact.kind.clone()
        };
        text.push_str(&format!(
            "\n### artifact {} ({})\n{}\n",
            artifact.artifact_id, header, content
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
    #[serde(default)]
    nature: application::review_exception::CandidateNature,
    #[serde(default)]
    qualifiers: Vec<application::qualifiers::KnowledgeQualifier>,
    question: String,
    choice: String,
    rationale: String,
    confidence: f64,
    confidence_reason: String,
    evidence_refs: Vec<String>,
    diff_summary: ModelDiffSummary,
    /// `decision`, `rule` or `detail`; absent reads as a decision.
    #[serde(default)]
    kind: Option<String>,
    /// Significance in `[0, 1]`; absent reads as the review threshold.
    #[serde(default)]
    significance: Option<f64>,
    /// Significance criteria ticked; unknown ones are dropped.
    #[serde(default)]
    criteria: Vec<String>,
    /// Map components the item applies to; absent reads as none.
    #[serde(default)]
    components: Vec<ModelComponent>,
}

/// A component of the project map the model says an item applies to.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelComponent {
    name: String,
    quote: String,
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
            for component in &proposal.components {
                if component.name.chars().count() > MAX_FIELD_CHARS
                    || component.quote.chars().count() > MAX_FIELD_CHARS
                {
                    return Err(ExtractError::Extractor(
                        "resposta do provedor com componente longo demais".to_string(),
                    ));
                }
            }
            let diff_summary = serde_json::to_string(&proposal.diff_summary).map_err(|_| {
                ExtractError::Extractor("resposta do provedor inválida".to_string())
            })?;
            proposals.push(CandidateProposal {
                nature: proposal.nature,
                qualifiers: proposal.qualifiers,
                question: proposal.question,
                choice: proposal.choice,
                rationale: proposal.rationale,
                confidence: proposal.confidence,
                confidence_reason: proposal.confidence_reason,
                signals: signals.to_vec(),
                evidence_refs: proposal.evidence_refs,
                diff_summary,
                kind: CandidateKind::parse(proposal.kind.as_deref().unwrap_or("decision")),
                significance: proposal
                    .significance
                    .filter(|value| value.is_finite())
                    .unwrap_or(application::extract::MIN_SIGNIFICANCE)
                    .clamp(0.0, 1.0),
                criteria: proposal
                    .criteria
                    .into_iter()
                    .filter(|criterion| SIGNIFICANCE_CRITERIA.contains(&criterion.as_str()))
                    .collect(),
                components: proposal
                    .components
                    .into_iter()
                    .map(|component| application::extract::ProposedComponent {
                        name: component.name,
                        quote: component.quote,
                    })
                    .collect(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_output_schema_requires_every_proposal_property() {
        let schema = output_schema();
        let item = &schema["properties"]["proposals"]["items"];
        let required = item["required"].as_array().expect("required");
        for key in item["properties"].as_object().expect("properties").keys() {
            assert!(required.iter().any(|value| value.as_str() == Some(key)));
        }
        assert_eq!(item["additionalProperties"], false);
    }

    #[test]
    fn reads_kind_significance_and_known_criteria() {
        let content = r#"{"proposals":[{"kind":"rule","question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","significance":1.7,"criteria":["security_or_privacy","made_up"],"evidence_refs":["a"],"diff_summary":{"files":[],"artifacts":1}}]}"#;
        let proposals =
            parse_model_output(content, &[RelevanceSignal::SecurityPrivacy]).expect("parse");
        assert_eq!(proposals[0].kind, CandidateKind::Rule);
        assert_eq!(proposals[0].significance, 1.0, "clamped");
        assert_eq!(proposals[0].criteria, vec!["security_or_privacy"]);

        let legacy = r#"{"proposals":[{"question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","evidence_refs":["a"],"diff_summary":{"files":[],"artifacts":1}}]}"#;
        let proposals =
            parse_model_output(legacy, &[RelevanceSignal::SecurityPrivacy]).expect("legacy");
        assert_eq!(proposals[0].kind, CandidateKind::Decision);
        assert_eq!(
            proposals[0].significance,
            application::extract::MIN_SIGNIFICANCE
        );
    }

    fn proposal_json(components: &str) -> String {
        format!(
            r#"{{"proposals":[{{"kind":"decision","question":"q","choice":"c","rationale":"r","confidence":0.7,"confidence_reason":"x","evidence_refs":["a"],"diff_summary":{{"files":[],"artifacts":1}}{components}}}]}}"#
        )
    }

    #[test]
    fn the_schema_requires_components_with_a_name_and_a_quote() {
        let schema = output_schema();
        let item = &schema["properties"]["proposals"]["items"];
        assert!(item["required"]
            .as_array()
            .expect("required")
            .iter()
            .any(|value| value == "components"));
        let components = &item["properties"]["components"];
        assert_eq!(components["maxItems"], 3);
        assert_eq!(components["items"]["additionalProperties"], false);
        assert_eq!(
            components["items"]["required"],
            serde_json::json!(["name", "quote"])
        );
    }

    #[test]
    fn components_are_read_and_an_old_answer_without_them_stays_valid() {
        let signals = [RelevanceSignal::SecurityPrivacy];
        let with =
            proposal_json(r#","components":[{"name":"storage-sqlite","quote":"grava no banco"}]"#);
        let proposals = parse_model_output(&with, &signals).expect("parse");
        assert_eq!(proposals[0].components.len(), 1);
        assert_eq!(proposals[0].components[0].name, "storage-sqlite");
        assert_eq!(proposals[0].components[0].quote, "grava no banco");
        let without = parse_model_output(&proposal_json(""), &signals).expect("old answer");
        assert!(without[0].components.is_empty());
        let extra = proposal_json(r#","components":[{"name":"a","quote":"b","why":"c"}]"#);
        assert!(parse_model_output(&extra, &signals).is_err(), "extra field");
    }

    fn evidence(content: &str) -> DecisionEvidence {
        DecisionEvidence {
            capture_id: "c".into(),
            project_id: "p".into(),
            adapter: None,
            session_id: None,
            observed_at: None,
            artifacts: vec![application::extract::EvidenceArtifact {
                artifact_id: "a1".into(),
                kind: "diff_hunk".into(),
                content: content.into(),
                metadata: "{}".into(),
            }],
        }
    }

    fn profile() -> AiProfile {
        AiProfile {
            id: "p".into(),
            kind: ProfileKind::Fake,
            model: String::new(),
            endpoint: None,
            max_input_chars: 10_000,
            external_calls_enabled: false,
            consent: None,
            chatgpt: None,
        }
    }

    fn map(count: usize) -> ExtractionBackground {
        ExtractionBackground {
            components: (0..count)
                .map(|n| application::extract::MapComponent {
                    entity_id: format!("e{n}"),
                    name: format!("component-{n:06}-xxx"),
                })
                .collect(),
            known: vec!["q → c".into()],
            ..ExtractionBackground::default()
        }
    }

    #[test]
    fn the_map_comes_first_and_is_the_same_bytes_for_any_capture() {
        let background = map(60);
        let signals = [RelevanceSignal::SecurityPrivacy];
        let one = build_user_content(&profile(), &evidence("+one"), &signals, &background);
        let other = build_user_content(
            &profile(),
            &evidence("-other\n+more"),
            &signals,
            &background,
        );
        let header = "## Project map components (write a name exactly as listed)\n";
        assert!(one.starts_with(header), "{one}");
        let prefix =
            |text: &str| text[..text.find("## Already recorded").expect("recorded")].to_string();
        assert_eq!(
            prefix(&one),
            prefix(&other),
            "a stable prefix for the cache"
        );
        assert_ne!(one, other);
        let bare = build_user_content(&profile(), &evidence("+one"), &signals, &map(0));
        assert!(!bare.contains("Project map components"));
        // What the list costs: 60 names of 20 characters.
        let bytes = prefix(&one).len();
        println!("gate extracted links: prompt_bytes_60_components={bytes}");
        assert!(bytes <= 2_048, "{bytes} bytes");
    }
}
