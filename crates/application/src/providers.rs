//! Ports for choosing an AI provider (ADR-0004): the model catalog and the
//! ChatGPT account sign-in. Implementations live in `ai-provider`; the desktop
//! reaches them only through these traits.

use std::time::Duration;

use crate::profile::{AiProfile, ChatGptAccount, SignedIn};

/// Where the user follows how much of the ChatGPT plan xemnas used.
pub const CHATGPT_USAGE_URL: &str = "https://chatgpt.com/settings/usage";

/// One model the configured destination offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInfo {
    /// Identifier sent back as the profile's `model`.
    pub id: String,
    /// Name shown in the picker.
    pub label: String,
}

/// Why a catalog, sign-in or sign-out could not complete. Messages are
/// product language and never carry tokens, URLs with secrets or raw errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    /// Nothing answered at the configured address.
    Unreachable,
    /// The destination refused the credential (or none was configured).
    Unauthorized,
    /// The ChatGPT account is not signed in, or the plan usage was not granted.
    NotSignedIn,
    /// The browser sign-in was cancelled, denied or timed out.
    Cancelled,
    /// The answer did not have the documented shape.
    InvalidResponse,
    /// The secret store could not be read or written.
    Keystore,
}

impl ProviderError {
    /// Product-language message for the settings screen.
    pub fn message(&self) -> &'static str {
        match self {
            Self::Unreachable => "Nada respondeu nesse endereço.",
            Self::Unauthorized => "O provedor recusou a credencial configurada.",
            Self::NotSignedIn => "Entre com a conta ChatGPT e permita o uso do plano.",
            Self::Cancelled => "O login foi cancelado ou expirou.",
            Self::InvalidResponse => "O provedor respondeu num formato inesperado.",
            Self::Keystore => "Não foi possível acessar o cofre do sistema.",
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for ProviderError {}

/// Lists the models the profile's destination offers. Blocking; callers run
/// it off the UI thread.
pub trait ModelCatalog: Send + Sync {
    /// Models for `profile`. `secret` is the credential stored for the
    /// profile's kind (API key), when there is one.
    fn list(
        &self,
        profile: &AiProfile,
        secret: Option<String>,
    ) -> Result<Vec<ModelInfo>, ProviderError>;
}

/// A browser sign-in that is waiting for the loopback callback.
pub trait PendingSignIn: Send {
    /// URL to open in the user's browser.
    fn authorize_url(&self) -> &str;

    /// A handle that stops [`PendingSignIn::wait`] early (Cancel button).
    fn canceller(&self) -> Box<dyn Fn() + Send + Sync>;

    /// Blocks until the callback arrives, the user cancels or `timeout`
    /// passes, then exchanges the code. Never runs on the UI thread.
    fn wait(self: Box<Self>, timeout: Duration) -> Result<SignedIn, ProviderError>;
}

/// ChatGPT account sign-in and sign-out (Sign in with ChatGPT for open
/// source apps).
pub trait PlanAccount: Send + Sync {
    /// Binds the loopback callback and prepares the authorization URL.
    /// `previous` reuses the issued client id and host id.
    fn start_sign_in(
        &self,
        previous: Option<&ChatGptAccount>,
    ) -> Result<Box<dyn PendingSignIn>, ProviderError>;

    /// Revokes `refresh_token` for the account. A network failure is
    /// reported so the UI can say the account can also be disconnected in
    /// ChatGPT's settings; local tokens are cleared either way by the caller.
    fn sign_out(
        &self,
        account: &ChatGptAccount,
        refresh_token: Option<String>,
    ) -> Result<(), ProviderError>;
}
