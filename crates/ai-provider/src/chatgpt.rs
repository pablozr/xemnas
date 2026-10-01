//! Sign in with ChatGPT for open-source local apps (ADR-0004).
//!
//! Follows `developers.openai.com/siwc/token-sharing-open-source`: a browser
//! sign-in with PKCE and a loopback callback, dynamic registration on the first
//! sign-in, the refresh token only in the OS secret store and the access token
//! only in memory. Inference goes through the public Responses API with
//! `store: false` and `stream: true`.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use application::extract::{
    CandidateExtractor, CandidateProposal, DecisionEvidence, ExtractError, ExtractionBackground,
    RelevanceSignal,
};
use application::profile::{
    consent_status, AiProfile, ChatGptAccount, ProfileKind, SecretStore, SignedIn, CHATGPT_API_BASE,
};
use application::providers::{PendingSignIn, PlanAccount, ProviderError};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    build_user_content, output_schema, parse_model_output, Attempt, KeyringSecretStore,
    RetryPolicy, MAX_RESPONSE_BYTES, SYSTEM_PROMPT,
};

/// OpenID issuer of Sign in with ChatGPT.
pub const ISSUER: &str = "https://auth.openai.com";
const AUTHORIZE_URL: &str = "https://auth.openai.com/api/accounts/authorize";
const TOKEN_URL: &str = "https://auth.openai.com/api/accounts/oauth/token";
const REVOKE_URL: &str = "https://auth.openai.com/api/accounts/oauth/revoke";
const USERINFO_URL: &str = "https://auth.openai.com/api/accounts/oauth/userinfo";
/// Client id used only for the first, registering sign-in; never stored.
const DYNAMIC_CLIENT: &str = "dynamic_agent_client";
/// Name shown to the user on the ChatGPT consent page.
const AGENT_NAME: &str = "xemnas";
/// Scopes for identity, refresh and plan usage.
const SCOPES: &str =
    "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
/// The scope that allows inference on the user's plan.
pub const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";
/// Where the user manages plan usage (UI guidelines).
pub const MANAGE_USAGE_URL: &str = application::providers::CHATGPT_USAGE_URL;
/// Callback path on the loopback listener.
const CALLBACK_PATH: &str = "/auth/callback";
/// Refresh this long before the access token expires.
const EXPIRY_MARGIN: Duration = Duration::from_secs(60);
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const INFERENCE_TIMEOUT: Duration = Duration::from_secs(180);

/// PKCE code challenge (S256) for a verifier.
pub fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn random_token(bytes: usize) -> Result<String, ProviderError> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|_| ProviderError::InvalidResponse)?;
    Ok(URL_SAFE_NO_PAD.encode(buffer))
}

/// A new `urn:uuid:` host id (UUID version 4).
fn new_host_id() -> Result<String, ProviderError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| ProviderError::InvalidResponse)?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "urn:uuid:{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn http_client(timeout: Duration) -> Result<reqwest::blocking::Client, ProviderError> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .timeout(timeout)
        .build()
        .map_err(|_| ProviderError::Unreachable)
}

/// Claims the ID token must carry.
#[derive(Debug, Deserialize)]
struct IdClaims {
    iss: String,
    aud: Audience,
    exp: u64,
    #[serde(default)]
    nonce: Option<String>,
    sub: String,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Audience {
    One(String),
    Many(Vec<String>),
}

impl Audience {
    fn contains(&self, client_id: &str) -> bool {
        match self {
            Self::One(audience) => audience == client_id,
            Self::Many(audiences) => audiences.iter().any(|audience| audience == client_id),
        }
    }
}

/// Validates the ID token received directly from the token endpoint over TLS
/// (OpenID Connect Core §3.1.3.7): issuer, audience, expiry and nonce. The
/// RS256 signature check is a recorded follow-up in ADR-0004.
fn validate_id_token(
    id_token: &str,
    client_id: &str,
    nonce: &str,
    now: u64,
) -> Result<IdClaims, ProviderError> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or(ProviderError::InvalidResponse)?;
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|_| ProviderError::InvalidResponse)?;
    let claims: IdClaims =
        serde_json::from_slice(&bytes).map_err(|_| ProviderError::InvalidResponse)?;
    let valid = claims.iss == ISSUER
        && claims.aud.contains(client_id)
        && claims.exp > now
        && claims.nonce.as_deref() == Some(nonce);
    if valid {
        Ok(claims)
    } else {
        Err(ProviderError::InvalidResponse)
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    expires_in: Option<u64>,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenError {
    #[serde(default)]
    error: String,
}

#[derive(Debug, Deserialize)]
struct UserInfo {
    sub: String,
    #[serde(default)]
    email: Option<String>,
}

struct CachedToken {
    client_id: String,
    access_token: String,
    expires_at: Instant,
}

/// The ChatGPT session shared by Settings (sign-in, catalog) and the jobs
/// worker (extraction). One instance per process: refreshes are serialized so
/// two callers never rotate the same refresh token twice.
pub struct ChatGptSession {
    secrets: KeyringSecretStore,
    cache: Mutex<Option<CachedToken>>,
}

impl std::fmt::Debug for ChatGptSession {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChatGptSession")
            .finish_non_exhaustive()
    }
}

impl Default for ChatGptSession {
    fn default() -> Self {
        Self::new(KeyringSecretStore::new())
    }
}

impl ChatGptSession {
    /// Builds the session over the secret store that holds refresh tokens.
    pub fn new(secrets: KeyringSecretStore) -> Self {
        Self {
            secrets,
            cache: Mutex::new(None),
        }
    }

    /// A valid access token for `profile`, refreshing (and rotating the stored
    /// refresh token) when the cached one is about to expire.
    pub fn access_token(&self, profile: &AiProfile) -> Result<String, ProviderError> {
        let account = profile
            .chatgpt
            .as_ref()
            .filter(|account| account.plan_usage)
            .ok_or(ProviderError::NotSignedIn)?;
        let client_id = account
            .client_id
            .clone()
            .ok_or(ProviderError::NotSignedIn)?;
        let mut cache = self.cache.lock().map_err(|_| ProviderError::Keystore)?;
        if let Some(cached) = cache.as_ref() {
            if cached.client_id == client_id && cached.expires_at > Instant::now() + EXPIRY_MARGIN {
                return Ok(cached.access_token.clone());
            }
        }
        let key = profile.credential_account();
        let refresh = self
            .secrets
            .get_secret(&key)
            .map_err(|_| ProviderError::Keystore)?
            .ok_or(ProviderError::NotSignedIn)?;
        let client = http_client(HTTP_TIMEOUT)?;
        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh.as_str()),
                ("client_id", client_id.as_str()),
                ("resource", CHATGPT_API_BASE),
            ])
            .send()
            .map_err(|_| ProviderError::Unreachable)?;
        let status = response.status();
        let body = response.text().map_err(|_| ProviderError::Unreachable)?;
        if !status.is_success() {
            let error = serde_json::from_str::<TokenError>(&body)
                .map(|error| error.error)
                .unwrap_or_default();
            if matches!(
                error.as_str(),
                "invalid_grant" | "invalid_refresh_token" | "token_expired"
            ) {
                // The documented recovery: clear the unusable token and sign in again.
                let _ = self.secrets.delete_secret(&key);
                *cache = None;
                return Err(ProviderError::NotSignedIn);
            }
            return Err(if status.is_server_error() {
                ProviderError::Unreachable
            } else {
                ProviderError::Unauthorized
            });
        }
        let token: TokenResponse =
            serde_json::from_str(&body).map_err(|_| ProviderError::InvalidResponse)?;
        if let Some(rotated) = token.refresh_token.as_deref() {
            // Each refresh returns a replacement token; losing it would sign
            // the user out, so a store failure is an error, not a warning.
            self.secrets
                .set_secret(&key, rotated)
                .map_err(|_| ProviderError::Keystore)?;
        }
        let lifetime = Duration::from_secs(token.expires_in.unwrap_or(3600));
        *cache = Some(CachedToken {
            client_id,
            access_token: token.access_token.clone(),
            expires_at: Instant::now() + lifetime,
        });
        Ok(token.access_token)
    }

    /// Drops the cached access token (after a 401 or a sign-out).
    pub fn invalidate(&self) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = None;
        }
    }
}

impl PlanAccount for ChatGptSession {
    fn start_sign_in(
        &self,
        previous: Option<&ChatGptAccount>,
    ) -> Result<Box<dyn PendingSignIn>, ProviderError> {
        let listener =
            TcpListener::bind(("127.0.0.1", 0)).map_err(|_| ProviderError::Unreachable)?;
        listener
            .set_nonblocking(true)
            .map_err(|_| ProviderError::Unreachable)?;
        let port = listener
            .local_addr()
            .map_err(|_| ProviderError::Unreachable)?
            .port();
        let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");
        let verifier = random_token(32)?;
        let state = random_token(16)?;
        let nonce = random_token(16)?;
        let host_id = match previous {
            Some(account) if !account.host_id.is_empty() => account.host_id.clone(),
            _ => new_host_id()?,
        };
        let issued = previous.and_then(|account| account.client_id.clone());
        let mut url = url::Url::parse(AUTHORIZE_URL).map_err(|_| ProviderError::InvalidResponse)?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("client_id", issued.as_deref().unwrap_or(DYNAMIC_CLIENT))
                .append_pair("response_type", "code")
                .append_pair("redirect_uri", &redirect_uri)
                .append_pair("scope", SCOPES)
                .append_pair("resource", CHATGPT_API_BASE)
                .append_pair("state", &state)
                .append_pair("nonce", &nonce)
                .append_pair("code_challenge_method", "S256")
                .append_pair("code_challenge", &code_challenge(&verifier))
                .append_pair("ext_agent_host_id", &host_id);
            if issued.is_none() {
                query.append_pair("agent_name_hint", AGENT_NAME);
            }
            if let Some(email) = previous.and_then(|account| account.email.as_deref()) {
                query.append_pair("login_hint", email);
            }
        }
        Ok(Box::new(LoopbackSignIn {
            listener,
            authorize_url: url.to_string(),
            redirect_uri,
            verifier,
            state,
            nonce,
            host_id,
            issued,
            cancelled: Arc::new(AtomicBool::new(false)),
        }))
    }

    fn sign_out(
        &self,
        account: &ChatGptAccount,
        refresh_token: Option<String>,
    ) -> Result<(), ProviderError> {
        self.invalidate();
        let (Some(token), Some(client_id)) = (refresh_token, account.client_id.as_deref()) else {
            return Ok(());
        };
        let client = http_client(HTTP_TIMEOUT)?;
        let response = client
            .post(REVOKE_URL)
            .form(&[
                ("token", token.as_str()),
                ("token_type_hint", "refresh_token"),
                ("client_id", client_id),
            ])
            .send()
            .map_err(|_| ProviderError::Unreachable)?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Unreachable)
        }
    }
}

/// A sign-in waiting on the loopback listener.
struct LoopbackSignIn {
    listener: TcpListener,
    authorize_url: String,
    redirect_uri: String,
    verifier: String,
    state: String,
    nonce: String,
    host_id: String,
    issued: Option<String>,
    cancelled: Arc<AtomicBool>,
}

/// Parsed loopback callback.
#[derive(Debug, PartialEq, Eq)]
struct Callback {
    params: HashMap<String, String>,
}

/// Reads the request line of one HTTP request and returns the query of the
/// callback path, or `None` for any other path.
fn read_callback(stream: &mut TcpStream) -> Option<Callback> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    reader.by_ref().take(8 * 1024).read_line(&mut line).ok()?;
    parse_callback_line(&line)
}

fn parse_callback_line(line: &str) -> Option<Callback> {
    let target = line.split_whitespace().nth(1)?;
    let url = url::Url::parse(&format!("http://127.0.0.1{target}")).ok()?;
    if url.path() != CALLBACK_PATH {
        return None;
    }
    Some(Callback {
        params: url.query_pairs().into_owned().collect(),
    })
}

fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let page = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>xemnas</title>\
         <body style=\"font-family:system-ui;background:#0d111a;color:#eceef4;\
         display:flex;align-items:center;justify-content:center;height:100vh;margin:0\">\
         <p>{body}</p></body>"
    );
    let _ = write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    let _ = stream.flush();
}

impl PendingSignIn for LoopbackSignIn {
    fn authorize_url(&self) -> &str {
        &self.authorize_url
    }

    fn canceller(&self) -> Box<dyn Fn() + Send + Sync> {
        let cancelled = self.cancelled.clone();
        Box::new(move || cancelled.store(true, Ordering::SeqCst))
    }

    fn wait(self: Box<Self>, timeout: Duration) -> Result<SignedIn, ProviderError> {
        let deadline = Instant::now() + timeout;
        let callback = loop {
            if self.cancelled.load(Ordering::SeqCst) || Instant::now() >= deadline {
                return Err(ProviderError::Cancelled);
            }
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_nonblocking(false);
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                    match read_callback(&mut stream) {
                        Some(callback) => break (stream, callback),
                        None => respond(&mut stream, "404 Not Found", "Página não encontrada."),
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(_) => return Err(ProviderError::Unreachable),
            }
        };
        let (mut stream, callback) = callback;
        let params = callback.params;
        if params.get("state").map(String::as_str) != Some(self.state.as_str()) {
            respond(
                &mut stream,
                "400 Bad Request",
                "Login inválido. Tente de novo no xemnas.",
            );
            return Err(ProviderError::InvalidResponse);
        }
        let Some(code) = params.get("code").cloned() else {
            respond(
                &mut stream,
                "200 OK",
                "Login cancelado. Você pode fechar esta aba.",
            );
            return Err(ProviderError::Cancelled);
        };
        let Some(client_id) = params.get("client_id").cloned().or(self.issued.clone()) else {
            respond(
                &mut stream,
                "400 Bad Request",
                "Login incompleto. Tente de novo no xemnas.",
            );
            return Err(ProviderError::InvalidResponse);
        };
        let result = self.exchange(&code, &client_id, params.get("scope"));
        respond(
            &mut stream,
            "200 OK",
            if result.is_ok() {
                "Login concluído. Você pode voltar ao xemnas."
            } else {
                "Não foi possível concluir o login. Volte ao xemnas e tente de novo."
            },
        );
        result
    }
}

impl LoopbackSignIn {
    fn exchange(
        &self,
        code: &str,
        client_id: &str,
        callback_scope: Option<&String>,
    ) -> Result<SignedIn, ProviderError> {
        let client = http_client(HTTP_TIMEOUT)?;
        let response = client
            .post(TOKEN_URL)
            .form(&[
                ("grant_type", "authorization_code"),
                ("client_id", client_id),
                ("code", code),
                ("code_verifier", self.verifier.as_str()),
                ("redirect_uri", self.redirect_uri.as_str()),
                ("resource", CHATGPT_API_BASE),
            ])
            .send()
            .map_err(|_| ProviderError::Unreachable)?;
        if !response.status().is_success() {
            return Err(ProviderError::Unauthorized);
        }
        let token: TokenResponse = response
            .json()
            .map_err(|_| ProviderError::InvalidResponse)?;
        let refresh_token = token
            .refresh_token
            .clone()
            .ok_or(ProviderError::InvalidResponse)?;
        let id_token = token
            .id_token
            .as_deref()
            .ok_or(ProviderError::InvalidResponse)?;
        let claims = validate_id_token(id_token, client_id, &self.nonce, now_seconds())?;
        let info: UserInfo = client
            .get(USERINFO_URL)
            .bearer_auth(&token.access_token)
            .send()
            .map_err(|_| ProviderError::Unreachable)?
            .json()
            .map_err(|_| ProviderError::InvalidResponse)?;
        if info.sub != claims.sub {
            return Err(ProviderError::InvalidResponse);
        }
        let granted = token
            .scope
            .as_deref()
            .or(callback_scope.map(String::as_str))
            .unwrap_or_default();
        Ok(SignedIn {
            account: ChatGptAccount {
                client_id: Some(client_id.to_string()),
                host_id: self.host_id.clone(),
                email: info.email.or(claims.email),
                subject: Some(claims.sub),
                plan_usage: granted.split([' ', '+']).any(|scope| scope == PLAN_SCOPE),
            },
            refresh_token,
        })
    }
}

/// Outcome of the Responses stream.
#[derive(Debug, PartialEq)]
enum StreamResult {
    Completed(String),
    Failed(String),
    Incomplete,
}

/// Reads a Responses API event stream and returns the final output text.
/// Only `response.completed` counts as success (plan usage guide).
fn read_stream(reader: impl Read) -> Result<StreamResult, ExtractError> {
    let mut deltas = String::new();
    let mut read = 0usize;
    for line in BufReader::new(reader).lines() {
        let line = line.map_err(|_| {
            ExtractError::Extractor("falha ao ler a resposta do provedor".to_string())
        })?;
        read += line.len();
        if read > MAX_RESPONSE_BYTES {
            return Err(ExtractError::Extractor(
                "a resposta do provedor excedeu o limite".to_string(),
            ));
        }
        let Some(data) = line.strip_prefix("data:") else {
            continue;
        };
        let Ok(event) = serde_json::from_str::<serde_json::Value>(data.trim()) else {
            continue;
        };
        match event.get("type").and_then(|kind| kind.as_str()) {
            Some("response.output_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(|delta| delta.as_str()) {
                    deltas.push_str(delta);
                }
            }
            Some("response.completed") => {
                let text = completed_text(&event).unwrap_or(deltas);
                return Ok(StreamResult::Completed(text));
            }
            Some("response.failed") => {
                let code = event
                    .pointer("/response/error/code")
                    .and_then(|code| code.as_str())
                    .unwrap_or("")
                    .to_string();
                return Ok(StreamResult::Failed(code));
            }
            Some("response.incomplete") => return Ok(StreamResult::Incomplete),
            _ => {}
        }
    }
    Ok(StreamResult::Incomplete)
}

fn completed_text(event: &serde_json::Value) -> Option<String> {
    let output = event.pointer("/response/output")?.as_array()?;
    let text: String = output
        .iter()
        .filter_map(|item| item.get("content")?.as_array())
        .flatten()
        .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("output_text"))
        .filter_map(|part| part.get("text")?.as_str())
        .collect();
    (!text.is_empty()).then_some(text)
}

/// Product message for a Responses-layer error code.
fn plan_error(code: &str) -> ExtractError {
    ExtractError::Extractor(
        match code {
            "subscription_sharing_usage_limit_exceeded" => {
                "limite de uso do plano ChatGPT atingido; gerencie em chatgpt.com/settings/usage"
            }
            "subscription_sharing_user_not_eligible" => {
                "esta conta ChatGPT não pode usar o plano neste app"
            }
            "subscription_sharing_invalid_user" => "entre de novo com a conta ChatGPT",
            "subscription_sharing_unsupported_capability" => {
                "o modelo escolhido não aceita este pedido pelo plano"
            }
            _ => "o provedor recusou o pedido",
        }
        .to_string(),
    )
}

/// Extractor on the user's ChatGPT plan, through the Responses API.
pub struct ChatGptExtractor {
    session: Arc<ChatGptSession>,
    profile: AiProfile,
    client: reqwest::blocking::Client,
    retry: RetryPolicy,
}

impl std::fmt::Debug for ChatGptExtractor {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ChatGptExtractor")
            .field("model", &self.profile.model)
            .finish_non_exhaustive()
    }
}

impl ChatGptExtractor {
    /// Builds the extractor for a ChatGPT-plan profile.
    pub fn new(session: Arc<ChatGptSession>, profile: &AiProfile) -> Result<Self, ExtractError> {
        if profile.kind != ProfileKind::ChatGptPlan {
            return Err(ExtractError::Extractor(
                "o perfil não usa a conta ChatGPT".to_string(),
            ));
        }
        profile
            .validate()
            .map_err(|_| ExtractError::Extractor("perfil de IA inválido".to_string()))?;
        let client = http_client(INFERENCE_TIMEOUT).map_err(|_| {
            ExtractError::Extractor("não foi possível preparar o cliente HTTP".to_string())
        })?;
        Ok(Self {
            session,
            profile: profile.clone(),
            client,
            retry: RetryPolicy::default(),
        })
    }

    fn attempt(&self, body: &serde_json::Value) -> Attempt<String> {
        let token = match self.session.access_token(&self.profile) {
            Ok(token) => token,
            Err(ProviderError::Unreachable) => return Attempt::Transient,
            Err(error) => return Attempt::Fatal(ExtractError::Extractor(error.message().into())),
        };
        let response = match self
            .client
            .post(format!("{CHATGPT_API_BASE}/responses"))
            .bearer_auth(token)
            .json(body)
            .send()
        {
            Ok(response) => response,
            Err(error) if error.is_timeout() || error.is_connect() => return Attempt::Transient,
            Err(_) => {
                return Attempt::Fatal(ExtractError::Extractor(
                    "falha na chamada externa".to_string(),
                ))
            }
        };
        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 401 {
                self.session.invalidate();
            }
            let code = response
                .json::<serde_json::Value>()
                .ok()
                .and_then(|value| {
                    value
                        .pointer("/error/code")
                        .and_then(|code| code.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_default();
            if status.as_u16() == 503 || code == "subscription_sharing_usage_unavailable" {
                return Attempt::Transient;
            }
            return Attempt::Fatal(if code.is_empty() {
                ExtractError::Extractor(format!("o provedor respondeu {}", status.as_u16()))
            } else {
                plan_error(&code)
            });
        }
        match read_stream(response) {
            Ok(StreamResult::Completed(text)) => Attempt::Success(text),
            Ok(StreamResult::Failed(code))
                if code == "subscription_sharing_usage_unavailable"
                    || code == "subscription_sharing_user_unavailable" =>
            {
                Attempt::Transient
            }
            Ok(StreamResult::Failed(code)) => Attempt::Fatal(plan_error(&code)),
            Ok(StreamResult::Incomplete) => Attempt::Fatal(ExtractError::Extractor(
                "a resposta do provedor veio incompleta".to_string(),
            )),
            Err(error) => Attempt::Fatal(error),
        }
    }
}

impl CandidateExtractor for ChatGptExtractor {
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

impl ChatGptExtractor {
    /// Asks the plan for one answer under a strict JSON Schema, with consent
    /// and retries.
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
        // `temperature`, `max_output_tokens` and `metadata` are not allowed on
        // plan requests; `store: false` and `stream: true` are mandatory.
        let body = json!({
            "model": self.profile.model,
            "instructions": system,
            "input": [{
                "role": "user",
                "content": [{ "type": "input_text", "text": user }],
            }],
            "store": false,
            "stream": true,
            "text": {
                "format": {
                    "type": "json_schema",
                    "name": schema_name,
                    "strict": true,
                    "schema": schema,
                }
            },
        });
        let mut attempt = 1u32;
        loop {
            match self.attempt(&body) {
                Attempt::Success(text) => return Ok(text),
                Attempt::Fatal(error) => return Err(error),
                Attempt::Transient => {
                    if attempt >= self.retry.max_attempts {
                        return Err(ExtractError::Extractor(format!(
                            "o provedor falhou após {attempt} tentativa(s)"
                        )));
                    }
                    std::thread::sleep(self.retry.delay_for(attempt));
                    attempt += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_is_base64url_sha256_without_padding() {
        assert_eq!(
            code_challenge("dBjftJeZ4CVP-mJ92K9dYXrQX4yQvXhG5XkXqK6pKNs"),
            "lE2kMf-s-9gt9-wgJgn1YUvMtu6pUGC2fMzTys2Hblk"
        );
    }

    #[test]
    fn host_ids_are_uuid_v4_urns() {
        let id = new_host_id().expect("host id");
        assert!(id.starts_with("urn:uuid:"));
        let uuid = &id["urn:uuid:".len()..];
        assert_eq!(uuid.len(), 36);
        assert_eq!(&uuid[14..15], "4");
    }

    fn token(claims: serde_json::Value) -> String {
        format!(
            "e30.{}.sig",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).expect("json"))
        )
    }

    #[test]
    fn id_token_checks_issuer_audience_expiry_and_nonce() {
        let good = json!({"iss": ISSUER, "aud": "oaiapp_1", "exp": 2000, "nonce": "n", "sub": "s"});
        assert!(validate_id_token(&token(good.clone()), "oaiapp_1", "n", 1000).is_ok());
        let list_audience =
            json!({"iss": ISSUER, "aud": ["x", "oaiapp_1"], "exp": 2000, "nonce": "n", "sub": "s"});
        assert!(validate_id_token(&token(list_audience), "oaiapp_1", "n", 1000).is_ok());
        assert!(validate_id_token(&token(good.clone()), "oaiapp_2", "n", 1000).is_err());
        assert!(validate_id_token(&token(good.clone()), "oaiapp_1", "other", 1000).is_err());
        assert!(validate_id_token(&token(good), "oaiapp_1", "n", 3000).is_err());
        let foreign = json!({"iss": "https://evil.test", "aud": "oaiapp_1", "exp": 2000, "nonce": "n", "sub": "s"});
        assert!(validate_id_token(&token(foreign), "oaiapp_1", "n", 1000).is_err());
    }

    #[test]
    fn callback_line_yields_only_the_callback_path() {
        let callback = parse_callback_line(
            "GET /auth/callback?code=abc&state=xyz&client_id=oaiapp_1&scope=openid+chatgpt.tokens.use.direct HTTP/1.1",
        )
        .expect("callback");
        assert_eq!(callback.params.get("code").map(String::as_str), Some("abc"));
        assert_eq!(
            callback.params.get("scope").map(String::as_str),
            Some("openid chatgpt.tokens.use.direct")
        );
        assert!(parse_callback_line("GET /favicon.ico HTTP/1.1").is_none());
    }

    #[test]
    fn stream_success_needs_response_completed() {
        let completed = "event: response.output_text.delta\n\
            data: {\"type\":\"response.output_text.delta\",\"delta\":\"{\\\"a\\\"\"}\n\n\
            data: {\"type\":\"response.completed\",\"response\":{\"output\":[{\"type\":\"message\",\
            \"content\":[{\"type\":\"output_text\",\"text\":\"{\\\"proposals\\\":[]}\"}]}]}}\n";
        assert_eq!(
            read_stream(completed.as_bytes()).expect("read"),
            StreamResult::Completed("{\"proposals\":[]}".into())
        );
        let failed = "data: {\"type\":\"response.failed\",\"response\":{\"error\":{\"code\":\"subscription_sharing_usage_limit_exceeded\"}}}\n";
        assert_eq!(
            read_stream(failed.as_bytes()).expect("read"),
            StreamResult::Failed("subscription_sharing_usage_limit_exceeded".into())
        );
        let cut = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"x\"}\n";
        assert_eq!(
            read_stream(cut.as_bytes()).expect("read"),
            StreamResult::Incomplete
        );
    }
}
