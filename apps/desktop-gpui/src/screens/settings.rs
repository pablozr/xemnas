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

use super::format::date_time;
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{mark_selected, section_label, status_pill};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{ControlSize, SpacingScale, TypeScale};

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
pub struct SettingsScreen {
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

const FIELD_LABELS: [&str; 3] = [
    "Endereço do provedor",
    "Modelo",
    "Limite de caracteres por item",
];

impl SettingsScreen {
    /// Mounts the page; nothing is read until [`Self::open`].
    pub fn new(cx: &mut Context<Self>, backend: Box<dyn AiBackend>) -> Self {
        let fields = std::array::from_fn(|index| {
            cx.new(|cx| {
                let mut field = SearchField::new(cx);
                field.stretch();
                field.set_context(FIELD_LABELS[index], cx);
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
        profile.validate().map_err(failure)?;
        Ok(profile)
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
                        this.notice = notice.map(str::to_owned);
                    }
                    Err(error) if this.stored.is_none() => this.load = Load::Failed(failure(error)),
                    Err(error) => this.error = Some(failure(error)),
                }
                cx.notify();
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
        let theme = Theme::current(cx);
        let focus = self
            .focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        let label = label.into();
        action_button(&theme, id, kind, enabled)
            .aria_label(label.clone())
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
            .child(label)
    }

    fn render_nav(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let back = self
            .button(
                "settings-close",
                ButtonKind::Ghost,
                true,
                "Projetos",
                Action::Close,
                cx,
            )
            .child(icon(
                IconName::ArrowLeft,
                14.0,
                button_foreground(theme, ButtonKind::Ghost, true),
            ));
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
            .child(div().mb(px(SpacingScale::S3)).child(back))
            .child(
                text_style(div(), TypeScale::LABEL)
                    .px(px(SpacingScale::S2))
                    .pb(px(SpacingScale::S2))
                    .text_color(theme.colors.text_muted())
                    .child("Configurações"),
            )
            .child(
                mark_selected(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .id("settings-section-ai")
                        .relative()
                        .h(px(ControlSize::SM))
                        .px(px(SpacingScale::S2))
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .rounded(theme.radius.control())
                        .role(Role::Tab)
                        .aria_selected(true)
                        .text_color(theme.colors.text_primary()),
                    theme,
                    true,
                )
                .child(icon(IconName::Shield, 14.0, theme.colors.text_primary()))
                .child("IA e privacidade"),
            )
    }

    fn render_status(&self, theme: &Theme, stored: &AiProfile) -> Stateful<Div> {
        let (color, pill, title, body): (_, _, &str, String) = match choose_extractor(Some(stored))
        {
            ExtractorChoice::OfflineFake => (
                theme.colors.status_info(),
                "Local",
                "Extração local, sem rede",
                "Candidatos são extraídos nesta máquina. Nenhum conteúdo das capturas é enviado."
                    .into(),
            ),
            ExtractorChoice::ExternalEnabled => (
                theme.colors.status_success(),
                "Ativo",
                "Provedor externo ativo",
                format!(
                    "Capturas são analisadas por {}, dentro dos limites da prévia abaixo.",
                    build_preview(stored)
                        .endpoint_host
                        .unwrap_or_else(|| "provedor configurado".into())
                ),
            ),
            ExtractorChoice::ExternalBlocked => (
                theme.colors.status_warning(),
                "Bloqueado",
                "Provedor externo bloqueado",
                format!(
                    "{}. Até resolver, nenhuma captura é enviada nem analisada.",
                    capitalize(
                        consent_status(stored)
                            .err()
                            .unwrap_or("consentimento ausente")
                    )
                ),
            ),
        };
        div()
            .id("settings-status")
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .p(px(SpacingScale::S4))
            .rounded(theme.radius.control())
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .role(Role::Status)
            .aria_label(format!("{title}. {body}"))
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
        let id = match kind {
            ProfileKind::Fake => "settings-kind-local",
            ProfileKind::OpenAiCompatible => "settings-kind-external",
        };
        let focus = self
            .focus
            .entry(id)
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone();
        div()
            .id(id)
            .flex_1()
            .flex()
            .gap(px(SpacingScale::S3))
            .p(px(SpacingScale::S3))
            .rounded(theme.radius.control())
            .border_1()
            .border_color(if selected {
                theme.colors.glass_edge_lavender()
            } else {
                theme.colors.hairline_divider()
            })
            .when(selected, |option| option.bg(theme.colors.selection()))
            .when(!selected, |option| {
                option.hover(move |style| style.bg(theme.colors.hover_veil()))
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
                    .mt(px(3.0))
                    .size(px(12.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .border_1()
                    .border_color(if selected {
                        theme.colors.accent_default()
                    } else {
                        theme.colors.glass_border_control()
                    })
                    .when(selected, |dot| {
                        dot.child(
                            div()
                                .size(px(6.0))
                                .rounded_full()
                                .bg(theme.colors.accent_default()),
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(text_style(div(), TypeScale::ROW_TITLE).child(title))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(theme.colors.text_muted())
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
            "Envia trechos das capturas ao endereço configurado, só com consentimento.",
            cx,
        );
        let save = self.button(
            "settings-save",
            ButtonKind::Secondary,
            !self.busy && edited && draft.is_ok(),
            if self.busy {
                "Salvando…"
            } else {
                "Salvar configuração"
            },
            Action::Save,
            cx,
        );
        let hint = match (&draft, edited) {
            (Err(message), true) => Some((message.clone(), true)),
            (Ok(_), true) if consent_active => Some((
                "Salvar muda a prévia: o consentimento atual deixa de valer e as chamadas externas param até você consentir de novo."
                    .to_owned(),
                false,
            )),
            _ => None,
        };
        section(theme, "Extrator")
            .child(
                div()
                    .id("settings-kind")
                    .flex()
                    .gap(px(SpacingScale::S2))
                    .role(Role::RadioGroup)
                    .aria_label("Extrator")
                    .child(local)
                    .child(remote),
            )
            .when(external, |section| {
                section.children(FIELD_LABELS.iter().enumerate().map(|(index, label)| {
                    field_row(theme, label, self.fields[index].clone().into_any_element())
                }))
            })
            .children(hint.map(|(message, danger)| {
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(if danger {
                        theme.colors.status_danger()
                    } else {
                        theme.colors.status_warning()
                    })
                    .child(message)
            }))
            .child(div().flex().child(save))
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
        section(theme, "Chave do provedor")
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_secondary())
                    .child(if self.has_secret {
                        "Há uma chave guardada no cofre de credenciais do sistema. Ela não é exibida aqui."
                    } else {
                        "Nenhuma chave guardada. A chave vai para o cofre de credenciais do sistema, nunca para o arquivo de configuração."
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(div().flex_1().child(self.key.clone()))
                    .child(store),
            )
    }

    fn render_preview(&self, theme: &Theme, cx: &App) -> Div {
        let profile = self.draft(cx).ok().or_else(|| self.stored.clone());
        let body: AnyElement = match profile {
            Some(profile) if profile.kind == ProfileKind::OpenAiCompatible => {
                let preview = build_preview(&profile);
                let mut rows = vec![
                    (
                        "Destino",
                        preview
                            .endpoint_host
                            .clone()
                            .unwrap_or_else(|| "Não configurado".into()),
                    ),
                    (
                        "Modelo",
                        if preview.model.is_empty() {
                            "Não configurado".into()
                        } else {
                            preview.model.clone()
                        },
                    ),
                ];
                rows.extend(preview.categories.iter().map(|category| {
                    (
                        category_label(&category.kind),
                        format!(
                            "até {} caracteres por item",
                            thousands(category.max_chars_per_item)
                        ),
                    )
                }));
                rows.push((
                    "Total aproximado",
                    format!(
                        "{} caracteres por análise",
                        thousands(preview.total_approximate_chars)
                    ),
                ));
                rows.push((
                    "Segredos",
                    if preview.redaction_on_ingest {
                        "Redigidos na captura, antes de qualquer envio".into()
                    } else {
                        "Sem redação na captura".into()
                    },
                ));
                div()
                    .flex()
                    .flex_col()
                    .rounded(theme.radius.control())
                    .border_1()
                    .border_color(theme.colors.hairline_divider())
                    .children(rows.into_iter().enumerate().map(|(index, (label, value))| {
                        div()
                            .flex()
                            .gap(px(SpacingScale::S4))
                            .px(px(SpacingScale::S3))
                            .py(px(SpacingScale::S2))
                            .when(index > 0, |row| {
                                row.border_t_1()
                                    .border_color(theme.colors.hairline_divider())
                            })
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .w(px(220.0))
                                    .flex_none()
                                    .text_color(theme.colors.text_muted())
                                    .child(label),
                            )
                            .child(text_style(div(), TypeScale::BODY_SMALL).child(value))
                    }))
                    .into_any_element()
            }
            _ => text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_secondary())
                .child("Com o extrator local, nenhum conteúdo das capturas sai desta máquina.")
                .into_any_element(),
        };
        section(theme, "O que sai da máquina").child(body)
    }

    fn render_consent(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let Some(stored) = self.stored.clone() else {
            return div();
        };
        let active = consent_status(&stored).is_ok();
        let saved = !self.edited(cx) && stored.kind == ProfileKind::OpenAiCompatible;
        let mut content = section(theme, "Consentimento");
        if active {
            let granted = stored
                .consent
                .as_ref()
                .map(|consent| date_time(&consent.granted_at))
                .unwrap_or_default();
            content = content.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_secondary())
                    .child(format!(
                        "Consentido em {granted} para a prévia acima. Qualquer mudança na configuração exige consentir de novo."
                    )),
            );
        } else {
            content = content
                .child(requirement(theme, saved, "Configuração do provedor salva"))
                .child(requirement(
                    theme,
                    self.has_secret,
                    "Chave guardada no cofre",
                ));
            let grant = self.button(
                "settings-grant",
                ButtonKind::Primary,
                !self.busy && saved && self.has_secret,
                "Consentir e ativar",
                Action::Grant,
                cx,
            );
            content = content.child(div().flex().child(grant));
        }
        if active || self.has_secret {
            content = if self.confirm_revoke {
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
                content.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .p(px(SpacingScale::S3))
                        .rounded(theme.radius.control())
                        .border_1()
                        .border_color(theme.colors.status_danger())
                        .id("settings-revoke-confirmation")
                        .role(Role::Alert)
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .text_color(theme.colors.text_secondary())
                                .child("Isso desliga as chamadas externas e apaga a chave do cofre. O endereço e o modelo continuam salvos."),
                        )
                        .child(cancel)
                        .child(confirm),
                )
            } else {
                let revoke = self.button(
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
                );
                content.child(div().flex().child(revoke))
            };
        }
        content
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let nav = self.render_nav(&theme, cx);
        let body: AnyElement = match &self.load {
            Load::Loading => text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child("Carregando configurações…")
                .into_any_element(),
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
                    .gap(px(28.0))
                    .child(status)
                    .children(self.notice.clone().map(|notice| {
                        text_style(div(), TypeScale::BODY_SMALL)
                            .id("settings-notice")
                            .role(Role::Status)
                            .text_color(theme.colors.status_success())
                            .child(notice)
                    }))
                    .children(self.error.clone().map(|error| {
                        text_style(div(), TypeScale::BODY_SMALL)
                            .id("settings-error")
                            .role(Role::Alert)
                            .text_color(theme.colors.status_danger())
                            .child(error)
                    }))
                    .child(extractor)
                    .children(key)
                    .child(preview)
                    .children(consent)
                    .into_any_element()
            }
        };
        div()
            .size_full()
            .flex()
            .child(nav)
            .child(
                div()
                    .id("settings-content")
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .overflow_y_scroll()
                    .px(px(32.0))
                    .py(px(28.0))
                    .child(
                        div()
                            .max_w(px(READING_WIDTH))
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S2))
                            .child(text_style(div(), TypeScale::HEADING_1).child("IA e privacidade"))
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .mb(px(SpacingScale::S5))
                                    .text_color(theme.colors.text_muted())
                                    .child("Como os candidatos a decisão são extraídos das capturas e o que pode sair desta máquina."),
                            )
                            .child(body),
                    ),
            )
    }
}

fn section(theme: &Theme, label: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S3))
        .child(section_label(theme, label))
}

fn field_row(theme: &Theme, label: &str, field: AnyElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            text_style(div(), TypeScale::LABEL)
                .text_color(theme.colors.text_secondary())
                .child(label.to_owned()),
        )
        .child(field)
}

fn requirement(theme: &Theme, met: bool, label: &'static str) -> Stateful<Div> {
    let color = if met {
        theme.colors.status_success()
    } else {
        theme.colors.text_muted()
    };
    text_style(div(), TypeScale::BODY_SMALL)
        .id(label)
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .text_color(if met {
            theme.colors.text_primary()
        } else {
            theme.colors.text_secondary()
        })
        .aria_label(format!(
            "{label}: {}",
            if met { "concluído" } else { "pendente" }
        ))
        .child(icon(
            if met {
                IconName::CheckCircle
            } else {
                IconName::Circle
            },
            14.0,
            color,
        ))
        .child(label)
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

/// `8192` → `8.192`.
fn thousands(value: usize) -> String {
    let digits = value.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push('.');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{capitalize, category_label, thousands};
    use application::profile::PREVIEW_CATEGORIES;

    #[test]
    fn formats_counts_with_thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(8_192), "8.192");
        assert_eq!(thousands(32_768), "32.768");
        assert_eq!(thousands(1_000_000), "1.000.000");
    }

    #[test]
    fn every_preview_category_has_product_copy() {
        for kind in PREVIEW_CATEGORIES {
            assert_ne!(category_label(kind), "Outro conteúdo", "{kind}");
        }
    }

    #[test]
    fn capitalizes_backend_reasons() {
        assert_eq!(capitalize("consentimento ausente"), "Consentimento ausente");
        assert_eq!(capitalize("é"), "É");
        assert_eq!(capitalize(""), "");
    }
}
