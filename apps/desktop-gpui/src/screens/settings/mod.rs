//! Application settings: how candidates are extracted and what leaves the machine.
//!
//! The page reads and writes only through [`AiBackend`], implemented by the
//! `AiSettings` use case, plus the provider ports of ADR-0004 (model catalog,
//! ChatGPT sign-in and the local Claude Code check). Every call touches the
//! profile file, the OS key vault, the network or a local CLI, so it runs on
//! the background executor. Stored credentials
//! are never read back into the view: the page only learns whether they exist.

use std::collections::BTreeMap;
use std::sync::Arc;

use application::profile::{
    build_preview, choose_extractor, consent_status, revoke_consent, AiProfile, AiSettings,
    ConsentPreview, ExtractorChoice, ProfileError, ProfileKind, ProfileStore, SecretStore,
    SignedIn, OPENCODE_GO_ENDPOINT, OPENCODE_ZEN_ENDPOINT,
};
use application::providers::{
    ClaudeCodeProbe, ClaudeCodeStatus, ModelCatalog, ModelInfo, PlanAccount,
};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, Context, Div, Entity, EventEmitter, FocusHandle, Render, Role,
    SharedString, Stateful, Subscription, Window,
};

use super::format::{date_time, thousands};
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    error_banner, mark_selected, radio_row, skeleton_list, toast, TOAST_DURATION,
};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{tint, SpacingScale, TypeScale};

/// Product copy too long to sit inside the element chains.
const REVOKE_WARNING: &str = "Isso desliga as chamadas externas e apaga a chave do cofre. \
                              O endereço e o modelo continuam salvos.";
const REVOKE_ONLY_WARNING: &str =
    "Isso desliga as chamadas externas. A configuração e a conta continuam salvas.";
const PAGE_SUBTITLE: &str = "Como as capturas viram candidatos e o que pode sair desta máquina.";
const LOCAL_ONLY: &str = "Com o extrator local, nenhum conteúdo das capturas sai desta máquina.";
const KEY_DESCRIPTION: &str =
    "Fica no Gerenciador de Credenciais do sistema, nunca em arquivo, e não é exibida aqui.";
const CONSENT_INVALIDATION: &str =
    "Salvar muda a prévia: o consentimento atual deixa de valer até você consentir de novo.";
const REDACTION_ON: &str = "Segredos são redigidos na captura, antes de qualquer envio.";

/// Width of the section navigation.
const NAV_WIDTH: f32 = 220.0;
/// Reading column of a settings section.
const READING_WIDTH: f32 = 720.0;

/// Which credentials the vault holds for the profile, one per provider kind.
/// Only presence reaches the view, never the values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Credentials {
    /// API key of the OpenAI-compatible provider.
    pub api_key: bool,
    /// Refresh token of the ChatGPT account.
    pub chatgpt: bool,
    /// API key of OpenCode Zen or Go.
    pub opencode: bool,
}

impl Credentials {
    /// Whether the credential of `kind` is stored.
    fn stored(self, kind: ProfileKind) -> bool {
        match kind {
            ProfileKind::Fake | ProfileKind::ClaudeCode => false,
            ProfileKind::OpenAiCompatible => self.api_key,
            ProfileKind::ChatGptPlan => self.chatgpt,
            ProfileKind::OpenCode => self.opencode,
        }
    }
}

/// `profile` as if it used `kind`: the credential account and the rules
/// follow the kind the form shows, before it is saved.
fn as_kind(profile: &AiProfile, kind: ProfileKind) -> AiProfile {
    AiProfile {
        kind,
        ..profile.clone()
    }
}

/// The AI settings operations the page needs, object-safe so the shell stays
/// generic only over its storage type.
pub trait AiBackend: Send + 'static {
    /// Loads (or seeds) the profile and reports which credentials are stored.
    fn load(&self) -> Result<(AiProfile, Credentials), ProfileError>;
    /// Persists a validated profile.
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError>;
    /// Stores or replaces the provider key for `account`.
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError>;
    /// Records consent for the previewed configuration and persists it.
    fn grant(
        &self,
        profile: &AiProfile,
        preview: &ConsentPreview,
        now: &str,
    ) -> Result<AiProfile, ProfileError>;
    /// Disables external calls and deletes the stored key.
    fn revoke(&self, profile: &AiProfile) -> Result<AiProfile, ProfileError>;
    /// The stored credential for `account`, for the model catalog only.
    fn secret(&self, account: &str) -> Result<Option<String>, ProfileError>;
    /// Stores a completed ChatGPT sign-in.
    fn store_sign_in(
        &self,
        profile: &AiProfile,
        signed_in: SignedIn,
    ) -> Result<AiProfile, ProfileError>;
    /// Forgets the ChatGPT sign-in and deletes its token.
    fn forget_sign_in(&self, profile: &AiProfile) -> Result<AiProfile, ProfileError>;
}

impl<S, T> AiBackend for AiSettings<S, T>
where
    S: ProfileStore + Send + 'static,
    T: SecretStore + Send + 'static,
{
    fn load(&self) -> Result<(AiProfile, Credentials), ProfileError> {
        let profile = self.load_or_seed()?;
        let stored = |kind| {
            AiSettings::secret(self, &as_kind(&profile, kind).credential_account())
                .map(|secret| secret.is_some())
        };
        let credentials = Credentials {
            api_key: stored(ProfileKind::OpenAiCompatible)?,
            chatgpt: stored(ProfileKind::ChatGptPlan)?,
            opencode: stored(ProfileKind::OpenCode)?,
        };
        Ok((profile, credentials))
    }
    fn save(&self, profile: &AiProfile) -> Result<(), ProfileError> {
        AiSettings::save(self, profile)
    }
    fn set_secret(&self, account: &str, secret: &str) -> Result<(), ProfileError> {
        AiSettings::set_secret(self, account, secret)
    }
    fn grant(
        &self,
        profile: &AiProfile,
        preview: &ConsentPreview,
        now: &str,
    ) -> Result<AiProfile, ProfileError> {
        AiSettings::grant(self, profile, preview, now)
    }
    fn revoke(&self, profile: &AiProfile) -> Result<AiProfile, ProfileError> {
        AiSettings::revoke(self, profile)
    }
    fn secret(&self, account: &str) -> Result<Option<String>, ProfileError> {
        AiSettings::secret(self, account)
    }
    fn store_sign_in(
        &self,
        profile: &AiProfile,
        signed_in: SignedIn,
    ) -> Result<AiProfile, ProfileError> {
        AiSettings::store_sign_in(self, profile, signed_in)
    }
    fn forget_sign_in(&self, profile: &AiProfile) -> Result<AiProfile, ProfileError> {
        AiSettings::forget_sign_in(self, profile)
    }
}

mod appearance;
mod diagnostics;
mod opencode;
mod parts;
mod providers;

pub use appearance::AppearancePanel;
pub use diagnostics::{DiagnosticsBackend, DiagnosticsPanel, DiagnosticsService};
pub use opencode::{IntegrationBackend, IntegrationService, OpenCodePanel};

use parts::{card, card_body, card_footer, status_hero, step};

/// Emitted when the user leaves the settings page.
pub struct CloseSettings;

enum Load {
    Loading,
    Failed(String),
    Ready,
}

#[derive(Clone, Copy)]
enum Action {
    Close,
    Kind(ProfileKind),
    Preset(usize),
    OpenCodePlan(bool),
    OpenCodeConsole,
    ListModels,
    PickModel(usize),
    Save,
    StoreKey,
    SignIn,
    CancelSignIn,
    ReopenBrowser,
    SignOut,
    ManageUsage,
    DismissPlanNotice,
    CheckClaudeCode,
    CopyLoginCommand,
    Grant,
    AskRevoke,
    CancelRevoke,
    Revoke,
    Retry,
}

type Loaded = Result<(AiProfile, Credentials), ProfileError>;
type Operation = Box<dyn FnOnce(&dyn AiBackend) -> Loaded + Send>;
/// Runs on the page after an operation reloaded the profile.
type After = Box<dyn FnOnce(&mut SettingsScreen, &mut Context<SettingsScreen>)>;

/// Models the configured destination offers, for the picker.
enum Models {
    Idle,
    Loading,
    Ready(Vec<ModelInfo>),
    Failed(&'static str),
}

/// The ChatGPT browser sign-in.
enum SignIn {
    Idle,
    Starting,
    Waiting {
        url: String,
        cancel: Box<dyn Fn() + Send + Sync>,
    },
}

/// What the local Claude Code reported the last time the page asked.
enum ClaudeCheck {
    Idle,
    Checking,
    Done(Result<ClaudeCodeStatus, &'static str>),
}

/// Everything the settings page reads and writes, built by the composition root.
pub struct SettingsServices {
    /// AI profile, key vault and consent.
    pub ai: Box<dyn AiBackend>,
    /// Lists the models of the chosen destination; absent, the model is typed.
    pub catalog: Option<Arc<dyn ModelCatalog>>,
    /// ChatGPT account sign-in; absent, the ChatGPT option cannot connect.
    pub account: Option<Arc<dyn PlanAccount>>,
    /// Checks the local Claude Code; absent, the Claude Code option cannot connect.
    pub claude_code: Option<Arc<dyn ClaudeCodeProbe>>,
    /// Integration status and connection test; absent without a database.
    pub integration: Option<Box<dyn IntegrationBackend>>,
    /// Diagnostics and job actions; absent without a database.
    pub diagnostics: Option<Box<dyn DiagnosticsBackend>>,
}

/// The sections of the settings page, in navigation order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsSection {
    /// Extraction provider, key vault and consent.
    Ai,
    /// Capture integration with OpenCode.
    OpenCode,
    /// Pipeline health and the sanitized diagnostics document.
    Diagnostics,
    /// Theme and background.
    Appearance,
}

impl SettingsSection {
    fn id(self) -> &'static str {
        match self {
            Self::Ai => "settings-section-ai",
            Self::OpenCode => "settings-section-opencode",
            Self::Diagnostics => "settings-section-diagnostics",
            Self::Appearance => "settings-section-appearance",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Ai => "IA e privacidade",
            Self::OpenCode => "OpenCode",
            Self::Diagnostics => "Diagnóstico",
            Self::Appearance => "Aparência",
        }
    }
    fn subtitle(self) -> &'static str {
        match self {
            Self::Ai => PAGE_SUBTITLE,
            Self::OpenCode => "Se as capturas do OpenCode estão chegando e, se não, por quê.",
            Self::Diagnostics => "Como o pipeline está indo e o que se perdeu no caminho.",
            Self::Appearance => "O tema do app e o fundo atrás das superfícies.",
        }
    }
    fn glyph(self) -> IconName {
        match self {
            Self::Ai => IconName::Shield,
            Self::OpenCode => IconName::Link,
            Self::Diagnostics => IconName::Activity,
            Self::Appearance => IconName::Contrast,
        }
    }
}

/// Settings page: AI and privacy, OpenCode and diagnostics.
pub struct SettingsScreen {
    section: SettingsSection,
    opencode: Option<Entity<OpenCodePanel>>,
    diagnostics: Option<Entity<DiagnosticsPanel>>,
    appearance: Entity<AppearancePanel>,
    backend: Option<Box<dyn AiBackend>>,
    catalog: Option<Arc<dyn ModelCatalog>>,
    account: Option<Arc<dyn PlanAccount>>,
    claude_code: Option<Arc<dyn ClaudeCodeProbe>>,
    claude: ClaudeCheck,
    /// Provider to show once the profile loads (`settings:claude-code`).
    requested_kind: Option<ProfileKind>,
    busy: bool,
    load: Load,
    stored: Option<AiProfile>,
    credentials: Credentials,
    kind: ProfileKind,
    /// Endpoint, model and per-item limit as stored, to detect edits.
    original: [String; 3],
    /// The same fields as last filled by the page (load or a switch of
    /// provider), so validation errors wait for the user's own typing.
    baseline: [String; 3],
    fields: [Entity<SearchField>; 3],
    models: Models,
    model_focus: Vec<FocusHandle>,
    sign_in: SignIn,
    /// Shown once after the first ChatGPT sign-in (usage guidelines).
    plan_notice: bool,
    key: Entity<SearchField>,
    _subscriptions: Vec<Subscription>,
    error: Option<String>,
    notice: Option<String>,
    confirm_revoke: bool,
    focus: BTreeMap<&'static str, FocusHandle>,
}

impl EventEmitter<CloseSettings> for SettingsScreen {}

const FIELD_PLACEHOLDERS: [&str; 3] = [
    "https://api.exemplo.com/v1",
    "Nome do modelo no provedor",
    "8192",
];

const FIELD_LABELS: [&str; 3] = [
    "Endereço do provedor",
    "Modelo",
    "Limite de caracteres por item",
];

impl SettingsScreen {
    /// Mounts the page; nothing is read until [`Self::open`].
    pub fn new(cx: &mut Context<Self>, services: SettingsServices) -> Self {
        let SettingsServices {
            ai: backend,
            catalog,
            account,
            claude_code,
            integration,
            diagnostics,
        } = services;
        let opencode = integration.map(|backend| cx.new(|_| OpenCodePanel::new(backend)));
        let diagnostics = diagnostics.map(|backend| cx.new(|_| DiagnosticsPanel::new(backend)));
        let appearance = cx.new(|_| AppearancePanel::default());
        let fields = std::array::from_fn(|index| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.stretch();
                field.set_context(FIELD_PLACEHOLDERS[index], cx);
                field
            })
        });
        let key = cx.new(|cx| {
            let mut field = SearchField::new(cx);
            field.secret();
            field.set_context("Chave de API do provedor", cx);
            field
        });
        let subscriptions = fields
            .iter()
            .chain([&key])
            .map(|field| cx.subscribe(field, |_, _, _: &SearchChanged, cx| cx.notify()))
            .collect();
        Self {
            section: SettingsSection::Ai,
            opencode,
            diagnostics,
            appearance,
            backend: Some(backend),
            catalog,
            account,
            claude_code,
            claude: ClaudeCheck::Idle,
            requested_kind: None,
            busy: false,
            load: Load::Loading,
            stored: None,
            credentials: Credentials::default(),
            kind: ProfileKind::Fake,
            original: Default::default(),
            baseline: Default::default(),
            fields,
            models: Models::Idle,
            model_focus: Vec::new(),
            sign_in: SignIn::Idle,
            plan_notice: false,
            key,
            _subscriptions: subscriptions,
            error: None,
            notice: None,
            confirm_revoke: false,
            focus: BTreeMap::new(),
        }
    }

    /// Reloads the stored profile each time the page opens, unless the user
    /// left unsaved edits behind.
    pub fn open(&mut self, cx: &mut Context<Self>) {
        let section = self.section;
        self.open_section(section, cx);
    }

    /// Polls only the visible read-only diagnostics; never reloads profile drafts.
    pub fn poll_visible(&mut self, cx: &mut Context<Self>) {
        if self.section == SettingsSection::Diagnostics {
            if let Some(panel) = &self.diagnostics {
                panel.update(cx, |panel, cx| panel.refresh(cx));
            }
        }
    }

    /// Supplies capture projection services to Diagnostics only.
    pub fn set_capture_progress(
        &mut self,
        read: crate::screens::inbox::ProgressRead,
        retry: crate::screens::inbox::ProgressRetry,
        cx: &mut Context<Self>,
    ) {
        if let Some(panel) = &self.diagnostics {
            panel.update(cx, |panel, _| panel.set_capture_progress(read, retry));
        }
    }

    /// Changes the project used by the diagnostics capture projection.
    pub fn set_progress_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if let Some(panel) = &self.diagnostics {
            panel.update(cx, |panel, cx| panel.set_project(project, cx));
        }
    }

    /// Shows `section` and reloads what it reads.
    pub fn open_section(&mut self, section: SettingsSection, cx: &mut Context<Self>) {
        self.section = section;
        match section {
            SettingsSection::Ai => self.open_ai(cx),
            SettingsSection::OpenCode => {
                if let Some(panel) = &self.opencode {
                    panel.update(cx, |panel, cx| panel.refresh(cx));
                }
            }
            SettingsSection::Diagnostics => {
                if let Some(panel) = &self.diagnostics {
                    panel.update(cx, |panel, cx| panel.refresh(cx));
                }
            }
            SettingsSection::Appearance => {}
        }
        cx.notify();
    }

    fn sections(&self) -> Vec<SettingsSection> {
        let mut sections = vec![SettingsSection::Ai];
        if self.opencode.is_some() {
            sections.push(SettingsSection::OpenCode);
        }
        if self.diagnostics.is_some() {
            sections.push(SettingsSection::Diagnostics);
        }
        sections.push(SettingsSection::Appearance);
        sections
    }

    fn open_ai(&mut self, cx: &mut Context<Self>) {
        if self.stored.is_some() && self.edited(cx) {
            return;
        }
        self.notice = None;
        self.confirm_revoke = false;
        self.run(Box::new(|backend| backend.load()), None, cx);
    }

    fn seed(&mut self, profile: AiProfile, credentials: Credentials, cx: &mut Context<Self>) {
        // The offline default carries a placeholder model name; it is not a
        // provider setting, so the provider form starts empty.
        let configured = profile.kind != ProfileKind::Fake;
        self.original = [
            profile.endpoint.clone().unwrap_or_default(),
            if configured {
                profile.model.clone()
            } else {
                String::new()
            },
            profile.max_input_chars.to_string(),
        ];
        let values = self.original.clone();
        self.fill(values, cx);
        if self.kind != profile.kind {
            self.models = Models::Idle;
        }
        self.kind = profile.kind;
        self.key_placeholder(cx);
        self.credentials = credentials;
        self.stored = Some(profile);
        self.load = Load::Ready;
        if let Some(kind) = self.requested_kind.take() {
            self.switch_kind(kind, cx);
        }
        if self.kind == ProfileKind::ClaudeCode && matches!(self.claude, ClaudeCheck::Idle) {
            self.check_claude_code(cx);
        }
    }

    /// Shows `kind` in the provider form after the next load, without
    /// saving it (the route a capture opens).
    pub fn show_kind(&mut self, kind: ProfileKind) {
        self.requested_kind = Some(kind);
    }

    /// Writes the provider form and makes it the new baseline.
    fn fill(&mut self, values: [String; 3], cx: &mut Context<Self>) {
        for (field, value) in self.fields.iter().zip(&values) {
            field.update(cx, |field, cx| field.set_value(value, cx));
        }
        self.baseline = values;
    }

    /// Shows the provider `kind` in the form. Fields keep the stored values
    /// when the stored profile already uses `kind`; otherwise they start from
    /// that kind's defaults, so an address never leaks from one kind to another.
    fn switch_kind(&mut self, kind: ProfileKind, cx: &mut Context<Self>) {
        if kind == self.kind {
            return;
        }
        self.kind = kind;
        self.error = None;
        self.models = Models::Idle;
        self.confirm_revoke = false;
        let same = self
            .stored
            .as_ref()
            .is_some_and(|stored| stored.kind == kind);
        let limit = self.value(2, cx);
        let values = if same {
            self.original.clone()
        } else {
            let endpoint = match kind {
                ProfileKind::OpenCode => OPENCODE_ZEN_ENDPOINT.to_owned(),
                _ => String::new(),
            };
            // Claude Code offers three fixed aliases; the default is the one
            // that measured fastest and cheapest per extraction.
            let model = match kind {
                ProfileKind::ClaudeCode => {
                    application::providers::CLAUDE_CODE_DEFAULT_MODEL.to_owned()
                }
                _ => String::new(),
            };
            [endpoint, model, limit]
        };
        self.fill(values, cx);
        self.key_placeholder(cx);
        if kind == ProfileKind::ClaudeCode && !matches!(self.claude, ClaudeCheck::Checking) {
            self.check_claude_code(cx);
        }
    }

    fn key_placeholder(&mut self, cx: &mut Context<Self>) {
        let placeholder = if self.kind == ProfileKind::OpenCode {
            "Chave de API do OpenCode"
        } else {
            "Chave de API do provedor"
        };
        self.key
            .update(cx, |field, cx| field.set_context(placeholder, cx));
    }

    fn value(&self, index: usize, cx: &App) -> String {
        self.fields[index].read(cx).value().trim().to_owned()
    }

    /// Whether the form differs from the stored profile.
    fn edited(&self, cx: &App) -> bool {
        let Some(stored) = &self.stored else {
            return false;
        };
        self.kind != stored.kind
            || (self.kind != ProfileKind::Fake
                && (0..3).any(|index| self.value(index, cx) != self.original[index]))
    }

    /// The profile the form describes, validated by the use case's rules.
    fn draft(&self, cx: &App) -> Result<AiProfile, String> {
        let profile = self.form(cx)?;
        profile.validate().map_err(failure)?;
        Ok(profile)
    }

    /// The profile as typed, before the use case's validation; the preview
    /// follows it so the user sees what a half-filled form would send.
    fn form(&self, cx: &App) -> Result<AiProfile, String> {
        let mut profile = self
            .stored
            .clone()
            .ok_or_else(|| "Configuração ainda não carregada.".to_owned())?;
        profile.kind = self.kind;
        if self.kind != ProfileKind::Fake {
            let endpoint = self.value(0, cx);
            let fixed = matches!(
                self.kind,
                ProfileKind::ChatGptPlan | ProfileKind::ClaudeCode
            );
            profile.endpoint = (!fixed && !endpoint.is_empty()).then_some(endpoint);
            profile.model = self.value(1, cx);
            profile.max_input_chars = self
                .value(2, cx)
                .parse()
                .map_err(|_| "O limite por item deve ser um número inteiro positivo.".to_owned())?;
        }
        Ok(profile)
    }

    /// Whether the user typed in the provider form (switching the extractor
    /// alone does not count, so errors wait for input).
    fn typed(&self, cx: &App) -> bool {
        (0..3).any(|index| self.value(index, cx) != self.baseline[index])
    }

    fn run(&mut self, operation: Operation, notice: Option<&'static str>, cx: &mut Context<Self>) {
        self.run_then(operation, notice, None, cx);
    }

    /// Runs `operation` on the background executor, reseeds the page from
    /// the profile it returns, then runs `after`.
    fn run_then(
        &mut self,
        operation: Operation,
        notice: Option<&'static str>,
        after: Option<After>,
        cx: &mut Context<Self>,
    ) {
        let Some(backend) = self.backend.take() else {
            return;
        };
        self.busy = true;
        self.error = None;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let (backend, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = operation(backend.as_ref());
                    (backend, outcome)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.backend = Some(backend);
                this.busy = false;
                match outcome {
                    Ok((profile, credentials)) => {
                        this.seed(profile, credentials, cx);
                        if let Some(notice) = notice {
                            this.show_notice(notice.to_owned(), cx);
                        }
                        if let Some(after) = after {
                            after(this, cx);
                        }
                    }
                    Err(error) if this.stored.is_none() => this.load = Load::Failed(failure(error)),
                    Err(error) => this.error = Some(failure(error)),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// A confirmation toast that leaves on its own.
    fn show_notice(&mut self, message: String, cx: &mut Context<Self>) {
        self.notice = Some(message.clone());
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TOAST_DURATION).await;
            let _ = this.update(cx, |this, cx| {
                if this.notice.as_deref() == Some(message.as_str()) {
                    this.notice = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn act(&mut self, action: Action, cx: &mut Context<Self>) {
        if self.busy && !matches!(action, Action::Close) {
            return;
        }
        self.notice = None;
        match action {
            Action::Close => cx.emit(CloseSettings),
            Action::Kind(kind) => self.switch_kind(kind, cx),
            Action::Preset(index) => {
                if let Some((_, endpoint)) = providers::LOCAL_PRESETS.get(index) {
                    self.fields[0].update(cx, |field, cx| field.set_value(endpoint, cx));
                    self.models = Models::Idle;
                }
            }
            Action::OpenCodePlan(go) => {
                let endpoint = if go {
                    OPENCODE_GO_ENDPOINT
                } else {
                    OPENCODE_ZEN_ENDPOINT
                };
                self.fields[0].update(cx, |field, cx| field.set_value(endpoint, cx));
                self.models = Models::Idle;
            }
            Action::OpenCodeConsole => cx.open_url(providers::OPENCODE_KEYS_URL),
            Action::ListModels => self.list_models(cx),
            Action::PickModel(index) => {
                if let Models::Ready(models) = &self.models {
                    if let Some(model) = models.get(index) {
                        let id = model.id.clone();
                        self.fields[1].update(cx, |field, cx| field.set_value(&id, cx));
                    }
                }
            }
            Action::SignIn => self.start_sign_in(cx),
            Action::CancelSignIn => {
                if let SignIn::Waiting { cancel, .. } = &self.sign_in {
                    cancel();
                }
            }
            Action::ReopenBrowser => {
                if let SignIn::Waiting { url, .. } = &self.sign_in {
                    cx.open_url(url);
                }
            }
            Action::SignOut => self.sign_out(cx),
            Action::ManageUsage => cx.open_url(application::providers::CHATGPT_USAGE_URL),
            Action::DismissPlanNotice => self.plan_notice = false,
            Action::CheckClaudeCode => self.check_claude_code(cx),
            Action::CopyLoginCommand => {
                cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                    application::providers::CLAUDE_CODE_LOGIN_COMMAND.to_owned(),
                ));
                self.show_notice("Comando copiado.".to_owned(), cx);
            }
            Action::Save => {
                if !self.edited(cx) {
                    return;
                }
                let profile = match self.draft(cx) {
                    Ok(profile) => revoke_consent(&profile),
                    Err(message) => {
                        self.error = Some(message);
                        cx.notify();
                        return;
                    }
                };
                let notice = if profile.kind == ProfileKind::Fake {
                    "Extração local ativada. Nenhum conteúdo sai desta máquina."
                } else {
                    "Configuração salva. Revise a prévia e consinta para ativar."
                };
                self.confirm_revoke = false;
                self.run(
                    Box::new(move |backend| {
                        backend.save(&profile)?;
                        backend.load()
                    }),
                    Some(notice),
                    cx,
                );
            }
            Action::StoreKey => {
                let (Some(stored), key) = (&self.stored, self.key.read(cx).value().trim()) else {
                    return;
                };
                if key.is_empty() {
                    return;
                }
                let account = as_kind(stored, self.kind).credential_account();
                let key = key.to_owned();
                let notice = "Chave guardada no cofre do sistema.";
                self.key_placeholder(cx);
                self.keeping_form(
                    Box::new(move |backend| {
                        backend.set_secret(&account, &key)?;
                        backend.load()
                    }),
                    Some(notice),
                    cx,
                );
            }
            Action::Grant => {
                let Some(stored) = self.stored.clone() else {
                    return;
                };
                let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                self.run(
                    Box::new(move |backend| {
                        backend.grant(&stored, &build_preview(&stored), &now)?;
                        backend.load()
                    }),
                    Some("Consentimento registrado. O provedor externo está ativo."),
                    cx,
                );
            }
            Action::AskRevoke => self.confirm_revoke = true,
            Action::CancelRevoke => self.confirm_revoke = false,
            Action::Revoke => {
                let Some(stored) = self.stored.clone() else {
                    return;
                };
                self.confirm_revoke = false;
                let notice = if stored.kind == ProfileKind::OpenAiCompatible {
                    "Chamadas externas desligadas e chave apagada do cofre."
                } else {
                    "Chamadas externas desligadas."
                };
                self.run(
                    Box::new(move |backend| {
                        backend.revoke(&stored)?;
                        backend.load()
                    }),
                    Some(notice),
                    cx,
                );
            }
            Action::Retry => {
                self.load = Load::Loading;
                self.run(Box::new(|backend| backend.load()), None, cx);
            }
        }
        cx.notify();
    }

    /// One product button wired to `action`, with a stable focus handle.
    fn button(
        &mut self,
        id: &'static str,
        kind: ButtonKind,
        enabled: bool,
        label: impl Into<SharedString>,
        action: Action,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = label.into();
        self.button_frame(id, kind, enabled, label.clone(), action, cx)
            .child(label)
    }

    /// The wired button without its content, for a leading icon.
    fn button_frame(
        &mut self,
        id: &'static str,
        kind: ButtonKind,
        enabled: bool,
        label: SharedString,
        action: Action,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let focus = self
            .focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        action_button(&theme, id, kind, enabled)
            .aria_label(label)
            .track_focus(&focus)
            .on_click(cx.listener(move |this, _, _, cx| {
                if enabled {
                    this.act(action, cx);
                }
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if enabled && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.act(action, cx);
                    cx.stop_propagation();
                }
            }))
    }

    fn render_nav(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let back = self
            .button_frame(
                "settings-close",
                ButtonKind::Ghost,
                true,
                "Voltar aos projetos".into(),
                Action::Close,
                cx,
            )
            .child(icon(
                IconName::ArrowLeft,
                14.0,
                button_foreground(theme, ButtonKind::Ghost, true),
            ))
            .child("Projetos");
        div()
            .w(px(NAV_WIDTH))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .p(px(SpacingScale::S3))
            .bg(theme.colors.rail())
            .border_r_1()
            .border_color(theme.colors.hairline_divider())
            .child(div().mb(px(SpacingScale::S4)).child(back))
            .child(
                text_style(div(), TypeScale::META)
                    .px(px(SpacingScale::S2))
                    .pb(px(SpacingScale::S2))
                    .text_color(theme.colors.text_muted())
                    .child("CONFIGURAÇÕES"),
            )
            .children(self.sections().into_iter().map(|section| {
                let selected = section == self.section;
                let focus = self
                    .focus
                    .entry(section.id())
                    .or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                let colors = theme.colors;
                mark_selected(
                    div()
                        .id(section.id())
                        .relative()
                        .px(px(SpacingScale::S2))
                        .py(px(SpacingScale::S2))
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .rounded(theme.radius.control())
                        .cursor_pointer()
                        .role(Role::Tab)
                        .aria_selected(selected)
                        .aria_label(section.title())
                        .track_focus(&focus)
                        .focus_visible(crate::ui::controls::focus_ring(theme))
                        .when(!selected, |row| {
                            row.hover(move |style| style.bg(colors.glass_fill_medium()))
                                .active(move |style| style.bg(colors.glass_fill_strong()))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| this.open_section(section, cx)))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.open_section(section, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    theme,
                    selected,
                )
                .child(icon(
                    section.glyph(),
                    16.0,
                    if selected {
                        colors.text_primary()
                    } else {
                        colors.text_muted()
                    },
                ))
                .child(
                    text_style(div(), TypeScale::ROW_TITLE)
                        .text_color(if selected {
                            colors.text_primary()
                        } else {
                            colors.text_secondary()
                        })
                        .child(section.title()),
                )
            }))
    }

    fn render_status(&self, theme: &Theme, stored: &AiProfile) -> Stateful<Div> {
        let (color, title, body): (_, &str, String) = match choose_extractor(Some(stored)) {
            ExtractorChoice::OfflineFake => (
                theme.colors.text_muted(),
                "Extração local, sem rede",
                "Candidatos são extraídos nesta máquina. Nenhum conteúdo das capturas é enviado."
                    .into(),
            ),
            ExtractorChoice::ExternalEnabled => (
                theme.colors.status_success(),
                "Provedor externo ativo",
                format!(
                    "Capturas são analisadas {}, dentro dos limites da prévia.",
                    providers::analysed_by(stored)
                ),
            ),
            ExtractorChoice::ExternalBlocked => (
                theme.colors.status_warning(),
                "Provedor externo bloqueado",
                format!(
                    "{} Até lá, nenhuma captura é enviada nem analisada.",
                    blocked_reason(consent_status(stored).err().unwrap_or_default())
                ),
            ),
        };
        status_hero(theme, "settings-status", color, title, body)
    }

    fn kind_option(
        &mut self,
        kind: ProfileKind,
        title: &'static str,
        body: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = Theme::current(cx);
        let selected = self.kind == kind;
        let (id, glyph) = match kind {
            ProfileKind::Fake => ("settings-kind-local", IconName::Cpu),
            ProfileKind::OpenAiCompatible => ("settings-kind-external", IconName::Cloud),
            ProfileKind::ChatGptPlan => ("settings-kind-chatgpt", IconName::User),
            ProfileKind::OpenCode => ("settings-kind-opencode", IconName::Link),
            ProfileKind::ClaudeCode => ("settings-kind-claude-code", IconName::Terminal),
        };
        let focus = self
            .focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let colors = theme.colors;
        radio_row(
            &theme,
            id,
            selected,
            kind != ProfileKind::Fake,
            glyph,
            title,
            body,
        )
        .when(!selected, |row| {
            row.hover(move |style| style.bg(colors.glass_fill_medium()))
                .active(move |style| style.bg(colors.glass_fill_strong()))
        })
        .track_focus(&focus)
        .focus_visible(crate::ui::controls::focus_ring(&theme))
        .on_click(cx.listener(move |this, _, _, cx| this.act(Action::Kind(kind), cx)))
        .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                this.act(Action::Kind(kind), cx);
                cx.stop_propagation();
            }
        }))
    }

    fn render_extractor(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let edited = self.edited(cx);
        let draft = self.draft(cx);
        let consent_active = self
            .stored
            .as_ref()
            .is_some_and(|stored| consent_status(stored).is_ok());
        let local = self.kind_option(
            ProfileKind::Fake,
            "Local",
            "Heurística offline. Nada sai desta máquina.",
            cx,
        );
        let remote = self.kind_option(
            ProfileKind::OpenAiCompatible,
            "Modelo local ou API",
            "Ollama, LM Studio ou um endereço compatível com OpenAI.",
            cx,
        );
        let chatgpt = self.kind_option(
            ProfileKind::ChatGptPlan,
            "Conta ChatGPT",
            "Usa o seu plano do ChatGPT, sem chave de API.",
            cx,
        );
        let opencode = self.kind_option(
            ProfileKind::OpenCode,
            "OpenCode Zen ou Go",
            "Cole a chave do OpenCode e escolha um modelo.",
            cx,
        );
        let claude_code = self.kind_option(
            ProfileKind::ClaudeCode,
            "Claude Code (experimental)",
            "Usa o Claude Code já conectado nesta máquina, sem chave de API.",
            cx,
        );
        let save = self.button(
            "settings-save",
            ButtonKind::Primary,
            !self.busy && edited && draft.is_ok(),
            if self.busy {
                "Salvando…"
            } else {
                "Salvar configuração"
            },
            Action::Save,
            cx,
        );
        let typed = self.typed(cx);
        let hint = match (&draft, edited) {
            (Err(message), true) if typed => Some((message.clone(), theme.colors.status_danger())),
            (Err(_), true) => Some((
                match self.kind {
                    ProfileKind::ChatGptPlan => "Escolha um modelo do plano para salvar.",
                    ProfileKind::OpenCode => "Escolha um modelo do OpenCode para salvar.",
                    ProfileKind::ClaudeCode => "Escolha um modelo do Claude Code para salvar.",
                    _ => "Informe o endereço e o modelo do provedor para salvar.",
                }
                .to_owned(),
                theme.colors.text_muted(),
            )),
            (Ok(_), true) if consent_active => Some((
                CONSENT_INVALIDATION.to_owned(),
                theme.colors.status_warning(),
            )),
            (Ok(_), true) => Some((
                "Alterações não salvas.".to_owned(),
                theme.colors.text_muted(),
            )),
            _ => None,
        };
        let fields = (self.kind != ProfileKind::Fake).then(|| self.render_fields(theme, cx));
        let footer = (edited || self.busy).then(|| {
            card_footer(theme)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .min_w(px(0.0))
                        .children(
                            hint.map(|(message, color)| div().text_color(color).child(message)),
                        ),
                )
                .child(save)
        });
        card(
            theme,
            "Extrator",
            "Quem lê as capturas para propor candidatos a decisão.",
        )
        .child(
            card_body()
                .child(
                    div()
                        .id("settings-kind")
                        .w_full()
                        .flex()
                        .flex_col()
                        .rounded(theme.radius.control())
                        .border_1()
                        .border_color(theme.colors.hairline_divider())
                        .overflow_hidden()
                        .role(Role::RadioGroup)
                        .aria_label("Extrator")
                        .child(local)
                        .child(remote)
                        .child(chatgpt)
                        .child(opencode)
                        .child(claude_code),
                )
                .children(fields),
        )
        .children(footer)
    }

    fn render_preview(&self, theme: &Theme, cx: &App) -> Div {
        let profile = self.form(cx).ok().or_else(|| {
            self.stored.clone().map(|mut profile| {
                profile.kind = self.kind;
                profile
            })
        });
        let body: AnyElement =
            match profile {
                Some(profile) if profile.kind != ProfileKind::Fake => {
                    let preview = build_preview(&profile);
                    let unset = || "Não configurado".to_owned();
                    let note = providers::destination_note(&profile);
                    let stats = [
                        (
                            "Destino",
                            providers::destination(&profile, &preview).unwrap_or_else(unset),
                        ),
                        (
                            "Modelo",
                            Some(preview.model.clone())
                                .filter(|model| !model.is_empty())
                                .unwrap_or_else(unset),
                        ),
                        (
                            "Por análise",
                            format!(
                                "≈ {} caracteres",
                                thousands(preview.total_approximate_chars)
                            ),
                        ),
                    ];
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S4))
                        .child(div().flex().gap(px(SpacingScale::S3)).children(
                            stats.into_iter().map(|(label, value)| {
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .p(px(SpacingScale::S3))
                                    .rounded(theme.radius.control())
                                    .bg(theme.colors.glass_fill_low())
                                    .border_1()
                                    .border_color(theme.colors.hairline_divider())
                                    .child(
                                        text_style(div(), TypeScale::META)
                                            .text_color(theme.colors.text_muted())
                                            .child(label.to_uppercase()),
                                    )
                                    .child(
                                        text_style(div(), TypeScale::ROW_TITLE)
                                            .truncate()
                                            .child(value),
                                    )
                            }),
                        ))
                        .child(
                            div().flex().flex_col().children(
                                preview
                                    .categories
                                    .iter()
                                    .enumerate()
                                    .map(|(index, category)| {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(SpacingScale::S3))
                                            .py(px(SpacingScale::S2))
                                            .when(index > 0, |row| {
                                                row.border_t_1()
                                                    .border_color(theme.colors.hairline_divider())
                                            })
                                            .child(icon(
                                                category_icon(&category.kind),
                                                14.0,
                                                theme.colors.text_muted(),
                                            ))
                                            .child(
                                                text_style(div(), TypeScale::BODY_SMALL)
                                                    .flex_1()
                                                    .child(category_label(&category.kind)),
                                            )
                                            .child(
                                                text_style(div(), TypeScale::BODY_SMALL)
                                                    .text_color(theme.colors.text_muted())
                                                    .child(format!(
                                                        "até {} por item",
                                                        thousands(category.max_chars_per_item)
                                                    )),
                                            )
                                    }),
                            ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S2))
                                .child(icon(
                                    IconName::CheckCircle,
                                    14.0,
                                    if preview.redaction_on_ingest {
                                        theme.colors.status_success()
                                    } else {
                                        theme.colors.status_warning()
                                    },
                                ))
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(theme.colors.text_secondary())
                                        .child(if preview.redaction_on_ingest {
                                            REDACTION_ON
                                        } else {
                                            "Sem redação de segredos na captura."
                                        }),
                                ),
                        )
                        .children(note.map(|(glyph, text)| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S2))
                                .child(icon(glyph, 14.0, theme.colors.text_muted()))
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(theme.colors.text_secondary())
                                        .child(text),
                                )
                        }))
                        .into_any_element()
                }
                _ => div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(icon(
                        IconName::CheckCircle,
                        14.0,
                        theme.colors.status_success(),
                    ))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(theme.colors.text_secondary())
                            .child(LOCAL_ONLY),
                    )
                    .into_any_element(),
            };
        card(
            theme,
            "O que sai da máquina",
            "Prévia exata do que o provedor pode receber. O consentimento fica ligado a ela.",
        )
        .child(card_body().child(body))
    }

    fn render_consent(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let Some(stored) = self.stored.clone() else {
            return div();
        };
        let active = consent_status(&stored).is_ok();
        let saved = !self.edited(cx) && stored.kind == self.kind;
        let ready = self.credential_ready(&stored);
        let mut body = card_body();
        if active {
            let granted = stored
                .consent
                .as_ref()
                .map(|consent| date_time(&consent.granted_at))
                .unwrap_or_default();
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .p(px(SpacingScale::S3))
                    .rounded(theme.radius.control())
                    .bg(tint(theme.colors.status_success(), 0.07))
                    .border_1()
                    .border_color(tint(theme.colors.status_success(), 0.25))
                    .child(icon(
                        IconName::CheckCircle,
                        16.0,
                        theme.colors.status_success(),
                    ))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(0.0))
                            .text_color(theme.colors.text_secondary())
                            .child(format!(
                                "Consentido em {granted} para a prévia acima. \
                                 Mudar a configuração exige consentir de novo."
                            )),
                    ),
            );
        } else {
            let steps = providers::consent_steps(self.kind, saved, ready, &stored);
            body =
                body.child(
                    div()
                        .id("settings-consent-steps")
                        .flex()
                        .flex_col()
                        .role(Role::List)
                        .children(steps.into_iter().enumerate().map(
                            |(index, (done, title, hint))| step(theme, index, done, title, hint),
                        )),
                );
        }
        let grant = (!active).then(|| {
            self.button(
                "settings-grant",
                ButtonKind::Primary,
                !self.busy && saved && ready,
                "Consentir e ativar",
                Action::Grant,
                cx,
            )
        });
        // Revoking an API-key profile also deletes the key; the other kinds
        // keep their credential (sign out and passwords live in their cards).
        let deletes_key = stored.kind == ProfileKind::OpenAiCompatible;
        let key_stored = deletes_key && self.kind == stored.kind && self.credentials.api_key;
        let revoke = ((active || key_stored) && !self.confirm_revoke).then(|| {
            self.button(
                "settings-revoke",
                ButtonKind::Ghost,
                !self.busy,
                if active {
                    "Revogar consentimento"
                } else {
                    "Apagar chave do cofre"
                },
                Action::AskRevoke,
                cx,
            )
        });
        let confirmation = self.confirm_revoke.then(|| {
            let cancel = self.button(
                "settings-revoke-cancel",
                ButtonKind::Ghost,
                true,
                "Cancelar",
                Action::CancelRevoke,
                cx,
            );
            let confirm = self.button(
                "settings-revoke-confirm",
                ButtonKind::Secondary,
                !self.busy,
                if deletes_key {
                    "Desligar e apagar chave"
                } else {
                    "Desligar chamadas externas"
                },
                Action::Revoke,
                cx,
            );
            div()
                .id("settings-revoke-confirmation")
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .p(px(SpacingScale::S3))
                .rounded(theme.radius.control())
                .bg(tint(theme.colors.status_danger(), 0.07))
                .border_1()
                .border_color(tint(theme.colors.status_danger(), 0.45))
                .role(Role::Alert)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_secondary())
                        .child(if deletes_key {
                            REVOKE_WARNING
                        } else {
                            REVOKE_ONLY_WARNING
                        }),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(SpacingScale::S2))
                        .child(cancel)
                        .child(confirm),
                )
        });
        card(
            theme,
            "Consentimento",
            "Nada é enviado antes deste passo, e você pode revogar a qualquer momento.",
        )
        .child(body.children(confirmation))
        .when(grant.is_some() || revoke.is_some(), |card| {
            card.child(
                card_footer(theme)
                    .child(div().flex_1().children(revoke))
                    .children(grant),
            )
        })
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let nav = self.render_nav(&theme, cx);
        let section = self.section;
        let panel: Option<AnyElement> = match section {
            SettingsSection::Ai => None,
            SettingsSection::OpenCode => self.opencode.clone().map(|p| p.into_any_element()),
            SettingsSection::Diagnostics => self.diagnostics.clone().map(|p| p.into_any_element()),
            SettingsSection::Appearance => Some(self.appearance.clone().into_any_element()),
        };
        let body: AnyElement = if let Some(panel) = panel {
            panel
        } else {
            match &self.load {
                Load::Loading => skeleton_list(&theme, "settings-skeleton", 4),
                Load::Failed(message) => {
                    let message = message.clone();
                    let retry = self.button(
                        "settings-retry",
                        ButtonKind::Secondary,
                        !self.busy,
                        "Tentar de novo",
                        Action::Retry,
                        cx,
                    );
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S3))
                        .id("settings-load-error")
                        .role(Role::Alert)
                        .child(
                            text_style(div(), TypeScale::HEADING_3)
                                .child("Não foi possível carregar as configurações"),
                        )
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(theme.colors.text_secondary())
                                .child(message),
                        )
                        .child(div().flex().child(retry))
                        .into_any_element()
                }
                Load::Ready => {
                    let stored = self.stored.clone().expect("ready implies a stored profile");
                    let external = self.kind != ProfileKind::Fake;
                    let status = self.render_status(&theme, &stored);
                    let extractor = self.render_extractor(&theme, cx);
                    let key = matches!(
                        self.kind,
                        ProfileKind::OpenAiCompatible | ProfileKind::OpenCode
                    )
                    .then(|| self.render_key(&theme, cx));
                    let account = match self.kind {
                        ProfileKind::ChatGptPlan => Some(self.render_account(&theme, cx)),
                        ProfileKind::ClaudeCode => Some(self.render_claude_code(&theme, cx)),
                        _ => None,
                    };
                    let preview = self.render_preview(&theme, cx);
                    let consent = external.then(|| self.render_consent(&theme, cx));
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S5))
                        .children(self.error.clone().map(|error| {
                            error_banner(&theme, &error)
                                .id("settings-error")
                                .role(Role::Alert)
                                .rounded(theme.radius.control())
                        }))
                        .child(status)
                        .child(extractor)
                        .children(account)
                        .children(key)
                        .child(preview)
                        .children(consent)
                        .into_any_element()
                }
            }
        };
        let notice = (section == SettingsSection::Ai)
            .then(|| self.notice.clone())
            .flatten()
            .map(|notice| toast(&theme, &notice, 24.0));
        div()
            .size_full()
            .relative()
            .flex()
            .child(nav)
            .children(notice)
            .child(
                div()
                    .id(("settings-content", section as usize))
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .overflow_y_scroll()
                    .px(px(40.0))
                    .pt(px(32.0))
                    .pb(px(64.0))
                    .child(
                        div()
                            .w_full()
                            .max_w(px(READING_WIDTH))
                            .mx_auto()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(SpacingScale::S1))
                                    .mb(px(SpacingScale::S6))
                                    .child(
                                        text_style(div(), TypeScale::HEADING_1)
                                            .child(section.title()),
                                    )
                                    .child(
                                        text_style(div(), TypeScale::BODY_SMALL)
                                            .text_color(theme.colors.text_muted())
                                            .child(section.subtitle()),
                                    ),
                            )
                            .child(body),
                    ),
            )
    }
}

fn category_icon(kind: &str) -> IconName {
    match kind {
        "diff_hunk" => IconName::File,
        "tool_summary" => IconName::Activity,
        _ => IconName::List,
    }
}

/// Product copy for a failure: validation messages are already sanitized
/// product text; storage and vault details go to the log only.
fn failure(error: ProfileError) -> String {
    match error {
        ProfileError::Invalid(message) => format!("{}.", capitalize(&message)),
        ProfileError::Io(detail) => {
            tracing::error!(
                error = %detail,
                operation = "ai_settings",
                "AI settings operation failed"
            );
            "Não foi possível acessar as configurações ou o cofre do sistema. Tente de novo.".into()
        }
    }
}

/// Actionable copy for [`consent_status`]'s sanitized reasons.
fn blocked_reason(reason: &str) -> String {
    match reason {
        "chamadas externas desativadas" | "consentimento ausente" => {
            "Falta consentir com a prévia abaixo.".into()
        }
        "a configuração mudou após o consentimento" => {
            "A configuração mudou depois do consentimento; consinta de novo.".into()
        }
        "configuração do provedor inválida" => {
            "A configuração do provedor está incompleta ou inválida.".into()
        }
        other => format!("{}.", capitalize(other)),
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

fn category_label(kind: &str) -> &'static str {
    match kind {
        "user_text" => "Mensagens que você escreveu",
        "assistant_text" => "Respostas do assistente",
        "diff_hunk" => "Trechos de diff",
        "tool_summary" => "Resumos de ferramentas",
        "document" => "Documentos do projeto",
        "document_metadata" => "Caminhos e metadados dos documentos",
        "decision_fields" => "Campos, escopo e qualificadores das decisões",
        "confirmed_examples" => "Exemplos confirmados na revisão",
        "rejected_examples" => "Exemplos rejeitados na revisão",
        "rules" => "Regras e seus qualificadores",
        "project_map" => "Itens e vínculos do mapa do projeto",
        "knowledge_review" => "Fontes e achados da revisão de conhecimento",
        "context_routing" => "Tarefa, arquivos e memória selecionada para avaliar relevância",
        _ => "Outro conteúdo",
    }
}

#[cfg(test)]
mod tests {
    use super::{capitalize, category_label};
    use application::profile::PREVIEW_CATEGORIES;

    #[test]
    fn every_preview_category_has_product_copy() {
        for kind in PREVIEW_CATEGORIES {
            assert_ne!(category_label(kind), "Outro conteúdo", "{kind}");
        }
    }

    #[test]
    fn maps_consent_reasons_to_actions() {
        use application::profile::{consent_status, offline_default_profile, ProfileKind};
        let mut profile = offline_default_profile();
        profile.kind = ProfileKind::OpenAiCompatible;
        profile.model = "m".into();
        profile.endpoint = Some("https://api.example.test/v1".into());
        let reason = consent_status(&profile).unwrap_err();
        assert_eq!(
            super::blocked_reason(reason),
            "Falta consentir com a prévia abaixo."
        );
        assert_eq!(super::blocked_reason("algo novo"), "Algo novo.");
    }

    #[test]
    fn capitalizes_backend_reasons() {
        assert_eq!(capitalize("consentimento ausente"), "Consentimento ausente");
        assert_eq!(capitalize("é"), "É");
        assert_eq!(capitalize(""), "");
    }
}
