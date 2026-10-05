//! The provider-specific parts of "IA e privacidade" (ADR-0004): the provider
//! form with its model picker, the credential card (API key or OpenCode
//! password), the ChatGPT account card with its browser sign-in and the
//! Claude Code card.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use application::profile::{
    build_preview, is_loopback_endpoint, AiProfile, ConsentPreview, ProfileKind, SignedIn,
    OPENCODE_GO_ENDPOINT,
};
use application::providers::{ProviderError, CLAUDE_CODE_LOGIN_COMMAND};
use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Div, Role, Toggled};

use super::parts::{card, card_body, card_footer, field_row};
use super::{
    as_kind, Action, ClaudeCheck, Models, SettingsScreen, SignIn, FIELD_LABELS, KEY_DESCRIPTION,
};
use crate::ui::controls::{button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{mark_selected, skeleton_list, status_pill};
use crate::ui::theme::{code_style, text_style, Theme};
use crate::ui::tokens::{tint, SpacingScale, TypeScale};

/// Local model servers the OpenAI-compatible form can fill in one click.
pub(super) const LOCAL_PRESETS: [(&str, &str); 2] = [
    ("Ollama", "http://127.0.0.1:11434/v1"),
    ("LM Studio", "http://127.0.0.1:1234/v1"),
];
const PRESET_IDS: [&str; 2] = ["settings-preset-ollama", "settings-preset-lmstudio"];

/// Where the user creates an OpenCode API key (Zen and Go share it).
pub(super) const OPENCODE_KEYS_URL: &str = "https://opencode.ai/auth";

/// The two OpenCode plans: (is Go, title, one-line description).
const OPENCODE_PLANS: [(bool, &str, &str); 2] = [
    (false, "Zen", "Paga por uso; inclui modelos gratuitos."),
    (true, "Go", "Assinatura mensal de modelos abertos."),
];

/// How long the browser sign-in may take before it gives up.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

const ACCOUNT_DESCRIPTION: &str = "Entre com a sua conta para extrair com o seu plano do \
                                   ChatGPT, sem chave de API. Só um token de renovação fica \
                                   no cofre do sistema.";
const PLAN_NOTICE: &str = "As análises da xemnas contam no uso do seu plano, como conversas \
                           no ChatGPT. Acompanhe em Gerenciar uso.";
const OPENCODE_KEY_DESCRIPTION: &str = "A mesma chave serve para o Zen e o Go. Fica no \
                                        Gerenciador de Credenciais do sistema e não é \
                                        exibida aqui.";
const CLAUDE_INSTALL: &str = "Instale o Claude Code e entre nele num terminal. Fora do PATH, \
                              aponte XEMNAS_CLAUDE_CODE para o executável.";
const CLAUDE_LOGIN: &str = "Entre no Claude Code num terminal com o comando abaixo e verifique \
                            de novo. A senha nunca passa pela xemnas.";
const CLAUDE_EXPERIMENTAL: &str = "Experimental. Cada análise conta no uso do seu plano, junto \
                                   com o próprio Claude Code.";
const WAITING_BROWSER: &str =
    "Conclua o login no navegador. Esta tela atualiza sozinha quando você voltar.";

/// Who analyses the captures of an active profile, for the status card.
pub(super) fn analysed_by(profile: &AiProfile) -> String {
    match profile.kind {
        ProfileKind::Fake => "nesta máquina".to_owned(),
        ProfileKind::OpenAiCompatible => format!(
            "por {}",
            build_preview(profile)
                .endpoint_host
                .unwrap_or_else(|| "provedor configurado".into())
        ),
        ProfileKind::ChatGptPlan => format!("pelo seu plano do ChatGPT ({})", profile.model),
        ProfileKind::OpenCode => format!("pelo {} com {}", opencode_plan(profile), profile.model),
        ProfileKind::ClaudeCode => format!("pelo Claude Code desta máquina ({})", profile.model),
    }
}

/// The destination the preview names: the host, the ChatGPT account or the
/// provider OpenCode forwards to.
pub(super) fn destination(profile: &AiProfile, preview: &ConsentPreview) -> Option<String> {
    match profile.kind {
        ProfileKind::ChatGptPlan => Some(
            match profile
                .chatgpt
                .as_ref()
                .and_then(|account| account.email.as_ref())
            {
                Some(email) => format!("OpenAI · {email}"),
                None => "OpenAI".to_owned(),
            },
        ),
        ProfileKind::OpenCode => Some(opencode_plan(profile).to_owned()),
        ProfileKind::ClaudeCode => Some("Anthropic, pelo Claude Code".to_owned()),
        _ => preview.endpoint_host.clone(),
    }
}

/// One more line for the preview when the destination needs explaining.
pub(super) fn destination_note(profile: &AiProfile) -> Option<(IconName, &'static str)> {
    match profile.kind {
        ProfileKind::OpenAiCompatible
            if profile
                .endpoint
                .as_deref()
                .is_some_and(is_loopback_endpoint) =>
        {
            Some((
                IconName::Cpu,
                "Endereço local: os trechos não saem desta máquina.",
            ))
        }
        ProfileKind::ChatGptPlan => Some((
            IconName::User,
            "Cada análise conta no uso do seu plano do ChatGPT.",
        )),
        ProfileKind::OpenCode => Some((
            IconName::Link,
            "O OpenCode repassa os trechos ao provedor do modelo escolhido.",
        )),
        ProfileKind::ClaudeCode => Some((
            IconName::Terminal,
            "O Claude Code desta máquina envia os trechos com o login dele.",
        )),
        _ => None,
    }
}

/// "OpenCode Zen" or "OpenCode Go", from the profile's gateway address.
fn opencode_plan(profile: &AiProfile) -> &'static str {
    if profile.endpoint.as_deref() == Some(OPENCODE_GO_ENDPOINT) {
        "OpenCode Go"
    } else {
        "OpenCode Zen"
    }
}

/// The steps before consent, per kind: saved configuration, the kind's
/// credential when it has one, then consent itself.
pub(super) fn consent_steps(
    kind: ProfileKind,
    saved: bool,
    ready: bool,
    stored: &AiProfile,
) -> Vec<(bool, &'static str, &'static str)> {
    let consent = (false, "Consentimento", "Liga as chamadas externas");
    match kind {
        ProfileKind::Fake => Vec::new(),
        ProfileKind::OpenAiCompatible => {
            let key = if saved && !stored.credential_required() {
                (true, "Sem chave", "Endereço local")
            } else {
                (ready, "Chave no cofre", "Guardada no sistema")
            };
            vec![
                (saved, "Configuração salva", "Endereço, modelo e limite"),
                key,
                consent,
            ]
        }
        ProfileKind::ChatGptPlan => vec![
            (saved, "Modelo salvo", "Modelo e limite"),
            (ready, "Conta conectada", "Uso do plano permitido"),
            consent,
        ],
        ProfileKind::OpenCode => vec![
            (saved, "Configuração salva", "Plano e modelo"),
            (ready, "Chave no cofre", "Guardada no sistema"),
            consent,
        ],
        ProfileKind::ClaudeCode => vec![
            (saved, "Modelo salvo", "Modelo e limite"),
            (ready, "Claude Code conectado", "Login feito no terminal"),
            consent,
        ],
    }
}

impl SettingsScreen {
    /// Whether the credential the form's kind needs is in place.
    pub(super) fn credential_ready(&self, stored: &AiProfile) -> bool {
        match self.kind {
            ProfileKind::Fake => false,
            ProfileKind::OpenAiCompatible => {
                self.credentials.api_key || !as_kind(stored, self.kind).credential_required()
            }
            ProfileKind::ChatGptPlan => {
                self.credentials.chatgpt
                    && stored
                        .chatgpt
                        .as_ref()
                        .is_some_and(|account| account.plan_usage)
            }
            ProfileKind::OpenCode => self.credentials.opencode,
            // Claude Code keeps its own login; xemnas only checks it.
            ProfileKind::ClaudeCode => {
                matches!(&self.claude, ClaudeCheck::Done(Ok(status)) if status.signed_in)
            }
        }
    }

    /// Runs `operation` and puts back the provider the user was editing,
    /// so storing a credential does not discard an unsaved form.
    pub(super) fn keeping_form(
        &mut self,
        operation: super::Operation,
        notice: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        let kind = self.kind;
        let edited = self.edited(cx);
        let values: [String; 3] = std::array::from_fn(|index| self.value(index, cx));
        self.run_then(
            operation,
            notice,
            Some(Box::new(move |this, cx| {
                if edited {
                    this.switch_kind(kind, cx);
                    for (field, value) in this.fields.iter().zip(&values) {
                        field.update(cx, |field, cx| field.set_value(value, cx));
                    }
                }
            })),
            cx,
        );
    }

    /// Asks the destination for its models, with the stored credential.
    pub(super) fn list_models(&mut self, cx: &mut Context<Self>) {
        let Some(catalog) = self.catalog.clone() else {
            return;
        };
        let Ok(profile) = self.form(cx) else {
            return;
        };
        if profile.kind == ProfileKind::Fake {
            return;
        }
        let Some(backend) = self.backend.take() else {
            return;
        };
        self.models = Models::Loading;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let (backend, outcome) = cx
                .background_executor()
                .spawn(async move {
                    // The ChatGPT catalog uses the shared session's token.
                    let secret = if profile.kind == ProfileKind::ChatGptPlan {
                        Ok(None)
                    } else {
                        backend.secret(&profile.credential_account())
                    };
                    let outcome = match secret {
                        Ok(secret) => catalog.list(&profile, secret),
                        Err(error) => {
                            tracing::error!(
                                error = %error,
                                operation = "model_catalog",
                                "reading the provider credential failed"
                            );
                            Err(ProviderError::Keystore)
                        }
                    };
                    (backend, outcome)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.backend = Some(backend);
                this.models = match outcome {
                    Ok(models) => Models::Ready(models),
                    Err(error) => Models::Failed(error.message()),
                };
                cx.notify();
            });
        })
        .detach();
    }

    /// Opens the browser sign-in and waits for its loopback callback off the
    /// UI thread.
    pub(super) fn start_sign_in(&mut self, cx: &mut Context<Self>) {
        let Some(account) = self.account.clone() else {
            return;
        };
        let Some(previous) = self.stored.as_ref().map(|stored| stored.chatgpt.clone()) else {
            return;
        };
        if !matches!(self.sign_in, SignIn::Idle) {
            return;
        }
        let first = previous
            .as_ref()
            .is_none_or(|account| account.client_id.is_none());
        self.sign_in = SignIn::Starting;
        self.error = None;
        self.plan_notice = false;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let started = cx
                .background_executor()
                .spawn(async move { account.start_sign_in(previous.as_ref()) })
                .await;
            let pending = match started {
                Ok(pending) => pending,
                Err(error) => {
                    let _ = this.update(cx, |this, cx| {
                        this.sign_in = SignIn::Idle;
                        this.error = Some(error.message().to_owned());
                        cx.notify();
                    });
                    return;
                }
            };
            let url = pending.authorize_url().to_owned();
            let cancel = pending.canceller();
            let shown = this.update(cx, |this, cx| {
                cx.open_url(&url);
                this.sign_in = SignIn::Waiting { url, cancel };
                cx.notify();
            });
            if shown.is_err() {
                return;
            }
            let outcome = cx
                .background_executor()
                .spawn(async move { pending.wait(SIGN_IN_TIMEOUT) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.sign_in = SignIn::Idle;
                match outcome {
                    Ok(signed_in) => this.finish_sign_in(signed_in, first, cx),
                    Err(ProviderError::Cancelled) => {
                        this.show_notice("Login não concluído.".to_owned(), cx)
                    }
                    Err(error) => this.error = Some(error.message().to_owned()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn finish_sign_in(&mut self, signed_in: SignedIn, first: bool, cx: &mut Context<Self>) {
        let Some(stored) = self.stored.clone() else {
            return;
        };
        if self.backend.is_none() {
            // Another operation holds the settings; the token is dropped
            // rather than stored half-way.
            self.error = Some("O login terminou durante outra operação. Entre de novo.".into());
            return;
        }
        let edited = self.kind == ProfileKind::ChatGptPlan && self.edited(cx);
        let values: [String; 3] = std::array::from_fn(|index| self.value(index, cx));
        self.run_then(
            Box::new(move |backend| {
                backend.store_sign_in(&stored, signed_in)?;
                backend.load()
            }),
            Some("Conta ChatGPT conectada."),
            Some(Box::new(move |this, cx| {
                this.switch_kind(ProfileKind::ChatGptPlan, cx);
                if edited {
                    for (field, value) in this.fields.iter().zip(&values) {
                        field.update(cx, |field, cx| field.set_value(value, cx));
                    }
                }
                this.plan_notice = first;
                this.list_models(cx);
            })),
            cx,
        );
    }

    /// Revokes the refresh token at OpenAI and forgets the account here,
    /// even when the revocation cannot be confirmed.
    pub(super) fn sign_out(&mut self, cx: &mut Context<Self>) {
        let (Some(account), Some(stored)) = (self.account.clone(), self.stored.clone()) else {
            return;
        };
        let Some(chatgpt) = stored.chatgpt.clone() else {
            return;
        };
        let unconfirmed = Arc::new(AtomicBool::new(false));
        let flag = unconfirmed.clone();
        self.plan_notice = false;
        self.run_then(
            Box::new(move |backend| {
                let token_account = as_kind(&stored, ProfileKind::ChatGptPlan).credential_account();
                let token = backend.secret(&token_account)?;
                if account.sign_out(&chatgpt, token).is_err() {
                    flag.store(true, Ordering::Relaxed);
                }
                backend.forget_sign_in(&stored)?;
                backend.load()
            }),
            None,
            Some(Box::new(move |this, cx| {
                this.switch_kind(ProfileKind::ChatGptPlan, cx);
                let notice = if unconfirmed.load(Ordering::Relaxed) {
                    "Conta desconectada aqui. A OpenAI não confirmou a revogação; \
                     remova o acesso também em chatgpt.com."
                } else {
                    "Conta ChatGPT desconectada e token apagado do cofre."
                };
                this.show_notice(notice.to_owned(), cx);
            })),
            cx,
        );
    }

    /// Address, model (with the picker) and per-item limit for the form's kind.
    pub(super) fn render_fields(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let kind = self.kind;
        let presets = (kind == ProfileKind::OpenAiCompatible).then(|| {
            let buttons: Vec<_> = LOCAL_PRESETS
                .iter()
                .enumerate()
                .map(|(index, (name, _))| {
                    self.button(
                        PRESET_IDS[index],
                        ButtonKind::Ghost,
                        !self.busy,
                        *name,
                        Action::Preset(index),
                        cx,
                    )
                })
                .collect();
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child("Modelo nesta máquina:"),
                )
                .children(buttons)
        });
        let endpoint = (kind == ProfileKind::OpenAiCompatible).then(|| {
            field_row(
                theme,
                FIELD_LABELS[0],
                self.fields[0].clone().into_any_element(),
            )
        });
        let plans = (kind == ProfileKind::OpenCode).then(|| self.render_plans(theme, cx));
        let loading = matches!(self.models, Models::Loading);
        let reachable = match kind {
            ProfileKind::ChatGptPlan => self.credentials.chatgpt,
            // The gateway lists its models without a key; Claude Code's are fixed.
            ProfileKind::OpenCode | ProfileKind::ClaudeCode => true,
            _ => !self.value(0, cx).is_empty(),
        };
        let list = self.catalog.is_some().then(|| {
            self.button(
                "settings-list-models",
                ButtonKind::Secondary,
                !self.busy && !loading && reachable,
                if loading {
                    "Buscando…"
                } else {
                    "Listar modelos"
                },
                Action::ListModels,
                cx,
            )
        });
        let models = self.render_model_list(theme, cx);
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S4))
            .pt(px(SpacingScale::S2))
            .children(presets)
            .children(plans)
            .children(endpoint)
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(SpacingScale::S3))
                    .child(div().flex_1().min_w(px(0.0)).child(field_row(
                        theme,
                        match kind {
                            ProfileKind::ChatGptPlan => "Modelo do plano",
                            _ => FIELD_LABELS[1],
                        },
                        self.fields[1].clone().into_any_element(),
                    )))
                    .children(list)
                    .child(div().w(px(180.0)).flex_none().child(field_row(
                        theme,
                        FIELD_LABELS[2],
                        self.fields[2].clone().into_any_element(),
                    ))),
            )
            .children(models)
    }

    /// Zen or Go, as two options; the choice is the gateway address.
    fn render_plans(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let go = self.value(0, cx) == OPENCODE_GO_ENDPOINT;
        let colors = theme.colors;
        let options: Vec<_> = OPENCODE_PLANS
            .iter()
            .map(|&(plan, title, body)| {
                let selected = plan == go;
                let id = if plan {
                    "settings-opencode-go"
                } else {
                    "settings-opencode-zen"
                };
                let focus = self
                    .focus
                    .entry(id)
                    .or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone();
                mark_selected(
                    div()
                        .id(id)
                        .relative()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .px(px(SpacingScale::S3))
                        .py(px(SpacingScale::S2))
                        .rounded(theme.radius.control())
                        .border_1()
                        .border_color(colors.hairline_divider())
                        .cursor_pointer()
                        .role(Role::RadioButton)
                        .aria_label(format!("OpenCode {title}"))
                        .aria_toggled(if selected {
                            Toggled::True
                        } else {
                            Toggled::False
                        })
                        .track_focus(&focus)
                        .focus_visible(crate::ui::controls::focus_ring(theme))
                        .when(!selected, |row| {
                            row.hover(move |style| style.bg(colors.glass_fill_medium()))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.act(Action::OpenCodePlan(plan), cx)
                        }))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.act(Action::OpenCodePlan(plan), cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                    theme,
                    selected,
                )
                .child(text_style(div(), TypeScale::ROW_TITLE).child(format!("OpenCode {title}")))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_muted())
                        .child(body),
                )
            })
            .collect();
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(
                text_style(div(), TypeScale::LABEL)
                    .text_color(colors.text_secondary())
                    .child("Plano"),
            )
            .child(
                div()
                    .id("settings-opencode-plan")
                    .flex()
                    .gap(px(SpacingScale::S3))
                    .role(Role::RadioGroup)
                    .aria_label("Plano do OpenCode")
                    .children(options),
            )
    }

    fn render_model_list(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Option<AnyElement> {
        let models = match &self.models {
            Models::Idle => return None,
            Models::Loading => {
                return Some(skeleton_list(theme, "settings-models-skeleton", 3));
            }
            Models::Failed(message) => {
                return Some(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .id("settings-models-error")
                        .role(Role::Alert)
                        .child(icon(IconName::Info, 14.0, theme.colors.status_danger()))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(theme.colors.text_secondary())
                                .child(format!("{message} Você ainda pode digitar o modelo.")),
                        )
                        .into_any_element(),
                );
            }
            Models::Ready(models) if models.is_empty() => {
                return Some(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child("Nenhum modelo disponível nesse destino. Digite o nome do modelo.")
                        .into_any_element(),
                );
            }
            Models::Ready(models) => models.clone(),
        };
        while self.model_focus.len() < models.len() {
            self.model_focus.push(cx.focus_handle().tab_stop(true));
        }
        let current = self.value(1, cx);
        let colors = theme.colors;
        let rows = models.iter().enumerate().map(|(index, model)| {
            let selected = model.id == current;
            mark_selected(
                div()
                    .id(("settings-model", index))
                    .relative()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .px(px(SpacingScale::S3))
                    .py(px(SpacingScale::S2))
                    .rounded(theme.radius.control())
                    .cursor_pointer()
                    .role(Role::RadioButton)
                    .aria_label(model.label.clone())
                    .aria_toggled(if selected {
                        Toggled::True
                    } else {
                        Toggled::False
                    })
                    .track_focus(&self.model_focus[index])
                    .focus_visible(crate::ui::controls::focus_ring(theme))
                    .when(!selected, |row| {
                        row.hover(move |style| style.bg(colors.glass_fill_medium()))
                    })
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.act(Action::PickModel(index), cx)),
                    )
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.act(Action::PickModel(index), cx);
                            cx.stop_propagation();
                        }
                    })),
                theme,
                selected,
            )
            .child(
                text_style(div(), TypeScale::ROW_TITLE)
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .child(model.label.clone()),
            )
            .when(model.label != model.id, |row| {
                row.child(
                    text_style(div(), TypeScale::META)
                        .flex_none()
                        .text_color(colors.text_muted())
                        .child(model.id.clone()),
                )
            })
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(match models.len() {
                            1 => "1 MODELO DISPONÍVEL".to_owned(),
                            count => format!("{count} MODELOS DISPONÍVEIS"),
                        }),
                )
                .child(
                    div()
                        .id("settings-models")
                        .max_h(px(232.0))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .p(px(SpacingScale::S1))
                        .rounded(theme.radius.control())
                        .border_1()
                        .border_color(colors.hairline_divider())
                        .bg(colors.glass_fill_low())
                        .role(Role::RadioGroup)
                        .aria_label("Modelos disponíveis")
                        .children(rows),
                )
                .into_any_element(),
        )
    }

    /// The API key card of the OpenAI-compatible provider or OpenCode.
    pub(super) fn render_key(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let opencode = self.kind == ProfileKind::OpenCode;
        let stored = self.credentials.stored(self.kind);
        let optional = !opencode
            && self
                .form(cx)
                .is_ok_and(|profile| !profile.credential_required());
        let typed = !self.key.read(cx).value().trim().is_empty();
        let store = self.button(
            "settings-store-key",
            ButtonKind::Secondary,
            !self.busy && typed,
            if stored {
                "Substituir chave"
            } else {
                "Guardar no cofre"
            },
            Action::StoreKey,
            cx,
        );
        let (color, state) = match (stored, optional) {
            (true, _) => (theme.colors.status_success(), "Guardada no cofre"),
            (false, true) => (theme.colors.text_muted(), "Opcional neste endereço"),
            (false, false) => (theme.colors.text_muted(), "Nenhuma chave"),
        };
        let (title, description) = if opencode {
            ("Chave do OpenCode", OPENCODE_KEY_DESCRIPTION)
        } else {
            ("Chave do provedor", KEY_DESCRIPTION)
        };
        let console = opencode.then(|| {
            let label = "Criar uma chave";
            self.button_frame(
                "settings-opencode-console",
                ButtonKind::Ghost,
                true,
                label.into(),
                Action::OpenCodeConsole,
                cx,
            )
            .child(label)
            .child(icon(
                IconName::ArrowUpRight,
                14.0,
                button_foreground(theme, ButtonKind::Ghost, true),
            ))
        });
        card(theme, title, description).child(
            card_body()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(px(SpacingScale::S2))
                        .child(status_pill(theme, color, state))
                        .children(console),
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

    /// The ChatGPT account: sign-in, the waiting browser, the connected
    /// account with its plan usage, and sign-out.
    pub(super) fn render_account(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let Some(stored) = self.stored.clone() else {
            return div();
        };
        let colors = theme.colors;
        let account = stored.chatgpt.clone().unwrap_or_default();
        let connected =
            self.credentials.chatgpt && (account.email.is_some() || account.subject.is_some());
        let available = self.account.is_some();
        let mut body = card_body();
        if self.plan_notice && connected {
            let dismiss = self.button(
                "settings-plan-notice",
                ButtonKind::Secondary,
                true,
                "Entendi",
                Action::DismissPlanNotice,
                cx,
            );
            body = body.child(
                div()
                    .id("settings-plan-notice-panel")
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S3))
                    .p(px(SpacingScale::S3))
                    .rounded(theme.radius.control())
                    .bg(tint(colors.status_info(), 0.07))
                    .border_1()
                    .border_color(tint(colors.status_info(), 0.28))
                    .role(Role::Status)
                    .child(icon(IconName::Info, 16.0, colors.status_info()))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                text_style(div(), TypeScale::ROW_TITLE)
                                    .child("Você está usando o seu plano do ChatGPT"),
                            )
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .text_color(colors.text_secondary())
                                    .child(PLAN_NOTICE),
                            ),
                    )
                    .child(dismiss),
            );
        }
        let waiting = matches!(self.sign_in, SignIn::Waiting { .. });
        let starting = matches!(self.sign_in, SignIn::Starting);
        let footer: Option<Div> = if waiting {
            {
                body = body.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .id("settings-sign-in-waiting")
                        .role(Role::Status)
                        .child(icon(IconName::Clock, 16.0, colors.text_muted()))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_secondary())
                                .child(WAITING_BROWSER),
                        ),
                );
                let reopen = self.button(
                    "settings-sign-in-reopen",
                    ButtonKind::Ghost,
                    true,
                    "Abrir o navegador de novo",
                    Action::ReopenBrowser,
                    cx,
                );
                let cancel = self.button(
                    "settings-sign-in-cancel",
                    ButtonKind::Secondary,
                    true,
                    "Cancelar",
                    Action::CancelSignIn,
                    cx,
                );
                Some(
                    card_footer(theme)
                        .child(div().flex_1().child(reopen))
                        .child(cancel),
                )
            }
        } else if connected {
            {
                let (pill_color, pill) = if account.plan_usage {
                    (colors.status_success(), "Usando o plano do ChatGPT")
                } else {
                    (colors.status_warning(), "Uso do plano não permitido")
                };
                body = body.child(
                    div()
                        .id("settings-account")
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .role(Role::Status)
                        .child(icon(IconName::User, 16.0, colors.text_muted()))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    text_style(div(), TypeScale::ROW_TITLE).truncate().child(
                                        account
                                            .email
                                            .clone()
                                            .unwrap_or_else(|| "Conta conectada".to_owned()),
                                    ),
                                )
                                .child(div().flex().child(status_pill(theme, pill_color, pill))),
                        ),
                );
                if !account.plan_usage {
                    body = body.child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child("Entre de novo e permita o uso do plano para extrair com ele."),
                    );
                }
                let sign_out = self.button(
                    "settings-sign-out",
                    ButtonKind::Ghost,
                    !self.busy && available,
                    "Sair da conta",
                    Action::SignOut,
                    cx,
                );
                let usage_label = "Gerenciar uso";
                let usage = self
                    .button_frame(
                        "settings-manage-usage",
                        ButtonKind::Ghost,
                        true,
                        usage_label.into(),
                        Action::ManageUsage,
                        cx,
                    )
                    .child(usage_label)
                    .child(icon(
                        IconName::ArrowUpRight,
                        14.0,
                        button_foreground(theme, ButtonKind::Ghost, true),
                    ));
                let again = (!account.plan_usage).then(|| {
                    self.button(
                        "settings-sign-in",
                        ButtonKind::Primary,
                        !self.busy && available && !starting,
                        "Entrar de novo",
                        Action::SignIn,
                        cx,
                    )
                });
                Some(
                    card_footer(theme)
                        .child(div().flex_1().child(sign_out))
                        .child(usage)
                        .children(again),
                )
            }
        } else {
            {
                body = body.child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(ACCOUNT_DESCRIPTION),
                );
                let sign_in = self.button(
                    "settings-sign-in",
                    ButtonKind::Primary,
                    !self.busy && available && !starting,
                    if starting {
                        "Abrindo o navegador…"
                    } else {
                        "Continuar com o ChatGPT"
                    },
                    Action::SignIn,
                    cx,
                );
                Some(
                    card_footer(theme)
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .min_w(px(0.0))
                                .text_color(colors.text_muted())
                                .when(!available, |hint| {
                                    hint.child("O login com o ChatGPT não está disponível aqui.")
                                }),
                        )
                        .child(sign_in),
                )
            }
        };
        card(
            theme,
            "Conta ChatGPT",
            "O login acontece no navegador; a senha nunca passa pela xemnas.",
        )
        .child(body)
        .children(footer)
    }

    /// Asks the local Claude Code for its installation and login, off the
    /// UI thread; spends no tokens.
    pub(super) fn check_claude_code(&mut self, cx: &mut Context<Self>) {
        let Some(probe) = self.claude_code.clone() else {
            self.claude = ClaudeCheck::Done(Err(
                "A verificação do Claude Code não está disponível aqui.",
            ));
            return;
        };
        self.claude = ClaudeCheck::Checking;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { probe.status() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.claude = ClaudeCheck::Done(outcome.map_err(|error| error.message()));
                cx.notify();
            });
        })
        .detach();
    }

    /// Claude Code: whether it is installed and signed in, the account it
    /// uses, and the login command when it is not.
    pub(super) fn render_claude_code(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let checking = matches!(self.claude, ClaudeCheck::Idle | ClaudeCheck::Checking);
        let mut body = card_body();
        let mut hint = None;
        match &self.claude {
            ClaudeCheck::Idle | ClaudeCheck::Checking => {
                body = body.child(skeleton_list(theme, "settings-claude-skeleton", 2));
            }
            ClaudeCheck::Done(Err(message)) => {
                let missing = *message == ProviderError::NotInstalled.message();
                body = body
                    .child(div().flex().child(status_pill(
                        theme,
                        colors.status_warning(),
                        if missing {
                            "Não encontrado"
                        } else {
                            "Indisponível"
                        },
                    )))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(format!("{message} {CLAUDE_INSTALL}")),
                    );
            }
            ClaudeCheck::Done(Ok(status)) if !status.signed_in => {
                let status = status.clone();
                body = body
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(status_pill(theme, colors.status_warning(), "Sem login"))
                            .children(status.version.map(|version| {
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(format!("Claude Code {version}"))
                            })),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(CLAUDE_LOGIN),
                    )
                    .child(self.render_login_command(theme, cx));
            }
            ClaudeCheck::Done(Ok(status)) => {
                let status = status.clone();
                let details = [
                    status
                        .subscription
                        .map(|plan| format!("Plano {}", super::capitalize(&plan))),
                    status.auth_method.map(|method| match method.as_str() {
                        "claude.ai" => "conta claude.ai".to_owned(),
                        other => other.to_owned(),
                    }),
                    status
                        .version
                        .map(|version| format!("Claude Code {version}")),
                ]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · ");
                body = body.child(
                    div()
                        .id("settings-claude-account")
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .role(Role::Status)
                        .child(icon(IconName::User, 16.0, colors.text_muted()))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(text_style(div(), TypeScale::ROW_TITLE).truncate().child(
                                    status.email.unwrap_or_else(|| "Conta conectada".to_owned()),
                                ))
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .truncate()
                                        .text_color(colors.text_muted())
                                        .child(details),
                                ),
                        )
                        .child(status_pill(theme, colors.status_success(), "Conectado")),
                );
                hint = Some(CLAUDE_EXPERIMENTAL);
            }
        }
        let check = self.button(
            "settings-claude-check",
            ButtonKind::Secondary,
            !checking && self.claude_code.is_some(),
            if checking {
                "Verificando…"
            } else {
                "Verificar de novo"
            },
            Action::CheckClaudeCode,
            cx,
        );
        card(
            theme,
            "Claude Code",
            "A xemnas usa o Claude Code desta máquina; o login fica com ele, nunca com a xemnas.",
        )
        .child(body)
        .child(
            card_footer(theme)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .min_w(px(0.0))
                        .text_color(colors.text_muted())
                        .children(hint),
                )
                .child(check),
        )
    }

    /// `claude auth login` in monospace, with a copy action.
    fn render_login_command(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let label = "Copiar comando";
        let copy = self
            .button_frame(
                "settings-claude-copy",
                ButtonKind::Ghost,
                true,
                label.into(),
                Action::CopyLoginCommand,
                cx,
            )
            .child(icon(
                IconName::Copy,
                14.0,
                button_foreground(theme, ButtonKind::Ghost, true),
            ))
            .child(label);
        div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .p(px(SpacingScale::S2))
            .pl(px(SpacingScale::S3))
            .rounded(theme.radius.control())
            .bg(theme.colors.glass_fill_low())
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .child(
                code_style(div(), TypeScale::BODY_SMALL)
                    .flex_1()
                    .min_w(px(0.0))
                    .child(CLAUDE_CODE_LOGIN_COMMAND),
            )
            .child(copy)
    }
}
