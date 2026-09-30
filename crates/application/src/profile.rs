//! AI execution profiles: configuration, consent and secret access.

use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

use integration_contracts::capture::artifact_fingerprint;
use serde::{Deserialize, Serialize};

/// Default per-artifact character cap sent to an external provider.
pub const DEFAULT_MAX_INPUT_CHARS: usize = 8_192;

/// The artifact categories the provider may receive.
pub const PREVIEW_CATEGORIES: &[&str] =
    &["user_text", "assistant_text", "diff_hunk", "tool_summary"];

/// Base URL of the OpenAI API used with a ChatGPT plan (ADR-0004).
pub const CHATGPT_API_BASE: &str = "https://api.openai.com/v1";

/// Default address of a local OpenCode server (`opencode serve`).
pub const OPENCODE_DEFAULT_ENDPOINT: &str = "http://127.0.0.1:4096";

/// Which extractor a profile selects (ADR-0004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileKind {
    /// Deterministic, offline extractor; never touches the network.
    Fake,
    /// OpenAI-compatible HTTP provider: an API key, or a local model on a
    /// loopback address (Ollama, LM Studio) that needs no key.
    OpenAiCompatible,
    /// The user's ChatGPT plan through Sign in with ChatGPT.
    ChatGptPlan,
    /// A model configured in the user's local OpenCode server.
    OpenCode,
}

/// Non-secret facts about the connected ChatGPT account. The refresh token
/// lives only in the OS secret store; the access token only in memory.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatGptAccount {
    /// Client id issued at the first sign-in (`oaiapp_…`); reused afterwards.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Stable per-installation host identifier (`urn:uuid:…`).
    pub host_id: String,
    /// E-mail of the signed-in account, for the UI and the consent preview.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// OpenID subject of the signed-in account.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Whether `chatgpt.tokens.use.direct` was granted (plan usage allowed).
    #[serde(default)]
    pub plan_usage: bool,
}

/// Recorded consent for external calls, tied to a preview hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentRecord {
    /// RFC 3339 timestamp the consent was granted.
    pub granted_at: String,
    /// Hash of the preview the user approved.
    pub preview_hash: String,
}

/// One AI execution profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiProfile {
    /// Stable identifier, also the secret account name.
    pub id: String,
    /// Which extractor this profile selects.
    pub kind: ProfileKind,
    /// Model name sent to the provider.
    pub model: String,
    /// Provider base URL; required for `open_ai_compatible`.
    pub endpoint: Option<String>,
    /// Maximum characters sent per artifact.
    #[serde(default = "default_max_input_chars")]
    pub max_input_chars: usize,
    /// Whether the user enabled external calls.
    #[serde(default)]
    pub external_calls_enabled: bool,
    /// Consent recorded when external calls were enabled.
    pub consent: Option<ConsentRecord>,
    /// ChatGPT account facts; only meaningful for [`ProfileKind::ChatGptPlan`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chatgpt: Option<ChatGptAccount>,
}

fn default_max_input_chars() -> usize {
    DEFAULT_MAX_INPUT_CHARS
}

impl AiProfile {
    /// Validates the profile for its kind.
    pub fn validate(&self) -> Result<(), ProfileError> {
        if self.max_input_chars == 0 {
            return Err(ProfileError::Invalid(
                "o limite de entrada deve ser positivo".to_string(),
            ));
        }
        match self.kind {
            ProfileKind::Fake => Ok(()),
            ProfileKind::OpenAiCompatible => {
                require_model(&self.model)?;
                validate_endpoint(self.endpoint.as_deref().unwrap_or_default())?;
                Ok(())
            }
            ProfileKind::ChatGptPlan => {
                require_model(&self.model)?;
                if self.endpoint.is_some() {
                    return Err(ProfileError::Invalid(
                        "a conta ChatGPT usa o endereço da OpenAI".to_string(),
                    ));
                }
                Ok(())
            }
            ProfileKind::OpenCode => {
                if !self.model.contains('/') || self.model.trim().starts_with('/') {
                    return Err(ProfileError::Invalid(
                        "escolha um modelo do OpenCode no formato provedor/modelo".to_string(),
                    ));
                }
                let endpoint = self.endpoint.as_deref().unwrap_or_default();
                validate_endpoint(endpoint)?;
                if !is_loopback_endpoint(endpoint) {
                    return Err(ProfileError::Invalid(
                        "o OpenCode precisa estar nesta máquina (endereço de loopback)".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// The account under which this profile's credential lives in the
    /// secret store: the API key, the ChatGPT refresh token or the OpenCode
    /// server password. Separate names keep switching kinds from mixing them.
    pub fn credential_account(&self) -> String {
        match self.kind {
            ProfileKind::Fake | ProfileKind::OpenAiCompatible => self.id.clone(),
            ProfileKind::ChatGptPlan => format!("{}.chatgpt", self.id),
            ProfileKind::OpenCode => format!("{}.opencode", self.id),
        }
    }

    /// Whether extraction cannot run without a stored credential. A local
    /// model on loopback and OpenCode (password optional) need none.
    pub fn credential_required(&self) -> bool {
        match self.kind {
            ProfileKind::Fake | ProfileKind::OpenCode => false,
            ProfileKind::OpenAiCompatible => {
                !self.endpoint.as_deref().is_some_and(is_loopback_endpoint)
            }
            ProfileKind::ChatGptPlan => true,
        }
    }
}

fn require_model(model: &str) -> Result<(), ProfileError> {
    if model.trim().is_empty() {
        return Err(ProfileError::Invalid("o modelo é obrigatório".to_string()));
    }
    Ok(())
}

/// Whether `endpoint` is an http(s) URL on a loopback IP (local model or
/// OpenCode on this machine).
pub fn is_loopback_endpoint(endpoint: &str) -> bool {
    let Ok(parsed) = url::Url::parse(endpoint) else {
        return false;
    };
    match parsed.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}

/// The default offline profile; external calls stay disabled.
pub fn offline_default_profile() -> AiProfile {
    AiProfile {
        id: "default-offline".to_string(),
        kind: ProfileKind::Fake,
        model: "offline".to_string(),
        endpoint: None,
        max_input_chars: DEFAULT_MAX_INPUT_CHARS,
        external_calls_enabled: false,
        consent: None,
        chatgpt: None,
    }
}

/// Failure modes of profile storage and consent handling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    /// Reading or writing the profile file failed.
    Io(String),
    /// The profile or the consent request is invalid.
    Invalid(String),
}

impl std::fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(message) => write!(formatter, "falha ao acessar o perfil: {message}"),
            Self::Invalid(message) => write!(formatter, "perfil de IA inválido: {message}"),
        }
    }
}

impl std::error::Error for ProfileError {}

/// Port that persists a single profile file.
pub trait ProfileStore {
    /// Loads the stored profile, or `None` when absent.
    fn load(&self) -> Result<Option<AiProfile>, ProfileError>;

    /// Persists the profile atomically.
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError>;
}

/// File-backed profile store at an injected path.
#[derive(Debug, Clone)]
pub struct FileProfileStore {
    path: PathBuf,
}

impl FileProfileStore {
    /// Wraps a profile file path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the profile file path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ProfileStore for FileProfileStore {
    fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
        match fs::read_to_string(&self.path) {
            Ok(text) => {
                let profile: AiProfile = serde_json::from_str(&text).map_err(|_| {
                    ProfileError::Invalid("o arquivo de perfil está corrompido".to_string())
                })?;
                Ok(Some(profile))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(ProfileError::Io(error.to_string())),
        }
    }

    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        profile.validate()?;
        let text = serde_json::to_string_pretty(profile)
            .map_err(|error| ProfileError::Invalid(error.to_string()))?;
        write_atomic(&self.path, &text).map_err(|error| ProfileError::Io(error.to_string()))
    }
}

/// Writes `text` to `path` via a temporary sibling and a rename.
fn write_atomic(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::now_v7()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// Port that stores one secret per account in an OS-protected backend.
pub trait SecretStore {
    /// Stores (or replaces) the secret for `account`.
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError>;

    /// Returns the secret for `account`, or `None` when absent.
    fn get_secret(&self, account: &str) -> Result<Option<String>, ProfileError>;

    /// Removes the secret for `account`; absent secrets are ignored.
    fn delete_secret(&self, account: &str) -> Result<(), ProfileError>;
}

/// The extractor a profile selects, given consent and enablement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractorChoice {
    /// Use the deterministic offline fake.
    OfflineFake,
    /// Use the configured external provider (consent + enablement present).
    ExternalEnabled,
    /// The profile wants external calls but consent/enablement is missing.
    ExternalBlocked,
}

/// Single source of truth for whether a profile may call an external provider.
pub fn consent_status(profile: &AiProfile) -> Result<(), &'static str> {
    if profile.validate().is_err() {
        return Err("configuração do provedor inválida");
    }
    if profile.kind == ProfileKind::Fake {
        return Err("apenas perfis externos exigem consentimento");
    }
    if !profile.external_calls_enabled {
        return Err("chamadas externas desativadas");
    }
    let Some(consent) = profile.consent.as_ref() else {
        return Err("consentimento ausente");
    };
    if consent.preview_hash != preview_hash(&build_preview(profile)) {
        return Err("a configuração mudou após o consentimento");
    }
    Ok(())
}

/// Chooses the extractor for a profile without looking at any provider name.
pub fn choose_extractor(profile: Option<&AiProfile>) -> ExtractorChoice {
    let Some(profile) = profile else {
        return ExtractorChoice::OfflineFake;
    };
    if consent_status(profile).is_ok() {
        return ExtractorChoice::ExternalEnabled;
    }
    let plain_offline = profile.kind == ProfileKind::Fake
        && !profile.external_calls_enabled
        && profile.consent.is_none();
    if plain_offline {
        ExtractorChoice::OfflineFake
    } else {
        ExtractorChoice::ExternalBlocked
    }
}

/// One artifact category in the consent preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreviewCategory {
    /// Artifact kind (for example `user_text`).
    pub kind: String,
    /// Maximum characters per item of this kind.
    pub max_chars_per_item: usize,
}

/// The deterministically built preview of what an external call would send.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsentPreview {
    /// Profile kind literal.
    pub kind: String,
    /// Host of the provider endpoint, without scheme or path.
    pub endpoint_host: Option<String>,
    /// Fingerprint of the entire configured endpoint, binding scheme, port and
    /// path without exposing URL credentials in the preview.
    pub endpoint_fingerprint: Option<String>,
    /// Model name.
    pub model: String,
    /// Artifact categories that may leave the machine.
    pub categories: Vec<PreviewCategory>,
    /// Whether content is already redacted at ingest.
    pub redaction_on_ingest: bool,
    /// Approximate total characters for a full-size request.
    pub total_approximate_chars: usize,
    /// The account whose plan is used (ChatGPT e-mail). Absent for the other
    /// kinds, so their previews and consent hashes are unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

/// Builds the consent preview for a profile.
pub fn build_preview(profile: &AiProfile) -> ConsentPreview {
    let categories: Vec<PreviewCategory> = PREVIEW_CATEGORIES
        .iter()
        .map(|kind| PreviewCategory {
            kind: (*kind).to_string(),
            max_chars_per_item: profile.max_input_chars,
        })
        .collect();
    let total_approximate_chars = profile.max_input_chars.saturating_mul(categories.len());
    // The ChatGPT plan always goes to the OpenAI API, whatever the profile
    // file says; the preview names that destination and the account.
    let destination = match profile.kind {
        ProfileKind::ChatGptPlan => Some(CHATGPT_API_BASE),
        _ => profile.endpoint.as_deref(),
    };
    ConsentPreview {
        kind: match profile.kind {
            ProfileKind::Fake => "fake".to_string(),
            ProfileKind::OpenAiCompatible => "open_ai_compatible".to_string(),
            ProfileKind::ChatGptPlan => "chat_gpt_plan".to_string(),
            ProfileKind::OpenCode => "open_code".to_string(),
        },
        endpoint_host: destination.and_then(endpoint_host),
        endpoint_fingerprint: destination.and_then(|endpoint| {
            validate_endpoint(endpoint)
                .ok()
                .map(|()| artifact_fingerprint(endpoint))
        }),
        model: profile.model.clone(),
        categories,
        redaction_on_ingest: true,
        total_approximate_chars,
        account: match profile.kind {
            ProfileKind::ChatGptPlan => profile
                .chatgpt
                .as_ref()
                .and_then(|account| account.email.clone().or_else(|| account.subject.clone())),
            _ => None,
        },
    }
}

/// Extracts only the hostname for previews and sanitized diagnostics.
pub(crate) fn endpoint_host(endpoint: &str) -> Option<String> {
    let parsed = url::Url::parse(endpoint).ok()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return None;
    }
    parsed.host_str().map(str::to_string)
}

/// Validates the destination without echoing any supplied URL into errors.
fn validate_endpoint(endpoint: &str) -> Result<(), ProfileError> {
    let invalid = || ProfileError::Invalid("endpoint do provedor inválido".to_string());
    let parsed = url::Url::parse(endpoint).map_err(|_| invalid())?;
    if parsed.host().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(invalid());
    }
    let loopback = match parsed.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && loopback) {
        return Err(ProfileError::Invalid(
            "use HTTPS ou HTTP com um endereço IP de loopback".to_string(),
        ));
    }
    Ok(())
}

/// Returns the stable hash of a preview.
pub fn preview_hash(preview: &ConsentPreview) -> String {
    artifact_fingerprint(&serde_json::to_string(preview).unwrap_or_default())
}

/// Grants consent to a profile, returning the updated profile.
///
/// `credential_ready` says whether the credential the kind needs is present:
/// the API key for a remote endpoint, the ChatGPT sign-in with plan usage.
/// Kinds that need no credential pass `true`.
pub fn grant_consent(
    profile: &AiProfile,
    preview: &ConsentPreview,
    now: &str,
    credential_ready: bool,
) -> Result<AiProfile, ProfileError> {
    if profile.kind == ProfileKind::Fake {
        return Err(ProfileError::Invalid(
            "apenas perfis externos exigem consentimento".to_string(),
        ));
    }
    profile.validate()?;
    if preview_hash(&build_preview(profile)) != preview_hash(preview) {
        return Err(ProfileError::Invalid(
            "a prévia de consentimento mudou; gere novamente".to_string(),
        ));
    }
    if profile.credential_required() && !credential_ready {
        return Err(ProfileError::Invalid(
            match profile.kind {
                ProfileKind::ChatGptPlan => {
                    "entre com a conta ChatGPT e permita o uso do plano antes de consentir"
                }
                _ => "configure a chave do provedor antes de consentir",
            }
            .to_string(),
        ));
    }
    let mut granted = profile.clone();
    granted.external_calls_enabled = true;
    granted.consent = Some(ConsentRecord {
        granted_at: now.to_string(),
        preview_hash: preview_hash(preview),
    });
    Ok(granted)
}

/// Revokes consent and disables external calls.
pub fn revoke_consent(profile: &AiProfile) -> AiProfile {
    let mut revoked = profile.clone();
    revoked.external_calls_enabled = false;
    revoked.consent = None;
    revoked
}

/// Current AI settings status for the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiStatus {
    /// Extractor selected by the stored profile.
    pub choice: ExtractorChoice,
    /// Whether a credential is stored for the profile's kind.
    pub has_secret: bool,
}

/// A completed ChatGPT sign-in, before it is stored.
#[derive(Clone, PartialEq, Eq)]
pub struct SignedIn {
    /// Account facts to keep in the profile.
    pub account: ChatGptAccount,
    /// Refresh token for the secret store; never logged or displayed.
    pub refresh_token: String,
}

impl std::fmt::Debug for SignedIn {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SignedIn")
            .field("account", &self.account)
            .finish_non_exhaustive()
    }
}

/// Facade used by the desktop settings screen and the composition root.
#[derive(Debug, Clone)]
pub struct AiSettings<S, T> {
    profiles: S,
    secrets: T,
}

impl<S, T> AiSettings<S, T>
where
    S: ProfileStore,
    T: SecretStore,
{
    /// Wraps a profile store and a secret store.
    pub fn new(profiles: S, secrets: T) -> Self {
        Self { profiles, secrets }
    }

    /// Loads the stored profile, creating the offline default when absent.
    pub fn load_or_seed(&self) -> Result<AiProfile, ProfileError> {
        match self.profiles.load()? {
            Some(profile) => Ok(profile),
            None => {
                let profile = offline_default_profile();
                self.profiles.save(&profile)?;
                Ok(profile)
            }
        }
    }

    /// Persists a profile.
    pub fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        profile.validate()?;
        self.profiles.save(profile)
    }

    /// Builds the consent preview for a profile.
    pub fn preview(&self, profile: &AiProfile) -> ConsentPreview {
        build_preview(profile)
    }

    /// Stores the provider secret for a profile id.
    pub fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
        self.secrets.set_secret(account, secret)
    }

    /// Returns the stored secret for a profile id, if any.
    pub fn secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
        self.secrets.get_secret(account)
    }

    /// Grants consent when the kind's credential is ready, and persists the
    /// profile.
    pub fn grant(
        &self,
        profile: &AiProfile,
        preview: &ConsentPreview,
        now: &str,
    ) -> Result<AiProfile, ProfileError> {
        let granted = grant_consent(profile, preview, now, self.credential_ready(profile)?)?;
        self.profiles.save(&granted)?;
        Ok(granted)
    }

    /// Whether the credential `profile` needs is present (always true for
    /// kinds that need none).
    pub fn credential_ready(&self, profile: &AiProfile) -> Result<bool, ProfileError> {
        if !profile.credential_required() {
            return Ok(true);
        }
        let stored = self
            .secrets
            .get_secret(&profile.credential_account())?
            .is_some();
        Ok(match profile.kind {
            ProfileKind::ChatGptPlan => {
                stored
                    && profile
                        .chatgpt
                        .as_ref()
                        .is_some_and(|account| account.plan_usage)
            }
            _ => stored,
        })
    }

    /// Revokes consent and disables external calls. For an API-key profile
    /// the key is deleted too; a ChatGPT sign-in stays until the user signs out.
    pub fn revoke(&self, profile: &AiProfile) -> Result<AiProfile, ProfileError> {
        let revoked = revoke_consent(profile);
        self.profiles.save(&revoked)?;
        if profile.kind == ProfileKind::OpenAiCompatible {
            self.secrets.delete_secret(&profile.credential_account())?;
        }
        Ok(revoked)
    }

    /// Stores a completed ChatGPT sign-in: refresh token in the secret store,
    /// account facts in the profile. Signing into another account than the
    /// consented one changes the preview, so consent must be given again.
    pub fn store_sign_in(
        &self,
        profile: &AiProfile,
        signed_in: SignedIn,
    ) -> Result<AiProfile, ProfileError> {
        let mut updated = profile.clone();
        updated.kind = ProfileKind::ChatGptPlan;
        updated.endpoint = None;
        updated.chatgpt = Some(signed_in.account);
        self.secrets
            .set_secret(&updated.credential_account(), &signed_in.refresh_token)?;
        self.save(&updated)?;
        Ok(updated)
    }

    /// Forgets the ChatGPT sign-in: deletes the refresh token, disables
    /// external calls and keeps the issued client id and host id for the next
    /// sign-in (ADR-0004).
    pub fn forget_sign_in(&self, profile: &AiProfile) -> Result<AiProfile, ProfileError> {
        let mut updated = revoke_consent(profile);
        let account = format!("{}.chatgpt", profile.id);
        if let Some(chatgpt) = updated.chatgpt.as_mut() {
            chatgpt.email = None;
            chatgpt.subject = None;
            chatgpt.plan_usage = false;
        }
        self.secrets.delete_secret(&account)?;
        self.save(&updated)?;
        Ok(updated)
    }

    /// Reports the current extractor choice and credential presence.
    pub fn status(&self) -> Result<AiStatus, ProfileError> {
        let profile = self.load_or_seed()?;
        let has_secret = self
            .secrets
            .get_secret(&profile.credential_account())?
            .is_some();
        Ok(AiStatus {
            choice: choose_extractor(Some(&profile)),
            has_secret,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_preview, choose_extractor, consent_status, grant_consent, offline_default_profile,
        preview_hash, revoke_consent, AiProfile, AiSettings, ConsentPreview, ExtractorChoice,
        FileProfileStore, ProfileError, ProfileKind, ProfileStore, SecretStore,
        DEFAULT_MAX_INPUT_CHARS,
    };
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn temporary_directory(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};

        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let directory = std::env::temp_dir().join(format!(
            "xemnas-profile-{tag}-{}-{nanos}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).expect("create temporary directory");
        directory
    }

    fn external_profile() -> AiProfile {
        AiProfile {
            id: "profile-1".to_string(),
            kind: ProfileKind::OpenAiCompatible,
            model: "gpt-test".to_string(),
            endpoint: Some("https://api.example.test/v1".to_string()),
            max_input_chars: 4_096,
            external_calls_enabled: false,
            consent: None,
            chatgpt: None,
        }
    }

    #[derive(Default)]
    struct MemoryProfiles {
        profile: RefCell<Option<AiProfile>>,
    }

    impl ProfileStore for MemoryProfiles {
        fn load(&self) -> Result<Option<AiProfile>, ProfileError> {
            Ok(self.profile.borrow().clone())
        }
        fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
            *self.profile.borrow_mut() = Some(profile.clone());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemorySecrets {
        secrets: RefCell<HashMap<String, String>>,
    }

    impl SecretStore for MemorySecrets {
        fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
            self.secrets
                .borrow_mut()
                .insert(account.to_string(), secret.to_string());
            Ok(())
        }
        fn get_secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
            Ok(self.secrets.borrow().get(account).cloned())
        }
        fn delete_secret(&self, account: &str) -> Result<(), ProfileError> {
            self.secrets.borrow_mut().remove(account);
            Ok(())
        }
    }

    #[test]
    fn file_roundtrip_and_rejects_unknown_fields() {
        let root = temporary_directory("roundtrip");
        let path = root.join("settings").join("ai-profile.json");
        let store = FileProfileStore::new(&path);
        assert_eq!(store.load().expect("load"), None);

        let profile = external_profile();
        store.save(&profile).expect("save");
        assert_eq!(store.load().expect("reload"), Some(profile));

        std::fs::write(
            &path,
            "{\"id\":\"x\",\"kind\":\"fake\",\"model\":\"m\",\"extra\":true}",
        )
        .expect("write");
        assert!(
            matches!(store.load(), Err(ProfileError::Invalid(_))),
            "unknown fields must be rejected"
        );

        std::fs::write(&path, "{ not json").expect("write");
        assert!(matches!(store.load(), Err(ProfileError::Invalid(_))));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_rules_for_both_kinds() {
        assert!(offline_default_profile().validate().is_ok());

        let mut profile = external_profile();
        assert!(profile.validate().is_ok());
        profile.endpoint = Some("ftp://example.test".to_string());
        assert!(profile.validate().is_err());
        profile.endpoint = Some("https://api.example.test".to_string());
        profile.model = "  ".to_string();
        assert!(profile.validate().is_err());
        profile.model = "gpt-test".to_string();
        profile.max_input_chars = 0;
        assert!(profile.validate().is_err());
    }

    #[test]
    fn choose_extractor_matrix() {
        assert_eq!(choose_extractor(None), ExtractorChoice::OfflineFake);
        assert_eq!(
            choose_extractor(Some(&offline_default_profile())),
            ExtractorChoice::OfflineFake
        );

        let profile = external_profile();
        assert_eq!(
            choose_extractor(Some(&profile)),
            ExtractorChoice::ExternalBlocked
        );

        let enabled_without_consent = AiProfile {
            external_calls_enabled: true,
            consent: None,
            ..external_profile()
        };
        assert_eq!(
            choose_extractor(Some(&enabled_without_consent)),
            ExtractorChoice::ExternalBlocked,
            "enabled without consent must stay blocked"
        );

        let forged = AiProfile {
            external_calls_enabled: true,
            consent: Some(super::ConsentRecord {
                granted_at: "2026-01-01T00:00:00Z".to_string(),
                preview_hash: "forged".to_string(),
            }),
            ..external_profile()
        };
        assert_eq!(
            choose_extractor(Some(&forged)),
            ExtractorChoice::ExternalBlocked,
            "a consent record that does not match the preview must stay blocked"
        );

        let preview = build_preview(&profile);
        let granted = grant_consent(&profile, &preview, "2026-01-01T00:00:00Z", true)
            .expect("grant succeeds");
        assert_eq!(
            choose_extractor(Some(&granted)),
            ExtractorChoice::ExternalEnabled
        );
    }

    #[test]
    fn consent_is_bound_to_the_approved_configuration() {
        let profile = external_profile();
        let preview = build_preview(&profile);
        let granted = grant_consent(&profile, &preview, "2026-01-01T00:00:00Z", true)
            .expect("grant succeeds");
        assert_eq!(
            choose_extractor(Some(&granted)),
            ExtractorChoice::ExternalEnabled
        );

        let mut moved_endpoint = granted.clone();
        moved_endpoint.endpoint = Some("https://other.example.test/v1".to_string());
        assert_eq!(
            choose_extractor(Some(&moved_endpoint)),
            ExtractorChoice::ExternalBlocked
        );
        assert!(consent_status(&moved_endpoint).is_err());

        let mut changed_model = granted.clone();
        changed_model.model = "another-model".to_string();
        assert_eq!(
            choose_extractor(Some(&changed_model)),
            ExtractorChoice::ExternalBlocked
        );
        assert!(consent_status(&changed_model).is_err());

        let mut changed_limit = granted.clone();
        changed_limit.max_input_chars = granted.max_input_chars + 1;
        assert_eq!(
            choose_extractor(Some(&changed_limit)),
            ExtractorChoice::ExternalBlocked
        );
        assert!(consent_status(&changed_limit).is_err());

        let mut changed_kind = granted.clone();
        changed_kind.kind = ProfileKind::Fake;
        assert_eq!(
            choose_extractor(Some(&changed_kind)),
            ExtractorChoice::ExternalBlocked
        );
        assert!(consent_status(&changed_kind).is_err());
    }

    #[test]
    fn preview_is_deterministic_and_sensitive() {
        let profile = external_profile();
        let first = build_preview(&profile);
        let second = build_preview(&profile);
        assert_eq!(first, second);
        assert_eq!(first.endpoint_host.as_deref(), Some("api.example.test"));
        assert_eq!(first.categories.len(), 4);
        assert_eq!(first.total_approximate_chars, 4_096 * 4);

        let mut other = profile.clone();
        other.model = "another-model".to_string();
        assert_ne!(preview_hash(&first), preview_hash(&build_preview(&other)));

        let mut moved = profile;
        moved.endpoint = Some("https://other.example.test/v1".to_string());
        assert_ne!(preview_hash(&first), preview_hash(&build_preview(&moved)));
    }

    #[test]
    fn consent_rejects_same_host_endpoint_changes() {
        let profile = external_profile();
        let granted = grant_consent(
            &profile,
            &build_preview(&profile),
            "2026-01-01T00:00:00Z",
            true,
        )
        .expect("grant");
        for endpoint in [
            "https://api.example.test/v2",
            "https://api.example.test:8443/v1",
            "http://api.example.test/v1",
        ] {
            let mut changed = granted.clone();
            changed.endpoint = Some(endpoint.into());
            assert!(consent_status(&changed).is_err(), "endpoint: {endpoint}");
            assert_ne!(
                preview_hash(&build_preview(&changed)),
                granted.consent.as_ref().expect("consent").preview_hash
            );
        }
    }

    #[test]
    fn legacy_host_only_consent_requires_a_new_approval() {
        let mut profile = external_profile();
        let mut legacy_preview = serde_json::to_value(build_preview(&profile)).expect("preview");
        legacy_preview
            .as_object_mut()
            .expect("object")
            .remove("endpoint_fingerprint");
        profile.external_calls_enabled = true;
        profile.consent = Some(super::ConsentRecord {
            granted_at: "2026-01-01T00:00:00Z".into(),
            preview_hash: integration_contracts::capture::artifact_fingerprint(
                &serde_json::to_string(&legacy_preview).expect("serialize"),
            ),
        });
        assert!(consent_status(&profile).is_err());
        assert_eq!(
            choose_extractor(Some(&profile)),
            ExtractorChoice::ExternalBlocked
        );
    }

    #[test]
    fn endpoints_require_tls_or_loopback_and_never_expose_credentials() {
        let mut profile = external_profile();
        for endpoint in [
            "http://127.0.0.1:11434/v1",
            "http://[::1]:11434/v1",
            "https://api.example.test/v1",
        ] {
            profile.endpoint = Some(endpoint.into());
            assert!(profile.validate().is_ok(), "endpoint: {endpoint}");
        }
        for endpoint in [
            "http://api.example.test/v1",
            "http://192.168.1.1/v1",
            "http://127.0.0.1.example.test/v1",
            "https://host/v1?key=SECRET-MARKER",
            "https://host/v1#frag",
            "https://user:SECRET-MARKER@host/v1",
            "https://",
        ] {
            profile.endpoint = Some(endpoint.into());
            let error = profile.validate().expect_err("reject invalid endpoint");
            assert!(!error.to_string().contains("SECRET-MARKER"));
            let preview = serde_json::to_string(&build_preview(&profile)).expect("serialize");
            assert!(!preview.contains("SECRET-MARKER"));
            assert!(!preview.contains("user:"));
        }
    }

    #[test]
    fn concurrent_profile_saves_always_leave_a_complete_profile() {
        let root = temporary_directory("concurrent");
        let store = FileProfileStore::new(root.join("profile.json"));
        store.save(&external_profile()).expect("seed");
        std::thread::scope(|scope| {
            for index in 0..4 {
                let store = &store;
                scope.spawn(move || {
                    let mut profile = external_profile();
                    profile.model = format!("model-{index}");
                    for _ in 0..20 {
                        store.save(&profile).expect("atomic save");
                        assert!(store.load().expect("complete JSON").is_some());
                    }
                });
            }
        });
        assert!(store.load().expect("complete final profile").is_some());
        assert_eq!(
            std::fs::read_dir(&root).expect("directory").count(),
            1,
            "no abandoned temporaries"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[cfg(windows)]
    #[test]
    fn failed_replacement_preserves_the_previous_profile() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = temporary_directory("replace-failure");
        let store = FileProfileStore::new(root.join("profile.json"));
        let previous = external_profile();
        store.save(&previous).expect("seed");
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(store.path())
            .expect("hold profile");
        let mut next = previous.clone();
        next.model = "new-model".into();
        assert!(store.save(&next).is_err());
        assert_eq!(store.load().expect("old profile survives"), Some(previous));
        drop(held);
        store
            .save(&next)
            .expect("replacement succeeds after releasing handle");
        assert_eq!(store.load().expect("new profile"), Some(next));
        assert_eq!(std::fs::read_dir(&root).expect("directory").count(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn grant_requires_secret_matching_preview_and_external_kind() {
        let profile = external_profile();
        let preview = build_preview(&profile);

        assert!(matches!(
            grant_consent(&profile, &preview, "2026-01-01T00:00:00Z", false),
            Err(ProfileError::Invalid(_))
        ));

        let mut stale = preview.clone();
        stale.model = "stale".to_string();
        assert!(matches!(
            grant_consent(&profile, &stale, "2026-01-01T00:00:00Z", true),
            Err(ProfileError::Invalid(_))
        ));

        assert!(matches!(
            grant_consent(&offline_default_profile(), &preview, "now", true),
            Err(ProfileError::Invalid(_))
        ));

        let granted = grant_consent(&profile, &preview, "2026-01-01T00:00:00Z", true)
            .expect("grant succeeds");
        assert!(granted.external_calls_enabled);
        assert_eq!(
            granted.consent.expect("consent").preview_hash,
            preview_hash(&preview)
        );
    }

    #[test]
    fn revoke_clears_enablement_and_consent() {
        let mut profile = external_profile();
        profile.external_calls_enabled = true;
        profile.consent = Some(super::ConsentRecord {
            granted_at: "2026-01-01T00:00:00Z".to_string(),
            preview_hash: "hash".to_string(),
        });

        let revoked = revoke_consent(&profile);
        assert!(!revoked.external_calls_enabled);
        assert!(revoked.consent.is_none());
        assert_eq!(
            choose_extractor(Some(&revoked)),
            ExtractorChoice::ExternalBlocked
        );
    }

    #[test]
    fn load_or_seed_creates_the_offline_default() {
        let settings = AiSettings::new(MemoryProfiles::default(), MemorySecrets::default());
        let profile = settings.load_or_seed().expect("seed");
        assert_eq!(profile.kind, ProfileKind::Fake);
        assert!(!profile.external_calls_enabled);
        assert_eq!(profile.max_input_chars, DEFAULT_MAX_INPUT_CHARS);
        assert_eq!(
            settings.status().expect("status").choice,
            ExtractorChoice::OfflineFake
        );
    }

    #[test]
    fn facade_grant_and_revoke_use_the_secret_store() {
        let settings = AiSettings::new(MemoryProfiles::default(), MemorySecrets::default());
        let profile = external_profile();
        settings.save(&profile).expect("save");
        settings
            .set_secret(&profile.id, "synthetic-secret")
            .expect("secret");

        let preview: ConsentPreview = settings.preview(&profile);
        let granted = settings
            .grant(&profile, &preview, "2026-01-01T00:00:00Z")
            .expect("grant");
        assert_eq!(
            settings.status().expect("status").choice,
            ExtractorChoice::ExternalEnabled
        );

        let _revoked = settings.revoke(&granted).expect("revoke");
        assert_eq!(
            settings.status().expect("status").choice,
            ExtractorChoice::ExternalBlocked
        );
        assert!(!settings.status().expect("status").has_secret);
    }

    fn chatgpt_profile() -> AiProfile {
        AiProfile {
            kind: ProfileKind::ChatGptPlan,
            model: "gpt-plan".to_string(),
            endpoint: None,
            chatgpt: Some(super::ChatGptAccount {
                client_id: Some("oaiapp_test".to_string()),
                host_id: "urn:uuid:00000000-0000-4000-8000-000000000000".to_string(),
                email: Some("person@example.test".to_string()),
                subject: Some("sub-1".to_string()),
                plan_usage: true,
            }),
            ..external_profile()
        }
    }

    #[test]
    fn existing_profiles_keep_their_preview_and_consent() {
        let preview = serde_json::to_string(&build_preview(&external_profile())).expect("json");
        assert!(!preview.contains("account"), "no new key for the old kinds");
        let legacy = "{\"id\":\"x\",\"kind\":\"open_ai_compatible\",\"model\":\"m\",            \"endpoint\":\"https://api.example.test/v1\",\"consent\":null}";
        let loaded: AiProfile = serde_json::from_str(legacy).expect("old file still loads");
        assert_eq!(loaded.chatgpt, None);
    }

    #[test]
    fn new_kinds_validate_their_destination() {
        let mut chatgpt = chatgpt_profile();
        assert!(chatgpt.validate().is_ok());
        chatgpt.endpoint = Some("https://elsewhere.test/v1".to_string());
        assert!(
            chatgpt.validate().is_err(),
            "the plan always goes to OpenAI"
        );

        let mut opencode = AiProfile {
            kind: ProfileKind::OpenCode,
            model: "anthropic/claude-test".to_string(),
            endpoint: Some(super::OPENCODE_DEFAULT_ENDPOINT.to_string()),
            ..external_profile()
        };
        assert!(opencode.validate().is_ok());
        opencode.model = "no-provider".to_string();
        assert!(opencode.validate().is_err());
        opencode.model = "anthropic/claude-test".to_string();
        opencode.endpoint = Some("https://remote.example.test".to_string());
        assert!(
            opencode.validate().is_err(),
            "OpenCode must be on this machine"
        );
    }

    #[test]
    fn credentials_are_required_only_where_the_kind_needs_them() {
        let mut local = external_profile();
        local.endpoint = Some("http://127.0.0.1:11434/v1".to_string());
        assert!(!local.credential_required(), "a local model needs no key");
        assert!(external_profile().credential_required());
        assert!(chatgpt_profile().credential_required());
        assert_eq!(chatgpt_profile().credential_account(), "profile-1.chatgpt");
        assert_eq!(external_profile().credential_account(), "profile-1");

        let granted = grant_consent(&local, &build_preview(&local), "now", false)
            .expect("local model consents without a key");
        assert_eq!(
            choose_extractor(Some(&granted)),
            ExtractorChoice::ExternalEnabled
        );
        assert!(grant_consent(
            &chatgpt_profile(),
            &build_preview(&chatgpt_profile()),
            "now",
            false
        )
        .is_err());
    }

    #[test]
    fn chatgpt_consent_is_bound_to_the_account() {
        let profile = chatgpt_profile();
        let preview = build_preview(&profile);
        assert_eq!(preview.endpoint_host.as_deref(), Some("api.openai.com"));
        assert_eq!(preview.account.as_deref(), Some("person@example.test"));
        let granted = grant_consent(&profile, &preview, "now", true).expect("grant");
        let mut other = granted.clone();
        other.chatgpt.as_mut().expect("account").email = Some("other@example.test".into());
        assert!(
            consent_status(&other).is_err(),
            "another account needs a new consent"
        );
    }

    #[test]
    fn sign_in_is_stored_and_forgotten_through_the_secret_store() {
        let settings = AiSettings::new(MemoryProfiles::default(), MemorySecrets::default());
        let profile = settings.load_or_seed().expect("seed");
        let mut account = chatgpt_profile().chatgpt.expect("account");
        account.plan_usage = true;
        let stored = settings
            .store_sign_in(
                &AiProfile {
                    model: "gpt-plan".to_string(),
                    ..profile
                },
                super::SignedIn {
                    account,
                    refresh_token: "refresh-synthetic".to_string(),
                },
            )
            .expect("store");
        assert_eq!(stored.kind, ProfileKind::ChatGptPlan);
        assert!(settings.credential_ready(&stored).expect("ready"));
        let preview = settings.preview(&stored);
        let granted = settings.grant(&stored, &preview, "now").expect("grant");
        assert_eq!(
            settings.status().expect("status").choice,
            ExtractorChoice::ExternalEnabled
        );

        let forgotten = settings.forget_sign_in(&granted).expect("forget");
        assert!(!forgotten.external_calls_enabled);
        let kept = forgotten.chatgpt.expect("client id kept");
        assert_eq!(kept.client_id.as_deref(), Some("oaiapp_test"));
        assert!(!kept.plan_usage);
        assert!(!settings.status().expect("status").has_secret);
    }
}
