//! OpenCode Zen and OpenCode Go as an extraction provider (ADR-0004).
//!
//! Both are OpenCode's model gateway reached with the user's OpenCode API
//! key: Zen charges per use, Go is a monthly subscription. The gateway speaks
//! a different wire protocol per model family (OpenAI Responses, Anthropic
//! Messages or OpenAI Chat Completions), so the request follows the model.

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, ExtractionBackground,
    RelevanceSignal,
};
use application::profile::{consent_status, AiProfile, ProfileKind, OPENCODE_GO_ENDPOINT};
use serde_json::json;

use crate::{
    build_user_content, output_schema, parse_model_output, retry_after, sanitized_transport_error,
    Attempt, RetryPolicy, MAX_RESPONSE_BYTES, SYSTEM_PROMPT,
};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
/// Output budget for the Anthropic Messages protocol, which requires one.
const MAX_OUTPUT_TOKENS: u32 = 8_192;
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Wire protocol the gateway uses for a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wire {
    /// `POST /chat/completions` (OpenAI-compatible).
    ChatCompletions,
    /// `POST /responses` (OpenAI Responses API).
    Responses,
    /// `POST /messages` (Anthropic Messages API).
    Messages,
}

/// The protocol the OpenCode gateway at `endpoint` uses for `model`, or
/// `None` for model families xemnas cannot call (Gemini, System One).
///
/// Mirrors the model tables of the Zen and Go documentation
/// (opencode.ai/docs/zen, opencode.ai/docs/go, checked on 2026-09-30): GPT,
/// Grok and Muse go through Responses, Claude through Messages, the rest
/// through Chat Completions, except Qwen (Messages, but Qwen3.8 Max on Zen)
/// and MiniMax (Messages on Go).
pub fn opencode_wire(endpoint: &str, model: &str) -> Option<Wire> {
    let go = endpoint.trim_end_matches('/') == OPENCODE_GO_ENDPOINT;
    let model = model.trim().to_ascii_lowercase();
    let starts = |prefix: &str| model.starts_with(prefix);
    if starts("gemini-") || starts("jev-") {
        return None;
    }
    Some(if starts("gpt-") || starts("grok-") || starts("muse-") {
        Wire::Responses
    } else if starts("claude-") {
        Wire::Messages
    } else if starts("qwen") {
        if !go && starts("qwen3.8-max") {
            Wire::ChatCompletions
        } else {
            Wire::Messages
        }
    } else if starts("minimax-") {
        if go {
            Wire::Messages
        } else {
            Wire::ChatCompletions
        }
    } else {
        Wire::ChatCompletions
    })
}

/// Extractor through OpenCode Zen or OpenCode Go.
pub struct OpenCodeExtractor {
    profile: AiProfile,
    key: String,
    wire: Wire,
    client: reqwest::blocking::Client,
    retry: RetryPolicy,
    sleep: Arc<dyn Fn(Duration) + Send + Sync>,
}

impl std::fmt::Debug for OpenCodeExtractor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenCodeExtractor")
            .field("model", &self.profile.model)
            .field("wire", &self.wire)
            .finish_non_exhaustive()
    }
}

impl OpenCodeExtractor {
    /// Builds the extractor for a validated profile and the OpenCode API key.
    pub fn new(profile: &AiProfile, key: String) -> Result<Self, ExtractError> {
        if profile.kind != ProfileKind::OpenCode {
            return Err(ExtractError::Extractor(
                "o perfil não usa o OpenCode".to_string(),
            ));
        }
        profile
            .validate()
            .map_err(|_| ExtractError::Extractor("perfil de IA inválido".to_string()))?;
        if key.trim().is_empty() {
            return Err(ExtractError::Extractor(
                "a chave do OpenCode não está configurada".to_string(),
            ));
        }
        let wire = opencode_wire(
            profile.endpoint.as_deref().unwrap_or_default(),
            &profile.model,
        )
        .ok_or_else(|| {
            ExtractError::Extractor("esse modelo do OpenCode não é suportado".to_string())
        })?;
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
            key,
            wire,
            client,
            retry: RetryPolicy::default(),
            sleep: Arc::new(|delay: Duration| std::thread::sleep(delay)),
        })
    }

    fn base(&self) -> &str {
        self.profile
            .endpoint
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches('/')
    }

    /// Request URL and body for the model's protocol.
    fn request(
        &self,
        system: &str,
        text: &str,
        schema_name: &str,
        schema: &serde_json::Value,
    ) -> (String, serde_json::Value) {
        let model = &self.profile.model;
        match self.wire {
            Wire::ChatCompletions => (
                format!("{}/chat/completions", self.base()),
                json!({
                    "model": model,
                    "response_format": { "type": "json_object" },
                    "messages": [
                        { "role": "system", "content": system },
                        { "role": "user", "content": text }
                    ]
                }),
            ),
            Wire::Responses => (
                format!("{}/responses", self.base()),
                json!({
                    "model": model,
                    "store": false,
                    "instructions": system,
                    "input": [{
                        "role": "user",
                        "content": [{ "type": "input_text", "text": text }]
                    }],
                    "text": {
                        "format": {
                            "type": "json_schema",
                            "name": schema_name,
                            "schema": schema,
                            "strict": true
                        }
                    }
                }),
            ),
            Wire::Messages => (
                format!("{}/messages", self.base()),
                json!({
                    "model": model,
                    "max_tokens": MAX_OUTPUT_TOKENS,
                    "system": system,
                    "messages": [{ "role": "user", "content": text }]
                }),
            ),
        }
    }

    fn attempt(&self, url: &str, body: &serde_json::Value) -> Attempt<String> {
        let mut request = self.client.post(url).json(body).bearer_auth(&self.key);
        if self.wire == Wire::Messages {
            request = request
                .header("x-api-key", &self.key)
                .header("anthropic-version", ANTHROPIC_VERSION);
        }
        let response = match request.send() {
            Ok(response) => response,
            Err(error) if error.is_timeout() || error.is_connect() => return Attempt::Transient,
            Err(error) => return Attempt::Fatal(sanitized_transport_error(error)),
        };
        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 429 {
                return Attempt::RateLimited(retry_after(response.headers()));
            }
            if status.is_server_error() {
                return Attempt::Transient;
            }
            return Attempt::Fatal(ExtractError::Extractor(match status.as_u16() {
                401 | 403 => "o OpenCode recusou a chave".to_string(),
                code => format!("o OpenCode respondeu {code}"),
            }));
        }
        let mut buffer = Vec::new();
        if response
            .take(MAX_RESPONSE_BYTES as u64 + 1)
            .read_to_end(&mut buffer)
            .is_err()
            || buffer.len() > MAX_RESPONSE_BYTES
        {
            return Attempt::Fatal(ExtractError::Extractor(
                "falha ao ler a resposta do OpenCode".to_string(),
            ));
        }
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&buffer) else {
            return Attempt::Fatal(ExtractError::Extractor(
                "resposta do OpenCode inválida".to_string(),
            ));
        };
        let Some(text) = response_text(self.wire, &value) else {
            return Attempt::Fatal(ExtractError::Extractor(
                "o OpenCode não devolveu texto".to_string(),
            ));
        };
        Attempt::Success(json_object(&text).to_string())
    }

    /// Asks the gateway for one JSON answer in the model's protocol, with
    /// consent and retries.
    pub fn complete(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &serde_json::Value,
    ) -> Result<String, ExtractError> {
        if let Err(reason) = consent_status(&self.profile) {
            return Err(ExtractError::Extractor(format!(
                "chamadas externas bloqueadas: {reason}"
            )));
        }
        let (url, body) = self.request(system, user, schema_name, schema);
        let mut attempt = 1u32;
        loop {
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
}

/// The model's text in each protocol's response shape.
fn response_text(wire: Wire, value: &serde_json::Value) -> Option<String> {
    let text: String = match wire {
        Wire::ChatCompletions => value
            .pointer("/choices/0/message/content")?
            .as_str()?
            .to_string(),
        Wire::Responses => value
            .get("output")?
            .as_array()?
            .iter()
            .filter(|item| item.get("type").and_then(|kind| kind.as_str()) == Some("message"))
            .filter_map(|item| item.get("content")?.as_array())
            .flatten()
            .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("output_text"))
            .filter_map(|part| part.get("text")?.as_str())
            .collect(),
        Wire::Messages => value
            .get("content")?
            .as_array()?
            .iter()
            .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("text"))
            .filter_map(|part| part.get("text")?.as_str())
            .collect(),
    };
    (!text.trim().is_empty()).then_some(text)
}

/// The JSON object inside a reply that may wrap it in a code fence or prose.
fn json_object(text: &str) -> &str {
    match (text.find('{'), text.rfind('}')) {
        (Some(start), Some(end)) if start < end => &text[start..=end],
        _ => text,
    }
}

impl CandidateExtractor for OpenCodeExtractor {
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

#[cfg(test)]
mod tests {
    use super::*;
    use application::profile::OPENCODE_ZEN_ENDPOINT;

    #[test]
    fn routes_each_model_family_to_its_protocol() {
        let zen = OPENCODE_ZEN_ENDPOINT;
        let go = OPENCODE_GO_ENDPOINT;
        assert_eq!(opencode_wire(zen, "gpt-5.5"), Some(Wire::Responses));
        assert_eq!(opencode_wire(go, "grok-4.7"), Some(Wire::Responses));
        assert_eq!(
            opencode_wire(zen, "claude-sonnet-4-5"),
            Some(Wire::Messages)
        );
        assert_eq!(opencode_wire(zen, "kimi-k3"), Some(Wire::ChatCompletions));
        assert_eq!(
            opencode_wire(go, "deepseek-v4-pro"),
            Some(Wire::ChatCompletions)
        );
        assert_eq!(
            opencode_wire(zen, "qwen3.8-max"),
            Some(Wire::ChatCompletions)
        );
        assert_eq!(opencode_wire(go, "qwen3.8-max"), Some(Wire::Messages));
        assert_eq!(opencode_wire(zen, "qwen3.7-plus"), Some(Wire::Messages));
        assert_eq!(
            opencode_wire(zen, "minimax-m3"),
            Some(Wire::ChatCompletions)
        );
        assert_eq!(opencode_wire(go, "minimax-m3"), Some(Wire::Messages));
        assert_eq!(opencode_wire(zen, "gemini-3-flash"), None);
        assert_eq!(opencode_wire(zen, "jev-1.13"), None);
    }

    #[test]
    fn reads_text_from_every_protocol() {
        let chat = json!({"choices": [{"message": {"content": "{\"proposals\":[]}"}}]});
        assert_eq!(
            response_text(Wire::ChatCompletions, &chat).as_deref(),
            Some("{\"proposals\":[]}")
        );
        let responses = json!({"output": [
            {"type": "reasoning", "summary": []},
            {"type": "message", "content": [{"type": "output_text", "text": "{\"proposals\":[]}"}]}
        ]});
        assert_eq!(
            response_text(Wire::Responses, &responses).as_deref(),
            Some("{\"proposals\":[]}")
        );
        let messages = json!({"content": [
            {"type": "thinking", "thinking": "…"},
            {"type": "text", "text": "```json\n{\"proposals\":[]}\n```"}
        ]});
        let text = response_text(Wire::Messages, &messages).expect("text");
        assert_eq!(json_object(&text), "{\"proposals\":[]}");
        assert_eq!(response_text(Wire::Messages, &json!({"content": []})), None);
    }
}
