//! Native review surface backed by the Inbox application port.
//! Storage runs in the background; technical failures never reach the view.

use application::inbox::{
    CandidateDetail, CandidateSummary, Inbox, InboxError, InboxFilter, InboxPage, InboxStore,
};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, Entity, FocusHandle, Render, Role, Stateful,
    Window,
};

use super::evidence;
use crate::ui::glass::focus_ring;
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

enum Outcome {
    Page(Result<(InboxPage, usize), InboxError>, bool),
    Detail(Result<CandidateDetail, InboxError>),
}

/// A paginated candidate list and its source-reading pane.
pub struct InboxScreen<S: InboxStore + Send + 'static> {
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
    project_id: Option<String>,
    generation: u64,
    search: Option<Entity<SearchField>>,
    total: Option<usize>,
}

impl<S: InboxStore + Send + 'static> InboxScreen<S> {
    /// Composes the application use case without opening storage in the view.
    pub fn new(cx: &mut Context<Self>, inbox: Inbox<S>) -> Self {
        Self {
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
            project_id: None,
            generation: 0,
            search: None,
            total: None,
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
        if self.busy || self.project_id.is_none() || (append && self.cursor.is_none()) {
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
        self.run(cx, move |inbox| Outcome::Detail(inbox.detail(&id)));
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
                        this.source_focus = detail
                            .artifacts
                            .iter()
                            .map(|_| cx.focus_handle().tab_stop(true))
                            .collect();
                        this.detail = Some(detail);
                    }
                    Outcome::Page(Err(error), _) | Outcome::Detail(Err(error)) => {
                        tracing::warn!(code = error.code(), "inbox view operation failed");
                        this.error = Some(failure_copy(&error));
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn row(&self, row: &CandidateSummary, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::quiet_glass();
        let selected = self.selected.as_deref() == Some(&row.id);
        let id = row.id.clone();
        let key_id = id.clone();
        let mut element = div()
            .id((ElementId::from("candidate"), row.id.clone()))
            .p(px(SpacingScale::S4))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .relative()
            .border_b_1()
            .border_color(theme.colors.hairline_divider())
            .bg(if selected {
                theme.colors.glass_surface_lavender()
            } else {
                theme.colors.rail()
            })
            .hover(move |style| {
                style.bg(if selected {
                    theme.colors.glass_surface_lavender()
                } else {
                    theme.colors.hover_veil()
                })
            })
            .when(selected, |row| {
                row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(2.0))
                        .bg(theme.colors.accent_subtle()),
                )
            })
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
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child(format!(
                        "{} · {}",
                        short_date(&row.received_at),
                        status_label(row.status)
                    )),
            )
            .child(text_style(div(), TypeScale::HEADING_3).child(row.question.clone()))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_secondary())
                    .child(row.choice.clone()),
            );
        if let Some((_, focus)) = self.row_focus.iter().find(|(id, _)| *id == row.id) {
            element = element.track_focus(focus);
        }
        element
    }

    fn button(
        &self,
        id: &'static str,
        label: &'static str,
        more: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let theme = Theme::quiet_glass();
        text_style(div(), TypeScale::BODY_SMALL)
            .id(id)
            .px(px(SpacingScale::S3))
            .h(px(36.0))
            .flex()
            .items_center()
            .rounded(theme.radius.control())
            .role(Role::Button)
            .aria_label(label)
            .track_focus(if more {
                &self.more_focus
            } else {
                &self.refresh_focus
            })
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .text_color(if self.busy {
                theme.colors.text_disabled()
            } else {
                theme.colors.text_secondary()
            })
            .hover(move |style| style.bg(theme.colors.hover_veil()))
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
        let theme = Theme::quiet_glass();
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
                    text_style(div(), TypeScale::BODY)
                        .text_color(theme.colors.text_muted())
                        .child(if self.busy {
                            "Carregando…"
                        } else {
                            "Selecione um candidato para ler as evidências."
                        }),
                )
                .into_any_element();
        };
        div()
            .id("inbox-reading")
            .size_full()
            .overflow_y_scroll()
            .p(px(SpacingScale::S8))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child(format!(
                        "{} · {}",
                        status_label(detail.summary.status),
                        short_date(&detail.summary.received_at)
                    )),
            )
            .child(text_style(div(), TypeScale::HEADING_1).child(detail.summary.question.clone()))
            .child(section("Escolha sugerida", &detail.summary.choice))
            .child(section("Motivo", &detail.rationale))
            .child(
                text_style(div(), TypeScale::HEADING_2)
                    .border_t_1()
                    .border_color(theme.colors.hairline_divider())
                    .pt(px(SpacingScale::S4))
                    .child(format!("Evidências · {} fontes", detail.artifacts.len())),
            )
            .child(
                div().flex().flex_wrap().gap(px(SpacingScale::S2)).children(
                    detail
                        .artifacts
                        .iter()
                        .enumerate()
                        .map(|(index, artifact)| {
                            let selected = index == self.source_index;
                            text_style(div(), TypeScale::BODY_SMALL)
                                .id(("source", index))
                                .p(px(SpacingScale::S2))
                                .rounded(theme.radius.control())
                                .border_1()
                                .border_color(if selected {
                                    theme.colors.accent_subtle()
                                } else {
                                    theme.colors.hairline_divider()
                                })
                                .role(Role::Button)
                                .aria_label(format!(
                                    "Fonte {}: {}",
                                    index + 1,
                                    evidence::label(artifact)
                                ))
                                .aria_selected(selected)
                                .track_focus(&self.source_focus[index])
                                .focus_visible(focus_ring(&theme))
                                .cursor_pointer()
                                .bg(if selected {
                                    theme.colors.glass_surface_lavender()
                                } else {
                                    theme.colors.layer_fill()
                                })
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.source_index = index;
                                    window.focus(&this.source_focus[index], cx);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    move |this, event: &gpui::KeyDownEvent, _, cx| {
                                        if matches!(event.keystroke.key.as_str(), "enter" | "space")
                                        {
                                            this.source_index = index;
                                            cx.notify();
                                            cx.stop_propagation();
                                        }
                                    },
                                ))
                                .child(evidence::label(artifact))
                        }),
                ),
            )
            .when(detail.artifacts.is_empty(), |pane| {
                pane.child(section(
                    "Fontes",
                    "Nenhuma fonte disponível para este candidato.",
                ))
            })
            .children(
                detail
                    .artifacts
                    .get(self.source_index)
                    .map(|artifact| evidence::snippet(artifact, self.source_index)),
            )
            .child(section(
                "Confiança da extração",
                &format!(
                    "{:.0}% · {}",
                    detail.summary.confidence * 100.0,
                    detail.summary.confidence_reason
                ),
            ))
            .child(section(
                "Origem",
                &format!(
                    "{}\nSessão: {}\nRecebido: {}",
                    detail.summary.project_location,
                    detail
                        .summary
                        .session_id
                        .as_deref()
                        .unwrap_or("não informada"),
                    detail.summary.received_at
                ),
            ))
            .into_any_element()
    }
}

impl<S: InboxStore + Send + 'static> Render for InboxScreen<S> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::quiet_glass();
        let visible: Vec<_> = self
            .rows
            .iter()
            .filter(|row| matches_query(row, &self.query))
            .collect();
        div().size_full().flex().flex_col()
            .children(self.error.map(|message| text_style(div(), TypeScale::BODY_SMALL).id("inbox-error").p(px(SpacingScale::S3))
                .role(Role::Status).text_color(theme.colors.status_danger()).child(message)))
            .child(div().flex_1().min_h(px(0.0)).flex()
                .child(div().w(px(320.0)).flex_none().h_full().flex().flex_col().bg(theme.colors.rail())
                    .border_r_1().border_color(theme.colors.hairline_divider())
                    .child(div().p(px(SpacingScale::S4)).flex().items_center().justify_between()
                        .child(text_style(div(), TypeScale::HEADING_2).child("Aguardando revisão"))
                        .child(self.button("inbox-refresh", "Atualizar", false, cx)))
                    .child(div().px(px(SpacingScale::S4)).pb(px(SpacingScale::S3)).children(self.search.clone()))
                    .child(text_style(div(), TypeScale::META).px(px(SpacingScale::S4)).pb(px(SpacingScale::S3)).text_color(theme.colors.text_muted())
                        .child(format!("{} carregados · {} visíveis", self.rows.len(), visible.len())))
                    .child(div().id("inbox-list").flex_1().min_h(px(0.0)).overflow_y_scroll()
                        .children(visible.iter().map(|row| self.row(row, cx)))
                        .when(visible.is_empty(), |list| list.child(text_style(div(), TypeScale::BODY_SMALL).p(px(SpacingScale::S6))
                            .text_color(theme.colors.text_muted()).child(if self.busy { "Carregando candidatos…" }
                            else if !self.loaded { "Atualize para carregar os candidatos." }
                            else if self.rows.is_empty() { "Nenhum candidato aguardando revisão. Novas capturas aparecerão aqui após a extração." }
                            else { "Nenhum candidato carregado corresponde à busca." }))))
                    .when(self.cursor.is_some(), |rail| rail.child(self.button("inbox-more", "Carregar mais", true, cx))))
                .child(div().flex_1().min_w(px(0.0)).h_full().child(self.reading_pane(cx))))
    }
}

fn section(label: &'static str, content: &str) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .border_t_1()
        .border_color(Theme::quiet_glass().colors.hairline_divider())
        .pt(px(SpacingScale::S4))
        .child(text_style(div(), TypeScale::HEADING_3).child(label))
        .child(
            text_style(div(), TypeScale::BODY)
                .text_color(Theme::quiet_glass().colors.text_secondary())
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

fn short_date(timestamp: &str) -> String {
    let months = [
        "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
    ];
    let date = timestamp.split('T').next().unwrap_or(timestamp);
    let parts: Vec<_> = date.split('-').collect();
    match parts.as_slice() {
        [year, month, day] => month
            .parse::<usize>()
            .ok()
            .and_then(|m| m.checked_sub(1))
            .and_then(|m| months.get(m))
            .map(|month| format!("{day} {month} {year}"))
            .unwrap_or_else(|| date.to_owned()),
        _ => date.to_owned(),
    }
}

fn status_label(status: application::inbox::CandidateStatus) -> &'static str {
    if status == application::inbox::CandidateStatus::Snoozed {
        "Adiado"
    } else {
        "A revisar"
    }
}

fn matches_query(row: &CandidateSummary, query: &str) -> bool {
    query.is_empty()
        || [&row.question, &row.choice, &row.project_location]
            .iter()
            .any(|value| value.to_lowercase().contains(query))
}

fn failure_copy(error: &InboxError) -> &'static str {
    match error {
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
