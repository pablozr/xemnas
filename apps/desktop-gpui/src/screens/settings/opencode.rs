//! Settings → OpenCode: whether captures reach this app, and why not.
//!
//! Everything shown comes from `application::integration`: the snapshot
//! (`Integration::status`) and the connection test (`Integration::check`),
//! whose messages are already product language. The only action besides the
//! test is moving stalled outbox items back to `pending/`.

use std::path::PathBuf;

use application::integration::{
    AdapterStatus, CheckKind, CheckOutcome, Compatibility, Integration, IntegrationCheck,
    IntegrationState, IntegrationStatus, IntegrationStore,
};
use application::outbox::retry_stalled;
use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Div, Render, Role, Window};

use super::parts::{card, card_body, card_footer, kv_row, stat_tile, status_hero};
use crate::i18n::settings as t;
use crate::screens::format::date_time;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{error_banner, skeleton_list, status_pill, toast, TOAST_DURATION};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

/// What the OpenCode section needs from the application layer.
pub trait IntegrationBackend: Send + 'static {
    /// The integration snapshot.
    fn status(&self) -> Result<IntegrationStatus, String>;
    /// Runs the connection test.
    fn check(&self) -> Result<Vec<IntegrationCheck>, String>;
    /// Moves stalled outbox items back to `pending/`; returns how many.
    fn retry_stalled(&self) -> Result<usize, String>;
}

/// The integration use case plus the outbox root it reports on.
pub struct IntegrationService<S> {
    integration: Integration<S>,
    outbox_dir: PathBuf,
}

impl<S> IntegrationService<S> {
    /// Wraps the use case built by the composition root.
    pub fn new(integration: Integration<S>, outbox_dir: PathBuf) -> Self {
        Self {
            integration,
            outbox_dir,
        }
    }
}

impl<S: IntegrationStore + Send + 'static> IntegrationBackend for IntegrationService<S> {
    fn status(&self) -> Result<IntegrationStatus, String> {
        self.integration.status().map_err(|error| {
            tracing::error!(error = %error, operation = "integration_status", "status failed");
            t::opencode_err_status().to_owned()
        })
    }
    fn check(&self) -> Result<Vec<IntegrationCheck>, String> {
        self.integration.check().map_err(|error| {
            tracing::error!(error = %error, operation = "integration_check", "check failed");
            t::opencode_err_check().to_owned()
        })
    }
    fn retry_stalled(&self) -> Result<usize, String> {
        retry_stalled(&self.outbox_dir).map_err(|error| {
            tracing::error!(error = %error, operation = "outbox_retry", "retry failed");
            t::opencode_err_retry().to_owned()
        })
    }
}

enum Outcome {
    Status(Result<IntegrationStatus, String>),
    Checked(Result<Vec<IntegrationCheck>, String>),
    Retried(Result<usize, String>),
}

/// Settings → OpenCode.
pub struct OpenCodePanel {
    backend: Option<Box<dyn IntegrationBackend>>,
    busy: bool,
    status: Option<IntegrationStatus>,
    checks: Option<Vec<IntegrationCheck>>,
    error: Option<String>,
    notice: Option<String>,
}

impl OpenCodePanel {
    /// Mounts the section; nothing is read until [`Self::refresh`].
    pub fn new(backend: Box<dyn IntegrationBackend>) -> Self {
        Self {
            backend: Some(backend),
            busy: false,
            status: None,
            checks: None,
            error: None,
            notice: None,
        }
    }

    /// Reloads the snapshot; called each time the section opens.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.run(|backend| Outcome::Status(backend.status()), cx);
    }

    fn run(
        &mut self,
        operation: impl FnOnce(&dyn IntegrationBackend) -> Outcome + Send + 'static,
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
                    Outcome::Status(Ok(status)) => this.status = Some(status),
                    Outcome::Checked(Ok(checks)) => {
                        this.checks = Some(checks);
                        this.refresh(cx);
                    }
                    Outcome::Retried(Ok(moved)) => {
                        this.show_notice(t::opencode_retried(moved), cx);
                        this.refresh(cx);
                    }
                    Outcome::Status(Err(error))
                    | Outcome::Checked(Err(error))
                    | Outcome::Retried(Err(error)) => this.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }

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

    fn render_hero(theme: &Theme, status: &IntegrationStatus) -> gpui::Stateful<Div> {
        let latest = status.adapters.first();
        match status.state {
            IntegrationState::Receiving => status_hero(
                theme,
                "opencode-state",
                theme.colors.status_success(),
                t::opencode_receiving_title(),
                t::opencode_receiving_body(
                    &latest
                        .map(|adapter| date_time(&adapter.last_received_at))
                        .unwrap_or_default(),
                ),
            ),
            IntegrationState::AwaitingFirstCapture => status_hero(
                theme,
                "opencode-state",
                theme.colors.status_info(),
                t::opencode_awaiting_title(),
                t::opencode_awaiting_body().into(),
            ),
            IntegrationState::ApiUnavailable => status_hero(
                theme,
                "opencode-state",
                theme.colors.status_danger(),
                t::opencode_api_down_title(),
                t::opencode_api_down_body().into(),
            ),
        }
    }

    fn render_test(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let checks = self.checks.clone();
        let summary = checks.as_ref().map(|checks| {
            let count = |outcome| {
                checks
                    .iter()
                    .filter(|check| check.outcome == outcome)
                    .count()
            };
            let mut parts = vec![t::opencode_summary_ok(count(CheckOutcome::Ok))];
            let warnings = count(CheckOutcome::Warning);
            if warnings > 0 {
                parts.push(t::opencode_summary_warnings(warnings));
            }
            let failures = count(CheckOutcome::Failed);
            if failures > 0 {
                parts.push(t::opencode_summary_failures(failures));
            }
            parts.join(" · ")
        });
        let body = match &checks {
            None => text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child(t::opencode_test_body())
                .into_any_element(),
            Some(checks) => div()
                .id("opencode-checks")
                .flex()
                .flex_col()
                .role(Role::List)
                .children(
                    checks
                        .iter()
                        .enumerate()
                        .map(|(index, check)| check_row(theme, index, check).into_any_element()),
                )
                .into_any_element(),
        };
        let button = action_button(theme, "opencode-test", ButtonKind::Primary, !self.busy)
            .aria_label(t::opencode_test())
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.busy {
                    this.run(|backend| Outcome::Checked(backend.check()), cx);
                }
            }))
            .child(if self.busy && checks.is_none() {
                t::opencode_testing()
            } else if checks.is_some() {
                t::opencode_test_again()
            } else {
                t::opencode_test()
            });
        card(
            theme,
            t::opencode_test_title(),
            t::opencode_test_card_body(),
        )
        .child(card_body().child(body))
        .child(
            card_footer(theme)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .text_color(colors.text_muted())
                        .children(summary),
                )
                .child(button),
        )
    }

    fn render_adapters(theme: &Theme, adapters: &[AdapterStatus]) -> Div {
        let colors = theme.colors;
        let body: AnyElement = if adapters.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child(t::opencode_no_adapters())
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .children(adapters.iter().enumerate().map(|(index, adapter)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .py(px(SpacingScale::S3))
                        .when(index > 0, |row| {
                            row.border_t_1().border_color(colors.hairline_divider())
                        })
                        .child(icon(IconName::Link, 16.0, colors.text_muted()))
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
                                        .child(
                                            text_style(div(), TypeScale::ROW_TITLE)
                                                .child(adapter_label(&adapter.adapter)),
                                        )
                                        .children(adapter.adapter_version.clone().map(|version| {
                                            text_style(div(), TypeScale::META)
                                                .font_family(Theme::font_mono())
                                                .text_color(colors.text_muted())
                                                .child(format!("v{version}"))
                                        }))
                                        .child(match adapter.compatibility {
                                            Compatibility::Compatible => status_pill(
                                                theme,
                                                colors.status_success(),
                                                t::opencode_compatible(),
                                            ),
                                            Compatibility::Unknown => status_pill(
                                                theme,
                                                colors.text_muted(),
                                                t::opencode_version_unknown(),
                                            ),
                                        }),
                                )
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(colors.text_muted())
                                        .child(t::opencode_adapter_meta(
                                            &t::sessions_count(adapter.sessions as usize),
                                            &date_time(&adapter.last_received_at),
                                        )),
                                ),
                        )
                }))
                .into_any_element()
        };
        card(
            theme,
            t::opencode_adapters_title(),
            t::opencode_adapters_body(),
        )
        .child(card_body().child(body))
    }

    fn render_outbox(
        &self,
        theme: &Theme,
        status: &IntegrationStatus,
        cx: &mut Context<Self>,
    ) -> Div {
        let outbox = &status.outbox;
        let colors = theme.colors;
        let tiles = [
            (
                t::opencode_outbox_pending(),
                outbox.pending,
                colors.status_info(),
            ),
            (
                t::opencode_outbox_accepted(),
                outbox.accepted,
                colors.status_success(),
            ),
            (
                t::opencode_outbox_rejected(),
                outbox.rejected,
                colors.status_danger(),
            ),
            (
                t::opencode_outbox_stalled(),
                outbox.stalled,
                colors.status_warning(),
            ),
        ];
        let stalled = outbox.stalled;
        card(theme, t::opencode_outbox_title(), t::opencode_outbox_body())
            .child(
                card_body()
                    .child(
                        div()
                            .flex()
                            .gap(px(SpacingScale::S3))
                            .children(tiles.into_iter().map(|(label, value, accent)| {
                                stat_tile(
                                    theme,
                                    label,
                                    value.to_string(),
                                    (value > 0).then_some(accent),
                                )
                            })),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_muted())
                            .truncate()
                            .child(if outbox.exists {
                                outbox.path.display().to_string()
                            } else {
                                t::opencode_outbox_missing(&outbox.path.display().to_string())
                            }),
                    ),
            )
            .when(stalled > 0, |card| {
                card.child(
                    card_footer(theme)
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .text_color(colors.text_muted())
                                .child(t::opencode_stalled_hint()),
                        )
                        .child(
                            action_button(
                                theme,
                                "opencode-retry",
                                ButtonKind::Secondary,
                                !self.busy,
                            )
                            .aria_label(t::opencode_retry_label())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.busy {
                                    this.run(
                                        |backend| Outcome::Retried(backend.retry_stalled()),
                                        cx,
                                    );
                                }
                            }))
                            .child(t::opencode_retry()),
                        ),
                )
            })
    }

    fn render_connection(theme: &Theme, status: &IntegrationStatus) -> Div {
        card(
            theme,
            t::opencode_connection_title(),
            t::opencode_connection_body(),
        )
        .child(
            card_body()
                .gap(px(0.0))
                .child(kv_row(
                    theme,
                    t::opencode_local_api(),
                    status
                        .api
                        .map(|api| t::opencode_api_value(api.port, api.protocol_version))
                        .unwrap_or_else(|| t::opencode_inactive().into()),
                    true,
                    false,
                ))
                .child(kv_row(
                    theme,
                    t::opencode_contract(),
                    format!("v{}", status.contract_version),
                    true,
                    true,
                ))
                .child(kv_row(
                    theme,
                    t::opencode_data_dir(),
                    status.data_dir.display().to_string(),
                    true,
                    true,
                )),
        )
    }
}

impl Render for OpenCodePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let content: AnyElement = match self.status.clone() {
            None if self.error.is_none() => skeleton_list(&theme, "opencode-skeleton", 4),
            None => div().into_any_element(),
            Some(status) => {
                let test = self.render_test(&theme, cx);
                let outbox = self.render_outbox(&theme, &status, cx);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S5))
                    .child(Self::render_hero(&theme, &status))
                    .child(test)
                    .child(Self::render_adapters(&theme, &status.adapters))
                    .child(outbox)
                    .child(Self::render_connection(&theme, &status))
                    .into_any_element()
            }
        };
        let retry = self.error.is_some().then(|| {
            action_button(&theme, "opencode-reload", ButtonKind::Ghost, !self.busy)
                .aria_label(t::action_try_again())
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .child(t::action_try_again())
        });
        div()
            .relative()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S4))
            .children(self.error.clone().map(|error| {
                error_banner(&theme, &error)
                    .rounded(theme.radius.control())
                    .children(retry)
            }))
            .child(content)
            .children(
                self.notice
                    .clone()
                    .map(|notice| toast(&theme, &notice, 24.0)),
            )
    }
}

fn check_row(theme: &Theme, index: usize, check: &IntegrationCheck) -> Div {
    let colors = theme.colors;
    let (glyph, color, label) = match check.outcome {
        CheckOutcome::Ok => (
            IconName::CheckCircle,
            colors.status_success(),
            t::opencode_check_ok(),
        ),
        CheckOutcome::Warning => (
            IconName::Info,
            colors.status_warning(),
            t::opencode_check_warning(),
        ),
        CheckOutcome::Failed => (
            IconName::Info,
            colors.status_danger(),
            t::opencode_check_failed(),
        ),
        CheckOutcome::Skipped => (
            IconName::Circle,
            colors.text_muted(),
            t::opencode_check_skipped(),
        ),
    };
    div()
        .flex()
        .items_start()
        .gap(px(SpacingScale::S3))
        .py(px(SpacingScale::S3))
        .when(index > 0, |row| {
            row.border_t_1().border_color(colors.hairline_divider())
        })
        .child(div().mt(px(2.0)).child(icon(glyph, 16.0, color)))
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
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE).child(check_label(check.kind)),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(color)
                                .child(label),
                        ),
                )
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .child(match &check.at {
                            Some(at) => t::opencode_check_message(&check.message, &date_time(at)),
                            None => check.message.clone(),
                        }),
                ),
        )
}

fn check_label(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::LocalApi => t::opencode_local_api(),
        CheckKind::Discovery => t::opencode_check_discovery(),
        CheckKind::Token => t::opencode_check_token(),
        CheckKind::Outbox => t::opencode_check_outbox(),
        CheckKind::Captures => t::opencode_check_captures(),
    }
}

fn adapter_label(name: &str) -> String {
    match name {
        "opencode" => "OpenCode".into(),
        other => other.to_owned(),
    }
}
