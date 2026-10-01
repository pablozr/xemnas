//! Native review surface backed by the Inbox application port.
//! Storage runs in the background; technical failures never reach the view.

use std::sync::Arc;

use application::adoption::{AdoptionApi, AdoptionError, AdoptionPreview, ProposedLink};
use application::extract::{CandidateKind, MIN_SIGNIFICANCE};
use application::inbox::{
    CandidateDetail, CandidateEdits, CandidateStatus, CandidateSummary, Inbox, InboxError,
    InboxFilter, InboxPage, InboxStore,
};
use domain::entities::{EdgeKind, EntityKind};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, Entity, FocusHandle, Render, Role, ScrollHandle,
    Stateful, Subscription, Window,
};

use super::evidence;
use super::format::short_date;
use super::review_editor::{EditorEvent, ReviewEditor};
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::glass::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    action_footer, count_chip, empty_panel, error_banner, fade_in, hover_tint, kbd, mark_selected,
    panel_title, reading_title, section_label, skeleton_list, status_pill, toast, track_hover,
    word_wrapped, READING_WIDTH, TOAST_DURATION,
};
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

enum Outcome {
    Page(Result<(InboxPage, usize, usize), InboxError>, bool),
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
    /// Notice whose dismissal timer is already running.
    notice_scheduled: Option<&'static str>,
    list_scroll: ScrollHandle,
    /// Whether low-significance candidates are listed too.
    show_low: bool,
    /// Pending candidates kept out of the queue for low significance.
    hidden_low: usize,
    low_focus: FocusHandle,
    /// Adoption with the map; without it, confirming only creates the decision.
    adoption: Option<Arc<dyn AdoptionApi>>,
    /// The ties the selected candidate's evidence points to, each kept or not.
    links: Option<MapLinks>,
}

/// The "No mapa" section of one candidate.
struct MapLinks {
    candidate_id: String,
    links: Vec<(ProposedLink, bool)>,
    uncovered: Vec<String>,
    focus: Vec<FocusHandle>,
}

impl<S: InboxStore + Send + 'static> InboxScreen<S> {
    /// Confirms through the adoption use case, so a decision lands on the
    /// map with the ties the person kept.
    pub fn set_adoption(&mut self, adoption: Arc<dyn AdoptionApi>) {
        self.adoption = Some(adoption);
    }

    fn load_links(&mut self, candidate_id: String, cx: &mut Context<Self>) {
        let Some(adoption) = self.adoption.clone() else {
            return;
        };
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let id = candidate_id.clone();
            let preview = cx
                .background_executor()
                .spawn(async move { adoption.preview(&id) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if generation != this.generation || this.selected.as_deref() != Some(&candidate_id)
                {
                    return;
                }
                match preview {
                    Ok(AdoptionPreview { links, uncovered }) => {
                        this.links = Some(MapLinks {
                            candidate_id,
                            focus: links
                                .iter()
                                .map(|_| cx.focus_handle().tab_stop(true))
                                .collect(),
                            links: links.into_iter().map(|link| (link, true)).collect(),
                            uncovered,
                        });
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "adoption preview failed");
                        this.links = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// Kept and declined ties of `candidate_id`, when its preview is loaded.
    fn chosen_links(&self, candidate_id: &str) -> Option<(Vec<ProposedLink>, Vec<ProposedLink>)> {
        let links = self
            .links
            .as_ref()
            .filter(|links| links.candidate_id == candidate_id)?;
        let (kept, declined): (Vec<_>, Vec<_>) =
            links.links.iter().cloned().partition(|(_, keep)| *keep);
        Some((
            kept.into_iter().map(|(link, _)| link).collect(),
            declined.into_iter().map(|(link, _)| link).collect(),
        ))
    }

    fn toggle_link(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some((_, keep)) = self
            .links
            .as_mut()
            .and_then(|links| links.links.get_mut(index))
        {
            *keep = !*keep;
            cx.notify();
        }
    }
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
            notice_scheduled: None,
            list_scroll: ScrollHandle::new(),
            show_low: false,
            hidden_low: 0,
            low_focus: cx.focus_handle().tab_stop(true),
            adoption: None,
            links: None,
        }
    }

    fn toggle_low(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.show_low = !self.show_low;
        self.page(false, cx);
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
            min_significance: (!self.show_low).then_some(MIN_SIGNIFICANCE),
            ..InboxFilter::default()
        };
        let everything = InboxFilter {
            min_significance: None,
            cursor: None,
            ..filter.clone()
        };
        let relevant = InboxFilter {
            min_significance: Some(MIN_SIGNIFICANCE),
            cursor: None,
            ..filter.clone()
        };
        self.run(cx, move |inbox| {
            Outcome::Page(
                inbox.list(&filter).and_then(|page| {
                    let count = inbox.count(&filter)?;
                    let hidden = inbox
                        .count(&everything)?
                        .saturating_sub(inbox.count(&relevant)?);
                    Ok((page, count, hidden))
                }),
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
        self.links = None;
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
                    Outcome::Page(Ok((page, total, hidden)), append) => {
                        this.total = Some(total);
                        this.hidden_low = hidden;
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
                        let id = detail.summary.id.clone();
                        this.detail = Some(*detail);
                        this.load_links(id, cx);
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

    /// Loaded candidates for the command palette: id and question.
    pub fn palette_rows(&self) -> Vec<(String, String)> {
        self.rows
            .iter()
            .map(|row| (row.id.clone(), row.question.clone()))
            .collect()
    }

    /// Opens a loaded candidate from the command palette.
    pub fn open_candidate(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(index) = self.rows.iter().position(|row| row.id == id) {
            self.list_scroll.scroll_to_item(index);
        }
        if let Some((_, focus)) = self.row_focus.iter().find(|(key, _)| *key == id) {
            window.focus(focus, cx);
        }
        self.select(id, cx);
    }

    /// Moves the selection through the visible queue and keeps it in view.
    pub fn move_selection(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_some() {
            return;
        }
        let visible: Vec<String> = self
            .rows
            .iter()
            .filter(|row| matches_query(row, &self.query))
            .map(|row| row.id.clone())
            .collect();
        if visible.is_empty() {
            return;
        }
        let current = self
            .selected
            .as_ref()
            .and_then(|id| visible.iter().position(|row| row == id));
        let next = match current {
            Some(index) => (index as isize + delta).clamp(0, visible.len() as isize - 1) as usize,
            None => 0,
        };
        let id = visible[next].clone();
        if let Some((_, focus)) = self.row_focus.iter().find(|(key, _)| *key == id) {
            window.focus(focus, cx);
        }
        self.list_scroll.scroll_to_item(next);
        self.select(id, cx);
    }

    /// Keyboard shortcut for a review action, in footer order:
    /// 0 reject, 1 snooze/resume, 2 adjust, 3 confirm.
    pub fn run_shortcut(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.editor.is_some() || self.detail.is_none() {
            return;
        }
        let action = [
            ReviewAction::Reject,
            ReviewAction::Snooze,
            ReviewAction::Edit,
            ReviewAction::Confirm,
        ][index.min(3)];
        self.review(action, window, cx);
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
                            let adoption = this.adoption.clone();
                            let chosen = this.chosen_links(&id);
                            this.run(cx, move |inbox| {
                                if !confirm {
                                    return Outcome::Action(
                                        inbox.adjust(&id, edits),
                                        "Ajustes salvos. O candidato continua na revisão.",
                                    );
                                }
                                match (adoption, chosen) {
                                    (Some(adoption), Some((kept, declined))) => adopted(
                                        adoption.adopt(&id, Some(edits), &kept, &declined),
                                        "Ajustes confirmados. Decisão criada e ligada ao mapa.",
                                        "Ajustes confirmados. Decisão criada.",
                                    ),
                                    _ => Outcome::Action(
                                        inbox.confirm(&id, Some(edits)).map(|_| ()),
                                        "Ajustes confirmados. Decisão criada.",
                                    ),
                                }
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
        let adoption = self.adoption.clone();
        let chosen = self.chosen_links(&id);
        self.notice = None;
        window.focus(&self.refresh_focus, cx);
        self.run(cx, move |inbox| {
            let (result, notice) = match action {
                ReviewAction::Confirm => match (adoption, chosen) {
                    (Some(adoption), Some((kept, declined))) => {
                        return adopted(
                            adoption.adopt(&id, None, &kept, &declined),
                            "Candidato confirmado. Decisão criada e ligada ao mapa.",
                            "Candidato confirmado. Decisão criada.",
                        );
                    }
                    _ => (
                        inbox.confirm(&id, None).map(|_| ()),
                        "Candidato confirmado. Decisão criada.",
                    ),
                },
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
        action_footer(&theme, self.busy.then_some(("Registrando…", false)))
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
                        .child(kbd(
                            button_foreground(&theme, kind, !self.busy),
                            ["R", "S", "A", "C"][index],
                        ))
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

    /// Shows or hides the low-significance candidates kept out of the queue.
    fn low_toggle(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let label = if self.show_low {
            "Esconder os de baixa relevância".to_owned()
        } else {
            format!("Mostrar {} de baixa relevância", self.hidden_low)
        };
        div()
            .px(px(SpacingScale::S4))
            .pb(px(SpacingScale::S2))
            .id("inbox-low-row")
            .child(
                action_button(&theme, "inbox-low", ButtonKind::Ghost, !self.busy)
                    .aria_label(label.clone())
                    .track_focus(&self.low_focus)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_low(cx)))
                    .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.toggle_low(cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(label),
            )
    }

    /// What adopting the candidate puts on the map: each tie its evidence
    /// points to, kept by default, and the files no component covers yet.
    fn map_section(
        &self,
        theme: &Theme,
        candidate_id: &str,
        kind: CandidateKind,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        let links = self
            .links
            .as_ref()
            .filter(|links| links.candidate_id == candidate_id)?;
        if links.links.is_empty() && links.uncovered.is_empty() {
            return None;
        }
        let colors = theme.colors;
        let kept = links.links.iter().filter(|(_, keep)| *keep).count();
        let mut list = div().flex().flex_col();
        for (index, (link, keep)) in links.links.iter().enumerate() {
            let keep = *keep;
            let verb = match link.kind {
                EdgeKind::Uses => "usa",
                EdgeKind::AppliesTo => "vale para",
                _ => "muda",
            };
            let glyph = match link.entity_kind {
                EntityKind::Component => IconName::Layers,
                EntityKind::Technology => IconName::Cpu,
            };
            list = list.child(
                div()
                    .id(("map-link", index))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .px(px(SpacingScale::S3))
                    .py(px(SpacingScale::S2))
                    .rounded(theme.radius.control())
                    .cursor_pointer()
                    .hover(move |style| style.bg(colors.glass_fill_medium()))
                    .role(Role::CheckBox)
                    .aria_label(format!("{verb} {}", link.entity_name))
                    .aria_toggled(if keep {
                        gpui::Toggled::True
                    } else {
                        gpui::Toggled::False
                    })
                    .track_focus(&links.focus[index])
                    .focus_visible(focus_ring(theme))
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_link(index, cx)))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.toggle_link(index, cx);
                            cx.stop_propagation();
                        }
                    }))
                    .child(
                        div()
                            .size(px(16.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(if keep {
                                colors.accent_hover()
                            } else {
                                colors.glass_border_card_hover()
                            })
                            .when(keep, |box_| {
                                box_.bg(colors.accent_hover()).child(icon(
                                    IconName::Check,
                                    11.0,
                                    colors.accent_on_emphasis(),
                                ))
                            }),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .w(px(64.0))
                            .flex_none()
                            .text_color(colors.text_muted())
                            .child(verb),
                    )
                    .child(icon(glyph, 13.0, colors.text_secondary()))
                    .child(
                        text_style(div(), TypeScale::ROW_TITLE)
                            .text_color(if keep {
                                colors.text_primary()
                            } else {
                                colors.text_muted()
                            })
                            .child(link.entity_name.clone()),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .font_family(Theme::font_mono())
                            .text_color(colors.text_muted())
                            .child(link.reason.clone()),
                    ),
            );
        }
        let uncovered = (!links.uncovered.is_empty()).then(|| {
            let shown: Vec<&str> = links.uncovered.iter().take(3).map(String::as_str).collect();
            let more = links.uncovered.len().saturating_sub(3);
            text_style(div(), TypeScale::META)
                .text_color(colors.text_muted())
                .child(format!(
                    "Sem componente no mapa: {}{}. Crie no Mapa para ligar da próxima vez.",
                    shown.join(", "),
                    if more > 0 {
                        format!(" e mais {more}")
                    } else {
                        String::new()
                    }
                ))
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(icon(IconName::Graph, 14.0, colors.text_muted()))
                        .child(
                            text_style(div(), TypeScale::HEADING_3)
                                .flex_1()
                                .child("No mapa"),
                        )
                        .when(!links.links.is_empty(), |row| {
                            row.child(count_chip(
                                theme,
                                format!("{kept} de {}", links.links.len()),
                            ))
                        }),
                )
                .when(!links.links.is_empty(), |section| {
                    section.child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(match kind {
                                CandidateKind::Rule => {
                                    "Ao confirmar, a regra passa a valer nos itens marcados."
                                }
                                _ => {
                                    "Ao confirmar, a decisão fica ligada aos itens marcados; \
                                      os desmarcados não voltam como sugestão."
                                }
                            }),
                    )
                })
                .child(list)
                .children(uncovered),
        )
    }

    fn reading_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let visible_detail = self
            .detail
            .as_ref()
            .filter(|detail| matches_query(&detail.summary, &self.query));
        let Some(detail) = visible_detail else {
            if self.loaded && self.rows.is_empty() && !self.busy {
                return empty_panel(
                    &theme,
                    IconName::List,
                    "Fila de revisão",
                    "Nada aguardando revisão",
                    "Quando uma sessão do OpenCode registrar uma escolha de engenharia, o extrator propõe um candidato aqui para você confirmar, ajustar ou rejeitar.",
                )
                .into_any_element();
            }
            return div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child(if self.busy {
                            ""
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
                    .when(detail.summary.kind == CandidateKind::Rule, |line| {
                        line.child(status_pill(&theme, theme.colors.status_info(), "Regra"))
                    })
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child(short_date(&detail.summary.received_at)),
                    ),
            )
            .when(!detail.criteria.is_empty(), |page| {
                page.child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_muted())
                        .child(format!(
                            "Por que importa: {}",
                            detail
                                .criteria
                                .iter()
                                .map(|criterion| criterion_label(criterion))
                                .collect::<Vec<_>>()
                                .join(" · ")
                        )),
                )
            })
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
            .children(self.map_section(&theme, &detail.summary.id, detail.summary.kind, cx))
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
                        confidence(
                            theme,
                            detail.summary.confidence as f32,
                            &detail.summary.confidence_reason,
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
        if self.notice.is_some() && self.notice != self.notice_scheduled {
            self.notice_scheduled = self.notice;
            let shown = self.notice;
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TOAST_DURATION).await;
                let _ = this.update(cx, |screen, cx| {
                    if screen.notice == shown {
                        screen.notice = None;
                        screen.notice_scheduled = None;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        let retry = self.error.map(|_| {
            action_button(&theme, "inbox-retry", ButtonKind::Secondary, !self.busy)
                .aria_label("Tentar novamente")
                .on_click(cx.listener(|this, _, _, cx| this.page(false, cx)))
                .child("Tentar novamente")
        });
        div().size_full().relative().flex().flex_col()
            .children(self.error.map(|message| error_banner(&theme, message).id("inbox-error").role(Role::Alert).children(retry)))
            .children(self.notice.map(|message| toast(&theme, message, 72.0)))
            .child(div().flex_1().min_h(px(0.0)).flex()
                .child(div().w(px(320.0)).flex_none().h_full().flex().flex_col().bg(theme.colors.pane())
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
                    .when(self.hidden_low > 0 || self.show_low, |rail| rail.child(self.low_toggle(cx)))
                    .child(div().id("inbox-list").flex_1().min_h(px(0.0)).overflow_y_scroll().track_scroll(&self.list_scroll)
                        .children(visible.iter().map(|row| self.row(row, cx)))
                        .when(visible.is_empty() && self.busy, |list| list.child(skeleton_list(&theme, "inbox-skeleton", 5)))
                        .when(visible.is_empty() && !self.busy, |list| list.child(text_style(div(), TypeScale::BODY_SMALL).p(px(SpacingScale::S4))
                            .text_color(theme.colors.text_muted()).child(
                            if !self.loaded { "Atualize para carregar os candidatos." }
                            else if self.rows.is_empty() { "Fila vazia." }
                            else { "Nenhum candidato carregado corresponde à busca." }))))
                    .when(self.cursor.is_some() || self.rows.len() != visible.len(), |rail| rail.child(text_style(div(), TypeScale::META).flex_none().px(px(SpacingScale::S4)).py(px(SpacingScale::S2))
                        .flex().items_center().justify_between()
                        .border_t_1().border_color(theme.colors.hairline_divider()).text_color(theme.colors.text_muted())
                        .child(format!("{} carregados · {} visíveis", self.rows.len(), visible.len()))
                        .when(self.cursor.is_some(), |footer| footer.child(self.button("inbox-more", "Carregar mais", true, cx))))))
                .child(div().flex_1().min_w(px(0.0)).h_full().flex().flex_col()
                    .child(div().flex_1().min_h(px(0.0)).child(if let Some(editor) = self.editor.as_ref().filter(|_| self.detail.as_ref().is_some_and(|detail| matches_query(&detail.summary, &self.query))) { editor.clone().into_any_element() } else { self.reading_pane(cx) }))
                    .when(self.editor.is_none(), |pane| pane.child(self.review_actions(cx)))))
    }
}

/// The outcome of an adoption: the map failing after the decision was
/// created still counts as confirmed, with its own notice.
fn adopted(
    result: Result<application::adoption::AdoptOutcome, AdoptionError>,
    linked: &'static str,
    plain: &'static str,
) -> Outcome {
    match result {
        Ok(outcome) if outcome.linked > 0 => Outcome::Action(Ok(()), linked),
        Ok(_) => Outcome::Action(Ok(()), plain),
        Err(AdoptionError::Inbox(error)) => Outcome::Action(Err(error), plain),
        Err(AdoptionError::Graph(error)) => {
            tracing::warn!(code = error.code(), "map update after adoption failed");
            Outcome::Action(
                Ok(()),
                "Decisão criada. Não foi possível atualizar o mapa; abra o Mapa para revisar.",
            )
        }
    }
}

/// Product copy for a significance criterion.
fn criterion_label(criterion: &str) -> &'static str {
    match criterion {
        "cross_cutting" => "afeta várias partes",
        "data_or_contract" => "dados ou contrato",
        "security_or_privacy" => "segurança ou privacidade",
        "external_dependency" => "dependência externa",
        "hard_to_reverse" => "difícil de reverter",
        "first_of_a_kind" => "primeira vez no projeto",
        "past_problem" => "resolve problema recorrente",
        "constrains_future_work" => "condiciona trabalho futuro",
        _ => "outro motivo",
    }
}

/// The extractor's own estimate, drawn as a short meter beside the number.
/// Labelled as an estimate: it is not a human assessment.
fn confidence(theme: Theme, value: f32, reason: &str) -> Div {
    let value = value.clamp(0.0, 1.0);
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .child(section_label(&theme, "Confiança da extração"))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .child(
                    div()
                        .w(px(120.0))
                        .h(px(4.0))
                        .rounded_full()
                        .bg(theme.colors.surface())
                        .child(
                            div()
                                .h_full()
                                .w(gpui::relative(value))
                                .rounded_full()
                                .bg(theme.colors.accent_default()),
                        ),
                )
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_primary())
                        .child(format!("{:.0}%", value * 100.0)),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_muted())
                        .child("estimativa do extrator"),
                ),
        )
        .child(
            text_style(div(), TypeScale::BODY)
                .text_color(theme.colors.text_secondary())
                .child(reason.to_owned()),
        )
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
