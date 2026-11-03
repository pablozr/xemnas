//! Visão: the project summarized and its main flows, written by the AI
//! provider from the recorded decisions, rules and map (`OverviewApi`).
//!
//! The overview is a reading, not authority: every sentence and step shows
//! the records it rests on, which open in Decisões or in the Mapa. It is
//! stored and regenerated only when the user asks; the page says how many
//! decisions arrived since.

use std::collections::BTreeMap;
use std::sync::Arc;

use application::overview::{
    Citation, OverviewApi, OverviewError, OverviewParagraph, OverviewView,
};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, EventEmitter, FocusHandle, Render, Role, SharedString,
    Stateful, Window,
};

use super::context::OpenDecision;
use super::format::short_date;
use crate::i18n::overview as t;
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    empty_panel, error_banner, reading_page, section_label, skeleton_list, toast,
    word_wrapped_with, TOAST_DURATION,
};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

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
                    this.show_notice(t::page_opened().into(), cx);
                }
                Err(error) => {
                    tracing::error!(%error, operation = "overview-page", "page not written");
                    this.error = Some(t::page_failed().into());
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
        // The overview is written in the language of the interface.
        let language =
            application::output_language::OutputLanguage::from_tag(crate::i18n::current().id());
        self.run(cx, move |api| {
            Outcome::Generated(api.generate(&project, language).map_err(product))
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
                                0 => t::updated().into(),
                                n => t::updated_queued(n),
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
            (0, 0) => t::page_empty().to_owned(),
            _ => t::page_what(&t::parts_count(parts), &t::flows_count(flows)),
        };
        let button = action_button(
            theme,
            "overview-page",
            ButtonKind::Primary,
            parts + flows > 0,
        )
        .aria_label(t::view_page())
        .tooltip(tooltip(t::page_hint(), None))
        .child(t::view_page());
        let button = self.pressable(button, "overview-page", |this, cx| this.open_page(cx), cx);
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(section_label(theme, t::page_section()))
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
            t::generating()
        } else if self.view.is_some() {
            t::update_overview()
        } else {
            t::generate_overview()
        };
        let button = action_button(theme, "overview-generate", kind, !self.busy)
            .aria_label(label)
            .tooltip(tooltip(t::sends(), None))
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

    /// A cited record, given its tooltip and accessible name; a decision
    /// also opens in Decisões on click, Enter or Space.
    fn cited(
        &mut self,
        theme: &Theme,
        element: Stateful<Div>,
        id: &str,
        citation: &Citation,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = source_label(citation);
        let element = element.aria_label(label.clone());
        let Some(open) = opens(citation) else {
            return element.tooltip(tooltip(label, None));
        };
        let title = source_title(citation);
        let colors = theme.colors;
        let element = element
            .cursor_pointer()
            .hover(move |style| style.bg(colors.glass_fill_low()))
            .role(Role::Link)
            .tooltip(tooltip(t::open_in_decisions(&title), None))
            .focus_visible(crate::ui::controls::focus_ring(theme));
        self.pressable(
            element,
            id,
            move |_, cx| cx.emit(OpenDecision(open.0.clone())),
            cx,
        )
    }

    /// The small raised numbers after a paragraph, one per record it rests
    /// on, numbered as in the page's list of sources.
    fn source_marks(
        &mut self,
        theme: &Theme,
        prefix: &str,
        citations: &[Citation],
        sources: &[Citation],
        cx: &mut Context<Self>,
    ) -> Div {
        let mut marks = div().flex().gap(px(1.0));
        for citation in unique(citations) {
            let Some(number) = source_number(sources, citation) else {
                continue;
            };
            let id = format!("{prefix}-src-{number}");
            let mark = text_style(div(), TypeScale::META)
                .id(SharedString::from(id.clone()))
                .relative()
                .top(px(-5.0))
                .px(px(3.0))
                .py(px(2.0))
                .rounded(theme.radius.control())
                .text_size(px(10.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.colors.accent_default())
                .child(number.to_string());
            marks = marks.child(self.cited(theme, mark, &id, citation, cx));
        }
        marks
    }

    /// The numbered list of the page's sources, with their whole titles.
    fn source_list(&mut self, theme: &Theme, sources: &[Citation], cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let mut list = div().flex().flex_col().gap(px(SpacingScale::S1));
        for (index, citation) in sources.iter().enumerate() {
            let id = format!("overview-source-{}", index + 1);
            let row = div()
                .id(SharedString::from(id.clone()))
                .flex()
                .items_start()
                .gap(px(SpacingScale::S2))
                .px(px(SpacingScale::S1))
                .py(px(2.0))
                .rounded(theme.radius.control())
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .w(px(20.0))
                        .flex_none()
                        .text_color(colors.accent_default())
                        .child(format!("{}.", index + 1)),
                )
                .child(div().pt(px(4.0)).flex_none().child(icon(
                    source_icon(citation),
                    11.0,
                    colors.text_muted(),
                )))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .min_w(px(0.0))
                        .text_color(colors.text_secondary())
                        .child(source_title(citation)),
                );
            list = list.child(self.cited(theme, row, &id, citation, cx));
        }
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(section_label(theme, t::sources()))
            .child(list)
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
        let decisions = t::decisions_count(overview.decisions);
        let rules = t::rules_count(overview.rules);
        let date = short_date(&overview.generated_at);
        let meta = if overview.documents > 0 {
            let documents = t::documents_count(overview.documents);
            t::generated_from_three(&date, &decisions, &rules, &documents)
        } else {
            t::generated_from_two(&date, &decisions, &rules)
        };
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
                        .child(t::stale(&t::new_decisions_count(view.new_decisions))),
                )
        });
        let title = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .child(text_style(div(), TypeScale::HEADING_1).child(t::project_overview()))
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(meta),
            )
            .children(stale);
        let sources = sources(&overview.summary);
        let mut summary = div().flex().flex_col().gap(px(SpacingScale::S4));
        for (index, paragraph) in overview.summary.iter().enumerate() {
            let prefix = format!("overview-p{index}");
            let marks = self.source_marks(theme, &prefix, &paragraph.citations, &sources, cx);
            summary = summary.child(word_wrapped_with(&paragraph.text, TypeScale::BODY, marks));
        }
        let sources = (!sources.is_empty()).then(|| self.source_list(theme, &sources, cx));
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
                    .child(section_label(theme, t::summary_section()))
                    .child(summary),
            )
            .children(sources)
            .child(page)
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(t::provenance()),
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
                        t::empty_label(),
                        t::empty_title(),
                        t::sends(),
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

/// Every record the page cites, once, in order of first appearance; a
/// record's number is its place in this list plus one.
fn sources(summary: &[OverviewParagraph]) -> Vec<Citation> {
    let all: Vec<&Citation> = summary
        .iter()
        .flat_map(|paragraph| &paragraph.citations)
        .collect();
    unique(all).into_iter().cloned().collect()
}

/// The citations without repeats of the same record, in order.
fn unique<'a>(citations: impl IntoIterator<Item = &'a Citation>) -> Vec<&'a Citation> {
    let mut seen: Vec<&Citation> = Vec::new();
    for citation in citations {
        if !seen.iter().any(|known| same_record(known, citation)) {
            seen.push(citation);
        }
    }
    seen
}

fn same_record(a: &Citation, b: &Citation) -> bool {
    a.kind == b.kind && a.id == b.id
}

/// The number shown for `citation` (1-based), from the page's sources.
fn source_number(sources: &[Citation], citation: &Citation) -> Option<usize> {
    sources
        .iter()
        .position(|source| same_record(source, citation))
        .map(|index| index + 1)
}

/// What pressing a source does: a decision opens in Decisões; documents and
/// rules only explain themselves.
fn opens(citation: &Citation) -> Option<OpenDecision> {
    (citation.kind == "decision").then(|| OpenDecision(citation.id.clone()))
}

fn source_title(citation: &Citation) -> String {
    if citation.title.is_empty() {
        citation.label.clone()
    } else {
        citation.title.clone()
    }
}

/// The accessible name of a source number or row (also the tooltip of
/// documents and rules).
fn source_label(citation: &Citation) -> String {
    let title = source_title(citation);
    match citation.kind.as_str() {
        "decision" => t::open_decision(&title),
        "document" => t::cite_document(&title, &citation.id),
        _ => t::cite_rule(&title),
    }
}

fn source_icon(citation: &Citation) -> IconName {
    match citation.kind.as_str() {
        "decision" => IconName::Decision,
        "document" => IconName::Book,
        _ => IconName::Shield,
    }
}

/// Product copy for an overview failure; storage detail goes to the log.
fn product(error: OverviewError) -> String {
    if let OverviewError::Storage(detail) = &error {
        tracing::error!(error = %detail, operation = "overview", "overview storage failed");
        return t::read_failed().into();
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
    use super::*;

    fn cite(kind: &str, id: &str) -> Citation {
        Citation {
            kind: kind.into(),
            id: id.into(),
            label: format!("L:{id}"),
            title: format!("Título {id}"),
        }
    }

    fn paragraph(citations: Vec<Citation>) -> OverviewParagraph {
        OverviewParagraph {
            text: "Texto do parágrafo.".into(),
            citations,
        }
    }

    fn summary() -> Vec<OverviewParagraph> {
        vec![
            paragraph(vec![cite("decision", "d1"), cite("document", "doc.md")]),
            paragraph(vec![
                cite("claim", "r1"),
                cite("decision", "d1"),
                cite("decision", "d1"),
            ]),
            paragraph(vec![cite("decision", "d2")]),
        ]
    }

    #[test]
    fn enter_on_the_first_number_opens_its_decision() {
        let sources = sources(&summary());
        let first = summary()[0].citations[0].clone();
        assert_eq!(source_number(&sources, &first), Some(1));
        assert_eq!(opens(&sources[0]).map(|open| open.0), Some("d1".into()));
        assert!(opens(&cite("document", "doc.md")).is_none());
        assert!(opens(&cite("claim", "r1")).is_none());
    }

    #[test]
    fn the_list_has_one_row_per_source_in_order_of_first_appearance() {
        let summary = summary();
        let sources = sources(&summary);
        let ids: Vec<&str> = sources.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["d1", "doc.md", "r1", "d2"]);
        // The second paragraph repeats d1 twice: one number, the same as before.
        let marks = unique(&summary[1].citations);
        assert_eq!(marks.len(), 2);
        assert_eq!(source_number(&sources, marks[1]), Some(1));
    }

    #[test]
    fn documents_and_rules_explain_themselves() {
        let document = cite("document", "doc.md");
        assert_eq!(
            source_label(&document),
            t::cite_document("Título doc.md", "doc.md")
        );
        assert_eq!(
            source_label(&cite("claim", "r1")),
            t::cite_rule("Título r1")
        );
        let untitled = Citation {
            title: String::new(),
            ..cite("decision", "d9")
        };
        assert_eq!(source_title(&untitled), "L:d9");
    }

    #[test]
    fn failures_read_as_sentences_and_hide_storage_detail() {
        assert_eq!(
            product(OverviewError::ProviderOff),
            "Ative um provedor de IA em Configurações › IA para gerar a visão."
        );
        assert!(!product(OverviewError::Storage("disk I/O".into())).contains("disk"));
    }
}
