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
    build_user_content, output_schema, parse_model_output, retry_after, Attempt,
    KeyringSecretStore, RetryPolicy, MAX_RESPONSE_BYTES, SYSTEM_PROMPT,
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

fn handle_invalid_credential(preserve: bool, delete: impl FnOnce()) {
    if !preserve {
        delete();
    }
}

/// Sanitized Responses HTTP failure metadata; never contains a response body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseFailureMetadata {
    /// HTTP status returned by the content endpoint.
    pub status: u16,
    /// Allowlisted error code, or `unknown` for anything else.
    pub code: &'static str,
    /// Allowlisted request parameter named by the server, if any.
    pub parameter: &'static str,
    /// Semantic classification of the server message, never its raw text.
    pub reason: &'static str,
    /// Whether the server supplied a string message.
    pub message_present: bool,
}

fn response_failure_metadata(status: u16, code: &str) -> ResponseFailureMetadata {
    ResponseFailureMetadata {
        status,
        parameter: "unknown",
        reason: "unknown",
        message_present: false,
        code: match code {
            "model_not_found" => "model_not_found",
            "unsupported_model" => "unsupported_model",
            "invalid_model" => "invalid_model",
            "access_denied" => "access_denied",
            "insufficient_quota" => "insufficient_quota",
            "rate_limit" => "rate_limit",
            "invalid_request" => "invalid_request",
            "not_signed_in" => "not_signed_in",
            _ => "unknown",
        },
    }
}

fn response_failure_details(status: u16, value: &serde_json::Value) -> ResponseFailureMetadata {
    let error = value.get("error").unwrap_or(value);
    let field = |name: &str| error.get(name).and_then(serde_json::Value::as_str);
    let mut metadata = response_failure_metadata(status, field("code").unwrap_or_default());
    metadata.parameter = match field("param").unwrap_or_default() {
        "model" => "model",
        "store" => "store",
        "stream" => "stream",
        "instructions" => "instructions",
        "input" => "input",
        "text" | "text.format" | "text.format.schema" => "text.format.schema",
        "temperature" => "temperature",
        "max_output_tokens" => "max_output_tokens",
        "tool_choice" => "tool_choice",
        _ => "unknown",
    };
    let message = field("message").or_else(|| field("detail"));
    metadata.message_present = message.is_some();
    let lower = message.unwrap_or_default().to_ascii_lowercase();
    metadata.reason = if lower.contains("schema")
        && (lower.contains("invalid")
            || lower.contains("required")
            || lower.contains("additionalproperties"))
    {
        "json_schema_invalid"
    } else if lower.contains("store") && (lower.contains("false") || lower.contains("disabled")) {
        "store_must_false"
    } else if lower.contains("stream") && (lower.contains("required") || lower.contains("true")) {
        "stream_required"
    } else if lower.contains("instructions")
        && (lower.contains("required") || lower.contains("missing"))
    {
        "instructions_required"
    } else if lower.contains("model")
        && (lower.contains("not supported")
            || lower.contains("unsupported")
            || lower.contains("not found")
            || lower.contains("does not exist"))
    {
        "model_not_supported"
    } else if lower.contains("unsupported parameter")
        || lower.contains("unknown parameter")
        || lower.contains("unrecognized parameter")
        || lower.contains("not supported parameter")
    {
        "unsupported_parameter"
    } else if lower.contains("access denied")
        || lower.contains("not authorized")
        || lower.contains("not eligible")
    {
        "account_access_denied"
    } else {
        "unknown"
    };
    metadata
}

#[cfg(test)]
mod credential_failure_policy_tests {
    #[test]
    fn failure_metadata_never_exposes_unrecognized_code() {
        let metadata = super::response_failure_metadata(400, "SYNTHETIC_SECRET_UNEXPECTED");
        assert_eq!(metadata.status, 400);
        assert_eq!(metadata.code, "unknown");
        assert!(!format!("{metadata:?}").contains("SYNTHETIC_SECRET"));
        assert_eq!(
            super::response_failure_metadata(404, "model_not_found").code,
            "model_not_found"
        );
    }
    #[test]
    fn loopback_failure_metadata_does_not_leak_body() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = [0; 4096];
            let received = stream.read(&mut request).expect("request");
            assert!(received > 0);
            let body = r#"{"error":{"code":"SYNTHETIC_SECRET_UNEXPECTED","message":"SYNTHETIC_SECRET_BODY"}}"#;
            write!(stream, "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).expect("response");
        });
        let response = reqwest::blocking::get(format!("http://{address}")).expect("response");
        let status = response.status().as_u16();
        let value: serde_json::Value = response.json().expect("json");
        let code = value
            .pointer("/error/code")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let metadata = super::response_failure_details(status, &value);
        assert_eq!(code, "SYNTHETIC_SECRET_UNEXPECTED");
        assert_eq!(metadata.code, "unknown");
        assert_eq!(metadata.status, 400);
        assert!(!format!("{metadata:?}").contains("SYNTHETIC_SECRET"));
        server.join().expect("server");
    }
    #[test]
    fn semantic_failure_details_are_bounded_literals() {
        let value = serde_json::json!({"error": {
            "code": "SYNTHETIC_SECRET", "param": "text.format.schema",
            "message": "Invalid schema: required must include nature. SYNTHETIC_SECRET"
        }});
        let metadata = super::response_failure_details(400, &value);
        assert_eq!(metadata.parameter, "text.format.schema");
        assert_eq!(metadata.reason, "json_schema_invalid");
        assert!(metadata.message_present);
        assert!(!format!("{metadata:?}").contains("SYNTHETIC_SECRET"));
    }
    #[test]
    fn assessment_preserves_and_default_deletes_invalid_credential() {
        let mut deletions = 0;
        super::handle_invalid_credential(true, || deletions += 1);
        assert_eq!(deletions, 0);
        super::handle_invalid_credential(false, || deletions += 1);
        assert_eq!(deletions, 1);
        assert!(
            super::ChatGptSession::with_preserve_invalid_credentials(
                crate::KeyringSecretStore::new()
            )
            .preserve_invalid_credentials
        );
        assert!(!super::ChatGptSession::default().preserve_invalid_credentials);
    }
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
    // Only successful provider refreshes enter this session-owned lineage.
    rotations: Mutex<Vec<(AiProfile, String, String)>>,
    secrets: KeyringSecretStore,
    cache: Mutex<Option<CachedToken>>,
    preserve_invalid_credentials: bool,
    response_failure_observer: Option<Arc<dyn Fn(ResponseFailureMetadata) + Send + Sync>>,
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
            rotations: Mutex::new(Vec::new()),
            preserve_invalid_credentials: false,
            response_failure_observer: None,
        }
    }

    /// Builds an assessment session that never deletes an invalid refresh token.
    /// Authentication still fails normally; successful refresh rotation is unchanged.
    pub fn with_preserve_invalid_credentials(secrets: KeyringSecretStore) -> Self {
        Self {
            preserve_invalid_credentials: true,
            ..Self::new(secrets)
        }
    }

    /// Observes only sanitized content-endpoint HTTP failures. No logging by default.
    pub fn with_response_failure_observer(
        mut self,
        observer: Arc<dyn Fn(ResponseFailureMetadata) + Send + Sync>,
    ) -> Self {
        self.response_failure_observer = Some(observer);
        self
    }

    /// A valid access token for `profile`, refreshing (and rotating the stored
    /// refresh token) when the cached one is about to expire.
    pub fn access_token(&self, profile: &AiProfile) -> Result<String, ProviderError> {
        self.access_token_checked(profile, None)
    }

    fn access_token_checked(
        &self,
        profile: &AiProfile,
        authorization: Option<&dyn application::external::Authorization>,
    ) -> Result<String, ProviderError> {
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
                handle_invalid_credential(self.preserve_invalid_credentials, || {
                    let _ = self.secrets.delete_secret(&key);
                });
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
            if let Some(authorization) = authorization {
                authorization
                    .check()
                    .map_err(|_| ProviderError::Unauthorized)?;
            }
            if self
                .secrets
                .get_secret(&key)
                .map_err(|_| ProviderError::Keystore)?
                .as_deref()
                != Some(refresh.as_str())
            {
                return Err(ProviderError::Unauthorized);
            }
            // Each refresh returns a replacement token; losing it would sign
            // the user out, so a store failure is an error, not a warning.
            self.secrets
                .set_secret(&key, rotated)
                .map_err(|_| ProviderError::Keystore)?;
            self.rotations
                .lock()
                .map_err(|_| ProviderError::Keystore)?
                .push((profile.clone(), refresh.clone(), rotated.to_string()));
            if let Some(authorization) = authorization {
                authorization
                    .rotated(&refresh, rotated)
                    .map_err(|_| ProviderError::Unauthorized)?;
            }
        }
        let lifetime = Duration::from_secs(token.expires_in.unwrap_or(3600));
        *cache = Some(CachedToken {
            client_id,
            access_token: token.access_token.clone(),
            expires_at: Instant::now() + lifetime,
        });
        Ok(token.access_token)
    }

    fn authorize_operation(
        &self,
        profile: &AiProfile,
        authorization: &dyn application::external::Authorization,
    ) -> Result<(), ExtractError> {
        let rotations = self
            .rotations
            .lock()
            .map_err(|_| ExtractError::Extractor("sessão indisponível".into()))?;
        let chain = rotations
            .iter()
            .filter(|(source, _, _)| source == profile)
            .map(|(_, previous, replacement)| (previous.clone(), replacement.clone()))
            .collect::<Vec<_>>();
        authorization.refresh_lineage(profile, &chain)
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

/// Longest single line of the stream: `response.completed` repeats the whole
/// text escaped as JSON, with the instructions and metadata around it.
const MAX_STREAM_LINE_BYTES: usize = 4 * MAX_RESPONSE_BYTES;

/// Most bytes read from one stream. Each delta event carries about 200 bytes of
/// protocol for a few characters of text, so a text at its cap takes about
/// 25 MiB; the limit on the text is [`MAX_RESPONSE_BYTES`], this one only
/// bounds time and bandwidth (lines are not kept).
const MAX_STREAM_BYTES: usize = 64 * MAX_RESPONSE_BYTES;

fn over_the_limit() -> ExtractError {
    ExtractError::Extractor("a resposta do provedor excedeu o limite".to_string())
}

/// Reads a Responses API event stream and returns the final output text.
/// Only `response.completed` counts as success (plan usage guide). The cap of
/// [`MAX_RESPONSE_BYTES`] is on the text kept, not on the SSE around it.
fn read_stream(reader: impl Read) -> Result<StreamResult, ExtractError> {
    let unreadable = || ExtractError::Extractor("falha ao ler a resposta do provedor".to_string());
    let mut deltas = String::new();
    let mut total = 0usize;
    let mut reader = BufReader::new(reader);
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        let read = (&mut reader)
            .take(MAX_STREAM_LINE_BYTES as u64 + 1)
            .read_until(b'\n', &mut buffer)
            .map_err(|_| unreadable())?;
        if read == 0 {
            break;
        }
        total += read;
        if total > MAX_STREAM_BYTES || buffer.len() > MAX_STREAM_LINE_BYTES {
            return Err(over_the_limit());
        }
        let line = std::str::from_utf8(&buffer)
            .map_err(|_| unreadable())?
            .trim_end_matches(['\r', '\n']);
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
                    if deltas.len() > MAX_RESPONSE_BYTES {
                        return Err(over_the_limit());
                    }
                }
            }
            Some("response.completed") => {
                let text = completed_text(&event).unwrap_or(deltas);
                if text.len() > MAX_RESPONSE_BYTES {
                    return Err(over_the_limit());
                }
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

    fn attempt(
        &self,
        body: &serde_json::Value,
        authorization: Option<&dyn application::external::Authorization>,
    ) -> Attempt<String> {
        let token = match self
            .session
            .access_token_checked(&self.profile, authorization)
        {
            Ok(token) => token,
            Err(ProviderError::Unreachable) => return Attempt::Transient,
            Err(error) => return Attempt::Fatal(ExtractError::Extractor(error.message().into())),
        };
        if let Some(authorization) = authorization {
            if let Err(error) = self
                .session
                .authorize_operation(&self.profile, authorization)
            {
                return Attempt::Fatal(error);
            }
        }
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
            let wait = retry_after(response.headers());
            let error_value = response.json::<serde_json::Value>().unwrap_or_default();
            let code = error_value
                .pointer("/error/code")
                .and_then(|code| code.as_str())
                .map(str::to_owned)
                .unwrap_or_default();
            if let Some(observer) = &self.session.response_failure_observer {
                observer(response_failure_details(status.as_u16(), &error_value));
            }
            if status.as_u16() == 503 || code == "subscription_sharing_usage_unavailable" {
                return Attempt::Transient;
            }
            // A spent plan quota is not a pause: it keeps its own message.
            if status.as_u16() == 429 && code != "subscription_sharing_usage_limit_exceeded" {
                return Attempt::RateLimited(wait);
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
        self.complete_checked(system, user, schema_name, schema, None)
    }

    pub(crate) fn profile(&self) -> &AiProfile {
        &self.profile
    }

    pub(crate) fn complete_checked(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &serde_json::Value,
        authorization: Option<&dyn application::external::Authorization>,
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
            if let Some(authorization) = authorization {
                self.session
                    .authorize_operation(&self.profile, authorization)?;
            }
            match self.attempt(&body, authorization) {
                Attempt::Success(text) => return Ok(text),
                Attempt::Fatal(error) => return Err(error),
                Attempt::RateLimited(retry_after) => {
                    return Err(ExtractError::RateLimited { retry_after })
                }
                Attempt::Transient => {
                    if attempt >= self.retry.max_attempts {
                        return Err(ExtractError::Unavailable { attempts: attempt });
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

    fn delta_line(sequence: usize, text: &str) -> String {
        format!(
            "data: {{\"type\":\"response.output_text.delta\",\"sequence_number\":{sequence},\
            \"item_id\":\"msg_{}\",\"output_index\":0,\"content_index\":0,\
            \"delta\":\"{text}\",\"logprobs\":[]}}\n\n",
            "x".repeat(60)
        )
    }

    const COMPLETED_WITHOUT_TEXT: &str =
        "data: {\"type\":\"response.completed\",\"response\":{\"output\":[]}}\n";

    #[test]
    fn many_tiny_deltas_over_the_raw_cap_still_complete() {
        let mut stream: String = (0..5_000).map(|n| delta_line(n, "a")).collect();
        stream.push_str(COMPLETED_WITHOUT_TEXT);
        assert!(stream.len() > MAX_RESPONSE_BYTES);
        assert_eq!(
            read_stream(stream.as_bytes()).expect("read"),
            StreamResult::Completed("a".repeat(5_000))
        );
    }

    #[test]
    fn text_over_the_cap_fails() {
        let chunk = "a".repeat(1_024);
        let mut stream: String = (0..=MAX_RESPONSE_BYTES / 1_024)
            .map(|n| delta_line(n, &chunk))
            .collect();
        stream.push_str(COMPLETED_WITHOUT_TEXT);
        assert!(read_stream(stream.as_bytes()).is_err());
    }

    #[test]
    fn a_single_line_over_the_line_cap_fails() {
        let line = format!("data: {}\n", "a".repeat(MAX_STREAM_LINE_BYTES + 10));
        assert!(read_stream(line.as_bytes()).is_err());
    }
}
