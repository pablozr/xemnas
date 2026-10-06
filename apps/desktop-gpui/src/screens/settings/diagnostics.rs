//! Settings → Diagnóstico: how the pipeline is doing, what was lost, and a
//! sanitized document to attach to a bug report.
//!
//! The numbers come from `Diagnostics::export`, which is sanitized by
//! construction (no artifact, decision or credential text). Job actions go
//! through `Jobs::reprocess` and `Jobs::cancel`; the running worker picks a
//! requeued job up on its next poll.

use std::path::PathBuf;

use application::calibration::{Verdict, MIN_DECIDED, PREDICTS_AT};
use application::diagnostics::{
    Diagnostics, DiagnosticsDocument, DiagnosticsStore, Distribution, JobDiagnostic,
};
use application::export::{write_pack, ExportFormat, PackDocument};
use application::jobs::{JobRepository, JobSettingsStore, JobSummary, Jobs, Lane};
use application::profile::{ProfileStore, SecretStore};
use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Div, Render, Role, Window};

use super::parts::{card, card_body, card_footer, stat_tile};
use crate::i18n::settings as t;
use crate::screens::format::date_time;
use crate::screens::inbox::{capture_facts, capture_pill, capture_summary};
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    error_banner, meter, segment_label, segmented, skeleton_list, status_pill, toast, word_wrapped,
    TOAST_DURATION,
};
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
    /// Queued, running and failed jobs per lane.
    fn lanes(&self) -> Result<Vec<(Lane, JobSummary)>, String>;
    /// "Análises em paralelo" as saved.
    fn parallel(&self) -> Result<u8, String>;
    /// Saves "análises em paralelo"; it applies on the next start.
    fn set_parallel(&self, value: u8) -> Result<(), String>;
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
    S: DiagnosticsStore + JobRepository + JobSettingsStore + Send + 'static,
    P: ProfileStore + Send + 'static,
    K: SecretStore + Send + 'static,
{
    fn document(&self) -> Result<DiagnosticsDocument, String> {
        self.diagnostics.export().map_err(|error| {
            tracing::error!(error = %error, operation = "diagnostics", "export failed");
            t::diag_err_document().to_owned()
        })
    }
    fn reprocess(&self, job_id: &str) -> Result<(), String> {
        self.jobs.reprocess(job_id).map_err(|error| {
            tracing::error!(error = %error, operation = "job_reprocess", "reprocess failed");
            t::diag_err_reprocess().to_owned()
        })
    }
    fn cancel(&self, job_id: &str) -> Result<(), String> {
        self.jobs.cancel(job_id).map_err(|error| {
            tracing::error!(error = %error, operation = "job_cancel", "cancel failed");
            t::diag_err_cancel().to_owned()
        })
    }
    fn lanes(&self) -> Result<Vec<(Lane, JobSummary)>, String> {
        self.jobs.lane_summaries().map_err(|error| {
            tracing::error!(error = %error, operation = "job_lanes", "lane counts failed");
            t::diag_err_lanes().to_owned()
        })
    }
    fn parallel(&self) -> Result<u8, String> {
        self.jobs.parallel_analyses().map_err(|error| {
            tracing::error!(error = %error, operation = "job_parallel", "read failed");
            t::diag_err_parallel_read().to_owned()
        })
    }
    fn set_parallel(&self, value: u8) -> Result<(), String> {
        self.jobs.set_parallel_analyses(value).map_err(|error| {
            tracing::error!(error = %error, operation = "job_parallel", "save failed");
            t::diag_err_parallel_save().to_owned()
        })
    }
}

/// What one refresh reads: the document, the lanes and the setting.
struct Snapshot {
    document: DiagnosticsDocument,
    lanes: Vec<(Lane, JobSummary)>,
    parallel: u8,
}

impl Snapshot {
    fn read(backend: &dyn DiagnosticsBackend) -> Result<Box<Self>, String> {
        Ok(Box::new(Self {
            document: backend.document()?,
            lanes: backend.lanes()?,
            parallel: backend.parallel()?,
        }))
    }
}

enum Outcome {
    Loaded(Result<Box<Snapshot>, String>),
    Changed(Result<&'static str, String>),
    Saved(Result<Option<PathBuf>, String>),
}

/// Settings → Diagnóstico.
pub struct DiagnosticsPanel {
    backend: Option<Box<dyn DiagnosticsBackend>>,
    busy: bool,
    document: Option<DiagnosticsDocument>,
    lanes: Vec<(Lane, JobSummary)>,
    parallel: Option<u8>,
    error: Option<String>,
    notice: Option<String>,
    progress_read: Option<crate::screens::inbox::ProgressRead>,
    progress_retry: Option<crate::screens::inbox::ProgressRetry>,
    project: Option<String>,
    generation: u64,
    progress_busy: bool,
    progress_error: bool,
    progress_action_error: Option<String>,
    progress: Vec<application::capture_progress::CaptureProgress>,
}

impl DiagnosticsPanel {
    /// Mounts the section; nothing is read until [`Self::refresh`].
    pub fn new(backend: Box<dyn DiagnosticsBackend>) -> Self {
        Self {
            backend: Some(backend),
            busy: false,
            document: None,
            lanes: Vec::new(),
            parallel: None,
            error: None,
            notice: None,
            progress_read: None,
            progress_retry: None,
            project: None,
            generation: 0,
            progress_busy: false,
            progress_error: false,
            progress_action_error: None,
            progress: Vec::new(),
        }
    }

    /// Reloads the document; called each time the section opens.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.poll_progress(cx);
        self.run(|backend| Outcome::Loaded(Snapshot::read(backend)), cx);
    }

    /// Installs bounded application reads and authorized capture retry.
    pub fn set_capture_progress(
        &mut self,
        read: crate::screens::inbox::ProgressRead,
        retry: crate::screens::inbox::ProgressRetry,
    ) {
        self.progress_read = Some(read);
        self.progress_retry = Some(retry);
    }

    /// Invalidates old project reads without refreshing hidden diagnostics.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if project == self.project {
            return;
        }
        self.project = project;
        self.generation += 1;
        self.progress.clear();
        self.progress_error = false;
        self.progress_action_error = None;
        cx.notify();
    }

    fn poll_progress(&mut self, cx: &mut Context<Self>) {
        if self.progress_busy {
            return;
        }
        let (Some(read), Some(project)) = (self.progress_read.clone(), self.project.clone()) else {
            return;
        };
        self.progress_busy = true;
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { read(&project) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.progress_busy = false;
                if accepts_progress(generation, this.generation) {
                    match result {
                        Ok(rows) => {
                            this.progress = rows;
                            this.progress_error = false;
                        }
                        Err(_) => this.progress_error = true,
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn retry_capture(&mut self, job: String, cx: &mut Context<Self>) {
        if self.progress_busy {
            return;
        }
        let Some(retry) = self.progress_retry.clone() else {
            return;
        };
        let retry_job = job.clone();
        self.progress_busy = true;
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { retry(&retry_job) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.progress_busy = false;
                if accepts_progress(generation, this.generation) {
                    crate::screens::inbox::record_retry_result(
                        &mut this.progress_action_error,
                        &job,
                        result.is_ok(),
                    );
                    if result.is_ok() {
                        this.show_notice(t::diag_capture_requeued().into(), cx);
                        this.poll_progress(cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
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
                    Outcome::Loaded(Ok(snapshot)) => {
                        let Snapshot {
                            document,
                            lanes,
                            parallel,
                        } = *snapshot;
                        this.document = Some(document);
                        this.lanes = lanes;
                        this.parallel = Some(parallel);
                    }
                    Outcome::Changed(Ok(message)) => {
                        this.show_notice(message.to_owned(), cx);
                        this.refresh(cx);
                    }
                    Outcome::Saved(Ok(Some(path))) => {
                        this.show_notice(t::diag_saved_at(&path.display().to_string()), cx)
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
                    Err(_) => return Outcome::Saved(Err(t::diag_err_file().into())),
                };
                let Some(path) = rfd::FileDialog::new()
                    .set_title(t::diag_export_title())
                    .add_filter("JSON", &["json"])
                    .set_file_name(t::diag_file_name())
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
                        .map_err(|_| t::diag_err_save_file().into()),
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
                        t::diag_stat_latency(),
                        median(&metrics.latency_capture_to_candidate_ms),
                        None,
                    ))
                    .child(stat_tile(
                        theme,
                        t::diag_stat_review_time(),
                        median(&metrics.review_time_ms),
                        None,
                    ))
                    .child(stat_tile(theme, t::diag_stat_dismissed(), noise, None))
                    .child(stat_tile(
                        theme,
                        t::diag_stat_context_blocks(),
                        (context.inject.blocks + context.shadow.blocks).to_string(),
                        None,
                    )),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(t::diag_review_time_note()),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(t::diag_metrics_note(
                        &p95(&metrics.latency_capture_to_candidate_ms),
                        &p95(&metrics.review_time_ms),
                        metrics.noise.decided_total,
                        context.inject.blocks,
                        context.shadow.blocks,
                    )),
            )
    }

    /// Whether the extractor's confidence predicts what is accepted: the
    /// gate for any automatic approval (ADR-0012).
    fn render_calibration(theme: &Theme, document: &DiagnosticsDocument) -> Div {
        let calibration = &document.metrics.calibration;
        let colors = theme.colors;
        let (tone, verdict) = match calibration.verdict {
            Verdict::TooFew => (
                colors.text_muted(),
                t::diag_calib_too_few(calibration.decided, MIN_DECIDED),
            ),
            Verdict::NoSignal => (colors.status_danger(), t::diag_calib_no_signal().to_owned()),
            Verdict::Weak => (colors.status_warning(), t::diag_calib_weak().to_owned()),
            Verdict::Predicts => (colors.status_success(), t::diag_calib_predicts().to_owned()),
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
                            .child(t::diag_bin_label(
                                share * 100.0,
                                &t::decisions_count(bin.total as usize),
                            )),
                    )
            });
        card(theme, t::diag_calib_title(), t::diag_calib_body()).child(
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
                        .child(t::diag_separation(
                            value,
                            PREDICTS_AT,
                            calibration.accepted,
                            calibration.edited,
                            calibration.dismissed,
                        ))
                })),
        )
    }

    fn render_losses(theme: &Theme, document: &DiagnosticsDocument) -> Div {
        let losses = &document.metrics.losses;
        let colors = theme.colors;
        let rows = [
            (
                t::diag_loss_assessments_failed(),
                losses.assessments_failed,
                colors.status_danger(),
            ),
            (
                t::diag_loss_assessments_skipped(),
                losses.assessments_skipped,
                colors.status_warning(),
            ),
            (
                t::diag_loss_jobs_failed(),
                losses.jobs_failed,
                colors.status_danger(),
            ),
            (
                t::diag_loss_outbox_rejected(),
                losses.outbox_rejected,
                colors.status_danger(),
            ),
        ];
        card(theme, t::diag_losses_title(), t::diag_losses_body()).child(
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

    /// Per-lane counts and the "análises em paralelo" choice.
    fn render_lanes(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let rows = self
            .lanes
            .iter()
            .enumerate()
            .map(|(index, (lane, summary))| {
                let (label, hint) = lane_label(*lane);
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
                            .child(text_style(div(), TypeScale::ROW_TITLE).child(label))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(hint),
                            ),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex_none()
                            .text_color(if summary.failed > 0 {
                                colors.status_danger()
                            } else {
                                colors.text_secondary()
                            })
                            .child(t::diag_lane_counts(
                                summary.queued,
                                summary.running,
                                summary.failed,
                            )),
                    )
            });
        let mut track = segmented(theme)
            .id("diagnostics-parallel")
            .role(Role::RadioGroup)
            .aria_label(t::diag_parallel_title());
        for (value, label) in [(1u8, "1"), (2, "2"), (3, "3"), (4, "4")] {
            let selected = self.parallel == Some(value);
            track = track.child(
                segment_label(
                    theme,
                    ("diagnostics-parallel", value as usize),
                    label,
                    selected,
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    if this.busy || this.parallel == Some(value) {
                        return;
                    }
                    this.run(
                        move |backend| {
                            Outcome::Changed(
                                backend
                                    .set_parallel(value)
                                    .map(|()| t::diag_parallel_saved()),
                            )
                        },
                        cx,
                    );
                })),
            );
        }
        card(theme, t::diag_lanes_title(), t::diag_lanes_body())
            .child(card_body().gap(px(0.0)).children(rows))
            .child(
                card_footer(theme)
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            .child(
                                text_style(div(), TypeScale::LABEL)
                                    .text_color(colors.text_secondary())
                                    .child(t::diag_parallel_title()),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(t::diag_parallel_hint()),
                            ),
                    )
                    .child(track),
            )
    }

    fn render_jobs(&self, theme: &Theme, jobs: &[JobDiagnostic], cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let body: AnyElement = if jobs.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child(t::diag_jobs_empty())
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
                        "failed" => Some((t::action_reprocess(), true)),
                        "queued" => Some((t::action_cancel(), false)),
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
                                        .child(t::diag_job_meta(
                                            &date_time(&job.updated_at),
                                            job.attempts,
                                            &job.error_code
                                                .as_deref()
                                                .map(t::diag_job_code)
                                                .unwrap_or_default(),
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
                                            backend.reprocess(&id).map(|()| t::diag_job_requeued())
                                        } else {
                                            backend.cancel(&id).map(|()| t::diag_job_cancelled())
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
        card(theme, t::diag_jobs_title(), t::diag_jobs_body()).child(card_body().child(body))
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
                t::diag_about_version(),
                t::diag_about_version_value(
                    &document.schema.app_version,
                    document.schema.migrations_version,
                ),
            ),
            (
                t::diag_about_data(),
                [
                    t::projects_count(counts.projects as usize),
                    t::captures_count(counts.captures as usize),
                    t::decisions_count(counts.decisions as usize),
                    t::revisions_count(counts.revisions as usize),
                ]
                .join(" · "),
            ),
            (
                t::diag_about_extraction(),
                match document.ai_profile.provider.as_deref() {
                    Some(host) => t::diag_extraction_external(host, &document.runtime.mode),
                    None => t::diag_extraction_local(&document.runtime.mode),
                },
            ),
        ];
        card(theme, t::diag_export_title(), t::diag_export_body())
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
                            .child(t::diag_export_footer()),
                    )
                    .child(
                        action_button(theme, "diagnostics-export", ButtonKind::Primary, !self.busy)
                            .aria_label(t::diag_export_title())
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.busy {
                                    this.export(cx);
                                }
                            }))
                            .child(icon(IconName::Export, 14.0, colors.accent_on_emphasis()))
                            .child(t::diag_export_button()),
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
                let lanes = self.render_lanes(&theme, cx);
                let jobs = self.render_jobs(&theme, &document.recent_jobs, cx);
                let about = self.render_about(&theme, &document, cx);
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S5))
                    .child(Self::render_metrics(&theme, &document))
                    .child(Self::render_calibration(&theme, &document))
                    .child(Self::render_losses(&theme, &document))
                    .child(lanes)
                    .child(jobs)
                    .child(about)
                    .into_any_element()
            }
        };
        let retry = self.error.is_some().then(|| {
            action_button(&theme, "diagnostics-reload", ButtonKind::Ghost, !self.busy)
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
            .child(self.render_progress(&theme, cx))
            .children(
                self.notice
                    .clone()
                    .map(|notice| toast(&theme, &notice, 24.0)),
            )
    }
}

fn accepts_progress(request: u64, current: u64) -> bool {
    request == current
}

impl DiagnosticsPanel {
    fn render_progress(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let mut column = div().flex().flex_col().gap(px(SpacingScale::S3)).child(
            crate::ui::patterns::section_label(theme, t::diag_captures_section()),
        );
        if let Some(job) = self.progress_action_error.clone() {
            column = column.child(
                error_banner(theme, t::diag_err_reprocess_capture())
                    .id("diagnostics-capture-action-error")
                    .role(Role::Alert)
                    .child(
                        action_button(
                            theme,
                            "diagnostics-capture-action-retry",
                            ButtonKind::Secondary,
                            !self.progress_busy,
                        )
                        .aria_label(t::diag_retry_reprocess_label())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.retry_capture(job.clone(), cx)),
                        )
                        .child(t::diag_retry_reprocess()),
                    )
                    .child(
                        action_button(
                            theme,
                            "diagnostics-capture-action-dismiss",
                            ButtonKind::Ghost,
                            true,
                        )
                        .aria_label(t::diag_dismiss_error_label())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.progress_action_error = None;
                            cx.notify();
                        }))
                        .child(t::diag_dismiss()),
                    ),
            );
        }
        if self.progress_error {
            column = column.child(
                error_banner(theme, t::diag_err_captures())
                    .id("diagnostics-captures-error")
                    .role(Role::Alert)
                    .child(
                        action_button(
                            theme,
                            "diagnostics-captures-retry",
                            ButtonKind::Ghost,
                            !self.progress_busy,
                        )
                        .aria_label(t::diag_refresh_captures_label())
                        .on_click(cx.listener(|this, _, _, cx| this.poll_progress(cx)))
                        .child(t::action_try_again()),
                    ),
            );
        }
        if self.project.is_none() {
            return column.child(t::diag_select_project());
        }
        if self.progress.is_empty() {
            return column.child(if self.progress_busy {
                t::diag_loading_captures()
            } else if self.progress_error {
                t::diag_captures_unavailable()
            } else {
                t::diag_no_captures()
            });
        }
        column = column.child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_secondary())
                .child(capture_summary(&self.progress)),
        );
        for (index, capture) in self.progress.iter().enumerate() {
            let (pill_color, pill_label) = capture_pill(theme, capture);
            let title = capture
                .title
                .as_deref()
                .unwrap_or(crate::i18n::inbox::capture_untitled());
            let mut row = div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(SpacingScale::S3))
                        .child(word_wrapped(title, TypeScale::BODY, Some(2)))
                        .child(status_pill(theme, pill_color, pill_label)),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_muted())
                        .child(capture_facts(capture)),
                );
            if capture.can_retry {
                if let Some(job) = capture.job_id.clone() {
                    row = row.child(
                        action_button(
                            theme,
                            ("diagnostics-capture-reprocess", index),
                            ButtonKind::Secondary,
                            !self.progress_busy,
                        )
                        .aria_label(t::diag_reprocess_capture_label())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.retry_capture(job.clone(), cx)),
                        )
                        .self_start()
                        .child(t::action_reprocess()),
                    );
                }
            }
            column = column.child(row);
        }
        column
    }
}

#[cfg(test)]
mod progress_tests {
    use super::*;
    #[test]
    fn old_project_progress_is_not_accepted() {
        assert!(accepts_progress(2, 2));
        assert!(!accepts_progress(2, 3));
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
        "queued" => (colors.status_info(), t::diag_state_queued()),
        "running" => (colors.accent_hover(), t::diag_state_running()),
        "completed" => (colors.status_success(), t::diag_state_completed()),
        "failed" => (colors.status_danger(), t::diag_state_failed()),
        "cancelled" => (colors.text_muted(), t::diag_state_cancelled()),
        _ => (colors.text_muted(), t::diag_state_unknown()),
    }
}

/// Name and purpose of a lane, in product language.
fn lane_label(lane: Lane) -> (&'static str, &'static str) {
    match lane {
        Lane::Now => (t::diag_lane_now(), t::diag_lane_now_hint()),
        Lane::Documents => (t::diag_lane_documents(), t::diag_lane_documents_hint()),
        Lane::Suggestions => (t::diag_lane_suggestions(), t::diag_lane_suggestions_hint()),
    }
}

fn job_kind(kind: &str) -> String {
    match kind {
        application::jobs::ANALYZE_CAPTURE_KIND => t::diag_kind_capture().into(),
        application::jobs::ANALYZE_DOCUMENT_KIND => t::diag_kind_document().into(),
        application::relation_suggestions::RELATION_JOB_KIND => t::diag_kind_relations().into(),
        application::claim_suggestions::CLAIM_JOB_KIND => t::diag_kind_rules().into(),
        application::search_terms::SEARCH_TERMS_JOB_KIND => t::diag_kind_search_terms().into(),
        application::link_suggestions::LINK_JOB_KIND => t::diag_kind_links().into(),
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
