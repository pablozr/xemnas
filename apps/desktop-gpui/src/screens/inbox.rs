//! Native review surface backed by the Inbox application port.
//! Storage runs in the background; technical failures never reach the view.

use application::inbox::{
    CandidateDetail, CandidateEdits, CandidateStatus, CandidateSummary, Inbox, InboxError,
    InboxFilter, InboxPage, InboxStore,
};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, Entity, FocusHandle, Render, Role, Stateful,
    Subscription, Window,
};

use super::evidence;
use super::format::short_date;
use super::review_editor::{EditorEvent, ReviewEditor};
use crate::ui::controls::{action_button, ButtonKind};
use crate::ui::glass::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    count_chip, fade_in, hover_tint, mark_selected, panel_title, reading_title, section_label,
    status_pill, track_hover, word_wrapped,
};
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

enum Outcome {
    Page(Result<(InboxPage, usize), InboxError>, bool),
    Detail(Result<Box<CandidateDetail>, InboxError>),
    Action(Result<(), InboxError>, &'static str),
}

#[derive(Clone, Copy)]
enum ReviewAction {
    Reject,
    Snooze,
    Edit,
    Confirm,
}

/// A paginated candidate list and its source-reading pane.
pub struct InboxScreen<S: InboxStore + Send + 'static> {
    /// Row under the pointer, driving the hover spring.
    hovered: Option<String>,
    inbox: Option<Inbox<S>>,
    rows: Vec<CandidateSummary>,
    cursor: Option<String>,
    selected: Option<String>,
    detail: Option<CandidateDetail>,
    query: String,
    busy: bool,
    loaded: bool,
    error: Option<&'static str>,
    row_focus: Vec<(String, FocusHandle)>,
    refresh_focus: FocusHandle,
    more_focus: FocusHandle,
    source_index: usize,
    source_focus: Vec<FocusHandle>,
    source_lines: Vec<evidence::SourceLines>,
    project_id: Option<String>,
    generation: u64,
    search: Option<Entity<SearchField>>,
    total: Option<usize>,
    editor: Option<Entity<ReviewEditor>>,
    editor_subscription: Option<Subscription>,
    action_focus: [FocusHandle; 4],
    notice: Option<&'static str>,
}

/// The reading column shared with the Decisions document.
const READING_WIDTH: f32 = 760.0;

impl<S: InboxStore + Send + 'static> InboxScreen<S> {
    /// Composes the application use case without opening storage in the view.
    pub fn new(cx: &mut Context<Self>, inbox: Inbox<S>) -> Self {
        Self {
            hovered: None,
            inbox: Some(inbox),
            rows: Vec::new(),
            cursor: None,
            selected: None,
            detail: None,
            query: String::new(),
            busy: false,
            loaded: false,
            error: None,
            row_focus: Vec::new(),
            refresh_focus: cx.focus_handle().tab_stop(true),
            more_focus: cx.focus_handle().tab_stop(true),
            source_index: 0,
            source_focus: Vec::new(),
            source_lines: Vec::new(),
            project_id: None,
            generation: 0,
            search: None,
            total: None,
            editor: None,
            editor_subscription: None,
            action_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
            notice: None,
        }
    }

    /// Mounts the workspace search inside the candidate list.
    pub fn attach_search(&mut self, search: Entity<SearchField>) {
        self.search = Some(search);
    }

    /// Entire project queue, independent of the loaded page and local search.
    pub fn total_count(&self) -> Option<usize> {
        self.total
    }

    /// Refreshes the first page when the destination opens.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.page(false, cx);
    }

    /// Changes the review scope, discarding responses from the previous project.
    pub fn set_project(&mut self, project_id: Option<String>, cx: &mut Context<Self>) {
        if self.project_id == project_id {
            return;
        }
        self.project_id = project_id;
        self.generation += 1;
        self.rows.clear();
        self.cursor = None;
        self.selected = None;
        self.detail = None;
        self.query.clear();
        self.error = None;
        self.loaded = false;
        self.total = None;
        self.editor = None;
        self.editor_subscription = None;
        self.notice = None;
        self.row_focus.clear();
        self.source_focus.clear();
        self.source_index = 0;
        self.page(false, cx);
        cx.notify();
    }

    /// Filters loaded rows, explicitly distinct from a database-wide search.
    pub fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        self.query = query.trim().to_lowercase();
        cx.notify();
    }

    fn page(&mut self, append: bool, cx: &mut Context<Self>) {
        if self.busy
            || self.editor.is_some()
            || self.project_id.is_none()
            || (append && self.cursor.is_none())
        {
            return;
        }
        let filter = InboxFilter {
            project_id: self.project_id.clone(),
            cursor: if append { self.cursor.clone() } else { None },
            ..InboxFilter::default()
        };
        self.run(cx, move |inbox| {
            Outcome::Page(
                inbox
                    .list(&filter)
                    .and_then(|page| inbox.count(&filter).map(|count| (page, count))),
                append,
            )
        });
    }

    fn select(&mut self, id: String, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.selected = Some(id.clone());
        self.detail = None;
        self.source_index = 0;
        self.source_focus.clear();
        self.editor = None;
        self.editor_subscription = None;
        self.run(cx, move |inbox| {
            Outcome::Detail(inbox.detail(&id).map(Box::new))
        });
    }

    fn run(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(&Inbox<S>) -> Outcome + Send + 'static,
    ) {
        let Some(inbox) = self.inbox.take() else {
            return;
        };
        self.busy = true;
        if let Some(editor) = &self.editor {
            editor.update(cx, |editor, cx| editor.set_busy(true, cx));
        }
        self.error = None;
        let generation = self.generation;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let (inbox, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = operation(&inbox);
                    (inbox, outcome)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.inbox = Some(inbox);
                this.busy = false;
                if let Some(editor) = &this.editor {
                    editor.update(cx, |editor, cx| editor.set_busy(false, cx));
                }
                if generation != this.generation {
                    this.page(false, cx);
                    cx.notify();
                    return;
                }
                match outcome {
                    Outcome::Page(Ok((page, total)), append) => {
                        this.total = Some(total);
                        this.loaded = true;
                        if !append {
                            this.rows.clear();
                            this.detail = None;
                        }
                        for row in page.candidates {
                            if let Some(existing) =
                                this.rows.iter_mut().find(|existing| existing.id == row.id)
                            {
                                *existing = row;
                            } else {
                                this.rows.push(row);
                            }
                        }
                        this.cursor = page.next_cursor;
                        this.row_focus
                            .retain(|(id, _)| this.rows.iter().any(|row| row.id == *id));
                        for row in &this.rows {
                            if !this.row_focus.iter().any(|(id, _)| *id == row.id) {
                                this.row_focus
                                    .push((row.id.clone(), cx.focus_handle().tab_stop(true)));
                            }
                        }
                        if !append {
                            let next = this
                                .selected
                                .as_ref()
                                .filter(|id| this.rows.iter().any(|row| row.id == **id))
                                .cloned()
                                .or_else(|| this.rows.first().map(|row| row.id.clone()));
                            this.selected = None;
                            if let Some(id) = next {
                                this.select(id, cx);
                            }
                        }
                    }
                    Outcome::Detail(Ok(detail)) => {
                        this.source_lines = detail
                            .artifacts
                            .iter()
                            .map(evidence::SourceLines::new)
                            .collect();
                        this.source_focus = detail
                            .artifacts
                            .iter()
                            .map(|_| cx.focus_handle().tab_stop(true))
                            .collect();
                        this.detail = Some(*detail);
                    }
                    Outcome::Action(Ok(()), notice) => {
                        this.editor = None;
                        this.editor_subscription = None;
                        this.notice = Some(notice);
                        this.page(false, cx);
                    }
                    Outcome::Page(Err(error), _)
                    | Outcome::Detail(Err(error))
                    | Outcome::Action(Err(error), _) => {
                        tracing::warn!(code = error.code(), "inbox view operation failed");
                        this.error = Some(failure_copy(&error));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn review(&mut self, action: ReviewAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(detail) = self
            .detail
            .as_ref()
            .filter(|detail| matches_query(&detail.summary, &self.query))
        else {
            return;
        };
        if let ReviewAction::Edit = action {
            let original = CandidateEdits {
                question: detail.summary.question.clone(),
                choice: detail.summary.choice.clone(),
                rationale: detail.rationale.clone(),
            };
            let id = detail.summary.id.clone();
            let editor = cx.new(|cx| ReviewEditor::new(original, cx));
            window.focus(&editor.read(cx).initial_focus(cx), cx);
            self.editor_subscription = Some(cx.subscribe(
                &editor,
                move |this, _, event: &EditorEvent, cx| {
                    match event {
                        EditorEvent::Cancel if !this.busy => {
                            this.editor = None;
                            this.editor_subscription = None;
                        }
                        EditorEvent::Save(edits, confirm) if !this.busy => {
                            if let Err(error) = edits.validate() {
                                this.error = Some(failure_copy(&error));
                                cx.notify();
                                return;
                            }
                            let id = id.clone();
                            let edits = edits.clone();
                            let confirm = *confirm;
                            this.run(cx, move |inbox| {
                                Outcome::Action(
                                    if confirm {
                                        inbox.confirm(&id, Some(edits)).map(|_| ())
                                    } else {
                                        inbox.adjust(&id, edits)
                                    },
                                    if confirm {
                                        "Ajustes confirmados. Decisão criada."
                                    } else {
                                        "Ajustes salvos. O candidato continua na revisão."
                                    },
                                )
                            });
                        }
                        _ => {}
                    }
                    cx.notify();
                },
            ));
            self.editor = Some(editor);
            self.error = None;
            self.notice = None;
            cx.notify();
            return;
        }
        let id = detail.summary.id.clone();
        let snoozed = detail.summary.status == CandidateStatus::Snoozed;
        self.notice = None;
        window.focus(&self.refresh_focus, cx);
        self.run(cx, move |inbox| {
            let (result, notice) = match action {
                ReviewAction::Confirm => (
                    inbox.confirm(&id, None).map(|_| ()),
                    "Candidato confirmado. Decisão criada.",
                ),
                ReviewAction::Reject => (inbox.reject(&id), "Candidato rejeitado."),
                ReviewAction::Snooze if snoozed => {
                    (inbox.unsnooze(&id), "Candidato retomado para revisão.")
                }
                ReviewAction::Snooze => (
                    inbox.snooze(&id),
                    "Candidato adiado. Você pode retomá-lo depois.",
                ),
                ReviewAction::Edit => unreachable!(),
            };
            Outcome::Action(result, notice)
        });
    }

    fn review_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let Some(detail) = self
            .detail
            .as_ref()
            .filter(|detail| matches_query(&detail.summary, &self.query))
        else {
            return div().into_any_element();
        };
        let labels = [
            "Rejeitar",
            if detail.summary.status == CandidateStatus::Snoozed {
                "Retomar"
            } else {
                "Adiar"
            },
            "Ajustar",
            "Confirmar",
        ];
        div()
            .flex_none()
            .border_t_1()
            .border_color(theme.colors.hairline_divider())
            .px(px(SpacingScale::S8))
            .py(px(SpacingScale::S3))
            .bg(theme.colors.canvas())
            .flex()
            .flex_wrap()
            .justify_end()
            .gap(px(SpacingScale::S2))
            .children(
                [
                    ReviewAction::Reject,
                    ReviewAction::Snooze,
                    ReviewAction::Edit,
                    ReviewAction::Confirm,
                ]
                .into_iter()
                .enumerate()
                .map(|(index, action)| {
                    let kind = if index == 3 {
                        ButtonKind::Primary
                    } else {
                        ButtonKind::Secondary
                    };
                    action_button(&theme, ("review-action", index), kind, !self.busy)
                        .px(px(SpacingScale::S4))
                        .aria_label(labels[index])
                        .track_focus(&self.action_focus[index])
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.review(action, window, cx)),
                        )
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.review(action, window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        ))
                        .child(labels[index])
                }),
            )
            .into_any_element()
    }

    fn row(&self, row: &CandidateSummary, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let selected = self.selected.as_deref() == Some(&row.id);
        let id = row.id.clone();
        let key_id = id.clone();
        let hover_key = id.clone();
        let hovered = self.hovered.as_deref() == Some(row.id.as_str());
        let element = div()
            .id((ElementId::from("candidate"), row.id.clone()))
            .relative()
            .px(px(SpacingScale::S4))
            .py(px(SpacingScale::S3))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if track_hover(&mut this.hovered, hover_key.clone(), *hovered) {
                    cx.notify();
                }
            }))
            .role(Role::Button)
            .aria_label(row.question.clone())
            .aria_selected(selected)
            .cursor_pointer()
            .focus_visible(focus_ring(&theme))
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some((_, focus)) = this.row_focus.iter().find(|(key, _)| *key == id) {
                    window.focus(focus, cx);
                }
                this.select(id.clone(), cx);
            }))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.select(key_id.clone(), cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(SpacingScale::S2))
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child(short_date(&row.received_at)),
                    )
                    .when(row.status != CandidateStatus::Pending, |line| {
                        line.child(status_badge(row.status, theme))
                    }),
            )
            .child(
                word_wrapped(&row.question, TypeScale::ROW_TITLE, Some(2))
                    .text_color(theme.colors.text_primary()),
            )
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_muted())
                    .line_clamp(2)
                    .child(row.choice.clone()),
            );
        let mut element = mark_selected(element, &theme, selected);
        if let Some((_, focus)) = self.row_focus.iter().find(|(id, _)| *id == row.id) {
            element = element.track_focus(focus);
        }
        hover_tint(
            element,
            ElementId::Name(format!("candidate-hover-{}", row.id).into()),
            hovered,
            !selected,
            &theme,
        )
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        more: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::current(cx);
        action_button(&theme, id, ButtonKind::Ghost, !self.busy)
            .aria_label(label)
            .track_focus(if more {
                &self.more_focus
            } else {
                &self.refresh_focus
            })
            .on_click(cx.listener(move |this, _, _, cx| this.page(more, cx)))
            .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.page(more, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label)
    }

    fn reading_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let visible_detail = self
            .detail
            .as_ref()
            .filter(|detail| matches_query(&detail.summary, &self.query));
        let Some(detail) = visible_detail else {
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child(if self.busy {
                            "Carregando…"
                        } else {
                            "Selecione um candidato para ler as evidências."
                        }),
                )
                .into_any_element();
        };
        let evidence_block = if detail.artifacts.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child("Nenhuma fonte disponível para este candidato.")
                .into_any_element()
        } else {
            let tabs = evidence::tab_strip(&theme, "evidence-tabs").children(
                detail
                    .artifacts
                    .iter()
                    .enumerate()
                    .map(|(index, artifact)| {
                        evidence::tab(
                            &theme,
                            ("source", index),
                            artifact,
                            index == self.source_index,
                        )
                        .track_focus(&self.source_focus[index])
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.source_index = index;
                            window.focus(&this.source_focus[index], cx);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.source_index = index;
                                    cx.notify();
                                    cx.stop_propagation();
                                }
                            },
                        ))
                    }),
            );
            evidence::frame(&theme)
                .child(tabs)
                .children(
                    detail
                        .artifacts
                        .get(self.source_index)
                        .zip(self.source_lines.get(self.source_index))
                        .map(|(artifact, lines)| {
                            div()
                                .flex()
                                .flex_col()
                                .child(evidence::caption_row(&theme, artifact, lines))
                                .child(evidence::body(
                                    artifact,
                                    format!("{}-{}", detail.summary.id, self.source_index),
                                    lines,
                                    280.0,
                                    theme,
                                ))
                        }),
                )
                .into_any_element()
        };
        let column = div()
            .w_full()
            .max_w(px(READING_WIDTH))
            .mx_auto()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .child(status_badge(detail.summary.status, theme))
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child(short_date(&detail.summary.received_at)),
                    ),
            )
            .child(reading_title(&detail.summary.question).text_color(theme.colors.text_primary()))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .border_l_2()
                    .border_color(theme.colors.accent_hover())
                    .pl(px(SpacingScale::S4))
                    .child(
                        section_label(&theme, "Escolha sugerida")
                            .text_color(theme.colors.accent_hover()),
                    )
                    .child(
                        text_style(div(), TypeScale::HEADING_2)
                            .text_color(theme.colors.text_primary())
                            .child(detail.summary.choice.clone()),
                    ),
            )
            .child(section(theme, "Motivo", &detail.rationale))
            .child(
                div()
                    .pt(px(SpacingScale::S2))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(icon(IconName::Link, 14.0, theme.colors.text_muted()))
                            .child(
                                text_style(div(), TypeScale::HEADING_3)
                                    .flex_1()
                                    .child("Evidências"),
                            )
                            .child(count_chip(
                                &theme,
                                match detail.artifacts.len() {
                                    1 => "1 fonte".to_string(),
                                    n => format!("{n} fontes"),
                                },
                            )),
                    )
                    .child(evidence_block),
            )
            .child(
                div()
                    .pt(px(SpacingScale::S5))
                    .border_t_1()
                    .border_color(theme.colors.hairline_divider())
                    .flex()
                    .flex_wrap()
                    .gap(px(SpacingScale::S8))
                    .child(
                        section(
                            theme,
                            "Confiança da extração",
                            &format!(
                                "{:.0}% · {}",
                                detail.summary.confidence * 100.0,
                                detail.summary.confidence_reason
                            ),
                        )
                        .flex_1()
                        .min_w(px(220.0)),
                    )
                    .child(
                        section(
                            theme,
                            "Origem",
                            &format!(
                                "{}\nSessão: {}\nRecebido: {}",
                                detail.summary.project_location,
                                detail
                                    .summary
                                    .session_id
                                    .as_deref()
                                    .unwrap_or("não informada"),
                                short_date(&detail.summary.received_at)
                            ),
                        )
                        .flex_1()
                        .min_w(px(220.0)),
                    ),
            );
        div()
            .id("inbox-reading")
            .size_full()
            .overflow_y_scroll()
            .px(px(SpacingScale::S8))
            .py(px(SpacingScale::S8))
            .child(fade_in(
                column,
                ElementId::Name(format!("reading-{}", detail.summary.id).into()),
            ))
            .into_any_element()
    }
}

impl<S: InboxStore + Send + 'static> Render for InboxScreen<S> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let visible: Vec<_> = self
            .rows
            .iter()
            .filter(|row| matches_query(row, &self.query))
            .collect();
        div().size_full().flex().flex_col()
            .children(self.notice.map(|message| text_style(div(), TypeScale::BODY_SMALL).id("review-notice").p(px(SpacingScale::S3))
                .role(Role::Status).text_color(theme.colors.text_secondary()).child(message)))
            .children(self.error.map(|message| text_style(div(), TypeScale::BODY_SMALL).id("inbox-error").p(px(SpacingScale::S3))
                .role(Role::Status).text_color(theme.colors.status_danger()).child(message)))
            .child(div().flex_1().min_h(px(0.0)).flex()
                .child(div().w(px(320.0)).flex_none().h_full().flex().flex_col().bg(theme.colors.rail())
                    .border_r_1().border_color(theme.colors.hairline_divider())
                    .child(div().px(px(SpacingScale::S4)).pt(px(SpacingScale::S3)).pb(px(SpacingScale::S2)).flex().items_center().gap(px(SpacingScale::S2))
                        .child(panel_title(&theme, "Aguardando revisão"))
                        .child(count_chip(&theme, if self.rows.len() == visible.len() {
                            visible.len().to_string()
                        } else {
                            format!("{} de {}", visible.len(), self.rows.len())
                        }))
                        .child(div().flex_1())
                        .child(self.button("inbox-refresh", "Atualizar", false, cx)))
                    .child(div().px(px(SpacingScale::S4)).pb(px(SpacingScale::S3)).children(self.search.clone()))
                    .child(div().id("inbox-list").flex_1().min_h(px(0.0)).overflow_y_scroll()
                        .children(visible.iter().map(|row| self.row(row, cx)))
                        .when(visible.is_empty(), |list| list.child(text_style(div(), TypeScale::BODY_SMALL).p(px(SpacingScale::S6))
                            .text_color(theme.colors.text_muted()).child(if self.busy { "Carregando candidatos…" }
                            else if !self.loaded { "Atualize para carregar os candidatos." }
                            else if self.rows.is_empty() { "Nenhum candidato aguardando revisão. Novas capturas aparecerão aqui após a extração." }
                            else { "Nenhum candidato carregado corresponde à busca." }))))
                    .child(text_style(div(), TypeScale::META).flex_none().px(px(SpacingScale::S4)).py(px(SpacingScale::S2))
                        .flex().items_center().justify_between()
                        .border_t_1().border_color(theme.colors.hairline_divider()).text_color(theme.colors.text_muted())
                        .child(format!("{} carregados · {} visíveis", self.rows.len(), visible.len()))
                        .when(self.cursor.is_some(), |footer| footer.child(self.button("inbox-more", "Carregar mais", true, cx)))))
                .child(div().flex_1().min_w(px(0.0)).h_full().flex().flex_col()
                    .child(div().flex_1().min_h(px(0.0)).child(if let Some(editor) = self.editor.as_ref().filter(|_| self.detail.as_ref().is_some_and(|detail| matches_query(&detail.summary, &self.query))) { editor.clone().into_any_element() } else { self.reading_pane(cx) }))
                    .when(self.editor.is_none(), |pane| pane.child(self.review_actions(cx)))))
    }
}

fn section(theme: Theme, label: &'static str, content: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .child(section_label(&theme, label))
        .child(
            text_style(div(), TypeScale::BODY)
                .text_color(theme.colors.text_secondary())
                .child(content.to_owned()),
        )
}

#[cfg(test)]
fn project_label(location: &str) -> &str {
    location
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(location)
}

fn status_badge(status: CandidateStatus, theme: Theme) -> Div {
    let (label, color) = match status {
        CandidateStatus::Pending => ("Pendente", theme.colors.status_warning()),
        CandidateStatus::Snoozed => ("Adiado", theme.colors.status_info()),
        CandidateStatus::Accepted => ("Confirmado", theme.colors.status_success()),
        CandidateStatus::EditedAndAccepted => {
            ("Ajustado e confirmado", theme.colors.status_success())
        }
        CandidateStatus::Dismissed => ("Rejeitado", theme.colors.status_danger()),
    };
    status_pill(&theme, color, label)
}

fn matches_query(row: &CandidateSummary, query: &str) -> bool {
    query.is_empty()
        || [&row.question, &row.choice, &row.project_location]
            .iter()
            .any(|value| value.to_lowercase().contains(query))
}

fn failure_copy(error: &InboxError) -> &'static str {
    match error {
        InboxError::InvalidEdits(_) => "Preencha pergunta, escolha e motivo. Limites: 500, 1.000 e 4.000 caracteres, respectivamente.",
        InboxError::NotFound | InboxError::InvalidState => {
            "Este candidato mudou. Atualize a Inbox para continuar."
        }
        _ => "Não foi possível carregar a Inbox. Tente atualizar; se persistir, reabra o app.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_details_never_reach_the_error_surface() {
        let message = failure_copy(&InboxError::Storage("C:\\private\\token-secret.db".into()));
        assert!(!message.contains("private"));
        assert!(!message.contains("secret"));
        assert_eq!(
            message,
            failure_copy(&InboxError::InvalidData("sensitive artifact".into()))
        );
    }

    #[test]
    fn project_labels_handle_both_path_styles_and_trailing_separator() {
        assert_eq!(project_label("C:\\work\\xemnas\\"), "xemnas");
        assert_eq!(project_label("/work/xemnas/"), "xemnas");
    }
}
