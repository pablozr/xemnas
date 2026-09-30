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

use super::parts::{card, card_body, card_footer, icon_tile, kv_row, stat_tile, status_hero};
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
            "Não foi possível ler o estado da integração.".to_owned()
        })
    }
    fn check(&self) -> Result<Vec<IntegrationCheck>, String> {
        self.integration.check().map_err(|error| {
            tracing::error!(error = %error, operation = "integration_check", "check failed");
            "Não foi possível executar o teste de conexão.".to_owned()
        })
    }
    fn retry_stalled(&self) -> Result<usize, String> {
        retry_stalled(&self.outbox_dir).map_err(|error| {
            tracing::error!(error = %error, operation = "outbox_retry", "retry failed");
            "Não foi possível mover as capturas paradas.".to_owned()
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
                        this.show_notice(
                            format!(
                                "{moved} captura(s) voltaram para a fila e serão importadas na \
                                 próxima abertura."
                            ),
                            cx,
                        );
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
                IconName::CheckCircle,
                "Recebendo capturas",
                "Conectado",
                format!(
                    "O OpenCode está enviando capturas para este app. A última chegou em {}.",
                    latest
                        .map(|adapter| date_time(&adapter.last_received_at))
                        .unwrap_or_default()
                ),
            ),
            IntegrationState::AwaitingFirstCapture => status_hero(
                theme,
                "opencode-state",
                theme.colors.status_info(),
                IconName::Clock,
                "Aguardando a primeira captura",
                "Pronto",
                "A API local está ativa. Conclua um turno no OpenCode, com o plugin do \
                 xemnas, dentro de um projeto cadastrado."
                    .into(),
            ),
            IntegrationState::ApiUnavailable => status_hero(
                theme,
                "opencode-state",
                theme.colors.status_danger(),
                IconName::Activity,
                "API local inativa",
                "Offline",
                "As capturas esperam na outbox e são importadas na próxima abertura do app. \
                 Reinicie o xemnas para reativar a API."
                    .into(),
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
            let mut parts = vec![format!("{} ok", count(CheckOutcome::Ok))];
            let warnings = count(CheckOutcome::Warning);
            if warnings > 0 {
                parts.push(format!("{warnings} com atenção"));
            }
            let failures = count(CheckOutcome::Failed);
            if failures > 0 {
                parts.push(format!(
                    "{failures} {}",
                    if failures == 1 { "falha" } else { "falhas" }
                ));
            }
            parts.join(" · ")
        });
        let body = match &checks {
            None => text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child(
                    "Verifica a API local, o arquivo de descoberta, o token de sessão, a \
                     outbox e as capturas recebidas.",
                )
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
            .aria_label("Testar conexão")
            .on_click(cx.listener(|this, _, _, cx| {
                if !this.busy {
                    this.run(|backend| Outcome::Checked(backend.check()), cx);
                }
            }))
            .child(if self.busy && checks.is_none() {
                "Testando…"
            } else if checks.is_some() {
                "Testar de novo"
            } else {
                "Testar conexão"
            });
        card(
            theme,
            IconName::Activity,
            "Teste de conexão",
            "O caminho que uma captura percorre do OpenCode até este app.",
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
                .child("Nenhum adapter enviou capturas ainda.")
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
                        .child(icon_tile(
                            theme,
                            IconName::Link,
                            colors.text_secondary(),
                            32.0,
                        ))
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
                                                "Compatível",
                                            ),
                                            Compatibility::Unknown => status_pill(
                                                theme,
                                                colors.text_muted(),
                                                "Versão não informada",
                                            ),
                                        }),
                                )
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(colors.text_muted())
                                        .child(format!(
                                            "{} sessão(ões) · última captura em {}",
                                            adapter.sessions,
                                            date_time(&adapter.last_received_at)
                                        )),
                                ),
                        )
                }))
                .into_any_element()
        };
        card(
            theme,
            IconName::Link,
            "Adapters",
            "Integrações que já entregaram capturas a este app.",
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
            ("Pendentes", outbox.pending, colors.status_info()),
            ("Aceitas", outbox.accepted, colors.status_success()),
            ("Rejeitadas", outbox.rejected, colors.status_danger()),
            ("Paradas", outbox.stalled, colors.status_warning()),
        ];
        let stalled = outbox.stalled;
        card(
            theme,
            IconName::Layers,
            "Fila de arquivos",
            "Onde o adapter guarda capturas enquanto o app está fechado.",
        )
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
                            format!("{} (ainda não criada)", outbox.path.display())
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
                            .child(
                                "Paradas são capturas de projetos não cadastrados. Cadastre o \
                                 projeto e mande-as de volta para a fila.",
                            ),
                    )
                    .child(
                        action_button(theme, "opencode-retry", ButtonKind::Secondary, !self.busy)
                            .aria_label("Reenviar capturas paradas")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.busy {
                                    this.run(
                                        |backend| Outcome::Retried(backend.retry_stalled()),
                                        cx,
                                    );
                                }
                            }))
                            .child("Reenviar paradas"),
                    ),
            )
        })
    }

    fn render_connection(theme: &Theme, status: &IntegrationStatus) -> Div {
        card(
            theme,
            IconName::Info,
            "Conexão local",
            "Como o adapter encontra este app.",
        )
        .child(
            card_body()
                .gap(px(0.0))
                .child(kv_row(
                    theme,
                    "API local",
                    status
                        .api
                        .map(|api| {
                            format!(
                                "127.0.0.1:{} · protocolo v{}",
                                api.port, api.protocol_version
                            )
                        })
                        .unwrap_or_else(|| "Inativa".into()),
                    true,
                    false,
                ))
                .child(kv_row(
                    theme,
                    "Contrato de captura",
                    format!("v{}", status.contract_version),
                    true,
                    true,
                ))
                .child(kv_row(
                    theme,
                    "Pasta de dados",
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
                .aria_label("Tentar de novo")
                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx)))
                .child("Tentar de novo")
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
        CheckOutcome::Ok => (IconName::CheckCircle, colors.status_success(), "OK"),
        CheckOutcome::Warning => (IconName::Info, colors.status_warning(), "Atenção"),
        CheckOutcome::Failed => (IconName::Info, colors.status_danger(), "Falha"),
        CheckOutcome::Skipped => (IconName::Circle, colors.text_muted(), "Não se aplica"),
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
                            Some(at) => format!("{} Última em {}.", check.message, date_time(at)),
                            None => check.message.clone(),
                        }),
                ),
        )
}

fn check_label(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::LocalApi => "API local",
        CheckKind::Discovery => "Arquivo de descoberta",
        CheckKind::Token => "Token de sessão",
        CheckKind::Outbox => "Outbox",
        CheckKind::Captures => "Capturas recebidas",
    }
}

fn adapter_label(name: &str) -> String {
    match name {
        "opencode" => "OpenCode".into(),
        other => other.to_owned(),
    }
}
