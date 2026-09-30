//! Model catalog for every provider kind (ADR-0004).

use std::sync::Arc;
use std::time::Duration;

use application::profile::{AiProfile, ProfileKind, CHATGPT_API_BASE};
use application::providers::{ModelCatalog, ModelInfo, ProviderError};

use crate::chatgpt::ChatGptSession;
use crate::opencode::OPENCODE_USER;

const TIMEOUT: Duration = Duration::from_secs(15);

/// Lists models over HTTP; the ChatGPT plan uses the shared session's token.
pub struct HttpModelCatalog {
    chatgpt: Arc<ChatGptSession>,
}

impl std::fmt::Debug for HttpModelCatalog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpModelCatalog")
            .finish_non_exhaustive()
    }
}

impl HttpModelCatalog {
    /// Builds the catalog over the process-wide ChatGPT session.
    pub fn new(chatgpt: Arc<ChatGptSession>) -> Self {
        Self { chatgpt }
    }
}

fn client() -> Result<reqwest::blocking::Client, ProviderError> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .timeout(TIMEOUT)
        .build()
        .map_err(|_| ProviderError::Unreachable)
}

fn fetch(request: reqwest::blocking::RequestBuilder) -> Result<serde_json::Value, ProviderError> {
    let response = request.send().map_err(|_| ProviderError::Unreachable)?;
    match response.status().as_u16() {
        200..=299 => response.json().map_err(|_| ProviderError::InvalidResponse),
        401 | 403 => Err(ProviderError::Unauthorized),
        _ => Err(ProviderError::InvalidResponse),
    }
}

/// OpenAI-compatible `{"data":[{"id":…}]}` (Ollama, LM Studio, OpenRouter…)
/// or the plan's `{"models":[{"slug","display_name","visibility"}]}`, keeping
/// only models meant to be listed.
pub fn parse_openai_models(value: &serde_json::Value) -> Vec<ModelInfo> {
    if let Some(models) = value.get("models").and_then(|models| models.as_array()) {
        return models
            .iter()
            .filter(|model| {
                model
                    .get("visibility")
                    .and_then(|visibility| visibility.as_str())
                    .is_none_or(|visibility| visibility == "list")
            })
            .filter_map(|model| {
                let id = model.get("slug").or_else(|| model.get("id"))?.as_str()?;
                let label = model
                    .get("display_name")
                    .and_then(|name| name.as_str())
                    .unwrap_or(id);
                Some(ModelInfo {
                    id: id.to_string(),
                    label: label.to_string(),
                })
            })
            .collect();
    }
    let mut models: Vec<ModelInfo> = value
        .get("data")
        .and_then(|data| data.as_array())
        .into_iter()
        .flatten()
        .filter_map(|model| model.get("id")?.as_str())
        .map(|id| ModelInfo {
            id: id.to_string(),
            label: id.to_string(),
        })
        .collect();
    models.sort_by(|a, b| a.id.cmp(&b.id));
    models
}

/// OpenCode `GET /provider`: `{ all: [{ id, name, models: { key: { id?, name? } } }],
/// connected: [id] }`. Only connected providers are offered; ids become
/// `provider/model`.
pub fn parse_opencode_providers(value: &serde_json::Value) -> Vec<ModelInfo> {
    let providers = value
        .get("all")
        .or_else(|| value.get("providers"))
        .and_then(|providers| providers.as_array());
    let connected: Option<Vec<&str>> = value
        .get("connected")
        .and_then(|connected| connected.as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_str()).collect());
    let mut models = Vec::new();
    for provider in providers.into_iter().flatten() {
        let Some(provider_id) = provider.get("id").and_then(|id| id.as_str()) else {
            continue;
        };
        if connected
            .as_ref()
            .is_some_and(|connected| !connected.contains(&provider_id))
        {
            continue;
        }
        let provider_name = provider
            .get("name")
            .and_then(|name| name.as_str())
            .unwrap_or(provider_id);
        let Some(map) = provider.get("models").and_then(|models| models.as_object()) else {
            continue;
        };
        for (key, model) in map {
            let model_id = model.get("id").and_then(|id| id.as_str()).unwrap_or(key);
            let model_name = model
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or(model_id);
            models.push(ModelInfo {
                id: format!("{provider_id}/{model_id}"),
                label: format!("{provider_name} · {model_name}"),
            });
        }
    }
    models.sort_by(|a, b| a.label.cmp(&b.label));
    models
}

impl ModelCatalog for HttpModelCatalog {
    fn list(
        &self,
        profile: &AiProfile,
        secret: Option<String>,
    ) -> Result<Vec<ModelInfo>, ProviderError> {
        let client = client()?;
        let secret = secret.filter(|secret| !secret.is_empty());
        match profile.kind {
            ProfileKind::Fake => Ok(Vec::new()),
            ProfileKind::OpenAiCompatible => {
                let endpoint = profile
                    .endpoint
                    .as_deref()
                    .ok_or(ProviderError::Unreachable)?
                    .trim_end_matches('/');
                let mut request = client.get(format!("{endpoint}/models"));
                if let Some(secret) = secret {
                    request = request.bearer_auth(secret);
                }
                Ok(parse_openai_models(&fetch(request)?))
            }
            ProfileKind::ChatGptPlan => {
                let token = self.chatgpt.access_token(profile)?;
                let request = client
                    .get(format!("{CHATGPT_API_BASE}/models"))
                    .bearer_auth(token);
                Ok(parse_openai_models(&fetch(request)?))
            }
            ProfileKind::OpenCode => {
                let endpoint = profile
                    .endpoint
                    .as_deref()
                    .ok_or(ProviderError::Unreachable)?
                    .trim_end_matches('/');
                let mut request = client.get(format!("{endpoint}/provider"));
                if let Some(secret) = secret {
                    request = request.basic_auth(OPENCODE_USER, Some(secret));
                }
                Ok(parse_opencode_providers(&fetch(request)?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn openai_shape_lists_ids_sorted() {
        let value = json!({"object": "list", "data": [{"id": "qwen3:8b"}, {"id": "llama3.2"}]});
        let ids: Vec<String> = parse_openai_models(&value)
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(ids, vec!["llama3.2", "qwen3:8b"]);
    }

    #[test]
    fn plan_shape_keeps_listed_models_with_display_names() {
        let value = json!({"models": [
            {"slug": "gpt-a", "display_name": "GPT A", "visibility": "list"},
            {"slug": "gpt-hidden", "display_name": "Hidden", "visibility": "hide"}
        ]});
        assert_eq!(
            parse_openai_models(&value),
            vec![ModelInfo {
                id: "gpt-a".into(),
                label: "GPT A".into()
            }]
        );
    }

    #[test]
    fn opencode_lists_connected_providers_as_provider_slash_model() {
        let value = json!({
            "all": [
                {"id": "anthropic", "name": "Anthropic", "models": {"claude-x": {"id": "claude-x", "name": "Claude X"}}},
                {"id": "openai", "name": "OpenAI", "models": {"gpt-y": {"name": "GPT Y"}}}
            ],
            "connected": ["anthropic"]
        });
        assert_eq!(
            parse_opencode_providers(&value),
            vec![ModelInfo {
                id: "anthropic/claude-x".into(),
                label: "Anthropic · Claude X".into()
            }]
        );
    }
}
