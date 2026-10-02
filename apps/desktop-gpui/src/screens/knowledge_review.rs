//! Ephemeral read-only review; the application owns all checks and provider policy.
use super::context::OpenDecision;
use crate::ui::{
    controls::{action_button, ButtonKind},
    icons::IconName,
    patterns::{
        action_footer, empty_panel, error_banner, reading_page, section_header, skeleton_list,
        toast, TOAST_DURATION,
    },
    theme::{text_style, Theme},
    tokens::{SpacingScale, TypeScale},
};
use application::knowledge_review::*;
use gpui::prelude::*;
use gpui::{div, px, Context, Div, EventEmitter, Render, Window};
use std::sync::Arc;

/// Separate review view, never borrowing Context's mutable services.
pub struct KnowledgeReviewScreen {
    api: Option<Arc<dyn KnowledgeReviewApi>>,
    project: Option<String>,
    generation: u64,
    busy: bool,
    token: Option<ReviewCancellation>,
    report: Option<ReviewReport>,
    error: bool,
    confirm: bool,
    notice: bool,
    focus: [gpui::FocusHandle; 3],
    source_focus: std::collections::BTreeMap<String, gpui::FocusHandle>,
}
impl EventEmitter<OpenDecision> for KnowledgeReviewScreen {}
impl Drop for KnowledgeReviewScreen {
    fn drop(&mut self) {
        if let Some(token) = &self.token {
            token.cancel();
        }
    }
}
impl KnowledgeReviewScreen {
    /// Mount without inspecting or sending data.
    pub fn new(api: Option<Arc<dyn KnowledgeReviewApi>>, cx: &mut Context<Self>) -> Self {
        Self {
            api,
            project: None,
            generation: 0,
            busy: false,
            token: None,
            report: None,
            error: false,
            confirm: false,
            notice: false,
            focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            source_focus: Default::default(),
        }
    }
    /// Whether the port is available.
    pub fn available(&self) -> bool {
        self.api.is_some()
    }
    /// Retiring a cached surface cancels its flight without unlocking it.
    pub fn leave(&mut self, cx: &mut Context<Self>) {
        retire_flight(&mut self.generation, &self.token);
        self.confirm = false;
        self.notice = false;
        cx.notify();
    }
    /// Preloaded synthetic results, only supplied by the demo composition root.
    pub fn preload(&mut self, report: ReviewReport, cx: &mut Context<Self>) {
        if !self.busy && self.project.as_ref() == Some(&report.project_id) {
            self.report = Some(report);
            cx.notify();
        }
    }
    /// Cancel and discard the old project, but keep the flight locked until return.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.project == project {
            return;
        }
        if let Some(token) = &self.token {
            token.cancel();
        }
        self.generation += 1;
        self.project = project;
        self.report = None;
        self.error = false;
        self.confirm = false;
        self.notice = false;
        cx.notify();
    }
    fn run(&mut self, semantic: bool, cx: &mut Context<Self>) {
        if flight_blocked(self.busy) {
            return;
        }
        let (Some(api), Some(project)) = (self.api.clone(), self.project.clone()) else {
            return;
        };
        self.generation += 1;
        let generation = self.generation;
        let token = ReviewCancellation::default();
        self.token = Some(token.clone());
        self.busy = true;
        clear_previous(&mut self.report, &mut self.notice);
        self.confirm = false;
        self.error = false;
        cx.spawn(async move |this, cx| {
            let requested = project.clone();
            let result = cx
                .background_executor()
                .spawn(async move {
                    if semantic {
                        api.review(&requested, token)
                    } else {
                        api.inspect(&requested)
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.busy = false;
                this.token = None;
                if accepts_completion(
                    this.generation,
                    generation,
                    this.project.as_deref(),
                    &project,
                ) {
                    match result {
                        Ok(report) => {
                            this.report = Some(report);
                            this.notice = true;
                        }
                        Err(_) => this.error = true,
                    }
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(TOAST_DURATION).await;
                        let _ = this.update(cx, |this, cx| {
                            if this.generation == generation {
                                this.notice = false;
                                cx.notify();
                            }
                        });
                    })
                    .detach();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn content(&mut self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let mut page = div().flex().flex_col().gap(px(SpacingScale::S4))
            .child(text_style(div(), TypeScale::HEADING_1).child("Revisar conhecimento"))
            .child("Revisão consultiva, sem alterar conhecimento. Tensão não é conflito comprovado; citar não garante uma inferência correta.");
        if !self.available() || self.project.is_none() {
            page = page.child(empty_panel(
                theme,
                IconName::Layers,
                "Revisão",
                "Revisão indisponível",
                "Selecione um projeto com armazenamento disponível.",
            ));
        } else if self.busy {
            page = page
                .child(
                    if self
                        .token
                        .as_ref()
                        .is_some_and(ReviewCancellation::is_cancelled)
                    {
                        "Cancelando… aguardando a chamada em curso retornar."
                    } else {
                        "Verificando conhecimento…"
                    },
                )
                .child(skeleton_list(theme, "knowledge-review-loading", 3));
        } else if self.report.is_none() {
            page = page.child(empty_panel(
                theme,
                IconName::Layers,
                "Revisão",
                "Verifique o conhecimento registrado",
                "Verificar localmente não envia textos. A IA exige confirmação explícita.",
            ));
        }
        if self.error {
            page = page.child(error_banner(
                theme,
                "Não foi possível ler o conhecimento. Tente verificar localmente novamente.",
            ));
        }
        if let Some(report) = self.report.clone() {
            let i = report.coverage.inventory;
            page = page
                .child(section_header(theme, "Verificações locais"))
                .child(format!(
                    "{} decisões em vigor · {} substituídas · {} regras válidas · {} relações",
                    i.current_decisions, i.superseded_decisions, i.valid_rules, i.relations
                ))
                .child(format!(
                    "Propostas pendentes: {} relações · {} regras · {} vínculos",
                    i.pending_relation_proposals,
                    i.pending_claim_proposals,
                    i.pending_edge_proposals
                ))
                .child(if report.coverage.deterministic_complete {
                    "Verificações locais concluídas."
                } else {
                    "Verificações locais incompletas."
                })
                .child(section_header(theme, "Revisão IA"))
                .child(match report.semantic_status {
                    SemanticStatus::NotRequested => "Não solicitada; nenhum texto enviado.",
                    SemanticStatus::Completed => "Concluída dentro da cobertura selecionada.",
                    SemanticStatus::Partial => "Parcial; nem todo o conhecimento foi revisado.",
                    SemanticStatus::Unavailable => {
                        "Indisponível; confira provedor e consentimento em Configurações."
                    }
                    SemanticStatus::Failed => {
                        "Falha ou resposta inválida; achados locais preservados."
                    }
                    SemanticStatus::Cancelled => "Cancelada; achados locais preservados.",
                });
            if let Some(p) = report.provenance {
                page = page.child(format!("Provedor: {} · modelo: {}", p.adapter, p.model));
            }
            for origin in [FindingOrigin::Deterministic, FindingOrigin::Semantic] {
                page = page.child(section_header(
                    theme,
                    if origin == FindingOrigin::Deterministic {
                        "Achados locais"
                    } else {
                        "Hipóteses consultivas"
                    },
                ));
                let findings: Vec<_> = report
                    .findings
                    .iter()
                    .filter(|f| f.origin == origin)
                    .collect();
                if findings.is_empty() {
                    page = page.child(
                        "Sem achados nesta cobertura. Isso não comprova ausência de problemas.",
                    );
                }
                for (n, finding) in findings.iter().enumerate() {
                    page = page
                        .child(finding.explanation.clone())
                        .child(finding.question.clone());
                    for (e, evidence) in finding.evidence.iter().enumerate() {
                        page = page
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(theme.colors.text_muted())
                                    .child(format!(
                                        "{} · versão {} · campo {}",
                                        evidence.source.title,
                                        evidence
                                            .source
                                            .version
                                            .map(|v| v.to_string())
                                            .unwrap_or_else(|| "—".into()),
                                        evidence.field
                                    )),
                            )
                            .child(format!("“{}”", evidence.quote));
                        if evidence.source.kind == ReviewSourceKind::Decision {
                            let id = evidence.source.id.clone();
                            let key = format!("review-source-{origin:?}-{n}-{e}");
                            let focus = self
                                .source_focus
                                .entry(key.clone())
                                .or_insert_with(|| cx.focus_handle().tab_stop(true))
                                .clone();
                            page =
                                page.child(
                                    action_button(
                                        theme,
                                        gpui::ElementId::Name(key.into()),
                                        ButtonKind::Ghost,
                                        true,
                                    )
                                    .track_focus(&focus)
                                    .aria_label("Abrir decisão citada")
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(OpenDecision(id.clone()))
                                    }))
                                    .child("Abrir decisão"),
                                );
                        }
                    }
                }
            }
            page = page
                .child(section_header(theme, "Cobertura"))
                .child(format!(
                    "{} unidades selecionadas · {} achados descartados",
                    report.coverage.semantic_units.len(),
                    report.coverage.discarded_findings
                ));
            for unit in report.coverage.semantic_units {
                page = page.child(format!(
                    "{} · {:?} · {}",
                    unit.selection,
                    unit.outcome,
                    unit.subject_ids.join(", ")
                ));
            }
            for omission in report.coverage.omissions {
                page = page.child(format!(
                    "Não revisado: {} · {}",
                    omission.reason,
                    omission.subject_ids.join(", ")
                ));
            }
        }
        if self.confirm {
            page = page.child("Enviar ao provedor configurado textos de decisões, regras, propostas e vínculos selecionados? Não envia código nem a pasta. Não substitui o consentimento vigente em Configurações.");
        }
        page
    }
}

fn retire_flight(generation: &mut u64, token: &Option<ReviewCancellation>) {
    if let Some(token) = token {
        token.cancel();
    }
    *generation += 1;
}
fn clear_previous(report: &mut Option<ReviewReport>, notice: &mut bool) {
    *report = None;
    *notice = false;
}
fn accepts_completion(current: u64, flight: u64, project: Option<&str>, requested: &str) -> bool {
    current == flight && project == Some(requested)
}
fn flight_blocked(busy: bool) -> bool {
    busy
}

/// Synthetic report used only by the explicit demo results route.
pub fn demo_report(project: &str) -> ReviewReport {
    let source = ReviewSource {
        kind: ReviewSourceKind::Decision,
        id: "demo-review-decision".into(),
        title: "Qual o limite de entrega? (fonte fictícia)".into(),
        version: Some(2),
        updated_at: None,
    };
    ReviewReport {
        project_id: project.into(),
        started_at: "2026-10-02T12:00:00Z".into(),
        finished_at: "2026-10-02T12:00:01Z".into(),
        input_hash: "synthetic".into(),
        semantic_status: SemanticStatus::Partial,
        coverage: ReviewCoverage {
            inventory: ReviewInventory {
                current_decisions: 3,
                superseded_decisions: 1,
                valid_rules: 2,
                relations: 1,
                ..Default::default()
            },
            deterministic_complete: true,
            semantic_units: vec![ReviewUnitCoverage {
                subject_ids: vec![source.id.clone()],
                selection: "Par fictício de limites".into(),
                outcome: ReviewUnitOutcome::Reviewed,
            }],
            omissions: vec![ReviewOmission {
                subject_ids: vec![],
                reason: "Dados sintéticos; nenhuma avaliação do projeto real.".into(),
            }],
            discarded_findings: 0,
        },
        findings: vec![
            ReviewFinding {
                kind: FindingKind::RuleFromSuperseded,
                origin: FindingOrigin::Deterministic,
                explanation: "Regra fictícia tem origem em decisão substituída.".into(),
                question: "Essa regra ainda deve valer?".into(),
                evidence: vec![],
            },
            ReviewFinding {
                kind: FindingKind::PossibleTension,
                origin: FindingOrigin::Semantic,
                explanation: "Hipótese sintética: os limites podem tratar de cenários distintos."
                    .into(),
                question: "O limite se aplica por sessão ou por entrega?".into(),
                evidence: vec![ReviewEvidence {
                    source,
                    field: "choice".into(),
                    text: "Até 300 tokens".into(),
                    start_byte: 0,
                    end_byte: "Até 300 tokens".len(),
                    quote: "Até 300 tokens".into(),
                }],
            },
        ],
        provenance: None,
    }
}
impl Render for KnowledgeReviewScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let page = self.content(&theme, cx);
        let enabled = !self.busy && self.available() && self.project.is_some();
        div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .relative()
            .flex()
            .flex_col()
            .child(reading_page("knowledge-review-page", page).flex_1())
            .child(
                action_footer(&theme, None)
                    .child(
                        action_button(&theme, "review-local", ButtonKind::Secondary, enabled)
                            .track_focus(&self.focus[0])
                            .aria_label("Verificar localmente")
                            .on_click(cx.listener(|this, _, _, cx| this.run(false, cx)))
                            .child("Verificar localmente"),
                    )
                    .child(
                        action_button(&theme, "review-ai", ButtonKind::Primary, enabled)
                            .track_focus(&self.focus[1])
                            .aria_label("Revisar com IA após confirmação")
                            .on_click(cx.listener(|this, _, _, cx| {
                                if this.confirm {
                                    this.run(true, cx);
                                } else {
                                    this.confirm = true;
                                    cx.notify();
                                }
                            }))
                            .child(if self.confirm {
                                "Confirmar envio e revisar"
                            } else {
                                "Revisar com IA…"
                            }),
                    )
                    .child(
                        action_button(
                            &theme,
                            "review-cancel",
                            ButtonKind::Ghost,
                            self.busy || self.confirm,
                        )
                        .track_focus(&self.focus[2])
                        .aria_label("Cancelar revisão ou confirmação")
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Some(token) = &this.token {
                                token.cancel();
                            }
                            this.confirm = false;
                            cx.notify();
                        }))
                        .child("Cancelar"),
                    ),
            )
            .children(
                self.notice
                    .then(|| toast(&theme, "Revisão concluída; nenhum dado alterado.", 24.0)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retiring_surface_cancels_and_invalidates_without_unlocking_flight() {
        let token = ReviewCancellation::default();
        let mut generation = 1;
        let mut busy = true;
        retire_flight(&mut generation, &Some(token.clone()));
        assert!(token.is_cancelled());
        assert!(flight_blocked(busy));
        assert!(!accepts_completion(generation, 1, Some("a"), "a"));
        busy = false; // Only the completed call releases the guard, even if stale.
        assert!(!flight_blocked(busy));
        assert!(!accepts_completion(1, 1, Some("b"), "a"));
        assert!(accepts_completion(2, 2, Some("b"), "b"));
    }
    #[test]
    fn rerun_clears_report_and_notice_before_failure() {
        let mut report = Some(demo_report("a"));
        let mut notice = true;
        clear_previous(&mut report, &mut notice);
        let failure: Result<ReviewReport, ReviewError> = Err(ReviewError::ProjectNotFound);
        assert!(failure.is_err());
        assert!(report.is_none());
        assert!(!notice);
        assert!(!ReviewCancellation::default().is_cancelled());
    }
}
