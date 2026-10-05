//! The provider-specific parts of "IA e privacidade" (ADR-0004): the provider
//! form with its model picker, the credential card (API key or OpenCode
//! password) and the ChatGPT account card with its browser sign-in.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use application::profile::{
    build_preview, is_loopback_endpoint, AiProfile, ConsentPreview, ProfileKind, SignedIn,
    OPENCODE_GO_ENDPOINT,
};
use application::providers::ProviderError;
use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Div, Role, Toggled};

use super::parts::{card, card_body, card_footer, field_row};
use super::{as_kind, field_label, Action, Models, SettingsScreen, SignIn};
use crate::i18n::settings as t;
use crate::ui::controls::{button_foreground, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{mark_selected, skeleton_list, status_pill};
use crate::ui::theme::{text_style, Theme};
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
fn opencode_plans() -> [(bool, &'static str, &'static str); 2] {
    [
        (false, "Zen", t::plan_zen_body()),
        (true, "Go", t::plan_go_body()),
    ]
}

/// How long the browser sign-in may take before it gives up.
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Who analyses the captures of an active profile, for the status card.
pub(super) fn analysed_by(profile: &AiProfile) -> String {
    match profile.kind {
        ProfileKind::Fake => t::analysed_here().to_owned(),
        ProfileKind::OpenAiCompatible => match build_preview(profile).endpoint_host {
            Some(host) => t::analysed_by_host(&host),
            None => t::analysed_by_host(t::analysed_configured_provider()),
        },
        ProfileKind::ChatGptPlan => t::analysed_by_chatgpt(&profile.model),
        ProfileKind::OpenCode => t::analysed_by_opencode(opencode_plan(profile), &profile.model),
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
            Some((IconName::Cpu, t::destination_local_note()))
        }
        ProfileKind::ChatGptPlan => Some((IconName::User, t::destination_chatgpt_note())),
        ProfileKind::OpenCode => Some((IconName::Link, t::destination_opencode_note())),
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
    let consent = (false, t::step_consent_title(), t::step_consent_hint());
    match kind {
        ProfileKind::Fake => Vec::new(),
        ProfileKind::OpenAiCompatible => {
            let key = if saved && !stored.credential_required() {
                (true, t::step_no_key_title(), t::step_no_key_hint())
            } else {
                (ready, t::step_key_title(), t::step_key_hint())
            };
            vec![
                (saved, t::step_saved_title(), t::step_saved_hint_address()),
                key,
                consent,
            ]
        }
        ProfileKind::ChatGptPlan => vec![
            (
                saved,
                t::step_model_saved_title(),
                t::step_model_saved_hint(),
            ),
            (ready, t::step_account_title(), t::step_account_hint()),
            consent,
        ],
        ProfileKind::OpenCode => vec![
            (saved, t::step_saved_title(), t::step_saved_hint_plan()),
            (ready, t::step_key_title(), t::step_key_hint()),
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
                        this.show_notice(t::account_sign_in_cancelled().to_owned(), cx)
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
            self.error = Some(t::account_err_busy().into());
            return;
        }
        let edited = self.kind == ProfileKind::ChatGptPlan && self.edited(cx);
        let values: [String; 3] = std::array::from_fn(|index| self.value(index, cx));
        self.run_then(
            Box::new(move |backend| {
                backend.store_sign_in(&stored, signed_in)?;
                backend.load()
            }),
            Some(t::account_notice_connected()),
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
                    t::account_notice_signed_out_unconfirmed()
                } else {
                    t::account_notice_signed_out()
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
                        .child(t::models_on_this_machine()),
                )
                .children(buttons)
        });
        let endpoint = (kind == ProfileKind::OpenAiCompatible).then(|| {
            field_row(
                theme,
                field_label(0),
                self.fields[0].clone().into_any_element(),
            )
        });
        let plans = (kind == ProfileKind::OpenCode).then(|| self.render_plans(theme, cx));
        let loading = matches!(self.models, Models::Loading);
        let reachable = match kind {
            ProfileKind::ChatGptPlan => self.credentials.chatgpt,
            // The gateway lists its models without a key.
            ProfileKind::OpenCode => true,
            _ => !self.value(0, cx).is_empty(),
        };
        let list = self.catalog.is_some().then(|| {
            self.button(
                "settings-list-models",
                ButtonKind::Secondary,
                !self.busy && !loading && reachable,
                if loading {
                    t::models_searching()
                } else {
                    t::models_list()
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
                            ProfileKind::ChatGptPlan => t::field_plan_model_label(),
                            _ => field_label(1),
                        },
                        self.fields[1].clone().into_any_element(),
                    )))
                    .children(list)
                    .child(div().w(px(180.0)).flex_none().child(field_row(
                        theme,
                        field_label(2),
                        self.fields[2].clone().into_any_element(),
                    ))),
            )
            .children(models)
    }

    /// Zen or Go, as two options; the choice is the gateway address.
    fn render_plans(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let go = self.value(0, cx) == OPENCODE_GO_ENDPOINT;
        let colors = theme.colors;
        let options: Vec<_> = opencode_plans()
            .into_iter()
            .map(|(plan, title, body)| {
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
                    .child(t::models_plan_label()),
            )
            .child(
                div()
                    .id("settings-opencode-plan")
                    .flex()
                    .gap(px(SpacingScale::S3))
                    .role(Role::RadioGroup)
                    .aria_label(t::models_plan_group_label())
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
                                .child(t::models_failed(message)),
                        )
                        .into_any_element(),
                );
            }
            Models::Ready(models) if models.is_empty() => {
                return Some(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child(t::models_none())
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
                        .child(t::models_available_count(models.len())),
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
                        .aria_label(t::models_group_label())
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
                t::key_replace()
            } else {
                t::key_store()
            },
            Action::StoreKey,
            cx,
        );
        let (color, state) = match (stored, optional) {
            (true, _) => (theme.colors.status_success(), t::key_state_stored()),
            (false, true) => (theme.colors.text_muted(), t::key_state_optional()),
            (false, false) => (theme.colors.text_muted(), t::key_state_none()),
        };
        let (title, description) = if opencode {
            (t::key_opencode_title(), t::key_opencode_description())
        } else {
            (t::key_provider_title(), t::ai_key_description())
        };
        let console = opencode.then(|| {
            let label = t::key_create();
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
                t::account_got_it(),
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
                                    .child(t::account_plan_notice_title()),
                            )
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .text_color(colors.text_secondary())
                                    .child(t::account_plan_notice()),
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
                                .child(t::account_waiting_browser()),
                        ),
                );
                let reopen = self.button(
                    "settings-sign-in-reopen",
                    ButtonKind::Ghost,
                    true,
                    t::account_reopen_browser(),
                    Action::ReopenBrowser,
                    cx,
                );
                let cancel = self.button(
                    "settings-sign-in-cancel",
                    ButtonKind::Secondary,
                    true,
                    t::action_cancel(),
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
                    (colors.status_success(), t::account_pill_plan_on())
                } else {
                    (colors.status_warning(), t::account_pill_plan_off())
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
                                            .unwrap_or_else(|| t::account_connected().to_owned()),
                                    ),
                                )
                                .child(div().flex().child(status_pill(theme, pill_color, pill))),
                        ),
                );
                if !account.plan_usage {
                    body = body.child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_secondary())
                            .child(t::account_sign_in_again_hint()),
                    );
                }
                let sign_out = self.button(
                    "settings-sign-out",
                    ButtonKind::Ghost,
                    !self.busy && available,
                    t::account_sign_out(),
                    Action::SignOut,
                    cx,
                );
                let usage_label = t::account_manage_usage();
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
                        t::account_sign_in_again(),
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
                        .child(t::account_description()),
                );
                let sign_in = self.button(
                    "settings-sign-in",
                    ButtonKind::Primary,
                    !self.busy && available && !starting,
                    if starting {
                        t::account_opening_browser()
                    } else {
                        t::account_continue()
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
                                .when(!available, |hint| hint.child(t::account_unavailable())),
                        )
                        .child(sign_in),
                )
            }
        };
        card(theme, t::account_title(), t::account_body())
            .child(body)
            .children(footer)
    }
}
