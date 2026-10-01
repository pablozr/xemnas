//! Settings → Diagnóstico: how the pipeline is doing, what was lost, and a
//! sanitized document to attach to a bug report.
//!
//! The numbers come from `Diagnostics::export`, which is sanitized by
//! construction (no artifact, decision or credential text). Job actions go
//! through `Jobs::reprocess` and `Jobs::cancel`; the running worker picks a
//! requeued job up on its next poll.

use std::path::PathBuf;

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
use crate::ui::patterns::{error_banner, skeleton_list, status_pill, toast, TOAST_DURATION};
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
}

/// The diagnostics and jobs use cases over one store.
pub struct DiagnosticsService<S, P, K> {
    diagnostics: Diagnostics<S, P, K>,
    jobs: Jobs<S>,
}

impl<S, P, K> DiagnosticsService<S, P, K> {
    /// Wraps the use cases built by the composition root.
    pub fn new(diagnostics: Diagnostics<S, P, K>, jobs: Jobs<S>) -> Self {
        Self { diagnostics, jobs }
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
}

enum Outcome {
    Loaded(Result<Box<DiagnosticsDocument>, String>),
    Changed(Result<&'static str, String>),
    Saved(Result<Option<PathBuf>, String>),
}

/// Settings → Diagnóstico.
pub struct DiagnosticsPanel {
    backend: Option<Box<dyn DiagnosticsBackend>>,
    busy: bool,
    document: Option<DiagnosticsDocument>,
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
            error: None,
            notice: None,
        }
    }

    /// Reloads the document; called each time the section opens.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.run(
            |backend| Outcome::Loaded(backend.document().map(Box::new)),
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
                    Outcome::Loaded(Ok(document)) => this.document = Some(*document),
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
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S5))
                    .child(Self::render_metrics(&theme, &document))
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
