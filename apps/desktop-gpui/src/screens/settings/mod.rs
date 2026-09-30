//! Application settings: how candidates are extracted and what leaves the machine.
//!
//! The page reads and writes only through [`AiBackend`], implemented by the
//! `AiSettings` use case. Every call touches the profile file or the OS key
//! vault, so it runs on the background executor. The stored key is never read
//! back into the view: the page only learns whether one exists.

use std::collections::BTreeMap;

use application::profile::{
    build_preview, choose_extractor, consent_status, revoke_consent, AiProfile, AiSettings,
    ConsentPreview, ExtractorChoice, ProfileError, ProfileKind, ProfileStore, SecretStore,
};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, App, Context, Div, Entity, EventEmitter, FocusHandle, Render, Role,
    SharedString, Stateful, Subscription, Toggled, Window,
};

use super::format::{date_time, thousands};
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    error_banner, mark_selected, skeleton_list, status_pill, toast, TOAST_DURATION,
};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{tint, SpacingScale, TypeScale};

/// Product copy too long to sit inside the element chains.
const REVOKE_WARNING: &str = "Isso desliga as chamadas externas e apaga a chave do cofre. \
                              O endereço e o modelo continuam salvos.";
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

/// The AI settings operations the page needs, object-safe so the shell stays
/// generic only over its storage type.
pub trait AiBackend: Send + 'static {
    /// Loads (or seeds) the profile and reports whether a key is stored.
    fn load(&self) -> Result<(AiProfile, bool), ProfileError>;
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
}

impl<S, T> AiBackend for AiSettings<S, T>
where
    S: ProfileStore + Send + 'static,
    T: SecretStore + Send + 'static,
{
    fn load(&self) -> Result<(AiProfile, bool), ProfileError> {
        let profile = self.load_or_seed()?;
        let has_secret = self.secret(&profile.id)?.is_some();
        Ok((profile, has_secret))
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
}

mod diagnostics;
mod opencode;
mod parts;

pub use diagnostics::{DiagnosticsBackend, DiagnosticsPanel, DiagnosticsService};
pub use opencode::{IntegrationBackend, IntegrationService, OpenCodePanel};

use parts::{card, card_body, card_footer, field_row, icon_tile, step};

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
    Save,
    StoreKey,
    Grant,
    AskRevoke,
    CancelRevoke,
    Revoke,
    Retry,
}

type Operation = Box<dyn FnOnce(&dyn AiBackend) -> Result<(AiProfile, bool), ProfileError> + Send>;

/// Settings page with the "IA e privacidade" section.
/// Everything the settings page reads and writes, built by the composition root.
pub struct SettingsServices {
    /// AI profile, key vault and consent.
    pub ai: Box<dyn AiBackend>,
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
}

impl SettingsSection {
    fn id(self) -> &'static str {
        match self {
            Self::Ai => "settings-section-ai",
            Self::OpenCode => "settings-section-opencode",
            Self::Diagnostics => "settings-section-diagnostics",
        }
    }
    fn title(self) -> &'static str {
        match self {
            Self::Ai => "IA e privacidade",
            Self::OpenCode => "OpenCode",
            Self::Diagnostics => "Diagnóstico",
        }
    }
    fn hint(self) -> &'static str {
        match self {
            Self::Ai => "Extração, chave e envio",
            Self::OpenCode => "Captura e conexão",
            Self::Diagnostics => "Saúde, perdas e tarefas",
        }
    }
    fn subtitle(self) -> &'static str {
        match self {
            Self::Ai => PAGE_SUBTITLE,
            Self::OpenCode => "Se as capturas do OpenCode estão chegando e, se não, por quê.",
            Self::Diagnostics => "Como o pipeline está indo e o que se perdeu no caminho.",
        }
    }
    fn glyph(self) -> IconName {
        match self {
            Self::Ai => IconName::Shield,
            Self::OpenCode => IconName::Link,
            Self::Diagnostics => IconName::Activity,
        }
    }
}

/// Settings page: AI and privacy, OpenCode and diagnostics.
pub struct SettingsScreen {
    section: SettingsSection,
    opencode: Option<Entity<OpenCodePanel>>,
    diagnostics: Option<Entity<DiagnosticsPanel>>,
    backend: Option<Box<dyn AiBackend>>,
    busy: bool,
    load: Load,
    stored: Option<AiProfile>,
    has_secret: bool,
    kind: ProfileKind,
    /// Endpoint, model and per-item limit as seeded, to detect edits.
    original: [String; 3],
    fields: [Entity<SearchField>; 3],
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
    pub fn new(
        cx: &mut Context<Self>,
        backend: Box<dyn AiBackend>,
        integration: Option<Box<dyn IntegrationBackend>>,
        diagnostics: Option<Box<dyn DiagnosticsBackend>>,
    ) -> Self {
        let opencode = integration.map(|backend| cx.new(|_| OpenCodePanel::new(backend)));
        let diagnostics = diagnostics.map(|backend| cx.new(|_| DiagnosticsPanel::new(backend)));
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
            backend: Some(backend),
            busy: false,
            load: Load::Loading,
            stored: None,
            has_secret: false,
            kind: ProfileKind::Fake,
            original: Default::default(),
            fields,
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

    fn seed(&mut self, profile: AiProfile, has_secret: bool, cx: &mut Context<Self>) {
        // The offline default carries a placeholder model name; it is not a
        // provider setting, so the external form starts empty.
        let configured = profile.endpoint.is_some();
        self.original = [
            profile.endpoint.clone().unwrap_or_default(),
            if configured {
                profile.model.clone()
            } else {
                String::new()
            },
            profile.max_input_chars.to_string(),
        ];
        for (field, value) in self.fields.iter().zip(&self.original) {
            field.update(cx, |field, cx| field.set_value(value, cx));
        }
        self.kind = profile.kind;
        self.has_secret = has_secret;
        self.stored = Some(profile);
        self.load = Load::Ready;
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
            || (self.kind == ProfileKind::OpenAiCompatible
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
        if self.kind == ProfileKind::OpenAiCompatible {
            let endpoint = self.value(0, cx);
            profile.endpoint = (!endpoint.is_empty()).then_some(endpoint);
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
        (0..3).any(|index| self.value(index, cx) != self.original[index])
    }

    fn run(&mut self, operation: Operation, notice: Option<&'static str>, cx: &mut Context<Self>) {
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
                    Ok((profile, has_secret)) => {
                        this.seed(profile, has_secret, cx);
                        if let Some(notice) = notice {
                            this.show_notice(notice.to_owned(), cx);
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
            Action::Kind(kind) => {
                self.kind = kind;
                self.error = None;
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
                let account = stored.id.clone();
                let key = key.to_owned();
                self.key.update(cx, |field, cx| {
                    field.set_context("Chave de API do provedor", cx)
                });
                self.run(
                    Box::new(move |backend| {
                        backend.set_secret(&account, &key)?;
                        backend.load()
                    }),
                    Some("Chave guardada no cofre do sistema."),
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
                self.run(
                    Box::new(move |backend| {
                        backend.revoke(&stored)?;
                        backend.load()
                    }),
                    Some("Chamadas externas desligadas e chave apagada do cofre."),
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
                .child(icon_tile(
                    theme,
                    section.glyph(),
                    if selected {
                        colors.accent_hover()
                    } else {
                        colors.text_secondary()
                    },
                    28.0,
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE)
                                .text_color(if selected {
                                    colors.text_primary()
                                } else {
                                    colors.text_secondary()
                                })
                                .child(section.title()),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_muted())
                                .child(section.hint()),
                        ),
                )
            }))
    }

    fn render_status(&self, theme: &Theme, stored: &AiProfile) -> Stateful<Div> {
        let (color, glyph, pill, title, body): (_, _, _, &str, String) = match choose_extractor(
            Some(stored),
        ) {
            ExtractorChoice::OfflineFake => (
                theme.colors.status_info(),
                IconName::Cpu,
                "Local",
                "Extração local, sem rede",
                "Candidatos são extraídos nesta máquina. Nenhum conteúdo das capturas é enviado."
                    .into(),
            ),
            ExtractorChoice::ExternalEnabled => (
                theme.colors.status_success(),
                IconName::CheckCircle,
                "Ativo",
                "Provedor externo ativo",
                format!(
                    "Capturas são analisadas por {}, dentro dos limites da prévia.",
                    build_preview(stored)
                        .endpoint_host
                        .unwrap_or_else(|| "provedor configurado".into())
                ),
            ),
            ExtractorChoice::ExternalBlocked => (
                theme.colors.status_warning(),
                IconName::Shield,
                "Bloqueado",
                "Provedor externo bloqueado",
                format!(
                    "{} Até lá, nenhuma captura é enviada nem analisada.",
                    blocked_reason(consent_status(stored).err().unwrap_or_default())
                ),
            ),
        };
        div()
            .id("settings-status")
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .p(px(SpacingScale::S4))
            .rounded(px(10.0))
            .border_1()
            .border_color(tint(color, 0.28))
            .bg(tint(color, 0.07))
            .role(Role::Status)
            .aria_label(format!("{title}. {body}"))
            .child(icon_tile(theme, glyph, color, 40.0))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(text_style(div(), TypeScale::HEADING_3).child(title))
                            .child(status_pill(theme, color, pill)),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(theme.colors.text_secondary())
                            .child(body),
                    ),
            )
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
        };
        let focus = self
            .focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let colors = theme.colors;
        div()
            .id(id)
            .relative()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .p(px(SpacingScale::S4))
            .rounded(px(10.0))
            .border_1()
            .border_color(if selected {
                colors.accent_default()
            } else {
                colors.glass_border_card()
            })
            .bg(if selected {
                colors.selection()
            } else {
                colors.glass_fill_card()
            })
            .when(!selected, |option| {
                option
                    .hover(move |style| {
                        style
                            .bg(colors.glass_fill_medium())
                            .border_color(colors.glass_border_card_hover())
                    })
                    .active(move |style| style.bg(colors.glass_fill_strong()))
            })
            .cursor_pointer()
            .role(Role::RadioButton)
            .aria_label(title)
            .aria_toggled(if selected {
                Toggled::True
            } else {
                Toggled::False
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
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(icon_tile(
                        &theme,
                        glyph,
                        if selected {
                            colors.accent_hover()
                        } else {
                            colors.text_secondary()
                        },
                        32.0,
                    ))
                    .child(
                        div()
                            .size(px(18.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .border_1()
                            .border_color(if selected {
                                colors.accent_hover()
                            } else {
                                colors.glass_border_control()
                            })
                            .when(selected, |mark| {
                                mark.bg(colors.accent_hover()).child(icon(
                                    IconName::Check,
                                    12.0,
                                    colors.accent_on_emphasis(),
                                ))
                            }),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(text_style(div(), TypeScale::ROW_TITLE).child(title))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_muted())
                            .child(body),
                    ),
            )
    }

    fn render_extractor(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let edited = self.edited(cx);
        let draft = self.draft(cx);
        let consent_active = self
            .stored
            .as_ref()
            .is_some_and(|stored| consent_status(stored).is_ok());
        let external = self.kind == ProfileKind::OpenAiCompatible;
        let local = self.kind_option(
            ProfileKind::Fake,
            "Local",
            "Extrator offline. Nada sai desta máquina.",
            cx,
        );
        let remote = self.kind_option(
            ProfileKind::OpenAiCompatible,
            "Compatível com OpenAI",
            "Envia trechos ao endereço configurado, só com consentimento.",
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
                "Informe o endereço e o modelo do provedor para salvar.".to_owned(),
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
        let fields = external.then(|| {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S4))
                .pt(px(SpacingScale::S2))
                .child(field_row(
                    theme,
                    FIELD_LABELS[0],
                    self.fields[0].clone().into_any_element(),
                ))
                .child(
                    div()
                        .flex()
                        .gap(px(SpacingScale::S4))
                        .child(div().flex_1().min_w(px(0.0)).child(field_row(
                            theme,
                            FIELD_LABELS[1],
                            self.fields[1].clone().into_any_element(),
                        )))
                        .child(div().w(px(220.0)).flex_none().child(field_row(
                            theme,
                            FIELD_LABELS[2],
                            self.fields[2].clone().into_any_element(),
                        ))),
                )
        });
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
            IconName::Cpu,
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
                        .gap(px(SpacingScale::S3))
                        .role(Role::RadioGroup)
                        .aria_label("Extrator")
                        .child(local)
                        .child(remote),
                )
                .children(fields),
        )
        .children(footer)
    }

    fn render_key(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let typed = !self.key.read(cx).value().trim().is_empty();
        let store = self.button(
            "settings-store-key",
            ButtonKind::Secondary,
            !self.busy && typed,
            if self.has_secret {
                "Substituir chave"
            } else {
                "Guardar no cofre"
            },
            Action::StoreKey,
            cx,
        );
        let (color, state) = if self.has_secret {
            (theme.colors.status_success(), "Guardada no cofre")
        } else {
            (theme.colors.text_muted(), "Nenhuma chave")
        };
        card(theme, IconName::Key, "Chave do provedor", KEY_DESCRIPTION).child(
            card_body()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(status_pill(theme, color, state)),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(div().flex_1().child(self.key.clone()))
                        .child(store),
                ),
        )
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
                Some(profile) if profile.kind == ProfileKind::OpenAiCompatible => {
                    let preview = build_preview(&profile);
                    let unset = || "Não configurado".to_owned();
                    let stats = [
                        (
                            "Destino",
                            preview.endpoint_host.clone().unwrap_or_else(unset),
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
            IconName::Eye,
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
        let saved = !self.edited(cx) && stored.kind == ProfileKind::OpenAiCompatible;
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
            let steps = [
                (saved, "Configuração salva", "Endereço, modelo e limite"),
                (self.has_secret, "Chave no cofre", "Guardada no sistema"),
                (false, "Consentimento", "Liga as chamadas externas"),
            ];
            body =
                body.child(
                    div()
                        .id("settings-consent-steps")
                        .flex()
                        .items_start()
                        .role(Role::List)
                        .children(steps.into_iter().enumerate().map(
                            |(index, (done, title, hint))| {
                                step(theme, index, done, title, hint, index + 1 < 3)
                            },
                        )),
                );
        }
        let grant = (!active).then(|| {
            self.button(
                "settings-grant",
                ButtonKind::Primary,
                !self.busy && saved && self.has_secret,
                "Consentir e ativar",
                Action::Grant,
                cx,
            )
        });
        let revoke = ((active || self.has_secret) && !self.confirm_revoke).then(|| {
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
                "Desligar e apagar chave",
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
                        .child(REVOKE_WARNING),
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
            IconName::Shield,
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
                    let external = self.kind == ProfileKind::OpenAiCompatible;
                    let status = self.render_status(&theme, &stored);
                    let extractor = self.render_extractor(&theme, cx);
                    let key = external.then(|| self.render_key(&theme, cx));
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
                                    .items_center()
                                    .gap(px(SpacingScale::S4))
                                    .mb(px(28.0))
                                    .child(icon_tile(
                                        &theme,
                                        section.glyph(),
                                        theme.colors.accent_hover(),
                                        48.0,
                                    ))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap(px(2.0))
                                            .child(
                                                text_style(div(), TypeScale::HEADING_1)
                                                    .child(section.title()),
                                            )
                                            .child(
                                                text_style(div(), TypeScale::BODY_SMALL)
                                                    .text_color(theme.colors.text_muted())
                                                    .child(section.subtitle()),
                                            ),
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
