//! Settings → Diagnóstico: how the pipeline is doing, what was lost, and a
//! sanitized document to attach to a bug report.
//!
//! The numbers come from `Diagnostics::export`, which is sanitized by
//! construction (no artifact, decision or credential text). Job actions go
//! through `Jobs::reprocess` and `Jobs::cancel`; the running worker picks a
//! requeued job up on its next poll.

use std::path::PathBuf;

use application::auto_approval::{ApprovalsApi, Blocked, Mode, Status, MIN_AUDITS};
use application::calibration::{Verdict, MIN_DECIDED, PREDICTS_AT};
use application::diagnostics::{
    Diagnostics, DiagnosticsDocument, DiagnosticsStore, Distribution, JobDiagnostic,
};
use application::export::{write_pack, ExportFormat, PackDocument};
use application::jobs::{JobRepository, Jobs};
use application::profile::{ProfileStore, SecretStore};
use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Div, Render, Role, Window};

use super::parts::{card, card_body, card_footer, stat_tile};
use crate::screens::format::{date_time, plural};
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{error_banner, meter, skeleton_list, status_pill, toast, TOAST_DURATION};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};

/// What the Diagnóstico section needs from the application layer.
pub trait DiagnosticsBackend: Send + 'static {
    /// The sanitized diagnostics document.
    fn document(&self) -> Result<DiagnosticsDocument, String>;
    /// Requeues a failed job.
    fn reprocess(&self, job_id: &str) -> Result<(), String>;
    /// Cancels a queued job.
    fn cancel(&self, job_id: &str) -> Result<(), String>;
    /// Where the automatic approval stands.
    fn approval(&self) -> Result<Status, String>;
    /// Turns the automatic approval on or off (on only when nothing blocks it).
    fn set_approval(&self, automatic: bool) -> Result<(), String>;
}

/// The diagnostics and jobs use cases over one store.
pub struct DiagnosticsService<S, P, K> {
    diagnostics: Diagnostics<S, P, K>,
    jobs: Jobs<S>,
    approvals: std::sync::Arc<dyn ApprovalsApi>,
}

impl<S, P, K> DiagnosticsService<S, P, K> {
    /// Wraps the use cases built by the composition root.
    pub fn new(
        diagnostics: Diagnostics<S, P, K>,
        jobs: Jobs<S>,
        approvals: std::sync::Arc<dyn ApprovalsApi>,
    ) -> Self {
        Self {
            diagnostics,
            jobs,
            approvals,
        }
    }
}

impl<S, P, K> DiagnosticsBackend for DiagnosticsService<S, P, K>
where
    S: DiagnosticsStore + JobRepository + Send + 'static,
    P: ProfileStore + Send + 'static,
    K: SecretStore + Send + 'static,
{
    fn document(&self) -> Result<DiagnosticsDocument, String> {
        self.diagnostics.export().map_err(|error| {
            tracing::error!(error = %error, operation = "diagnostics", "export failed");
            "Não foi possível montar o diagnóstico.".to_owned()
        })
    }
    fn reprocess(&self, job_id: &str) -> Result<(), String> {
        self.jobs.reprocess(job_id).map_err(|error| {
            tracing::error!(error = %error, operation = "job_reprocess", "reprocess failed");
            "Não foi possível reprocessar a tarefa; ela pode já ter mudado de estado.".to_owned()
        })
    }
    fn cancel(&self, job_id: &str) -> Result<(), String> {
        self.jobs.cancel(job_id).map_err(|error| {
            tracing::error!(error = %error, operation = "job_cancel", "cancel failed");
            "Não foi possível cancelar a tarefa; ela pode já ter começado.".to_owned()
        })
    }
    fn approval(&self) -> Result<Status, String> {
        self.approvals.status().map_err(|error| {
            tracing::error!(error = %error, operation = "approval_status", "status failed");
            "Não foi possível ler o estado da aprovação automática.".to_owned()
        })
    }
    fn set_approval(&self, automatic: bool) -> Result<(), String> {
        let mode = if automatic {
            Mode::Automatic
        } else {
            Mode::Manual
        };
        self.approvals.set_mode(mode).map_err(|error| match error {
            application::auto_approval::ApprovalError::Blocked(_) => {
                "A aprovação automática ainda não pode ser ligada.".to_owned()
            }
            other => {
                tracing::error!(error = %other, operation = "approval_mode", "change failed");
                "Não foi possível mudar a aprovação automática.".to_owned()
            }
        })
    }
}

enum Outcome {
    Loaded(Result<Box<(DiagnosticsDocument, Option<Status>)>, String>),
    Changed(Result<&'static str, String>),
    Saved(Result<Option<PathBuf>, String>),
}

/// Settings → Diagnóstico.
pub struct DiagnosticsPanel {
    backend: Option<Box<dyn DiagnosticsBackend>>,
    busy: bool,
    document: Option<DiagnosticsDocument>,
    approval: Option<Status>,
    error: Option<String>,
    notice: Option<String>,
}

impl DiagnosticsPanel {
    /// Mounts the section; nothing is read until [`Self::refresh`].
    pub fn new(backend: Box<dyn DiagnosticsBackend>) -> Self {
        Self {
            backend: Some(backend),
            busy: false,
            document: None,
            approval: None,
            error: None,
            notice: None,
        }
    }

    /// Reloads the document; called each time the section opens.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.run(
            |backend| {
                Outcome::Loaded(
                    backend
                        .document()
                        .map(|document| Box::new((document, backend.approval().ok()))),
                )
            },
            cx,
        );
    }

    fn run(
        &mut self,
        operation: impl FnOnce(&dyn DiagnosticsBackend) -> Outcome + Send + 'static,
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
                    Outcome::Loaded(Ok(loaded)) => {
                        let (document, approval) = *loaded;
                        this.document = Some(document);
                        this.approval = approval;
                    }
                    Outcome::Changed(Ok(message)) => {
                        this.show_notice(message.to_owned(), cx);
                        this.refresh(cx);
                    }
                    Outcome::Saved(Ok(Some(path))) => {
                        this.show_notice(format!("Diagnóstico salvo em {}", path.display()), cx)
                    }
                    Outcome::Saved(Ok(None)) => {}
                    Outcome::Loaded(Err(error))
                    | Outcome::Changed(Err(error))
                    | Outcome::Saved(Err(error)) => this.error = Some(error),
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

    fn set_approval(&mut self, automatic: bool, cx: &mut Context<Self>) {
        self.run(
            move |backend| {
                Outcome::Changed(backend.set_approval(automatic).map(|_| {
                    if automatic {
                        "Aprovação automática ligada."
                    } else {
                        "Aprovação automática desligada."
                    }
                }))
            },
            cx,
        );
    }

    fn export(&mut self, cx: &mut Context<Self>) {
        let Some(document) = self.document.clone() else {
            return;
        };
        self.run(
            move |_| {
                let content = match serde_json::to_string_pretty(&document) {
                    Ok(mut json) => {
                        json.push('\n');
                        json
                    }
                    Err(_) => {
                        return Outcome::Saved(Err("Não foi possível gerar o arquivo.".into()))
                    }
                };
                let Some(path) = rfd::FileDialog::new()
                    .set_title("Exportar diagnóstico")
                    .add_filter("JSON", &["json"])
                    .set_file_name("xemnas-diagnostico.json")
                    .save_file()
                else {
                    return Outcome::Saved(Ok(None));
                };
                // The native dialog already confirmed any replacement. The write
                // reuses the atomic writer of the exports.
                let written = write_pack(
                    &PackDocument {
                        format: ExportFormat::Json,
                        bytes: content.len(),
                        content,
                    },
                    &path,
                    true,
                );
                Outcome::Saved(
                    written
                        .map(|_| Some(path))
                        .map_err(|_| "Não foi possível salvar o arquivo nesse destino.".into()),
                )
            },
            cx,
        );
    }

    fn render_metrics(theme: &Theme, document: &DiagnosticsDocument) -> Div {
        let metrics = &document.metrics;
        let colors = theme.colors;
        let noise = metrics
            .noise
            .dismissed_ratio
            .map(|ratio| format!("{:.0}%", ratio * 100.0))
            .unwrap_or_else(|| "—".into());
        let context = &metrics.context;
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                div()
                    .flex()
                    .gap(px(SpacingScale::S6))
                    .px(px(SpacingScale::S5))
                    .py(px(SpacingScale::S4))
                    .rounded(RadiusScale.surface())
                    .border_1()
                    .border_color(colors.glass_border_card())
                    .bg(colors.glass_fill_card())
                    .child(stat_tile(
                        theme,
                        "Captura → candidato",
                        median(&metrics.latency_capture_to_candidate_ms),
                        None,
                    ))
                    .child(stat_tile(
                        theme,
                        "Tempo de revisão",
                        median(&metrics.review_time_ms),
                        None,
                    ))
                    .child(stat_tile(theme, "Descartados", noise, None))
                    .child(stat_tile(
                        theme,
                        "Blocos de contexto",
                        (context.inject.blocks + context.shadow.blocks).to_string(),
                        None,
                    )),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(format!(
                        "Medianas com p95 de {} e {} · {} candidatos decididos · contexto: {} \
                         enviados, {} só medidos",
                        p95(&metrics.latency_capture_to_candidate_ms),
                        p95(&metrics.review_time_ms),
                        metrics.noise.decided_total,
                        context.inject.blocks,
                        context.shadow.blocks
                    )),
            )
    }

    /// Whether the extractor's confidence predicts what is accepted: the
    /// gate for any automatic approval (docs/pesquisas/exibicao-e-aprovacao.md).
    fn render_calibration(theme: &Theme, document: &DiagnosticsDocument) -> Div {
        let calibration = &document.metrics.calibration;
        let colors = theme.colors;
        let (tone, verdict) = match calibration.verdict {
            Verdict::TooFew => (
                colors.text_muted(),
                format!(
                    concat!(
                        "Ainda não dá para saber: {} de {} decisões, e é preciso ter aceitado ",
                        "e descartado pelo menos uma."
                    ),
                    calibration.decided, MIN_DECIDED
                ),
            ),
            Verdict::NoSignal => (
                colors.status_danger(),
                concat!(
                    "A confiança não separa o que você aceita do que descarta. ",
                    "Aprovação automática não deve se apoiar nela."
                )
                .to_owned(),
            ),
            Verdict::Weak => (
                colors.status_warning(),
                concat!(
                    "A confiança separa um pouco o que você aceita, mas não o bastante ",
                    "para aprovar sozinha."
                )
                .to_owned(),
            ),
            Verdict::Predicts => (
                colors.status_success(),
                concat!(
                    "A confiança prevê o que você aceita: ",
                    "dá para apoiar aprovação automática nela."
                )
                .to_owned(),
            ),
        };
        let rows = calibration
            .bins
            .iter()
            .filter(|bin| bin.total > 0)
            .map(|bin| {
                let share = bin.kept_share().unwrap_or(0.0) as f32;
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .child(
                        text_style(div(), TypeScale::META)
                            .w(px(72.0))
                            .flex_none()
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_secondary())
                            .child(format!("{:.0}–{:.0}%", bin.from * 100.0, bin.to * 100.0)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(meter(theme, share, colors.accent_default())),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .w(px(150.0))
                            .flex_none()
                            .text_color(colors.text_muted())
                            .child(format!(
                                "{:.0}% mantidas · {}",
                                share * 100.0,
                                plural(bin.total as usize, "decisão", "decisões")
                            )),
                    )
            });
        card(
            theme,
            "A confiança da extração presta?",
            "Entre os candidatos que você já decidiu: quanto da faixa de confiança foi aceito.",
        )
        .child(
            card_body()
                .gap(px(SpacingScale::S3))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(SpacingScale::S3))
                        .child(
                            div()
                                .mt(px(5.0))
                                .size(px(6.0))
                                .flex_none()
                                .rounded_full()
                                .bg(tone),
                        )
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .min_w(px(0.0))
                                .text_color(colors.text_primary())
                                .child(verdict),
                        ),
                )
                .children(rows)
                .children(calibration.confidence_separation.map(|value| {
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(format!(
                            concat!(
                                "Separação {:.2} (0,50 é acaso; {:.2} ou mais prevê) · ",
                                "{} aceitas, {} editadas, {} descartadas"
                            ),
                            value,
                            PREDICTS_AT,
                            calibration.accepted,
                            calibration.edited,
                            calibration.dismissed
                        ))
                })),
        )
    }

    /// The automatic approval: what it does, where it stands and the one
    /// switch. It is closed by default and opens only when the numbers allow.
    fn render_approval(&self, theme: &Theme, status: &Status, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let checks = status.health.checked;
        let agreed = status.health.agreed;
        let (tone, line) = match (status.automatic, status.blocked()) {
            (true, _) => (
                colors.status_success(),
                format!(
                    "Ligada. {} aguardando; um em dez é conferido às cegas. {agreed} de {checks} \
                     conferências recentes concordaram.",
                    status.held
                ),
            ),
            (false, Some(Blocked::NotCalibrated)) => (
                colors.text_muted(),
                "Bloqueada: a confiança da extração ainda não prevê o que você aceita (cartão \
                 acima)."
                    .to_owned(),
            ),
            (false, Some(Blocked::BandUnproven)) => (
                colors.text_muted(),
                "Bloqueada: a faixa de 85% a 100% ainda não tem 10 decisões com 90% mantidas."
                    .to_owned(),
            ),
            (false, Some(Blocked::Unobserved)) => (
                colors.status_info(),
                format!(
                    "Em observação: {checks} de {MIN_AUDITS} conferências. O sistema registra o \
                     que aprovaria e compara com o que você decide, sem aceitar nada."
                ),
            ),
            (false, Some(Blocked::Tripped)) => (
                colors.status_warning(),
                format!(
                    "Fechada: as conferências recentes discordaram ({agreed} de {checks} \
                     concordaram). Ela volta a poder ser ligada quando concordarem de novo."
                ),
            ),
            (false, None) => (
                colors.status_success(),
                format!(
                    "Pronta para ligar: {agreed} de {checks} conferências recentes concordaram."
                ),
            ),
        };
        let automatic = status.automatic;
        let blocked = !automatic && status.blocked().is_some();
        let switch = action_button(
            theme,
            "approval-switch",
            ButtonKind::Secondary,
            !self.busy && !blocked,
        )
        .aria_label(if automatic {
            "Desligar a aprovação automática"
        } else {
            "Ligar a aprovação automática"
        })
        .on_click(cx.listener(move |this, _, _, cx| this.set_approval(!automatic, cx)))
        .child(if automatic { "Desligar" } else { "Ligar" });
        card(
            theme,
            "Aprovação automática",
            "Decisões de alta confiança esperam 24 h na Revisão e entram sozinhas se você não \
             agir. Desligada por padrão.",
        )
        .child(
            card_body()
                .gap(px(SpacingScale::S3))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(SpacingScale::S3))
                        .child(div().mt(px(5.0)).size(px(6.0)).flex_none().rounded_full().bg(tone))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .flex_1()
                                .min_w(px(0.0))
                                .text_color(colors.text_primary())
                                .child(line),
                        )
                        .child(switch),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child(
                            "Regras, decisões parecidas com o que já foi registrado, pouca evidência \
                             e mudanças em muitos arquivos ficam sempre para você.",
                        ),
                ),
        )
    }

    fn render_losses(theme: &Theme, document: &DiagnosticsDocument) -> Div {
        let losses = &document.metrics.losses;
        let colors = theme.colors;
        let rows = [
            (
                "Análises que falharam",
                losses.assessments_failed,
                colors.status_danger(),
            ),
            (
                "Análises puladas por falta de consentimento",
                losses.assessments_skipped,
                colors.status_warning(),
            ),
            (
                "Tarefas que falharam",
                losses.jobs_failed,
                colors.status_danger(),
            ),
            (
                "Capturas rejeitadas na outbox",
                losses.outbox_rejected,
                colors.status_danger(),
            ),
        ];
        card(
            theme,
            "Perdas",
            "Trabalho que não virou candidato. Zero em tudo é o esperado.",
        )
        .child(
            card_body()
                .gap(px(0.0))
                .children(
                    rows.into_iter()
                        .enumerate()
                        .map(|(index, (label, value, color))| {
                            div()
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S3))
                                .py(px(SpacingScale::S2))
                                .when(index > 0, |row| {
                                    row.border_t_1().border_color(colors.hairline_divider())
                                })
                                .child(div().size(px(6.0)).flex_none().rounded_full().bg(
                                    if value > 0 {
                                        color
                                    } else {
                                        colors.hairline_divider()
                                    },
                                ))
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .flex_1()
                                        .text_color(colors.text_secondary())
                                        .child(label),
                                )
                                .child(
                                    text_style(div(), TypeScale::ROW_TITLE)
                                        .text_color(if value > 0 {
                                            color
                                        } else {
                                            colors.text_muted()
                                        })
                                        .child(value.to_string()),
                                )
                        }),
                ),
        )
    }

    fn render_jobs(&self, theme: &Theme, jobs: &[JobDiagnostic], cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let body: AnyElement = if jobs.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child("Nenhuma tarefa registrada ainda. Cada captura recebida gera uma análise.")
                .into_any_element()
        } else {
            div()
                .id("diagnostics-jobs")
                .flex()
                .flex_col()
                .role(Role::List)
                .children(jobs.iter().enumerate().map(|(index, job)| {
                    let (color, state) = job_state(theme, &job.state);
                    let id = job.id.clone();
                    let action = match job.state.as_str() {
                        "failed" => Some(("Reprocessar", true)),
                        "queued" => Some(("Cancelar", false)),
                        _ => None,
                    };
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S3))
                        .py(px(SpacingScale::S2))
                        .when(index > 0, |row| {
                            row.border_t_1().border_color(colors.hairline_divider())
                        })
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
                                                .child(job_kind(&job.kind)),
                                        )
                                        .child(status_pill(theme, color, state)),
                                )
                                .child(
                                    text_style(div(), TypeScale::META)
                                        .text_color(colors.text_muted())
                                        .child(format!(
                                            "{} · {} tentativa(s){}",
                                            date_time(&job.updated_at),
                                            job.attempts,
                                            job.error_code
                                                .as_ref()
                                                .map(|code| format!(" · código {code}"))
                                                .unwrap_or_default()
                                        )),
                                ),
                        )
                        .children(action.map(|(label, reprocess)| {
                            action_button(
                                theme,
                                ("diagnostics-job", index),
                                ButtonKind::Ghost,
                                !self.busy,
                            )
                            .aria_label(label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if this.busy {
                                    return;
                                }
                                let id = id.clone();
                                this.run(
                                    move |backend| {
                                        Outcome::Changed(if reprocess {
                                            backend
                                                .reprocess(&id)
                                                .map(|()| "Tarefa de volta na fila.")
                                        } else {
                                            backend.cancel(&id).map(|()| "Tarefa cancelada.")
                                        })
                                    },
                                    cx,
                                );
                            }))
                            .child(label)
                        }))
                }))
                .into_any_element()
        };
        card(
            theme,
            "Tarefas recentes",
            "As últimas análises de captura. Falhas podem ser reprocessadas.",
        )
        .child(card_body().child(body))
    }

    fn render_about(
        &self,
        theme: &Theme,
        document: &DiagnosticsDocument,
        cx: &mut Context<Self>,
    ) -> Div {
        let counts = &document.counts;
        let colors = theme.colors;
        let facts = [
            (
                "Versão",
                format!(
                    "{} · banco v{}",
                    document.schema.app_version, document.schema.migrations_version
                ),
            ),
            (
                "Dados",
                [
                    plural(counts.projects as usize, "projeto", "projetos"),
                    plural(counts.captures as usize, "captura", "capturas"),
                    plural(counts.decisions as usize, "decisão", "decisões"),
                    plural(counts.revisions as usize, "revisão", "revisões"),
                ]
                .join(" · "),
            ),
            (
                "Extração",
                match document.ai_profile.provider.as_deref() {
                    Some(host) => format!("externa via {host} ({})", document.runtime.mode),
                    None => format!("local ({})", document.runtime.mode),
                },
            ),
        ];
        card(
            theme,
            "Exportar diagnóstico",
            "Um JSON com contagens, métricas e códigos de erro. Sem conversas, diffs, decisões \
             ou credenciais.",
        )
        .child(
            card_body()
                .gap(px(0.0))
                .children(
                    facts
                        .into_iter()
                        .enumerate()
                        .map(|(index, (label, value))| {
                            super::parts::kv_row(theme, label, value, false, index > 0)
                        }),
                ),
        )
        .child(
            card_footer(theme)
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .text_color(colors.text_muted())
                        .child("Anexe a um relato de problema; revise o arquivo antes de enviar."),
                )
                .child(
                    action_button(theme, "diagnostics-export", ButtonKind::Primary, !self.busy)
                        .aria_label("Exportar diagnóstico")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if !this.busy {
                                this.export(cx);
                            }
                        }))
                        .child(icon(IconName::Export, 14.0, colors.accent_on_emphasis()))
                        .child("Exportar…"),
                ),
        )
    }
}

impl Render for DiagnosticsPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let content: AnyElement = match self.document.clone() {
            None if self.error.is_none() => skeleton_list(&theme, "diagnostics-skeleton", 4),
            None => div().into_any_element(),
            Some(document) => {
                let jobs = self.render_jobs(&theme, &document.recent_jobs, cx);
                let about = self.render_about(&theme, &document, cx);
                let approval = self
                    .approval
                    .clone()
                    .map(|status| self.render_approval(&theme, &status, cx));
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S5))
                    .child(Self::render_metrics(&theme, &document))
                    .child(Self::render_calibration(&theme, &document))
                    .children(approval)
                    .child(Self::render_losses(&theme, &document))
                    .child(jobs)
                    .child(about)
                    .into_any_element()
            }
        };
        let retry = self.error.is_some().then(|| {
            action_button(&theme, "diagnostics-reload", ButtonKind::Ghost, !self.busy)
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

/// `p50` as a short duration, or a dash without samples.
fn median(distribution: &Distribution) -> String {
    distribution.p50.map(duration).unwrap_or_else(|| "—".into())
}

fn p95(distribution: &Distribution) -> String {
    distribution.p95.map(duration).unwrap_or_else(|| "—".into())
}

/// `850 ms`, `12 s`, `4 min`, `3 h`, `2 d`.
fn duration(ms: i64) -> String {
    let ms = ms.max(0);
    match ms {
        0..=999 => format!("{ms} ms"),
        1_000..=59_999 => format!("{} s", ms / 1_000),
        60_000..=3_599_999 => format!("{} min", ms / 60_000),
        3_600_000..=86_399_999 => format!("{} h", ms / 3_600_000),
        _ => format!("{} d", ms / 86_400_000),
    }
}

fn job_state(theme: &Theme, state: &str) -> (gpui::Rgba, &'static str) {
    let colors = theme.colors;
    match state {
        "queued" => (colors.status_info(), "Na fila"),
        "running" => (colors.accent_hover(), "Executando"),
        "completed" => (colors.status_success(), "Concluída"),
        "failed" => (colors.status_danger(), "Falhou"),
        "cancelled" => (colors.text_muted(), "Cancelada"),
        _ => (colors.text_muted(), "Desconhecido"),
    }
}

fn job_kind(kind: &str) -> String {
    match kind {
        application::jobs::ANALYZE_CAPTURE_KIND => "Análise de captura".into(),
        other => other.replace('_', " "),
    }
}

#[cfg(test)]
mod tests {
    use super::duration;

    #[test]
    fn durations_use_the_largest_whole_unit() {
        assert_eq!(duration(0), "0 ms");
        assert_eq!(duration(999), "999 ms");
        assert_eq!(duration(1_500), "1 s");
        assert_eq!(duration(90_000), "1 min");
        assert_eq!(duration(7_200_000), "2 h");
        assert_eq!(duration(172_800_000), "2 d");
        assert_eq!(duration(-5), "0 ms");
    }
}
