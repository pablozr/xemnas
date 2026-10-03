//! Visão: the project summarized and its main flows, written by the AI
//! provider from the recorded decisions, rules and map (`OverviewApi`).
//!
//! The overview is a reading, not authority: every sentence and step shows
//! the records it rests on, which open in Decisões or in the Mapa. It is
//! stored and regenerated only when the user asks; the page says how many
//! decisions arrived since.

use std::collections::BTreeMap;
use std::sync::Arc;

use application::overview::{Citation, OverviewApi, OverviewError, OverviewView};
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

const SENDS: &str = "Gera um resumo e os principais fluxos com o provedor de IA configurado. \
                     Envia as decisões em vigor, as regras, os nomes do mapa e títulos, seções \
                     e o primeiro parágrafo da documentação; nada do código.";

const PAGE_HINT: &str = "Gera um arquivo HTML com a arquitetura e os fluxos e o abre no navegador. Funciona offline e pode ser enviado a alguém.";

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
    /// Name of the project, for the title of the exported page.
    name: String,
    generation: u64,
    busy: bool,
    generating: bool,
    loaded: bool,
    view: Option<OverviewView>,
    error: Option<String>,
    notice: Option<String>,
    focus: BTreeMap<String, FocusHandle>,
}

impl EventEmitter<OpenDecision> for OverviewScreen {}

impl OverviewScreen {
    /// Mounts the screen; nothing is read until a project is set.
    pub fn new(api: Arc<dyn OverviewApi>) -> Self {
        Self {
            api: Some(api),
            project: None,
            name: String::new(),
            generation: 0,
            busy: false,
            generating: false,
            loaded: false,
            view: None,
            error: None,
            notice: None,
            focus: BTreeMap::new(),
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
        self.loaded = false;
        self.error = None;
        self.refresh(cx);
    }

    /// Tells the screen the project's name (the exported page carries it).
    pub fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
    }

    /// Writes the overview as a self-contained HTML page and opens it in the
    /// default browser (`application::page`).
    fn open_page(&mut self, cx: &mut Context<Self>) {
        let (Some(project), Some(api), true) =
            (self.project.clone(), self.api.clone(), self.view.is_some())
        else {
            return;
        };
        let name = self.name.clone();
        cx.spawn(async move |this, cx| {
            let written = cx
                .background_executor()
                .spawn(async move {
                    let html = api
                        .page(&project, &name)
                        .map_err(|error| std::io::Error::other(error.to_string()))?;
                    let dir = std::env::temp_dir().join("xemnas");
                    std::fs::create_dir_all(&dir)?;
                    let path = dir.join(application::page::file_name(&name));
                    std::fs::write(&path, html)?;
                    Ok::<_, std::io::Error>(path)
                })
                .await;
            let _ = this.update(cx, |this, cx| match written {
                Ok(path) => {
                    let local = path.to_string_lossy().replace('\\', "/");
                    let url = format!("file:///{}", local.replace(' ', "%20"));
                    cx.open_url(&url);
                    this.show_notice("Página aberta no navegador.".into(), cx);
                }
                Err(error) => {
                    tracing::error!(%error, operation = "overview-page", "page not written");
                    this.error = Some("Não foi possível criar a página. Tente de novo.".into());
                    cx.notify();
                }
            });
        })
        .detach();
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
                    }
                    Outcome::Generated(Ok(view)) => {
                        this.loaded = true;
                        let queued = view.queued_documents;
                        this.view = Some(view);
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

    /// The way to the page of architecture and flows: what it holds, and
    /// the button that writes and opens it.
    fn page_card(
        &mut self,
        theme: &Theme,
        overview: &application::overview::ProjectOverview,
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let parts = overview.architecture.containers.len();
        let flows = overview.flows.len();
        let what = match (parts, flows) {
            (0, 0) => "Ainda não há partes nem fluxos. Gere a visão de novo quando o mapa e as decisões cobrirem mais do projeto."
                .to_owned(),
            _ => format!(
                "{} e {} em uma página para ler com calma: um diagrama que se explora e cada fluxo passo a passo. Abre no navegador; nada sai do computador.",
                plural(parts, "parte", "partes"),
                plural(flows, "fluxo", "fluxos"),
            ),
        };
        let button = action_button(
            theme,
            "overview-page",
            ButtonKind::Primary,
            parts + flows > 0,
        )
        .aria_label("Ver arquitetura e fluxos")
        .tooltip(tooltip(PAGE_HINT, None))
        .child("Ver arquitetura e fluxos");
        let button = self.pressable(button, "overview-page", |this, cx| this.open_page(cx), cx);
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(section_label(theme, "Arquitetura e fluxos"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S4))
                    .p(px(SpacingScale::S4))
                    .rounded(RadiusScale.surface())
                    .border_1()
                    .border_color(colors.glass_border_card())
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(0.0))
                            .text_color(colors.text_secondary())
                            .child(what),
                    )
                    .child(button),
            )
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

    fn render_summary(
        &mut self,
        theme: &Theme,
        view: &OverviewView,
        cx: &mut Context<Self>,
    ) -> Div {
        let colors = theme.colors;
        let overview = &view.overview;
        let update = self.generate_button(theme, ButtonKind::Secondary, cx);
        let page = self.page_card(theme, overview, cx);
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
            .child(page)
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(PROVENANCE),
            )
    }
}

impl Render for OverviewScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _probe = crate::ui::perf::Probe::start("overview");
        let theme = Theme::current(cx);
        let body: AnyElement = if !self.loaded {
            div()
                .p(px(SpacingScale::S8))
                .child(skeleton_list(&theme, "overview-skeleton", 5))
                .into_any_element()
        } else if let Some(view) = self.view.clone() {
            let column = self.render_summary(&theme, &view, cx);
            reading_page("overview-page", column).into_any_element()
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
                        "Um resumo do projeto, com arquitetura e fluxos",
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
