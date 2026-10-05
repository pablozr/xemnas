//! Native review surface backed by the Inbox application port.
//! Storage runs in the background; technical failures never reach the view.

use application::capture_progress::{AssessmentReason, CaptureProgress, CaptureState};
use std::sync::Arc;

/// Project-scoped, bounded capture destination read supplied by application.
pub type ProgressRead = Arc<dyn Fn(&str) -> Result<Vec<CaptureProgress>, String> + Send + Sync>;
/// Reprocessing action that applies backend job authorization.
pub type ProgressRetry = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

use application::adoption::{AdoptionApi, AdoptionError, AdoptionPreview, ProposedLink};
use application::auto_approval::{ApprovalsApi, By, Entry, ItemKind, Mode, Status, Verdict};
use application::briefing::Briefing;
use application::extract::{CandidateKind, MIN_SIGNIFICANCE};
use application::inbox::{
    CandidateDetail, CandidateEdits, CandidateStatus, CandidateSummary, Inbox, InboxError,
    InboxFilter, InboxPage, InboxStore,
};
use application::review_exception::{ReviewExceptionStore, ReviewGroup, ReviewMetrics};
use domain::entities::{EdgeKind, EntityKind};
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, Entity, EventEmitter, FocusHandle, Render, Role,
    ScrollHandle, Stateful, Subscription, Window,
};

use super::context::OpenDecision;
use super::evidence;
use super::format::{relative, short_date};
use super::review_editor::{EditorEvent, ReviewEditor};
use crate::i18n::inbox as t;
use crate::ui::controls::{action_button, button_foreground, ButtonKind};
use crate::ui::glass::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    action_footer, count_chip, error_banner, fade_in, hover_tint, kbd, mark_selected, meter,
    panel_title, reading_title, section_header, section_label, skeleton_list, status_pill, tag,
    toast_with, track_hover, word_wrapped, READING_WIDTH, TOAST_DURATION,
};
use crate::ui::search_field::SearchField;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;
use crate::ui::visits;

/// What a person can take back from the last action.
#[derive(Clone)]
enum Undo {
    /// A rejected candidate goes back to the queue.
    Reject(String),
    /// A snoozed candidate goes back to the queue.
    Snooze(String),
}

/// A page, the queue's size, how many are kept out for low significance and,
/// on a first page after a visit, what changed since the previous visit.
type PageLoad = (
    InboxPage,
    usize,
    usize,
    Option<Briefing>,
    Option<Vec<Entry>>,
);

enum Outcome {
    Group(Result<(ReviewGroup, ReviewMetrics), InboxError>, usize),
    Member(Result<Box<CandidateDetail>, InboxError>, String),
    Live(Result<(InboxPage, usize, usize), InboxError>),
    Page(Result<PageLoad, InboxError>, bool),
    Detail(Result<Box<CandidateDetail>, InboxError>),
    Action(Result<(), InboxError>, &'static str),
}

fn evidence_detail<'a>(
    representative: &'a CandidateDetail,
    member: Option<&'a CandidateDetail>,
) -> &'a CandidateDetail {
    member.unwrap_or(representative)
}

fn has_more_members(offset: usize, loaded: usize, total: usize) -> bool {
    loaded > 0 && offset.saturating_add(loaded) < total
}

#[cfg(test)]
mod group_tests {
    use super::has_more_members;

    #[test]
    fn pagination_is_bounded_and_empty_pages_do_not_loop() {
        assert!(has_more_members(0, 20, 800));
        assert!(!has_more_members(780, 20, 800));
        assert!(!has_more_members(20, 0, 800));
        assert!(!has_more_members(usize::MAX, 20, 800));
    }
}

#[derive(Clone, Copy)]
enum ReviewAction {
    Reject,
    Snooze,
    Edit,
    Confirm,
}

/// A paginated candidate list and its source-reading pane.
pub struct InboxScreen<S: InboxStore + ReviewExceptionStore + Send + 'static> {
    /// Row under the pointer, driving the hover spring.
    hovered: Option<String>,
    inbox: Option<Inbox<S>>,
    rows: Vec<CandidateSummary>,
    cursor: Option<String>,
    selected: Option<String>,
    detail: Option<CandidateDetail>,
    group: Option<ReviewGroup>,
    review_metrics: Option<ReviewMetrics>,
    group_offset: usize,
    group_error: bool,
    member: Option<CandidateDetail>,
    member_id: Option<String>,
    member_error: bool,
    member_focus: Vec<FocusHandle>,
    group_focus: [FocusHandle; 3],
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
    progress_read: Option<ProgressRead>,
    progress_retry: Option<ProgressRetry>,
    progress: Vec<CaptureProgress>,
    progress_busy: bool,
    progress_error: bool,
    progress_action_error: Option<String>,
    /// The previous visit to the project, read once to build the briefing.
    since_visit: Option<String>,
    /// What changed since that visit, shown as one quiet line.
    briefing: Option<Briefing>,
    /// Whether the person dismissed the briefing line.
    briefing_hidden: bool,
    /// The undo the current action would offer once it succeeds.
    pending_undo: Option<Undo>,
    /// The undo offered beside the notice, until the notice goes away.
    undo: Option<Undo>,
    /// Whether the written reason is open (it starts closed: the evidence
    /// comes first and the explanation can nudge a decision).
    reason_open: bool,
    /// The automatic review, when the store has it.
    approvals: Option<Arc<dyn ApprovalsApi>>,
    /// Where the mode stands (read once and after each change).
    status: Option<Status>,
    /// Whether a pass of the review is running.
    reviewing: bool,
    /// What the automatic approval holds or accepted in this project.
    ledger: Vec<Entry>,
    /// Whether the list of what was accepted on its own is open.
    ledger_open: bool,
    /// When the last confirmations happened, to notice a reflex.
    streak: Vec<std::time::Instant>,
}

/// Confirmations in a row, each within `STREAK_GAP`, that read as a reflex.
const STREAK_LEN: usize = 8;
const STREAK_GAP: std::time::Duration = std::time::Duration::from_secs(4);
/// Lines of the ledger shown when it is open.
const LEDGER_SHOWN: usize = 8;

impl<S: InboxStore + ReviewExceptionStore + Send + 'static> EventEmitter<OpenDecision>
    for InboxScreen<S>
{
}

/// The "No mapa" section of one candidate.
struct MapLinks {
    candidate_id: String,
    links: Vec<(ProposedLink, bool)>,
    uncovered: Vec<String>,
    focus: Vec<FocusHandle>,
}

impl<S: InboxStore + ReviewExceptionStore + Send + 'static> InboxScreen<S> {
    fn clear_group(&mut self) {
        self.group = None;
        self.review_metrics = None;
        self.group_offset = 0;
        self.group_error = false;
        self.member = None;
        self.member_id = None;
        self.member_error = false;
        self.member_focus.clear();
    }

    fn set_sources(&mut self, detail: &CandidateDetail, cx: &mut Context<Self>) {
        self.source_index = 0;
        self.source_lines = detail
            .artifacts
            .iter()
            .map(evidence::SourceLines::new)
            .collect();
        self.source_focus = detail
            .artifacts
            .iter()
            .map(|_| cx.focus_handle().tab_stop(true))
            .collect();
    }

    fn load_group(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(id) = self.selected.clone() else {
            return;
        };
        self.group_error = false;
        let Some(project) = self.project_id.clone() else {
            return;
        };
        self.run(cx, move |inbox| {
            Outcome::Group(
                (|| {
                    Ok((
                        inbox.group_detail(&id, offset, 20)?,
                        inbox.review_metrics(&project)?,
                    ))
                })(),
                offset,
            )
        });
    }

    fn load_member(&mut self, id: String, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.member_id = Some(id.clone());
        self.member = None;
        self.member_error = false;
        self.run(cx, move |inbox| {
            Outcome::Member(inbox.detail(&id).map(Box::new), id)
        });
    }

    fn group_section(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        // Most candidates appear once: the section only exists when the same
        // decision showed up in several conversations, so one confirmation
        // settles all of them.
        if self.group_error {
            return div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(error_banner(theme, t::group_load_error()))
                .child(
                    action_button(theme, "retry-group", ButtonKind::Ghost, !self.busy)
                        .aria_label(t::try_again())
                        .child(t::try_again())
                        .track_focus(&self.group_focus[0])
                        .on_click(
                            cx.listener(|this, _, _, cx| this.load_group(this.group_offset, cx)),
                        )
                        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.load_group(this.group_offset, cx);
                                cx.stop_propagation();
                            }
                        })),
                )
                .into_any_element();
        }
        let Some(group) = self
            .group
            .as_ref()
            .filter(|group| group.occurrence_count > 1)
        else {
            return div().into_any_element();
        };
        let mut chips = div().flex().flex_wrap().gap(px(SpacingScale::S2));
        for (index, member) in group.members.iter().enumerate() {
            let id = member.candidate_id.clone();
            let key_id = id.clone();
            let number = self.group_offset + index + 1;
            let label = if member.already_represented {
                t::conversation_confirmed(number)
            } else {
                t::conversation(number)
            };
            chips = chips.child(
                action_button(theme, ("member", index), ButtonKind::Ghost, !self.busy)
                    .aria_label(t::read_conversation_evidence(&label))
                    .tooltip(tooltip(t::capture_tooltip(&member.capture_id), None))
                    .child(label)
                    .track_focus(&self.member_focus[index])
                    .when(
                        self.member_id.as_deref() == Some(&member.candidate_id),
                        |button| mark_selected(button, theme, true),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| this.load_member(id.clone(), cx)))
                    .on_key_down(cx.listener(move |this, event: &gpui::KeyDownEvent, _, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.load_member(key_id.clone(), cx);
                            cx.stop_propagation();
                        }
                    })),
            );
        }
        for (index, label, offset, enabled) in [
            (
                1,
                t::previous(),
                self.group_offset.saturating_sub(20),
                self.group_offset > 0,
            ),
            (
                2,
                t::more_conversations(),
                self.group_offset + 20,
                has_more_members(
                    self.group_offset,
                    group.members.len(),
                    group.occurrence_count,
                ),
            ),
        ] {
            if enabled {
                chips = chips.child(
                    action_button(theme, ("group-page", index), ButtonKind::Ghost, !self.busy)
                        .aria_label(label)
                        .child(label)
                        .track_focus(&self.group_focus[index])
                        .on_click(cx.listener(move |this, _, _, cx| this.load_group(offset, cx)))
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, _, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.load_group(offset, cx);
                                    cx.stop_propagation();
                                }
                            },
                        )),
                );
            }
        }
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(
                section_header(theme, t::also_appeared_in()).child(count_chip(
                    theme,
                    t::conversations_count(group.occurrence_count),
                )),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child(t::group_rule_hint()),
            )
            .child(chips)
            .into_any_element()
    }

    /// Installs capture reads and retry without starting integrations.
    pub fn set_progress(&mut self, read: ProgressRead, retry: ProgressRetry) {
        self.progress_read = Some(read);
        self.progress_retry = Some(retry);
    }

    fn poll_progress(&mut self, cx: &mut Context<Self>) {
        if self.progress_busy {
            return;
        }
        let (Some(read), Some(project)) = (self.progress_read.clone(), self.project_id.clone())
        else {
            return;
        };
        let generation = self.generation;
        self.progress_busy = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { read(&project) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.progress_busy = false;
                if generation != this.generation {
                    this.poll_progress(cx);
                    return;
                }
                match result {
                    Ok(rows) => {
                        this.progress = rows;
                        this.progress_error = false;
                    }
                    Err(_) => this.progress_error = true,
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn retry_progress(&mut self, job: String, cx: &mut Context<Self>) {
        if self.progress_busy {
            return;
        }
        let Some(retry) = self.progress_retry.clone() else {
            return;
        };
        let retry_job = job.clone();
        let generation = self.generation;
        self.progress_busy = true;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { retry(&retry_job) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.progress_busy = false;
                if generation == this.generation {
                    record_retry_result(&mut this.progress_action_error, &job, result.is_ok());
                    if result.is_ok() {
                        this.notice = Some(t::notice_requeued());
                    }
                }
                this.poll_progress(cx);
                cx.notify();
            });
        })
        .detach();
    }
    /// Confirms through the adoption use case, so a decision lands on the
    /// map with the ties the person kept.
    pub fn set_approvals(&mut self, approvals: Arc<dyn ApprovalsApi>, cx: &mut Context<Self>) {
        self.approvals = Some(approvals.clone());
        cx.spawn(async move |this, cx| {
            let status = cx
                .background_executor()
                .spawn(async move { approvals.status().ok() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.status = status;
                cx.notify();
            });
        })
        .detach();
    }

    /// Turns the automatic review on or off; on, a pass runs at once.
    fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        let Some(api) = self.approvals.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let status = cx
                .background_executor()
                .spawn(async move { api.set_mode(mode).ok().and_then(|_| api.status().ok()) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if status.is_some() {
                    this.status = status;
                    this.review_pass(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// One pass of the automatic review, off the interface thread (it may
    /// ask the AI), refreshing the queue when it changed anything.
    fn review_pass(&mut self, cx: &mut Context<Self>) {
        let (Some(api), Some(project)) = (self.approvals.clone(), self.project_id.clone()) else {
            return;
        };
        if self.reviewing || !self.status.is_some_and(|status| status.automatic) {
            return;
        }
        self.reviewing = true;
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let report = cx
                .background_executor()
                .spawn(async move { api.run(&project) })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.reviewing = false;
                if generation != this.generation {
                    return;
                }
                match report {
                    Ok(report) if report.changed() => this.page(false, cx),
                    Ok(_) => {}
                    Err(error) => tracing::warn!(error = %error, "automatic review failed"),
                }
            });
        })
        .detach();
    }

    /// Puts back what the review discarded.
    fn put_back(&mut self, kind: ItemKind, item_id: String, cx: &mut Context<Self>) {
        let Some(api) = self.approvals.clone() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let done = cx
                .background_executor()
                .spawn(async move { api.undo(kind, &item_id).is_ok() })
                .await;
            let _ = this.update(cx, |this, cx| {
                this.notice = Some(if done {
                    t::notice_back_to_queue()
                } else {
                    t::notice_undo_failed()
                });
                this.page(false, cx);
            });
        })
        .detach();
    }

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
            group: None,
            review_metrics: None,
            group_offset: 0,
            group_error: false,
            member: None,
            member_id: None,
            member_error: false,
            member_focus: Vec::new(),
            group_focus: std::array::from_fn(|_| cx.focus_handle().tab_stop(true)),
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
            progress_read: None,
            progress_retry: None,
            progress: Vec::new(),
            progress_busy: false,
            progress_error: false,
            progress_action_error: None,
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
            since_visit: None,
            briefing: None,
            briefing_hidden: false,
            pending_undo: None,
            undo: None,
            reason_open: false,
            approvals: None,
            status: None,
            reviewing: false,
            ledger: Vec::new(),
            ledger_open: false,
            streak: Vec::new(),
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
        self.progress.clear();
        self.progress_error = false;
        self.progress_action_error = None;
        self.poll_progress(cx);
        self.briefing = None;
        self.briefing_hidden = false;
        self.ledger.clear();
        self.streak.clear();
        self.since_visit = self
            .project_id
            .as_deref()
            .and_then(|project| visits::previous(cx, project));
        if let Some(project) = self.project_id.as_deref() {
            visits::mark(cx, project, &visits::now());
        }
        self.rows.clear();
        self.cursor = None;
        self.selected = None;
        self.detail = None;
        self.clear_group();
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
        let visit = if append {
            None
        } else {
            self.since_visit.take().zip(self.project_id.clone())
        };
        let approvals = if append {
            None
        } else {
            self.approvals.clone().zip(self.project_id.clone())
        };
        self.run(cx, move |inbox| {
            let ledger = approvals
                .as_ref()
                .map(|(api, project)| api.ledger(project).unwrap_or_default());
            Outcome::Page(
                inbox.list(&filter).and_then(|page| {
                    let count = inbox.count(&filter)?;
                    let hidden = inbox
                        .count(&everything)?
                        .saturating_sub(inbox.count(&relevant)?);
                    let briefing =
                        visit.and_then(|(since, project)| inbox.briefing(&project, &since).ok());
                    Ok((page, count, hidden, briefing, ledger))
                }),
                append,
            )
        });
    }

    /// Refreshes the loaded window, keeping the reader and any unsaved editor mounted.
    pub fn poll_visible(&mut self, cx: &mut Context<Self>) {
        self.poll_progress(cx);
        if !can_poll_rows(self.busy, self.project_id.is_some(), self.editor.is_some()) {
            return;
        }
        let filter = InboxFilter {
            project_id: self.project_id.clone(),
            min_significance: (!self.show_low).then_some(MIN_SIGNIFICANCE),
            ..InboxFilter::default()
        };
        let loaded = self.rows.len().max(filter.limit);
        let selected = self.selected.clone();
        self.run(cx, move |inbox| {
            Outcome::Live((|| {
                let mut scan = filter.clone();
                let mut candidates = Vec::new();
                let next_cursor = loop {
                    let page = inbox.list(&scan)?;
                    candidates.extend(page.candidates);
                    if candidates.len() >= loaded || page.next_cursor.is_none() {
                        break page.next_cursor;
                    }
                    scan.cursor = page.next_cursor;
                };
                let total = inbox.count(&filter)?;
                // Absence from the bounded page is not evidence of resolution.
                if let Some(id) = selected
                    .as_ref()
                    .filter(|id| !candidates.iter().any(|row| &row.id == *id))
                {
                    if let Some(project) = filter.project_id.as_deref() {
                        if inbox.eligible_in_review(project, id, &filter)? {
                            match inbox.detail(id) {
                                Ok(detail) => {
                                    retain_eligible_selection(&mut candidates, detail.summary, true)
                                }
                                Err(InboxError::NotFound) => {}
                                Err(error) => return Err(error),
                            }
                        }
                    }
                }
                let everything = InboxFilter {
                    min_significance: None,
                    ..filter.clone()
                };
                let relevant = InboxFilter {
                    min_significance: Some(MIN_SIGNIFICANCE),
                    ..filter
                };
                let hidden = inbox
                    .count(&everything)?
                    .saturating_sub(inbox.count(&relevant)?);
                Ok((
                    InboxPage {
                        candidates,
                        next_cursor,
                    },
                    total,
                    hidden,
                ))
            })())
        });
    }

    fn select(&mut self, id: String, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.selected = Some(id.clone());
        self.detail = None;
        self.clear_group();
        self.links = None;
        self.reason_open = false;
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
                if !accepts_refresh(generation, this.generation) {
                    this.page(false, cx);
                    cx.notify();
                    return;
                }
                match outcome {
                    Outcome::Live(Ok((page, total, hidden))) => {
                        let selected_change = selected_change(
                            this.detail.as_ref().map(|detail| &detail.summary),
                            &page.candidates,
                        );
                        this.total = Some(total);
                        this.hidden_low = hidden;
                        this.cursor = page.next_cursor;
                        this.rows = page.candidates;
                        this.row_focus
                            .retain(|(id, _)| this.rows.iter().any(|row| row.id == *id));
                        for row in &this.rows {
                            if !this.row_focus.iter().any(|(id, _)| *id == row.id) {
                                this.row_focus
                                    .push((row.id.clone(), cx.focus_handle().tab_stop(true)));
                            }
                        }
                        match selected_change {
                            SelectedChange::Removed => {
                                this.selected = None;
                                this.detail = None;
                                this.clear_group();
                                this.links = None;
                                this.source_lines.clear();
                                this.source_focus.clear();
                            }
                            SelectedChange::Changed => {
                                if let Some(id) = this.selected.clone() {
                                    this.select(id, cx);
                                }
                            }
                            SelectedChange::Unchanged => {}
                        }
                        if selected_change != SelectedChange::Removed
                            && this.selected.is_none()
                            && this.editor.is_none()
                        {
                            if let Some(row) = this.rows.first() {
                                this.select(row.id.clone(), cx);
                            }
                        }
                    }
                    Outcome::Live(Err(error)) => {
                        tracing::warn!(code = error.code(), "inbox live refresh failed");
                        this.error = Some(failure_copy(&error));
                    }
                    Outcome::Page(Ok((page, total, hidden, briefing, ledger)), append) => {
                        this.total = Some(total);
                        this.hidden_low = hidden;
                        if let Some(ledger) = ledger {
                            this.ledger = ledger;
                        }
                        if !append {
                            this.review_pass(cx);
                        }
                        if briefing.is_some() {
                            this.briefing = briefing;
                        }
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
                        this.load_group(0, cx);
                    }
                    Outcome::Group(result, offset) => match result {
                        Ok((group, metrics)) => {
                            this.review_metrics = Some(metrics);
                            this.group_offset = offset;
                            this.member_focus = group
                                .members
                                .iter()
                                .map(|_| cx.focus_handle().tab_stop(true))
                                .collect();
                            this.group = Some(group);
                            this.group_error = false;
                        }
                        Err(_) => this.group_error = true,
                    },
                    Outcome::Member(result, id) => match result {
                        Ok(detail) => {
                            this.member_id = Some(id);
                            this.set_sources(&detail, cx);
                            this.member = Some(*detail);
                            this.member_error = false;
                        }
                        Err(_) => this.member_error = true,
                    },
                    Outcome::Action(Ok(()), notice) => {
                        this.editor = None;
                        this.editor_subscription = None;
                        this.notice = Some(notice);
                        this.undo = this.pending_undo.take();
                        this.note_confirmation(notice);
                        this.page(false, cx);
                    }
                    Outcome::Page(Err(error), _)
                    | Outcome::Detail(Err(error))
                    | Outcome::Action(Err(error), _) => {
                        this.pending_undo = None;
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
                qualifiers: detail.qualifiers.clone(),
            };
            let id = detail.summary.id.clone();
            let reviewed = detail.clone();
            let editor = cx.new(|cx| ReviewEditor::new(original, cx));
            editor.update(cx, |editor, _| {
                editor.set_evidence(detail.artifacts.clone())
            });
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
                            let reviewed = reviewed.clone();
                            let edits = edits.clone();
                            let confirm = *confirm;
                            let adoption = this.adoption.clone();
                            let chosen = this.chosen_links(&id);
                            this.run(cx, move |inbox| {
                                if !confirm {
                                    return Outcome::Action(
                                        inbox.adjust(&id, edits),
                                        t::notice_edits_saved(),
                                    );
                                }
                                match (adoption, chosen) {
                                    (Some(adoption), Some((kept, declined))) => adopted(
                                        adoption.adopt_reviewed(
                                            &reviewed,
                                            Some(edits),
                                            &kept,
                                            &declined,
                                        ),
                                        t::notice_edits_confirmed_linked(),
                                        t::notice_edits_confirmed(),
                                    ),
                                    _ => confirmed(inbox.confirm_reviewed(&reviewed, Some(edits))),
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
        let reviewed = detail.clone();
        let snoozed = detail.summary.status == CandidateStatus::Snoozed;
        let adoption = self.adoption.clone();
        let chosen = self.chosen_links(&id);
        self.notice = None;
        self.undo = None;
        self.pending_undo = match action {
            ReviewAction::Reject => Some(Undo::Reject(id.clone())),
            ReviewAction::Snooze if !snoozed => Some(Undo::Snooze(id.clone())),
            _ => None,
        };
        window.focus(&self.refresh_focus, cx);
        self.run(cx, move |inbox| {
            let (result, notice) = match action {
                ReviewAction::Confirm => match (adoption, chosen) {
                    (Some(adoption), Some((kept, declined))) => {
                        return adopted(
                            adoption.adopt_reviewed(&reviewed, None, &kept, &declined),
                            t::notice_confirmed_linked(),
                            t::notice_confirmed(),
                        );
                    }
                    _ => return confirmed(inbox.confirm_reviewed(&reviewed, None)),
                },
                ReviewAction::Reject => (inbox.reject(&id), t::notice_rejected()),
                ReviewAction::Snooze if snoozed => (inbox.unsnooze(&id), t::notice_resumed()),
                ReviewAction::Snooze => (inbox.snooze(&id), t::notice_snoozed()),
                ReviewAction::Edit => unreachable!(),
            };
            Outcome::Action(result, notice)
        });
    }

    /// Keeps the time of each confirmation. A run of them, each a few seconds
    /// after the last, is the habit that makes approving stop being checking
    /// (approval rises and scrutiny falls with practice), so the notice says
    /// so once and does not block anything.
    fn note_confirmation(&mut self, notice: &'static str) {
        let confirmations = [
            t::notice_confirmed_linked(),
            t::notice_confirmed(),
            t::notice_edits_confirmed_linked(),
            t::notice_edits_confirmed(),
        ];
        if !confirmations.contains(&notice) {
            self.streak.clear();
            return;
        }
        let now = std::time::Instant::now();
        if self
            .streak
            .last()
            .is_some_and(|last| now.duration_since(*last) > STREAK_GAP)
        {
            self.streak.clear();
        }
        self.streak.push(now);
        if self.streak.len() >= STREAK_LEN {
            self.streak.clear();
            self.notice = Some(t::notice_streak());
        }
    }

    /// Takes back the last rejection or deferral.
    fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(undo) = self.undo.take() else {
            return;
        };
        self.notice = None;
        self.run(cx, move |inbox| match undo {
            Undo::Reject(id) => Outcome::Action(inbox.reopen(&id), t::notice_reject_undone()),
            Undo::Snooze(id) => Outcome::Action(inbox.unsnooze(&id), t::notice_snooze_undone()),
        });
    }

    /// The written reason, closed until asked for: the evidence above it is
    /// what a decision should rest on, and a persuasive explanation raises
    /// acceptance whether or not it is right.
    fn reason_disclosure(&self, theme: &Theme, rationale: &str, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let open = self.reason_open;
        let toggle = div()
            .id("inbox-reason-toggle")
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .cursor_pointer()
            .role(Role::Button)
            .aria_label(if open {
                t::reason_hide_aria()
            } else {
                t::reason_show_aria()
            })
            .focus_visible(focus_ring(theme))
            .on_click(cx.listener(|this, _, _, cx| {
                this.reason_open = !this.reason_open;
                cx.notify();
            }))
            .child(icon(
                if open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                },
                13.0,
                colors.text_muted(),
            ))
            .child(section_label(theme, t::reason_title()));
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(toggle)
            .when(open, |column| {
                column.child(
                    text_style(div(), TypeScale::BODY)
                        .text_color(colors.text_secondary())
                        .child(rationale.to_owned()),
                )
            })
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
            t::review_reject(),
            if detail.summary.status == CandidateStatus::Snoozed {
                t::review_resume()
            } else {
                t::review_snooze()
            },
            t::review_adjust(),
            t::review_confirm(),
        ];
        action_footer(&theme, self.busy.then_some((t::review_recording(), false)))
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
        let rule = row.kind == CandidateKind::Rule;
        // A rule is blue, as on the timeline; a decision not yet confirmed
        // stays neutral.
        let tone = if rule {
            theme.colors.status_info()
        } else {
            theme.colors.text_muted()
        };
        let element = div()
            .id((ElementId::from("candidate"), row.id.clone()))
            .relative()
            .px(px(SpacingScale::S4))
            .py(px(SpacingScale::S3))
            .flex()
            .gap(px(SpacingScale::S3))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if track_hover(&mut this.hovered, hover_key.clone(), *hovered) {
                    cx.notify();
                }
            }))
            .role(Role::Button)
            .aria_label(t::row_aria(
                if rule {
                    t::kind_rule()
                } else {
                    t::kind_decision()
                },
                &row.question,
                row.significance as f32 * 100.0,
                row.confidence as f32 * 100.0,
            ))
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
                    .size(px(22.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_full()
                    .bg(crate::ui::tokens::tint(tone, 0.14))
                    .child(icon(
                        if rule {
                            IconName::Shield
                        } else {
                            IconName::Decision
                        },
                        12.0,
                        tone,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap(px(SpacingScale::S2))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(theme.colors.text_muted())
                                    .child(relative(&row.received_at)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(SpacingScale::S2))
                                    .when(row.status != CandidateStatus::Pending, |line| {
                                        line.child(status_badge(row.status, theme))
                                    })
                                    .children(self.left_for_you(&row.id).map(|_| {
                                        text_style(div(), TypeScale::META)
                                            .text_color(theme.colors.status_info())
                                            .child(t::left_for_you_tag())
                                    })),
                            ),
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
                    ),
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

    /// What the review decided to leave for the person about a candidate.
    fn left_for_you(&self, candidate_id: &str) -> Option<&Entry> {
        self.ledger.iter().find(|entry| {
            entry.kind == ItemKind::Candidate
                && entry.item_id == candidate_id
                && entry.verdict == Verdict::NeedsHuman
                && entry.undone_at.is_none()
        })
    }

    /// The switch between deciding everything and letting the AI handle the
    /// cycle, with one line of what that means right now.
    fn mode_switch(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let status = self.status?;
        let colors = theme.colors;
        let mut track = crate::ui::patterns::segmented(theme)
            .id("inbox-mode")
            .role(Role::RadioGroup);
        for (mode, id, label) in [
            (Mode::Manual, "inbox-mode-manual", t::mode_manual()),
            (Mode::Automatic, "inbox-mode-auto", t::mode_automatic()),
        ] {
            track = track.child(
                crate::ui::patterns::segment_label(
                    theme,
                    id,
                    label,
                    (mode == Mode::Automatic) == status.automatic,
                )
                .on_click(cx.listener(move |this, _, _, cx| this.set_mode(mode, cx))),
            );
        }
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .px(px(SpacingScale::S4))
                .pb(px(SpacingScale::S2))
                .child(div().flex().child(track))
                .when(status.automatic, |column| {
                    column.child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(if status.judge {
                                t::mode_judge_on()
                            } else {
                                t::mode_judge_off()
                            }),
                    )
                }),
        )
    }

    /// What the review did on its own, a short list under the queue that
    /// opens on request: an accepted candidate opens its decision, a
    /// discarded one can be put back.
    fn ledger_section(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let accepted: Vec<&Entry> = self
            .ledger
            .iter()
            .filter(|entry| entry.verdict != Verdict::NeedsHuman && entry.undone_at.is_none())
            .collect();
        if accepted.is_empty() {
            return None;
        }
        let colors = theme.colors;
        let open = self.ledger_open;
        let header = div()
            .id("inbox-ledger-toggle")
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .px(px(SpacingScale::S4))
            .py(px(SpacingScale::S2))
            .cursor_pointer()
            .role(Role::Button)
            .aria_label(t::ledger_toggle_aria())
            .focus_visible(focus_ring(theme))
            .on_click(cx.listener(|this, _, _, cx| {
                this.ledger_open = !this.ledger_open;
                cx.notify();
            }))
            .child(icon(
                if open {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                },
                13.0,
                colors.text_muted(),
            ))
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_secondary())
                    .child(t::ledger_title(accepted.len())),
            );
        let mut list = div().flex().flex_col();
        if open {
            for (index, entry) in accepted.iter().enumerate().take(LEDGER_SHOWN) {
                let opens = (entry.kind == ItemKind::Candidate
                    && entry.verdict == Verdict::Accepted)
                    .then(|| entry.result_id.clone())
                    .flatten();
                let put_back = (entry.kind == ItemKind::Candidate
                    && entry.verdict == Verdict::Discarded)
                    .then(|| entry.item_id.clone());
                let verdict = match entry.verdict {
                    Verdict::Accepted => t::ledger_verdict_accepted(),
                    _ => t::ledger_verdict_discarded(),
                };
                let kind = match entry.kind {
                    ItemKind::Candidate => t::ledger_kind_candidate(),
                    ItemKind::Relation => t::ledger_kind_relation(),
                    ItemKind::Claim => t::ledger_kind_claim(),
                    ItemKind::Link => t::ledger_kind_link(),
                };
                let by = match entry.by {
                    By::Rules => t::ledger_by_rules(),
                    By::Ai => t::ledger_by_ai(),
                };
                let mut line = div()
                    .id(("inbox-ledger-row", index))
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S2))
                    .px(px(SpacingScale::S4))
                    .py(px(SpacingScale::S2))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .line_clamp(2)
                                    .text_color(colors.text_primary())
                                    .child(entry.title.clone()),
                            )
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(colors.text_muted())
                                    .child(t::ledger_line(
                                        kind,
                                        verdict,
                                        by,
                                        &relative(&entry.created_at),
                                        &entry.reason,
                                    )),
                            ),
                    );
                if let Some(decision) = opens {
                    line = line
                        .cursor_pointer()
                        .role(Role::Link)
                        .aria_label(t::open_decision_aria(&entry.title))
                        .hover(move |style| style.bg(colors.glass_fill_low()))
                        .focus_visible(focus_ring(theme))
                        .on_click(
                            cx.listener(move |_, _, _, cx| cx.emit(OpenDecision(decision.clone()))),
                        );
                }
                if let Some(item_id) = put_back {
                    line = line.child(
                        action_button(theme, ("inbox-put-back", index), ButtonKind::Ghost, true)
                            .aria_label(t::undo())
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.put_back(ItemKind::Candidate, item_id.clone(), cx)
                            }))
                            .child(t::undo()),
                    );
                }
                list = list.child(line);
            }
        }
        Some(
            div()
                .flex_none()
                .border_t_1()
                .border_color(colors.hairline_divider())
                .child(header)
                .child(list),
        )
    }

    /// One quiet line of what changed since the previous visit: the cue to
    /// resume from. Nothing when nothing changed or the person closed it.
    fn briefing_strip(&self, theme: &Theme, cx: &mut Context<Self>) -> Option<Div> {
        let briefing = self
            .briefing
            .as_ref()
            .filter(|briefing| !briefing.is_quiet() && !self.briefing_hidden)?;
        let colors = theme.colors;
        let mut parts = Vec::new();
        if briefing.new_candidates > 0 {
            parts.push(t::briefing_new_candidates(briefing.new_candidates));
        }
        if briefing.decisions > 0 {
            parts.push(t::briefing_decisions(briefing.decisions));
        }
        if briefing.deliveries > 0 {
            parts.push(t::briefing_deliveries(
                briefing.deliveries,
                briefing.sessions,
            ));
        }
        let label = t::briefing_line(&relative(&briefing.since), &parts.join(" · "));
        Some(
            div()
                .flex()
                .items_start()
                .gap(px(SpacingScale::S2))
                .mx(px(SpacingScale::S4))
                .mb(px(SpacingScale::S3))
                .px(px(SpacingScale::S3))
                .py(px(SpacingScale::S2))
                .rounded(theme.radius.control())
                .bg(colors.glass_fill_low())
                .child(
                    div()
                        .mt(px(2.0))
                        .child(icon(IconName::Clock, 12.0, colors.text_muted())),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .flex_1()
                        .min_w(px(0.0))
                        .text_color(colors.text_secondary())
                        .child(label),
                )
                .child(
                    crate::ui::controls::icon_action(theme, "inbox-briefing-close", t::dismiss())
                        .size(px(18.0))
                        .child(icon(IconName::Close, 10.0, colors.text_muted()))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.briefing_hidden = true;
                            cx.notify();
                        })),
                ),
        )
    }

    /// Shows or hides the low-significance candidates kept out of the queue.
    fn low_toggle(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let theme = Theme::current(cx);
        let label = if self.show_low {
            t::low_hide().to_owned()
        } else {
            t::low_show(self.hidden_low)
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
                EdgeKind::Uses => t::link_uses(),
                EdgeKind::AppliesTo => t::link_applies_to(),
                _ => t::link_changes(),
            };
            let glyph = match link.entity_kind {
                EntityKind::Component => IconName::Component,
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
                .child(if more > 0 {
                    t::map_uncovered_more(&shown.join(", "), more)
                } else {
                    t::map_uncovered(&shown.join(", "))
                })
        });
        Some(
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    section_header(theme, t::map_title()).when(!links.links.is_empty(), |row| {
                        row.child(count_chip(theme, t::count_of(kept, links.links.len())))
                    }),
                )
                .when(!links.links.is_empty(), |section| {
                    section.child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(match kind {
                                CandidateKind::Rule => t::map_rule_hint(),
                                _ => t::map_decision_hint(),
                            }),
                    )
                })
                .child(list)
                .children(uncovered),
        )
    }

    fn progress_panel(&self, cx: &mut Context<Self>) -> Div {
        let theme = Theme::current(cx);
        let mut panel = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S4))
            .max_w(px(READING_WIDTH))
            .p(px(SpacingScale::S6))
            .child(section_label(&theme, t::progress_title()));
        if let Some(job) = self.progress_action_error.clone() {
            panel = panel.child(
                error_banner(&theme, t::reprocess_error())
                    .id("capture-action-error")
                    .role(Role::Alert)
                    .child(
                        action_button(
                            &theme,
                            "capture-action-retry",
                            ButtonKind::Secondary,
                            !self.progress_busy,
                        )
                        .aria_label(t::reprocess_retry_aria())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.retry_progress(job.clone(), cx)),
                        )
                        .child(t::reprocess_retry()),
                    )
                    .child(
                        action_button(&theme, "capture-action-dismiss", ButtonKind::Ghost, true)
                            .aria_label(t::reprocess_dismiss_aria())
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.progress_action_error = None;
                                cx.notify();
                            }))
                            .child(t::dismiss()),
                    ),
            );
        }
        if self.progress_error {
            panel = panel.child(
                error_banner(&theme, t::progress_error())
                    .id("capture-progress-error")
                    .role(Role::Alert)
                    .child(
                        action_button(
                            &theme,
                            "capture-progress-retry-read",
                            ButtonKind::Ghost,
                            !self.progress_busy,
                        )
                        .aria_label(t::progress_refresh_aria())
                        .on_click(cx.listener(|this, _, _, cx| this.poll_progress(cx)))
                        .child(t::try_again()),
                    ),
            );
        }
        if self.progress.is_empty() {
            return panel.child(if self.progress_busy {
                t::progress_loading()
            } else if self.progress_error {
                t::progress_unavailable()
            } else {
                t::progress_empty()
            });
        }
        panel = panel.child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_secondary())
                .child(capture_summary(&self.progress)),
        );
        for (index, capture) in self.progress.iter().take(PROGRESS_ROWS).enumerate() {
            let (pill_color, pill_label) = capture_pill(&theme, capture);
            let title = capture.title.as_deref().unwrap_or(t::capture_untitled());
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
                        .child(status_pill(&theme, pill_color, pill_label)),
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
                            &theme,
                            ("capture-reprocess", index),
                            ButtonKind::Secondary,
                            !self.progress_busy,
                        )
                        .aria_label(t::reprocess_aria())
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.retry_progress(job.clone(), cx)),
                        )
                        .self_start()
                        .child(t::reprocess()),
                    );
                }
            }
            panel = panel.child(row);
        }
        panel
    }

    fn reading_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let visible_detail = self
            .detail
            .as_ref()
            .filter(|detail| matches_query(&detail.summary, &self.query));
        let Some(detail) = visible_detail else {
            if self.loaded && self.rows.is_empty() && !self.busy {
                if self.progress_read.is_some() {
                    return self.progress_panel(cx).into_any_element();
                }
                return crate::ui::patterns::empty_panel_mascot(
                    &theme,
                    crate::screens::assistant::resting(),
                    t::empty_eyebrow(),
                    t::empty_title(),
                    t::empty_body(),
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
                        .child(if self.busy { "" } else { t::select_candidate() }),
                )
                .into_any_element();
        };
        let source_detail = evidence_detail(detail, self.member.as_ref());
        let evidence_block = if self.member_error {
            error_banner(&theme, t::source_read_error()).into_any_element()
        } else if self.member_id.is_some() && self.member.is_none() {
            skeleton_list(&theme, "member-loading", 3)
        } else if source_detail.artifacts.is_empty() {
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child(t::no_source())
                .into_any_element()
        } else {
            let tabs = evidence::tab_strip(&theme, "evidence-tabs").children(
                source_detail
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
                    source_detail
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
                                    format!("{}-{}", source_detail.summary.id, self.source_index),
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
                        line.child(tag(&theme, t::kind_rule()))
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
                        .child(t::why_it_matters(
                            &detail
                                .criteria
                                .iter()
                                .map(|criterion| criterion_label(criterion))
                                .collect::<Vec<_>>()
                                .join(" · "),
                        )),
                )
            })
            .children(self.left_for_you(&detail.summary.id).map(|entry| {
                div()
                    .flex()
                    .items_start()
                    .gap(px(SpacingScale::S2))
                    .child(
                        div()
                            .mt(px(6.0))
                            .size(px(6.0))
                            .flex_none()
                            .rounded_full()
                            .bg(theme.colors.status_info()),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex_1()
                            .min_w(px(0.0))
                            .text_color(theme.colors.text_secondary())
                            .child(t::left_for_you_reason(&entry.reason)),
                    )
            }))
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
                        section_label(&theme, t::editor_choice())
                            .text_color(theme.colors.accent_hover()),
                    )
                    .child(
                        text_style(div(), TypeScale::HEADING_2)
                            .text_color(theme.colors.text_primary())
                            .child(detail.summary.choice.clone()),
                    ),
            )
            .child(super::review_editor::qualifier_reading(
                &theme,
                &detail.qualifiers,
            ))
            .child(self.group_section(&theme, cx))
            .child(
                div()
                    .pt(px(SpacingScale::S2))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(
                        section_header(&theme, t::evidence_title()).child(count_chip(
                            &theme,
                            t::sources_count(source_detail.artifacts.len()),
                        )),
                    )
                    .when(
                        self.member_id
                            .as_deref()
                            .is_some_and(|member| member != detail.summary.id),
                        |section| {
                            section.child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .text_color(theme.colors.text_muted())
                                    .child(t::member_evidence_note()),
                            )
                        },
                    )
                    .when(self.member.is_some(), |section| {
                        section
                            .child(super::review_editor::qualifier_reading(
                                &theme,
                                &source_detail.qualifiers,
                            ))
                            .child(section_label(&theme, t::member_origin_title()))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(theme.colors.text_muted())
                                    .child(t::received_inline(
                                        &short_date(&source_detail.summary.received_at),
                                        &source_detail.summary.project_location,
                                    )),
                            )
                    })
                    .child(evidence_block),
            )
            .child(self.reason_disclosure(&theme, &detail.rationale, cx))
            .children(self.map_section(&theme, &detail.summary.id, detail.summary.kind, cx))
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
                            t::origin_title(),
                            &t::received_lines(
                                &short_date(&detail.summary.received_at),
                                &detail.summary.project_location,
                            ),
                        )
                        .flex_1()
                        .min_w(px(220.0)),
                    ),
            );
        div()
            .id(ElementId::Name(
                format!("inbox-reading-{}", detail.summary.id).into(),
            ))
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

impl<S: InboxStore + ReviewExceptionStore + Send + 'static> Render for InboxScreen<S> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _probe = crate::ui::perf::Probe::start("inbox");
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
                        screen.undo = None;
                        screen.notice_scheduled = None;
                        cx.notify();
                    }
                });
            })
            .detach();
        }
        let retry = self.error.map(|_| {
            action_button(&theme, "inbox-retry", ButtonKind::Secondary, !self.busy)
                .aria_label(t::try_again())
                .on_click(cx.listener(|this, _, _, cx| this.page(false, cx)))
                .child(t::try_again())
        });
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .children(self.error.map(|message| {
                error_banner(&theme, message)
                    .id("inbox-error")
                    .role(Role::Alert)
                    .children(retry)
            }))
            .children(self.notice.map(|message| {
                let action = self.undo.is_some().then(|| {
                    action_button(&theme, "inbox-undo", ButtonKind::Ghost, !self.busy)
                        .aria_label(t::undo())
                        .on_click(cx.listener(|this, _, _, cx| this.undo(cx)))
                        .child(t::undo())
                        .into_any_element()
                });
                toast_with(&theme, message, 72.0, action)
            }))
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.0))
                    .flex()
                    .child(
                        div()
                            .w(px(320.0))
                            .flex_none()
                            .h_full()
                            .flex()
                            .flex_col()
                            .bg(theme.colors.pane())
                            .border_r_1()
                            .border_color(theme.colors.hairline_divider())
                            .child(
                                div()
                                    .px(px(SpacingScale::S4))
                                    .pt(px(SpacingScale::S3))
                                    .pb(px(SpacingScale::S2))
                                    .flex()
                                    .items_center()
                                    .gap(px(SpacingScale::S2))
                                    .child(panel_title(&theme, t::queue_title()))
                                    .child(count_chip(
                                        &theme,
                                        if self.rows.len() == visible.len() {
                                            visible.len().to_string()
                                        } else {
                                            t::count_of(visible.len(), self.rows.len())
                                        },
                                    ))
                                    .child(div().flex_1())
                                    .child(self.button("inbox-refresh", t::refresh(), false, cx)),
                            )
                            .children(self.mode_switch(&theme, cx))
                            .child(
                                div()
                                    .px(px(SpacingScale::S4))
                                    .pb(px(SpacingScale::S3))
                                    .children(self.search.clone()),
                            )
                            .children(self.briefing_strip(&theme, cx))
                            .when(self.hidden_low > 0 || self.show_low, |rail| {
                                rail.child(self.low_toggle(cx))
                            })
                            .child(
                                div()
                                    .id("inbox-list")
                                    .flex_1()
                                    .min_h(px(0.0))
                                    .overflow_y_scroll()
                                    .track_scroll(&self.list_scroll)
                                    .children(visible.iter().enumerate().map(|(index, row)| {
                                        crate::ui::motion::cascade(
                                            ElementId::Name(
                                                format!("candidate-in-{}", row.id).into(),
                                            ),
                                            index,
                                            div().child(self.row(row, cx)),
                                        )
                                    }))
                                    .when(visible.is_empty() && self.busy, |list| {
                                        list.child(skeleton_list(&theme, "inbox-skeleton", 5))
                                    })
                                    .when(visible.is_empty() && !self.busy, |list| {
                                        list.child(
                                            text_style(div(), TypeScale::BODY_SMALL)
                                                .p(px(SpacingScale::S4))
                                                .text_color(theme.colors.text_muted())
                                                .child(if !self.loaded {
                                                    t::queue_not_loaded()
                                                } else if self.rows.is_empty() {
                                                    t::queue_empty()
                                                } else {
                                                    t::queue_no_match()
                                                }),
                                        )
                                    }),
                            )
                            .children(self.ledger_section(&theme, cx))
                            .when(
                                self.cursor.is_some() || self.rows.len() != visible.len(),
                                |rail| {
                                    rail.child(
                                        text_style(div(), TypeScale::META)
                                            .flex_none()
                                            .px(px(SpacingScale::S4))
                                            .py(px(SpacingScale::S2))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .border_t_1()
                                            .border_color(theme.colors.hairline_divider())
                                            .text_color(theme.colors.text_muted())
                                            .child(t::loaded_visible(
                                                self.rows.len(),
                                                visible.len(),
                                            ))
                                            .when(self.cursor.is_some(), |footer| {
                                                footer.child(self.button(
                                                    "inbox-more",
                                                    t::load_more(),
                                                    true,
                                                    cx,
                                                ))
                                            }),
                                    )
                                },
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .h_full()
                            .flex()
                            .flex_col()
                            .child(div().flex_1().min_h(px(0.0)).child(
                                if let Some(editor) = self.editor.as_ref().filter(|_| {
                                    self.detail.as_ref().is_some_and(|detail| {
                                        matches_query(&detail.summary, &self.query)
                                    })
                                }) {
                                    editor.clone().into_any_element()
                                } else {
                                    self.reading_pane(cx)
                                },
                            ))
                            .when(self.editor.is_none(), |pane| {
                                pane.child(self.review_actions(cx))
                            }),
                    ),
            )
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
        Ok(outcome) if outcome.rule => {
            Outcome::Action(Ok(()), confirmation_notice(true, outcome.linked > 0))
        }
        Ok(outcome) if outcome.linked > 0 => Outcome::Action(Ok(()), linked),
        Ok(_) => Outcome::Action(Ok(()), plain),
        Err(AdoptionError::Inbox(error)) => Outcome::Action(Err(error), plain),
        Err(AdoptionError::Graph(error)) => {
            tracing::warn!(code = error.code(), "map update after adoption failed");
            Outcome::Action(Ok(()), t::notice_map_failed())
        }
    }
}

fn accepts_refresh(request_generation: u64, current_generation: u64) -> bool {
    request_generation == current_generation
}

fn can_poll_rows(busy: bool, has_project: bool, editing: bool) -> bool {
    !busy && has_project && !editing
}

fn retain_eligible_selection(
    rows: &mut Vec<CandidateSummary>,
    row: CandidateSummary,
    eligible: bool,
) {
    if eligible {
        rows.push(row);
    }
}

pub(crate) fn record_retry_result(error: &mut Option<String>, job: &str, successful: bool) {
    *error = if successful {
        None
    } else {
        Some(job.to_owned())
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectedChange {
    Unchanged,
    Changed,
    Removed,
}

fn selected_change(
    current: Option<&CandidateSummary>,
    rows: &[CandidateSummary],
) -> SelectedChange {
    let Some(current) = current else {
        return SelectedChange::Unchanged;
    };
    match rows.iter().find(|row| row.id == current.id) {
        None => SelectedChange::Removed,
        Some(row) if row == current => SelectedChange::Unchanged,
        Some(_) => SelectedChange::Changed,
    }
}

/// Rows shown in the captures panel; the reader loads no more than this.
const PROGRESS_ROWS: usize = 8;

/// Pill colour and label of a capture's state.
fn capture_pill(theme: &Theme, capture: &CaptureProgress) -> (gpui::Rgba, &'static str) {
    let colors = &theme.colors;
    let failed =
        capture.state == CaptureState::Completed && capture.reason == AssessmentReason::Failed;
    match capture.state {
        CaptureState::Queued => (colors.status_info(), t::capture_state_queued()),
        CaptureState::Running => (colors.accent_default(), t::capture_state_running()),
        _ if failed => (colors.status_danger(), t::capture_state_failed()),
        CaptureState::Completed => (colors.status_success(), t::capture_state_done()),
        CaptureState::Failed => (colors.status_danger(), t::capture_state_failed()),
        CaptureState::Skipped => (colors.status_warning(), t::capture_state_skipped()),
        CaptureState::Cancelled => (colors.text_muted(), t::capture_state_cancelled()),
        CaptureState::Unknown => (colors.text_muted(), t::capture_state_received()),
    }
}

/// Joins the parts that exist; zero counts never appear.
fn dot_joined(parts: Vec<String>) -> String {
    parts.join(" · ")
}

/// One line over the loaded captures: only the non-zero parts.
fn capture_summary(captures: &[CaptureProgress]) -> String {
    let count = |keep: fn(&CaptureProgress) -> bool| captures.iter().filter(|c| keep(c)).count();
    let waiting = count(|c| matches!(c.state, CaptureState::Queued | CaptureState::Running));
    let analysed = count(|c| c.state == CaptureState::Completed);
    let failed = count(|c| c.state == CaptureState::Failed);
    let proposals: usize = captures.iter().map(|c| c.candidates.pending).sum();
    let parts = [
        (waiting, t::capture_waiting as fn(usize) -> String),
        (analysed, t::capture_analysed),
        (failed, t::capture_failed_count),
        (proposals, t::capture_proposals),
    ];
    dot_joined(
        parts
            .into_iter()
            .filter(|(n, _)| *n > 0)
            .map(|(n, text)| text(n))
            .collect(),
    )
}

/// Time, adapter, model and the counts that are not zero.
fn capture_facts(capture: &CaptureProgress) -> String {
    let counts = &capture.candidates;
    let mut parts = vec![relative(&capture.received_at)];
    parts.extend(capture.adapter.clone());
    parts.extend(capture.model.clone());
    for (n, text) in [
        (counts.pending, t::capture_proposals as fn(usize) -> String),
        (counts.adopted, t::capture_confirmed),
        (counts.dismissed, t::capture_rejected),
        (counts.snoozed, t::capture_postponed),
    ] {
        if n > 0 {
            parts.push(text(n));
        }
    }
    if capture.attempts > 1 {
        parts.push(t::capture_attempt(capture.attempts as usize));
    }
    dot_joined(parts)
}

pub(super) fn progress_copy(capture: &CaptureProgress) -> &'static str {
    match capture.state {
        CaptureState::Queued => t::progress_queued(),
        CaptureState::Running => t::progress_running(),
        CaptureState::Failed => t::progress_failed_nothing(),
        CaptureState::Skipped => t::progress_skipped(),
        CaptureState::Cancelled => t::progress_cancelled(),
        CaptureState::Unknown => t::progress_unknown(),
        CaptureState::Completed => match capture.reason {
            AssessmentReason::Candidates => t::progress_done_candidates(),
            AssessmentReason::Detail => t::progress_done_detail(),
            AssessmentReason::Empty => t::progress_done_empty(),
            AssessmentReason::Failed => t::progress_failed(),
            AssessmentReason::Skipped => t::progress_skipped(),
            AssessmentReason::Unknown => t::progress_done_unknown(),
        },
    }
}

fn confirmation_notice(rule: bool, linked: bool) -> &'static str {
    match (rule, linked) {
        (true, true) => t::notice_rule_linked(),
        (true, false) => t::notice_rule(),
        (false, true) => t::notice_decision_linked(),
        (false, false) => t::notice_decision(),
    }
}

fn confirmed(result: Result<application::inbox::ConfirmOutcome, InboxError>) -> Outcome {
    match result {
        Ok(outcome) => Outcome::Action(Ok(()), confirmation_notice(outcome.rule, false)),
        Err(error) => Outcome::Action(Err(error), ""),
    }
}

/// Product copy for a significance criterion.
fn criterion_label(criterion: &str) -> &'static str {
    match criterion {
        "cross_cutting" => t::criterion_cross_cutting(),
        "data_or_contract" => t::criterion_data_or_contract(),
        "security_or_privacy" => t::criterion_security_or_privacy(),
        "external_dependency" => t::criterion_external_dependency(),
        "hard_to_reverse" => t::criterion_hard_to_reverse(),
        "first_of_a_kind" => t::criterion_first_of_a_kind(),
        "past_problem" => t::criterion_past_problem(),
        "constrains_future_work" => t::criterion_constrains_future_work(),
        _ => t::criterion_other(),
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
        .child(section_label(&theme, t::confidence_title()))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .child(div().w(px(120.0)).child(meter(
                    &theme,
                    value,
                    theme.colors.accent_default(),
                )))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_primary())
                        .child(format!("{:.0}%", value * 100.0)),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_muted())
                        .child(t::confidence_estimate()),
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
        CandidateStatus::Pending => (t::status_pending(), theme.colors.text_muted()),
        CandidateStatus::Snoozed => (t::status_snoozed(), theme.colors.status_info()),
        CandidateStatus::Accepted => (t::status_accepted(), theme.colors.status_success()),
        CandidateStatus::EditedAndAccepted => (t::status_edited(), theme.colors.status_success()),
        CandidateStatus::Dismissed => (t::status_dismissed(), theme.colors.status_danger()),
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
        InboxError::InvalidEdits(_) => t::error_invalid_edits(),
        InboxError::NotFound | InboxError::InvalidState => t::error_candidate_changed(),
        _ => t::error_load(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reading_member_evidence_does_not_replace_confirmation_snapshot() {
        let representative = CandidateDetail {
            summary: sample_summary(),
            qualifiers: vec![],
            criteria: vec![],
            rationale: "Original reviewed rationale".into(),
            evidence_refs: vec!["representative-source".into()],
            diff_summary: application::inbox::DiffSummary {
                files: vec![],
                artifacts: 0,
            },
            artifacts: vec![],
        };
        let mut member = representative.clone();
        member.summary.id = "sibling".into();
        member.evidence_refs = vec!["sibling-source".into()];
        assert_eq!(
            evidence_detail(&representative, Some(&member)).summary.id,
            "sibling"
        );
        assert_eq!(
            evidence_detail(&representative, None).summary.id,
            representative.summary.id
        );
        assert_eq!(representative.evidence_refs, ["representative-source"]);
        assert_ne!(representative.summary.id, member.summary.id);
    }

    #[test]
    fn refresh_from_previous_project_is_rejected() {
        assert!(accepts_refresh(3, 3));
        assert!(!accepts_refresh(3, 4));
    }

    #[test]
    fn polling_leaves_drafts_alone_and_never_overlaps_row_requests() {
        assert!(!can_poll_rows(false, true, true));
        assert!(!can_poll_rows(true, true, false));
        assert!(!can_poll_rows(false, false, false));
        assert!(can_poll_rows(false, true, false));
    }

    #[test]
    fn selected_candidate_is_reconciled_after_external_changes() {
        let row = sample_summary();
        let current = &row;
        assert_eq!(
            selected_change(Some(current), std::slice::from_ref(current)),
            SelectedChange::Unchanged
        );
        assert_eq!(selected_change(Some(current), &[]), SelectedChange::Removed);
        let mut changed = current.clone();
        changed.updated_at = "2026-10-05T00:00:00Z".into();
        assert_eq!(
            selected_change(Some(current), &[changed]),
            SelectedChange::Changed
        );
    }

    #[test]
    fn new_arrivals_outside_window_do_not_resolve_selection() {
        let selected = sample_summary();
        // Newer arrivals have displaced the selected row from the read window.
        let mut window = Vec::new();
        retain_eligible_selection(&mut window, selected.clone(), true);
        assert_eq!(
            selected_change(Some(&selected), &window),
            SelectedChange::Unchanged
        );
        let mut resolved = selected.clone();
        resolved.status = CandidateStatus::Accepted;
        assert_eq!(
            selected_change(Some(&selected), &[]),
            SelectedChange::Removed
        );
    }

    #[test]
    fn sibling_acceptance_removes_raw_pending_selection_when_projection_excludes_it() {
        let selected = sample_summary();
        assert_eq!(selected.status, CandidateStatus::Pending);
        let mut rows = vec![];
        retain_eligible_selection(&mut rows, selected.clone(), false);
        assert_eq!(
            selected_change(Some(&selected), &rows),
            SelectedChange::Removed
        );
        // Removed reconciliation clears detail and links, so footer actions disappear.
        assert!(rows.is_empty());
    }

    fn sample_summary() -> CandidateSummary {
        CandidateSummary {
            kind: CandidateKind::Decision,
            significance: 1.0,
            id: "selected".into(),
            project_id: "demo-xemnas".into(),
            project_location: "demo".into(),
            capture_id: "capture".into(),
            status: CandidateStatus::Pending,
            question: "Pergunta".into(),
            choice: "Escolha".into(),
            confidence: 1.0,
            confidence_reason: "Fixture".into(),
            signals: vec![],
            adapter: None,
            session_id: None,
            observed_at: None,
            received_at: "2026-10-04".into(),
            created_at: "2026-10-04".into(),
            updated_at: "2026-10-04".into(),
        }
    }

    #[test]
    fn successful_read_does_not_clear_failed_retry() {
        let mut action_error = None;
        record_retry_result(&mut action_error, "job", false);
        let mut read_error = true;
        assert!(read_error);
        read_error = false;
        assert!(!read_error);
        assert_eq!(action_error.as_deref(), Some("job"));
        record_retry_result(&mut action_error, "job", true);
        assert!(action_error.is_none());
    }

    #[test]
    fn summary_and_facts_leave_out_zero_parts() {
        let mut capture = CaptureProgress {
            project_id: "p".into(),
            capture_id: "c".into(),
            received_at: "not a date".into(),
            title: None,
            adapter: Some("claude-code".into()),
            job_id: None,
            attempts: 1,
            state: CaptureState::Queued,
            reason: AssessmentReason::Unknown,
            source: None,
            model: None,
            durable: 0,
            detail: 0,
            candidates: Default::default(),
            can_retry: false,
        };
        assert_eq!(capture_facts(&capture), "not a date · claude-code");
        assert_eq!(
            capture_summary(&[capture.clone()]),
            "1 waiting for analysis"
        );
        capture.state = CaptureState::Completed;
        capture.attempts = 2;
        capture.model = Some("m".into());
        capture.candidates.pending = 2;
        capture.candidates.adopted = 1;
        assert_eq!(
            capture_facts(&capture),
            "not a date · claude-code · m · 2 proposals · 1 confirmed · attempt 2"
        );
        assert_eq!(
            capture_summary(&[capture.clone(), capture]),
            "2 analysed · 4 proposals"
        );
    }

    #[test]
    fn capture_copy_does_not_reuse_old_assessment_while_running() {
        let mut capture = CaptureProgress {
            project_id: "p".into(),
            capture_id: "c".into(),
            received_at: "2026-10-04".into(),
            title: None,
            adapter: None,
            job_id: Some("j".into()),
            attempts: 2,
            state: CaptureState::Running,
            reason: AssessmentReason::Detail,
            source: None,
            model: None,
            durable: 0,
            detail: 1,
            candidates: Default::default(),
            can_retry: false,
        };
        assert_eq!(
            progress_copy(&capture),
            "Capture received · analysis in progress"
        );
        capture.state = CaptureState::Completed;
        assert!(progress_copy(&capture).contains("only implementation details"));
        capture.reason = AssessmentReason::Unknown;
        assert!(progress_copy(&capture).contains("not reported"));
    }

    #[test]
    fn confirmation_names_the_created_item_and_only_effective_links() {
        assert_eq!(confirmation_notice(true, false), "Rule created.");
        assert_eq!(confirmation_notice(false, false), "Decision created.");
        assert_eq!(
            confirmation_notice(true, true),
            "Rule created and linked to the map."
        );
        assert_eq!(
            confirmation_notice(false, true),
            "Decision created and linked to the map."
        );
    }

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
