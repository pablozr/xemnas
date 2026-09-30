//! OpenCode as an extraction provider (ADR-0004).
//!
//! Talks to the user's local OpenCode server: creates a session whose title
//! starts with [`EXTRACTION_SESSION_TITLE`] (the xemnas plugin ignores those),
//! sends one prompt with a JSON Schema `format`, reads
//! `info.structured_output` and deletes the session whatever happens.

use std::time::Duration;

use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, RelevanceSignal,
};
use application::profile::{consent_status, AiProfile, ProfileKind};
use serde_json::json;

use crate::{build_user_content, output_schema, parse_model_output, SYSTEM_PROMPT};

/// Title prefix of the sessions xemnas creates; the OpenCode plugin skips
/// sessions whose title starts with it, so an extraction never becomes a capture.
pub const EXTRACTION_SESSION_TITLE: &str = "xemnas · extração";

/// User name OpenCode uses for basic auth when a server password is set.
pub(crate) const OPENCODE_USER: &str = "opencode";

/// OpenCode tools turned off for the extraction prompt: it must only answer.
const DISABLED_TOOLS: &[&str] = &[
    "bash",
    "edit",
    "write",
    "read",
    "grep",
    "glob",
    "list",
    "patch",
    "todowrite",
    "todoread",
    "webfetch",
    "websearch",
    "task",
];

const TIMEOUT: Duration = Duration::from_secs(240);

/// Splits `provider/model` at the first slash.
pub fn split_model(model: &str) -> Option<(&str, &str)> {
    let (provider, model) = model.split_once('/')?;
    (!provider.trim().is_empty() && !model.trim().is_empty()).then_some((provider, model))
}

/// Extractor through the local OpenCode server.
pub struct OpenCodeExtractor {
    profile: AiProfile,
    password: String,
    client: reqwest::blocking::Client,
}

impl std::fmt::Debug for OpenCodeExtractor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OpenCodeExtractor")
            .field("model", &self.profile.model)
            .finish_non_exhaustive()
    }
}

impl OpenCodeExtractor {
    /// Builds the extractor; `password` is the optional server password.
    pub fn new(profile: &AiProfile, password: String) -> Result<Self, ExtractError> {
        if profile.kind != ProfileKind::OpenCode {
            return Err(ExtractError::Extractor(
                "o perfil não usa o OpenCode".to_string(),
            ));
        }
        profile
            .validate()
            .map_err(|_| ExtractError::Extractor("perfil de IA inválido".to_string()))?;
        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(TIMEOUT)
            .build()
            .map_err(|_| {
                ExtractError::Extractor("não foi possível preparar o cliente HTTP".to_string())
            })?;
        Ok(Self {
            profile: profile.clone(),
            password,
            client,
        })
    }

    fn base(&self) -> String {
        self.profile
            .endpoint
            .as_deref()
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string()
    }

    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::blocking::RequestBuilder {
        let request = self
            .client
            .request(method, format!("{}{path}", self.base()));
        if self.password.is_empty() {
            request
        } else {
            request.basic_auth(OPENCODE_USER, Some(&self.password))
        }
    }

    fn prompt(&self, session: &str, text: String) -> Result<String, ExtractError> {
        let (provider, model) = split_model(&self.profile.model)
            .ok_or_else(|| ExtractError::Extractor("modelo do OpenCode inválido".to_string()))?;
        let tools: serde_json::Map<String, serde_json::Value> = DISABLED_TOOLS
            .iter()
            .map(|tool| ((*tool).to_string(), json!(false)))
            .collect();
        let response = self
            .request(
                reqwest::Method::POST,
                &format!("/session/{session}/message"),
            )
            .json(&json!({
                "model": { "providerID": provider, "modelID": model },
                "system": SYSTEM_PROMPT,
                "tools": tools,
                "parts": [{ "type": "text", "text": text }],
                "format": { "type": "json_schema", "schema": output_schema() },
            }))
            .send()
            .map_err(transport)?;
        let status = response.status();
        if !status.is_success() {
            return Err(ExtractError::Extractor(format!(
                "o OpenCode respondeu {}",
                status.as_u16()
            )));
        }
        let value: serde_json::Value = response
            .json()
            .map_err(|_| ExtractError::Extractor("resposta do OpenCode inválida".to_string()))?;
        structured_text(&value)
            .ok_or_else(|| ExtractError::Extractor("o OpenCode não devolveu JSON".to_string()))
    }
}

fn transport(error: reqwest::Error) -> ExtractError {
    ExtractError::Extractor(
        if error.is_timeout() {
            "o OpenCode demorou demais para responder"
        } else if error.is_connect() {
            "o OpenCode não está aberto nesse endereço"
        } else {
            "falha ao falar com o OpenCode"
        }
        .to_string(),
    )
}

/// The model's JSON: `info.structured_output` first, then the text parts.
fn structured_text(value: &serde_json::Value) -> Option<String> {
    if let Some(structured) = value.pointer("/info/structured_output") {
        if !structured.is_null() {
            return serde_json::to_string(structured).ok();
        }
    }
    let text: String = value
        .get("parts")?
        .as_array()?
        .iter()
        .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("text"))
        .filter_map(|part| part.get("text")?.as_str())
        .collect();
    (!text.trim().is_empty()).then_some(text)
}

impl CandidateExtractor for OpenCodeExtractor {
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
        let session: serde_json::Value = self
            .request(reqwest::Method::POST, "/session")
            .json(&json!({ "title": EXTRACTION_SESSION_TITLE }))
            .send()
            .map_err(transport)?
            .json()
            .map_err(|_| ExtractError::Extractor("resposta do OpenCode inválida".to_string()))?;
        let id = session
            .get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| ExtractError::Extractor("o OpenCode não criou a sessão".to_string()))?
            .to_string();
        let text = self.prompt(&id, build_user_content(&self.profile, input, signals));
        // Always remove the session, even when the prompt failed.
        let _ = self
            .request(reqwest::Method::DELETE, &format!("/session/{id}"))
            .send();
        parse_model_output(&text?, signals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_provider_and_model_at_the_first_slash() {
        assert_eq!(
            split_model("openrouter/anthropic/claude"),
            Some(("openrouter", "anthropic/claude"))
        );
        assert_eq!(
            split_model("anthropic/claude"),
            Some(("anthropic", "claude"))
        );
        assert_eq!(split_model("no-slash"), None);
        assert_eq!(split_model("/model"), None);
    }

    #[test]
    fn prefers_structured_output_then_text_parts() {
        let structured = json!({"info": {"structured_output": {"proposals": []}}, "parts": []});
        assert_eq!(
            structured_text(&structured).as_deref(),
            Some("{\"proposals\":[]}")
        );
        let text = json!({"info": {}, "parts": [{"type": "text", "text": "{\"proposals\":[]}"}]});
        assert_eq!(
            structured_text(&text).as_deref(),
            Some("{\"proposals\":[]}")
        );
        assert_eq!(structured_text(&json!({"info": {}, "parts": []})), None);
    }
}
