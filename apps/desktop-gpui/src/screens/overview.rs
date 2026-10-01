//! Visão: the project summarized and its main flows, written by the AI
//! provider from the recorded decisions, rules and map (`OverviewApi`).
//!
//! The overview is a reading, not authority: every sentence and step shows
//! the records it rests on, which open in Decisões or in the Mapa. It is
//! stored and regenerated only when the user asks; the page says how many
//! decisions arrived since.

use std::collections::BTreeMap;
use std::sync::Arc;

use application::overview::{Citation, OverviewApi, OverviewError, OverviewFlow, OverviewView};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, EventEmitter, FocusHandle, Render, Role, SharedString,
    Stateful, Window,
};

use super::context::OpenDecision;
use super::format::{clipped, plural, short_date};
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    empty_panel, error_banner, reading_page, section_label, skeleton_list, toast, TOAST_DURATION,
};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

/// Asks the shell to open an entity in the Mapa.
pub struct OpenEntity(pub String);

const SENDS: &str = "Gera um resumo e os principais fluxos com o provedor de IA configurado. \
                     Envia as decisões em vigor, as regras, os nomes do mapa e títulos, seções \
                     e o primeiro parágrafo da documentação; nada do código.";

const PROVENANCE: &str = "Escrita pela IA a partir das decisões e regras confirmadas e da \
                          documentação do projeto. Cada trecho mostra as fontes; o que não \
                          tinha fonte ficou de fora.";

enum Outcome {
    Loaded(Result<Option<OverviewView>, String>),
    Generated(Result<OverviewView, String>),
}

/// The Visão destination of a selected project.
pub struct OverviewScreen {
    api: Option<Arc<dyn OverviewApi>>,
    project: Option<String>,
    generation: u64,
    busy: bool,
    generating: bool,
    loaded: bool,
    view: Option<OverviewView>,
    flow: Option<usize>,
    error: Option<String>,
    notice: Option<String>,
    focus: BTreeMap<String, FocusHandle>,
    /// Demo-only flow to open once the overview is read.
    route_flow: Option<usize>,
}

impl EventEmitter<OpenDecision> for OverviewScreen {}
impl EventEmitter<OpenEntity> for OverviewScreen {}

impl OverviewScreen {
    /// Mounts the screen; nothing is read until a project is set.
    pub fn new(api: Arc<dyn OverviewApi>) -> Self {
        Self {
            api: Some(api),
            project: None,
            generation: 0,
            busy: false,
            generating: false,
            loaded: false,
            view: None,
            flow: None,
            error: None,
            notice: None,
            focus: BTreeMap::new(),
            route_flow: None,
        }
    }

    /// Clears the previous project and reads the stored overview.
    pub fn set_project(&mut self, project: Option<String>, cx: &mut Context<Self>) {
        if self.project == project {
            return;
        }
        self.project = project;
        self.generation += 1;
        self.view = None;
        self.flow = None;
        self.loaded = false;
        self.error = None;
        self.refresh(cx);
    }

    /// Opens a flow by position once the overview is read (demo captures).
    pub fn open_flow(&mut self, index: usize) {
        self.route_flow = Some(index);
    }

    /// Reads the stored overview again (cheap; never calls the provider).
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        self.run(cx, move |api| {
            Outcome::Loaded(api.current(&project).map_err(product))
        });
    }

    fn generate(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.clone() else {
            return;
        };
        if self.busy {
            return;
        }
        self.generating = true;
        self.run(cx, move |api| {
            Outcome::Generated(api.generate(&project).map_err(product))
        });
    }

    fn run(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(&dyn OverviewApi) -> Outcome + Send + 'static,
    ) {
        let Some(api) = self.api.take() else {
            return;
        };
        self.busy = true;
        self.error = None;
        let generation = self.generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let (api, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = operation(api.as_ref());
                    (api, outcome)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.api = Some(api);
                this.busy = false;
                this.generating = false;
                if generation != this.generation {
                    this.refresh(cx);
                    return;
                }
                match outcome {
                    Outcome::Loaded(Ok(view)) => {
                        this.loaded = true;
                        this.view = view;
                        if let Some(flow) = this.route_flow.take() {
                            this.flow = Some(flow);
                        }
                    }
                    Outcome::Generated(Ok(view)) => {
                        this.loaded = true;
                        let queued = view.queued_documents;
                        this.view = Some(view);
                        this.flow = None;
                        this.show_notice(
                            match queued {
                                0 => "Visão atualizada.".into(),
                                1 => "Visão atualizada. 1 documento foi para análise; os \
                                      candidatos aparecem na Revisão."
                                    .into(),
                                n => format!(
                                    "Visão atualizada. {n} documentos foram para análise; os \
                                     candidatos aparecem na Revisão."
                                ),
                            },
                            cx,
                        );
                    }
                    Outcome::Loaded(Err(error)) => {
                        this.loaded = true;
                        this.error = Some(error);
                    }
                    Outcome::Generated(Err(error)) => this.error = Some(error),
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

    fn focus_for(&mut self, id: &str, cx: &mut Context<Self>) -> FocusHandle {
        self.focus
            .entry(id.to_owned())
            .or_insert_with(|| cx.focus_handle().tab_stop(true))
            .clone()
    }

    /// A focusable control that runs `on_press` on click, Enter or Space.
    fn pressable(
        &mut self,
        element: Stateful<Div>,
        id: &str,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let focus = self.focus_for(id, cx);
        let on_press = std::rc::Rc::new(on_press);
        let on_key = on_press.clone();
        element
            .track_focus(&focus)
            .on_click(cx.listener(move |this, _, _, cx| on_press(this, cx)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    on_key(this, cx);
                    cx.stop_propagation();
                }
            }))
    }

    fn generate_button(
        &mut self,
        theme: &Theme,
        kind: ButtonKind,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = if self.generating {
            "Gerando…"
        } else if self.view.is_some() {
            "Atualizar visão"
        } else {
            "Gerar visão"
        };
        let button = action_button(theme, "overview-generate", kind, !self.busy)
            .aria_label(label)
            .tooltip(tooltip(SENDS, None))
            .child(label);
        let enabled = !self.busy;
        self.pressable(
            button,
            "overview-generate",
            move |this, cx| {
                if enabled {
                    this.generate(cx);
                }
            },
            cx,
        )
    }

    /// Small chips for the records a sentence or step rests on.
    fn citations(
        &mut self,
        theme: &Theme,
        prefix: &str,
        citations: &[Citation],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let mut row = div().flex().flex_wrap().gap(px(SpacingScale::S1));
        for (index, citation) in citations.iter().enumerate() {
            let decision = citation.kind == "decision";
            let title = if citation.title.is_empty() {
                citation.label.clone()
            } else {
                citation.title.clone()
            };
            let chip = div()
                .id(SharedString::from(format!("{prefix}-cite-{index}")))
                .flex()
                .items_center()
                .gap(px(SpacingScale::S1))
                .px(px(SpacingScale::S2))
                .py(px(2.0))
                .rounded(theme.radius.control())
                .bg(colors.glass_fill_low())
                .child(icon(
                    match citation.kind.as_str() {
                        "decision" => IconName::Decision,
                        "document" => IconName::Book,
                        _ => IconName::Shield,
                    },
                    11.0,
                    colors.text_muted(),
                ))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_secondary())
                        .child(clipped(&title, CHIP_CHARS)),
                );
            if decision {
                let id = citation.id.clone();
                let chip = chip
                    .cursor_pointer()
                    .hover(move |style| style.bg(colors.glass_fill_medium()))
                    .role(Role::Link)
                    .aria_label(format!("Abrir a decisão: {title}"))
                    .tooltip(tooltip(format!("{title} · abrir em Decisões"), None))
                    .focus_visible(crate::ui::controls::focus_ring(theme));
                row = row.child(self.pressable(
                    chip,
                    &format!("{prefix}-cite-{index}"),
                    move |_, cx| cx.emit(OpenDecision(id.clone())),
                    cx,
                ));
            } else {
                let label = if citation.kind == "document" {
                    format!("Documento: {} ({})", title, citation.id)
                } else {
                    format!("Regra: {title}")
                };
                row = row.child(chip.aria_label(label.clone()).tooltip(tooltip(label, None)));
            }
        }
        row
    }

    fn render_flows(
        &mut self,
        theme: &Theme,
        flows: &[OverviewFlow],
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        // A numbered list read top to bottom: flows are not a grid of cards.
        let mut list = div()
            .flex()
            .flex_col()
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(colors.glass_border_card())
            .overflow_hidden();
        for (index, flow) in flows.iter().enumerate() {
            let components: Vec<String> = flow
                .steps
                .iter()
                .filter_map(|step| step.entity_name.clone())
                .fold(Vec::new(), |mut names, name| {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                    names
                });
            let row = div()
                .id(SharedString::from(format!("overview-flow-{index}")))
                .flex()
                .items_start()
                .gap(px(SpacingScale::S4))
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S4))
                .when(index > 0, |row| {
                    row.border_t_1().border_color(colors.hairline_divider())
                })
                .cursor_pointer()
                .hover(move |style| style.bg(colors.glass_fill_medium()))
                .active(move |style| style.bg(colors.glass_fill_strong()))
                .role(Role::Button)
                .aria_label(format!("Fluxo: {}", flow.title))
                .focus_visible(crate::ui::controls::focus_ring(theme))
                .child(
                    text_style(div(), TypeScale::META)
                        .w(px(20.0))
                        .flex_none()
                        .pt(px(2.0))
                        .font_family(Theme::font_mono())
                        .text_color(colors.text_muted())
                        .child(format!("{:02}", index + 1)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(2.0))
                        .child(text_style(div(), TypeScale::ROW_TITLE).child(flow.title.clone()))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_secondary())
                                .child(flow.description.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .mt(px(2.0))
                                .text_color(colors.text_muted())
                                .child(if components.is_empty() {
                                    plural(flow.steps.len(), "passo", "passos")
                                } else {
                                    format!(
                                        "{} · {}",
                                        plural(flow.steps.len(), "passo", "passos"),
                                        components.join(", ")
                                    )
                                }),
                        ),
                )
                .child(div().pt(px(2.0)).child(icon(
                    IconName::ChevronRight,
                    14.0,
                    colors.text_muted(),
                )));
            list = list.child(self.pressable(
                row,
                &format!("overview-flow-{index}"),
                move |this, cx| {
                    this.flow = Some(index);
                    cx.notify();
                },
                cx,
            ));
        }
        list
    }

    fn render_flow(&mut self, theme: &Theme, flow: &OverviewFlow, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let back = action_button(theme, "overview-back", ButtonKind::Ghost, true)
            .aria_label("Todos os fluxos")
            .child(icon(IconName::ArrowLeft, 14.0, colors.text_secondary()))
            .child("Todos os fluxos");
        let back = self.pressable(
            back,
            "overview-back",
            |this, cx| {
                this.flow = None;
                cx.notify();
            },
            cx,
        );
        let count = flow.steps.len();
        let mut steps = div().flex().flex_col();
        for (index, step) in flow.steps.iter().enumerate() {
            let last = index + 1 == count;
            let node = div()
                .w(px(28.0))
                .flex_none()
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .size(px(24.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_full()
                        .border_1()
                        .border_color(colors.glass_border_card_hover())
                        .bg(colors.glass_fill_card())
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(colors.text_secondary())
                                .child((index + 1).to_string()),
                        ),
                )
                .when(!last, |node| {
                    node.child(
                        div()
                            .w(px(1.0))
                            .flex_1()
                            .min_h(px(24.0))
                            .bg(colors.hairline_divider()),
                    )
                });
            let component =
                step.entity_id
                    .clone()
                    .zip(step.entity_name.clone())
                    .map(|(id, name)| {
                        let chip = div()
                            .id(SharedString::from(format!("overview-step-{index}-entity")))
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S1))
                            .px(px(6.0))
                            .rounded(theme.radius.control())
                            .border_1()
                            .border_color(colors.hairline_divider())
                            .cursor_pointer()
                            .hover(move |style| style.bg(colors.glass_fill_medium()))
                            .role(Role::Link)
                            .aria_label(format!("Abrir {name} no Mapa"))
                            .tooltip(tooltip("Abrir no Mapa", None))
                            .focus_visible(crate::ui::controls::focus_ring(theme))
                            .child(icon(IconName::Component, 12.0, colors.text_muted()))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_secondary())
                                    .child(name),
                            );
                        self.pressable(
                            chip,
                            &format!("overview-step-{index}-entity"),
                            move |_, cx| cx.emit(OpenEntity(id.clone())),
                            cx,
                        )
                    });
            let citations = self.citations(
                theme,
                &format!("overview-step-{index}"),
                &step.citations,
                cx,
            );
            steps = steps.child(
                div().flex().gap(px(SpacingScale::S3)).child(node).child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .pb(px(if last { 0.0 } else { SpacingScale::S5 }))
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S2))
                        .child(
                            text_style(div(), TypeScale::ROW_TITLE)
                                .pt(px(3.0))
                                .child(step.title.clone()),
                        )
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_secondary())
                                .child(step.text.clone()),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap(px(SpacingScale::S2))
                                .children(component)
                                .child(citations),
                        ),
                ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(div().flex().child(back))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(text_style(div(), TypeScale::HEADING_1).child(flow.title.clone()))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_muted())
                            .child(flow.description.clone()),
                    ),
            )
            .child(steps)
    }
    fn render_summary(
        &mut self,
        theme: &Theme,
        view: &OverviewView,
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let overview = &view.overview;
        let update = self.generate_button(theme, ButtonKind::Secondary, cx);
        let mut sources = vec![
            plural(overview.decisions, "decisão", "decisões"),
            plural(overview.rules, "regra", "regras"),
        ];
        if overview.documents > 0 {
            sources.push(plural(overview.documents, "documento", "documentos"));
        }
        let last = sources.pop().unwrap_or_default();
        let meta = format!(
            "Gerada em {} a partir de {} e {}",
            short_date(&overview.generated_at),
            sources.join(", "),
            last
        );
        let stale = (view.new_decisions > 0).then(|| {
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(colors.status_warning()),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_secondary())
                        .child(format!(
                            "{} desde então. Atualize para incluí-las.",
                            if view.new_decisions == 1 {
                                "1 decisão nova".to_owned()
                            } else {
                                format!("{} decisões novas", view.new_decisions)
                            }
                        )),
                )
        });
        let title = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .child(text_style(div(), TypeScale::HEADING_1).child("Visão do projeto"))
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(meta),
            )
            .children(stale);
        let mut summary = div().flex().flex_col().gap(px(SpacingScale::S4));
        for (index, paragraph) in overview.summary.iter().enumerate() {
            let prefix = format!("overview-p{index}");
            let citations = self.citations(theme, &prefix, &paragraph.citations, cx);
            summary = summary.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(text_style(div(), TypeScale::BODY).child(paragraph.text.clone()))
                    .child(citations),
            );
        }
        let flows = overview.flows.clone();
        let flows_grid = self.render_flows(theme, &flows, cx);
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S8))
            .child(
                div()
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S4))
                    .child(title)
                    .child(update),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(section_label(theme, "Resumo"))
                    .child(summary),
            )
            .when(!flows.is_empty(), |column| {
                column.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S3))
                        .child(section_label(theme, "Principais fluxos"))
                        .child(flows_grid),
                )
            })
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(PROVENANCE),
            )
    }
}

impl Render for OverviewScreen {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let body: AnyElement = if !self.loaded {
            div()
                .p(px(SpacingScale::S8))
                .child(skeleton_list(&theme, "overview-skeleton", 5))
                .into_any_element()
        } else if let Some(view) = self.view.clone() {
            if let Some(flow) = self
                .flow
                .and_then(|index| view.overview.flows.get(index).cloned())
            {
                let page = self.render_flow(&theme, &flow, cx);
                reading_page("overview-flow-page", page).into_any_element()
            } else {
                let column = self.render_summary(&theme, &view, cx);
                reading_page("overview-page", column).into_any_element()
            }
        } else {
            let generate = self.generate_button(&theme, ButtonKind::Primary, cx);
            div()
                .size_full()
                .flex()
                .flex_col()
                .child(
                    empty_panel(
                        &theme,
                        IconName::Compass,
                        "Visão",
                        "Um resumo do projeto e seus fluxos",
                        SENDS,
                    )
                    .flex_1(),
                )
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .pb(px(SpacingScale::S8))
                        .child(generate),
                )
                .into_any_element()
        };
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .children(self.error.clone().map(|error| {
                error_banner(&theme, &error)
                    .id("overview-error")
                    .role(Role::Alert)
            }))
            .child(div().flex_1().min_h(px(0.0)).flex().flex_col().child(body))
            .children(
                self.notice
                    .clone()
                    .map(|notice| toast(&theme, &notice, 24.0)),
            )
    }
}

/// Longest source title shown on a chip; the tooltip has the rest.
const CHIP_CHARS: usize = 48;

/// Product copy for an overview failure; storage detail goes to the log.
fn product(error: OverviewError) -> String {
    if let OverviewError::Storage(detail) = &error {
        tracing::error!(error = %detail, operation = "overview", "overview storage failed");
        return "Não foi possível ler a visão do projeto. Tente de novo.".into();
    }
    let text = error.to_string();
    let mut chars = text.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().chain(chars).collect::<String>() + ".")
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::product;
    use application::overview::OverviewError;

    #[test]
    fn failures_read_as_sentences_and_hide_storage_detail() {
        assert_eq!(
            product(OverviewError::ProviderOff),
            "Ative um provedor de IA em Configurações › IA para gerar a visão."
        );
        assert!(!product(OverviewError::Storage("disk I/O".into())).contains("disk"));
    }
}
