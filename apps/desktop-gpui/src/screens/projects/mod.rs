//! Projects screen: register, list and remove tracked directories.
//!
//! The screen talks only to the `application::projects` use cases; it never
//! opens a database or names a storage type. The composition root (`main.rs`)
//! mounts the concrete repository into [`Projects`] before the shell builds this
//! view. Removing a Project deletes the tracking row only — the directory on
//! disk is never touched.
//!
//! Every storage call runs on GPUI's background executor (ASYNC-001), never on
//! the UI thread: the handlers move the whole [`Projects`] use case into the
//! task and get it back with the outcome. The view keeps only the UI state and
//! applies the outcome back on the UI thread.

use application::projects::{
    ProjectError, ProjectRepository, Projects, RemovalError, RemovalImpact,
};
use domain::projects::ProjectSummary;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, Entity, EventEmitter, FocusHandle,
    PathPromptOptions, Render, Role, Stateful, Subscription, Window,
};

use super::format::date_time;
use crate::ui::controls::{action_button, button_foreground, icon_action, ButtonKind};
use crate::ui::feedback::{error_state, status_dot, StatusKind};
use crate::ui::glass::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{
    count_chip, hover_tint, mark_selected, panel_title, section_label, track_hover,
};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};
use crate::ui::tooltip::tooltip;

/// The project currently visible in the workspace, including empty selections.
pub struct ProjectChanged(pub Option<ProjectSummary>);

/// What the list area is currently showing.
enum ListState {
    /// A load is in flight on the background executor.
    Loading,
    /// The tracked Projects, ordered by registration time.
    Ready(Vec<ProjectSummary>),
    /// The storage read or write failed; the detail was logged, not rendered.
    Error(StorageFailure),
}

/// Which background operation is currently in flight, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InFlight {
    /// No operation is running; the use case is parked in the view.
    None,
    /// Listing the tracked projects.
    Listing,
    /// Registering the selected folder.
    Registering,
    /// Removing a tracking row.
    Removing,
}

/// The operation that failed, used to pick the product-language copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StorageContext {
    /// Reading the list.
    Load,
    /// Registering or removing a tracking row.
    Mutate,
}

impl StorageContext {
    /// Product-language error surface for this context.
    fn failure(self) -> StorageFailure {
        match self {
            Self::Load => StorageFailure {
                title: "Não foi possível carregar os projetos",
                body: "O armazenamento local não respondeu; a lista pode estar desatualizada.",
                recovery: "Tente de novo. Se o erro continuar, feche e abra o app.",
            },
            Self::Mutate => StorageFailure {
                title: "Não foi possível concluir a ação",
                body: "O armazenamento local recusou a operação; nada foi alterado.",
                recovery: "Tente de novo. Se o erro continuar, feche e abra o app.",
            },
        }
    }

    /// Stable, non-sensitive operation tag for the diagnostic log.
    fn operation(self) -> &'static str {
        match self {
            Self::Load => "list",
            Self::Mutate => "mutate",
        }
    }
}

/// Product-language storage error; the technical detail never reaches the UI.
#[derive(Clone, Copy)]
struct StorageFailure {
    /// What happened, in one short sentence.
    title: &'static str,
    /// Impact on the user's work.
    body: &'static str,
    /// What the user can do next.
    recovery: &'static str,
}

/// Outcome of one background storage operation.
enum IoOutcome {
    /// `list()` result.
    Listed(Result<Vec<ProjectSummary>, ProjectError>),
    /// `register()` result, collapsed to success or failure.
    Registered(Result<(), ProjectError>),
    /// `remove()` result (`true` when a row was deleted).
    Removed(Result<bool, ProjectError>),
    /// What deleting a project with its data would erase.
    Impact(String, Result<RemovalImpact, ProjectError>),
    /// `remove_with_data()` result.
    Purged(Result<RemovalImpact, RemovalError>),
}

/// The "delete the project and its data" confirmation of one project.
struct Purge {
    id: String,
    impact: Option<RemovalImpact>,
}

/// The Projects screen view.
///
/// `R` is the repository port the composition root mounted; the screen is
/// generic over it so no concrete storage type leaks into the UI layer. The
/// `Send` bound is what lets the use case cross into the background executor.
pub struct ProjectsScreen<R: ProjectRepository + Send + 'static> {
    /// Row under the pointer, driving the hover spring.
    hovered: Option<String>,
    /// Parked here while no operation runs; `take()`n for the duration of a
    /// background task and put back with the outcome.
    projects: Option<Projects<R>>,
    list: ListState,
    in_flight: InFlight,
    selected_id: Option<String>,
    /// Product-language error for `InvalidLocation`/`AlreadyRegistered`.
    inline_error: Option<String>,
    /// Identifier of the row awaiting an inline removal confirmation.
    pending_removal: Option<String>,
    /// The destructive removal being confirmed, with its measured impact.
    purge: Option<Purge>,
    /// Where the project name is typed to confirm the purge.
    purge_name: Entity<SearchField>,
    _purge_subscription: Subscription,
    register_focus: FocusHandle,
    empty_focus: FocusHandle,
    remove_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
    /// One focus handle per listed row, kept stable across frames for Tab order.
    row_focus: Vec<(String, FocusHandle)>,
    /// Filter owned by the persistent project sidebar.
    query: String,
    search: Entity<SearchField>,
    _search_subscription: Subscription,
}

impl<R: ProjectRepository + Send + 'static> EventEmitter<ProjectChanged> for ProjectsScreen<R> {}

impl<R: ProjectRepository + Send + 'static> ProjectsScreen<R> {
    /// Builds the screen around an already composed `Projects` use case.
    pub fn new(cx: &mut Context<Self>, projects: Projects<R>) -> Self {
        let search = cx.new(SearchField::new);
        search.update(cx, |search, _| search.set_width(208.0));
        let subscription = cx.subscribe(&search, |screen, _, event: &SearchChanged, cx| {
            screen.set_query(&event.0, cx);
        });
        let purge_name = cx.new(|cx| {
            let mut field = SearchField::new(cx);
            field.stretch();
            field.set_context("Nome do projeto", cx);
            field
        });
        let purge_subscription =
            cx.subscribe(&purge_name, |_, _, _: &SearchChanged, cx| cx.notify());
        Self {
            hovered: None,
            projects: Some(projects),
            list: ListState::Loading,
            in_flight: InFlight::None,
            selected_id: None,
            inline_error: None,
            pending_removal: None,
            purge: None,
            purge_name,
            _purge_subscription: purge_subscription,
            register_focus: cx.focus_handle().tab_stop(true),
            empty_focus: cx.focus_handle().tab_stop(true),
            remove_focus: cx.focus_handle().tab_stop(true),
            confirm_focus: cx.focus_handle().tab_stop(true),
            cancel_focus: cx.focus_handle().tab_stop(true),
            row_focus: Vec::new(),
            query: String::new(),
            search,
            _search_subscription: subscription,
        }
    }

    /// The primary action receives focus when the window opens.
    pub fn initial_focus(&self) -> FocusHandle {
        self.register_focus.clone()
    }

    /// Returns the project visible after applying the sidebar filter.
    pub fn selected_project(&self) -> Option<ProjectSummary> {
        let visible = self.visible_rows();
        visible
            .iter()
            .find(|item| Some(item.id().as_str()) == self.selected_id.as_deref())
            .or_else(|| visible.first())
            .cloned()
    }

    /// Schedules the first list load, painting the loading state first.
    pub(crate) fn start(&mut self, cx: &mut Context<Self>, query: &str) {
        self.set_query(query, cx);
        self.reload(cx);
    }

    /// Sets the search filter and re-renders.
    ///
    /// The full list is always held; the filter narrows what is painted, so
    /// clearing the field restores the list without a second read.
    pub(crate) fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        if self.query == query {
            return;
        }
        self.query = query.to_string();
        cx.emit(ProjectChanged(self.selected_project()));
        cx.notify();
    }

    /// The rows that match the current search, newest first.
    fn visible_rows(&self) -> Vec<ProjectSummary> {
        let ListState::Ready(list) = &self.list else {
            return Vec::new();
        };
        let needle = self.query.trim().to_lowercase();
        list.iter()
            .rev()
            .filter(|summary| {
                needle.is_empty()
                    || summary.name().to_lowercase().contains(&needle)
                    || summary
                        .location()
                        .to_string()
                        .to_lowercase()
                        .contains(&needle)
            })
            .cloned()
            .collect()
    }

    /// Paints the loading state and schedules a background list read.
    fn reload(&mut self, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            // An operation already owns the use case; it will reload on finish.
            return;
        }
        self.list = ListState::Loading;
        self.pending_removal = None;
        self.in_flight = InFlight::Listing;
        cx.notify();
        self.spawn_io(cx, |projects| IoOutcome::Listed(projects.list()));
    }

    /// Moves the use case into the background executor and returns it with the
    /// operation outcome.
    ///
    /// `rusqlite::Connection` is `Send` but **not** `Sync`, so the use case is
    /// moved into the task instead of shared: `take()` removes it from the view
    /// while the task runs and puts it back with the result. That both satisfies
    /// the `Send` bound and serializes operations — a second request finds the
    /// use case absent and is a no-op until the first finishes — so the single
    /// connection is never used concurrently.
    ///
    /// The task always yields `(Projects<R>, IoOutcome)`, so the view repairs
    /// its state even when the operation fails and never stays on `loading`.
    fn spawn_io(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(&Projects<R>) -> IoOutcome + Send + 'static,
    ) {
        let Some(projects) = self.projects.take() else {
            return;
        };
        cx.spawn(async move |this, cx| {
            let (projects, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = operation(&projects);
                    (projects, outcome)
                })
                .await;
            // Apply back on the UI thread. If the view was dropped, the handle
            // is simply discarded with the use case.
            let _ = this.update(cx, |screen, cx| {
                screen.projects = Some(projects);
                screen.finish(outcome, cx);
            });
        })
        .detach();
    }

    /// Applies a background outcome on the UI thread.
    fn finish(&mut self, outcome: IoOutcome, cx: &mut Context<Self>) {
        self.in_flight = InFlight::None;
        // Measuring a purge or a mistyped confirmation leaves the list as it
        // was; announcing a project change would close the open panel.
        let unchanged = matches!(
            outcome,
            IoOutcome::Impact(_, Ok(_))
                | IoOutcome::Purged(Err(RemovalError::ConfirmationMismatch))
        );
        match outcome {
            IoOutcome::Listed(Ok(list)) => {
                self.sync_row_focus(&list, cx);
                if !list
                    .iter()
                    .any(|item| Some(item.id().as_str()) == self.selected_id.as_deref())
                {
                    self.selected_id = list.last().map(|item| item.id().as_str().to_string());
                }
                self.list = ListState::Ready(list);
            }
            IoOutcome::Listed(Err(error)) => {
                self.list = ListState::Error(storage_failure(StorageContext::Load, &error));
            }
            IoOutcome::Registered(Ok(())) => {
                self.inline_error = None;
                self.reload(cx);
            }
            IoOutcome::Registered(Err(error)) => match inline_message(&error) {
                Some(message) => self.inline_error = Some(message.to_string()),
                None => {
                    self.list = ListState::Error(storage_failure(StorageContext::Mutate, &error));
                }
            },
            IoOutcome::Removed(Ok(true)) => {
                self.inline_error = None;
                self.reload(cx);
            }
            IoOutcome::Removed(Ok(false)) => {
                self.inline_error = Some("Esse projeto já não estava na lista.".to_string());
                self.reload(cx);
            }
            IoOutcome::Removed(Err(error)) => {
                self.list = ListState::Error(storage_failure(StorageContext::Mutate, &error));
            }
            IoOutcome::Impact(id, Ok(impact)) => {
                if let Some(purge) = self.purge.as_mut().filter(|purge| purge.id == id) {
                    purge.impact = Some(impact);
                }
            }
            IoOutcome::Impact(_, Err(error)) => {
                self.purge = None;
                self.list = ListState::Error(storage_failure(StorageContext::Load, &error));
            }
            IoOutcome::Purged(Ok(_)) => {
                self.purge = None;
                self.inline_error = None;
                self.reload(cx);
            }
            IoOutcome::Purged(Err(RemovalError::ConfirmationMismatch)) => {
                self.inline_error = Some("O nome digitado não confere; nada foi apagado.".into());
            }
            IoOutcome::Purged(Err(RemovalError::Project(error))) => {
                self.purge = None;
                self.list = ListState::Error(storage_failure(StorageContext::Mutate, &error));
            }
        }
        if !unchanged {
            cx.emit(ProjectChanged(self.selected_project()));
        }
        cx.notify();
    }

    /// Keeps a stable focus handle per project so Tab order survives reloads.
    fn sync_row_focus(&mut self, list: &[ProjectSummary], cx: &mut Context<Self>) {
        let mut handles = Vec::with_capacity(list.len());
        for summary in list {
            let id = summary.id().as_str().to_string();
            let handle = self
                .row_focus
                .iter()
                .find(|(existing, _)| existing == &id)
                .map(|(_, handle)| handle.clone())
                .unwrap_or_else(|| cx.focus_handle().tab_stop(true));
            handles.push((id, handle));
        }
        self.row_focus = handles;
    }

    /// Opens the system's directory picker. Cancellation changes nothing.
    fn open_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Abrir pasta".into()),
        });
        cx.spawn(async move |this, cx| match receiver.await {
            Ok(Ok(Some(paths))) => {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |screen, cx| screen.register_path(path, cx));
                }
            }
            Ok(Ok(None)) => {}
            error => {
                tracing::error!(
                    ?error,
                    operation = "folder_picker",
                    "could not select a folder"
                );
                let _ = this.update(cx, |screen, cx| {
                    screen.inline_error =
                        Some("Não foi possível abrir o seletor de pastas.".into());
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Registers only the selected path; no directory contents are copied.
    fn register_path(&mut self, path: std::path::PathBuf, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            return;
        }
        self.inline_error = None;
        self.in_flight = InFlight::Registering;
        cx.notify();
        self.spawn_io(cx, move |projects| {
            IoOutcome::Registered(projects.register(&path).map(|_| ()))
        });
    }

    /// Opens the inline confirmation for a row.
    fn request_remove(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.pending_removal = Some(id);
        self.inline_error = None;
        window.focus(&self.cancel_focus, cx);
        cx.notify();
    }

    /// Opens the destructive confirmation and measures what it would erase.
    fn request_purge(&mut self, id: String, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            return;
        }
        self.pending_removal = None;
        self.inline_error = None;
        self.purge = Some(Purge {
            id: id.clone(),
            impact: None,
        });
        self.purge_name
            .update(cx, |field, cx| field.set_context("Nome do projeto", cx));
        cx.notify();
        self.spawn_io(cx, move |projects| {
            let impact = projects.removal_impact(&id);
            IoOutcome::Impact(id, impact)
        });
    }

    fn cancel_purge(&mut self, cx: &mut Context<Self>) {
        self.purge = None;
        self.inline_error = None;
        cx.notify();
    }

    /// Deletes the project and all its data; the folder on disk is untouched.
    fn confirm_purge(&mut self, id: String, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            return;
        }
        let typed = self.purge_name.read(cx).value().to_owned();
        self.in_flight = InFlight::Removing;
        cx.notify();
        self.spawn_io(cx, move |projects| {
            IoOutcome::Purged(projects.remove_with_data(&id, &typed))
        });
    }

    /// Dismisses the inline confirmation.
    fn cancel_remove(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.pending_removal = None;
        window.focus(&self.remove_focus, cx);
        cx.notify();
    }

    /// Removes the tracking row; the directory on disk is left untouched.
    fn confirm_remove(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            return;
        }
        self.pending_removal = None;
        self.in_flight = InFlight::Removing;
        window.focus(&self.register_focus, cx);
        cx.notify();
        self.spawn_io(cx, move |projects| IoOutcome::Removed(projects.remove(&id)));
    }

    fn select(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_id.as_deref() != Some(id.as_str()) {
            self.pending_removal = None;
        }
        if let Some((_, focus)) = self.row_focus.iter().find(|(key, _)| key == &id) {
            window.focus(focus, cx);
        }
        self.selected_id = Some(id);
        cx.emit(ProjectChanged(self.selected_project()));
        cx.notify();
    }

    /// First-run composition: one real action, framed without invented data.
    fn render_empty(&self, theme: &Theme, cx: &mut Context<Self>) -> Stateful<Div> {
        let button = action_button(theme, "projects-empty-open", ButtonKind::Primary, true)
            .px(px(SpacingScale::S4))
            .aria_label("Abrir pasta no seletor do sistema")
            .track_focus(&self.empty_focus)
            .on_click(cx.listener(|this, _, _, cx| this.open_folder(cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.open_folder(cx);
                    cx.stop_propagation();
                }
            }))
            .child(icon(
                IconName::Plus,
                16.0,
                button_foreground(theme, ButtonKind::Primary, true),
            ))
            .child("Abrir pasta…");

        div()
            .id("projects-empty")
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .role(Role::Status)
            .aria_label("Nenhum projeto acompanhado")
            .child(
                div()
                    .w_full()
                    .max_w(px(420.0))
                    .px(px(SpacingScale::S6))
                    .flex()
                    .flex_col()
                    .items_start()
                    .gap(px(SpacingScale::S3))
                    .child(
                        div()
                            .size(px(40.0))
                            .mb(px(SpacingScale::S2))
                            .rounded(theme.radius.surface())
                            .border_1()
                            .border_color(theme.colors.hairline_divider())
                            .bg(theme.colors.surface())
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(icon(IconName::Folder, 20.0, theme.colors.text_secondary())),
                    )
                    .child(section_label(theme, "Primeiro projeto"))
                    .child(
                        text_style(div(), TypeScale::HEADING_1)
                            .text_color(theme.colors.text_primary())
                            .child("Comece por uma pasta"),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY)
                            .text_color(theme.colors.text_secondary())
                            .child("Escolha uma pasta existente para acompanhar. Seus arquivos permanecem no lugar."),
                    )
                    .child(div().mt(px(SpacingScale::S3)).child(button)),
            )
    }

    /// A compact selectable entry in the project sidebar.
    fn render_row(
        &self,
        theme: &Theme,
        summary: &ProjectSummary,
        selected_id: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let id = summary.id().as_str().to_string();
        let key_id = id.clone();
        let selected = selected_id == Some(id.as_str());
        let focus = self
            .row_focus
            .iter()
            .find(|(key, _)| key == &id)
            .map(|(_, handle)| handle.clone())
            .unwrap_or_else(|| self.confirm_focus.clone());
        // Hover must not erase the persistent selection state.
        let row = div()
            .id((ElementId::from("project-entry"), id.clone()))
            .relative()
            .h(px(48.0))
            .px(px(SpacingScale::S4))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3));
        mark_selected(row, theme, selected)
            .role(Role::Button)
            .aria_label(format!("Selecionar {}", summary.name()))
            .aria_selected(selected)
            .track_focus(&focus)
            .focus_visible(focus_ring(theme))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, window, cx| this.select(id.clone(), window, cx)))
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.select(key_id.clone(), window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .child(icon(
                IconName::Folder,
                16.0,
                if selected {
                    theme.colors.text_primary()
                } else {
                    theme.colors.text_muted()
                },
            ))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        text_style(div(), TypeScale::ROW_TITLE)
                            .text_color(theme.colors.text_primary())
                            .truncate()
                            .child(summary.name().to_string()),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .text_ellipsis_start()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(summary.location().to_string()),
                    ),
            )
    }
}

impl<R: ProjectRepository + Send + 'static> ProjectsScreen<R> {
    /// Persistent project navigation, shared by all project destinations.
    pub fn render_sidebar(
        &mut self,
        footer: Option<AnyElement>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = Theme::current(cx);
        let visible = self.visible_rows();
        let total = match &self.list {
            ListState::Ready(list) => list.len(),
            _ => 0,
        };
        let selected = self.selected_project();
        let selected_id = selected.as_ref().map(|item| item.id().as_str());
        let rows: Vec<AnyElement> = visible
            .iter()
            .map(|item| {
                let id = item.id().as_str().to_string();
                let selected = selected_id == Some(id.as_str());
                let hovered = self.hovered.as_deref() == Some(id.as_str());
                let key = id.clone();
                let row = self
                    .render_row(&theme, item, selected_id, cx)
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        if track_hover(&mut this.hovered, key.clone(), *hovered) {
                            cx.notify();
                        }
                    }));
                hover_tint(
                    row,
                    ElementId::Name(format!("project-hover-{id}").into()),
                    hovered,
                    !selected,
                    &theme,
                )
            })
            .collect();
        let open = icon_action(
            &theme,
            "projects-open-folder",
            "Abrir pasta no seletor do sistema",
        )
        .tooltip(tooltip("Abrir pasta…", None))
        .track_focus(&self.register_focus)
        .on_click(cx.listener(|this, _, _, cx| this.open_folder(cx)))
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                this.open_folder(cx);
                cx.stop_propagation();
            }
        }))
        .child(icon(
            IconName::FolderPlus,
            16.0,
            theme.colors.text_secondary(),
        ));

        let sidebar = div()
            .id("projects-sidebar")
            .w(px(248.0))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .when(!theme.colors.is_glass(), |sidebar| {
                sidebar.bg(theme.colors.rail())
            })
            .when(!theme.colors.is_glass(), |sidebar| {
                sidebar
                    .border_r_1()
                    .border_color(theme.colors.hairline_divider())
            })
            .child(
                div()
                    .px(px(SpacingScale::S4))
                    .pt(px(SpacingScale::S3))
                    .pb(px(SpacingScale::S3))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(panel_title(&theme, "Projetos"))
                            .child(count_chip(
                                &theme,
                                if self.query.is_empty() {
                                    total.to_string()
                                } else {
                                    format!("{} de {total}", visible.len())
                                },
                            ))
                            .child(div().flex_1())
                            .child(open),
                    )
                    .child(self.search.clone())
                    .children(
                        self.inline_error
                            .as_ref()
                            .map(|message| inline_alert(&theme, "projects-inline-error", message)),
                    ),
            )
            .child(
                div()
                    .id("projects-list-scroll")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .children(rows),
            )
            .children(footer);

        sidebar.into_any_element()
    }

    /// Projects for the command palette: id, name and location.
    pub fn palette_projects(&self) -> Vec<(String, String, String)> {
        match &self.list {
            ListState::Ready(list) => list
                .iter()
                .map(|project| {
                    (
                        project.id().as_str().to_owned(),
                        project.name().to_owned(),
                        project.location().to_string(),
                    )
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Selects a project from the command palette.
    pub fn select_project(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
        self.select(id, window, cx);
    }

    /// Opens the system folder picker from the command palette.
    pub fn open_folder_dialog(&mut self, cx: &mut Context<Self>) {
        self.open_folder(cx);
    }

    /// Focus target when the project panel opens: its first action.
    pub fn panel_focus(&self) -> FocusHandle {
        self.remove_focus.clone()
    }

    /// Properties and the remove action of the selected project, shown in the
    /// panel under the breadcrumb. The panel frame belongs to the shell.
    pub fn render_project_panel(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let theme = Theme::current(cx);
        let summary = self.selected_project()?;
        let id = summary.id().as_str().to_string();
        let pending = self.pending_removal.as_deref() == Some(id.as_str());
        let purging = self
            .purge
            .as_ref()
            .filter(|purge| purge.id == id)
            .map(|purge| purge.impact);
        let actions: AnyElement = if let Some(impact) = purging {
            self.render_purge(&theme, &summary, impact, cx)
        } else if pending {
            div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_secondary())
                        .child("Remover da lista? A pasta permanece no disco."),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap(px(SpacingScale::S2))
                        .child(
                            detail_action(&theme, "projects-cancel", "Cancelar", false)
                                .track_focus(&self.cancel_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.cancel_remove(window, cx)
                                })),
                        )
                        .child(
                            detail_action(&theme, "projects-confirm", "Remover", true)
                                .track_focus(&self.confirm_focus)
                                .on_click({
                                    let id = id.clone();
                                    cx.listener(move |this, _, window, cx| {
                                        this.confirm_remove(id.clone(), window, cx)
                                    })
                                }),
                        ),
                )
                .into_any_element()
        } else {
            let purge_id = id.clone();
            div()
                .flex()
                .gap(px(SpacingScale::S2))
                .child(
                    detail_action(&theme, "projects-remove", "Remover da lista", false)
                        .track_focus(&self.remove_focus)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.request_remove(id.clone(), window, cx)
                        })),
                )
                .child(
                    detail_action(&theme, "projects-purge", "Apagar dados…", true).on_click(
                        cx.listener(move |this, _, _, cx| this.request_purge(purge_id.clone(), cx)),
                    ),
                )
                .into_any_element()
        };
        Some(
            div()
                .id("project-detail")
                .w(px(360.0))
                .p(px(SpacingScale::S4))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S1))
                .child(section_label(&theme, "Projeto"))
                .child(
                    text_style(div(), TypeScale::HEADING_3)
                        .pb(px(SpacingScale::S1))
                        .truncate()
                        .child(summary.name().to_string()),
                )
                .child(detail_line(
                    &theme,
                    "Localização",
                    summary.location().to_string(),
                ))
                .child(detail_line(
                    &theme,
                    "Adicionado",
                    date_time(summary.registered_at()),
                ))
                .child(
                    div()
                        .mt(px(SpacingScale::S2))
                        .pt(px(SpacingScale::S3))
                        .border_t_1()
                        .border_color(theme.colors.hairline_divider())
                        .child(actions),
                )
                .into_any_element(),
        )
    }

    /// The destructive confirmation: what will be erased, then the typed name.
    fn render_purge(
        &self,
        theme: &Theme,
        summary: &ProjectSummary,
        impact: Option<RemovalImpact>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        let name = summary.name().to_string();
        let id = summary.id().as_str().to_string();
        let matches = self.purge_name.read(cx).value().trim() == name;
        let counts: AnyElement = match impact {
            None => text_style(div(), TypeScale::META)
                .text_color(colors.text_muted())
                .child("Medindo o que será apagado…")
                .into_any_element(),
            Some(impact) if impact.is_empty() => text_style(div(), TypeScale::BODY_SMALL)
                .text_color(colors.text_secondary())
                .child("Este projeto ainda não tem dados; só o cadastro será apagado.")
                .into_any_element(),
            Some(impact) => div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .children(
                    [
                        (impact.decisions, "decisão(ões) com versões e relações"),
                        (impact.candidates, "candidato(s)"),
                        (impact.claims, "regra(s) do projeto"),
                        (impact.captures, "captura(s) com evidências"),
                        (impact.injections, "registro(s) de contexto enviado"),
                    ]
                    .into_iter()
                    .filter(|(count, _)| *count > 0)
                    .map(|(count, label)| {
                        div()
                            .flex()
                            .gap(px(SpacingScale::S2))
                            .child(
                                text_style(div(), TypeScale::ROW_TITLE)
                                    .w(px(40.0))
                                    .flex_none()
                                    .text_color(colors.status_danger())
                                    .child(count.to_string()),
                            )
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .text_color(colors.text_secondary())
                                    .child(label),
                            )
                    }),
                )
                .into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                text_style(div(), TypeScale::ROW_TITLE)
                    .text_color(colors.status_danger())
                    .child("Apagar o projeto e todos os dados?"),
            )
            .child(counts)
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(format!(
                        "Não dá para desfazer. A pasta no disco não é tocada. \
                         Digite {name} para confirmar."
                    )),
            )
            .child(self.purge_name.clone())
            .children(self.inline_error.clone().map(|error| {
                text_style(div(), TypeScale::META)
                    .text_color(colors.status_danger())
                    .child(error)
            }))
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(SpacingScale::S2))
                    .child(
                        detail_action(theme, "projects-purge-cancel", "Cancelar", false)
                            .on_click(cx.listener(|this, _, _, cx| this.cancel_purge(cx))),
                    )
                    .child(
                        action_button(
                            theme,
                            "projects-purge-confirm",
                            ButtonKind::Secondary,
                            matches && self.projects.is_some(),
                        )
                        .aria_label("Apagar tudo")
                        .when(matches, |button| button.text_color(colors.status_danger()))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if matches {
                                this.confirm_purge(id.clone(), cx)
                            }
                        }))
                        .child("Apagar tudo"),
                    ),
            )
            .into_any_element()
    }

    /// Project properties and tracking controls without a duplicate sidebar.
    pub fn render_details(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = Theme::current(cx);
        let visible = self.visible_rows();
        let content: AnyElement = match &self.list {
            ListState::Loading => loading_state(&theme).into_any_element(),
            ListState::Error(failure) => error_state(
                &theme,
                "projects-error",
                failure.title,
                failure.body,
                failure.recovery,
            )
            .into_any_element(),
            ListState::Ready(list) if list.is_empty() => {
                self.render_empty(&theme, cx).into_any_element()
            }
            ListState::Ready(_) if !self.query.is_empty() && visible.is_empty() => {
                no_matches_state(&theme, &self.query).into_any_element()
            }
            // A selected project is read through its destinations; its
            // properties live in the project panel opened from the breadcrumb.
            ListState::Ready(_) => div().into_any_element(),
        };
        div()
            .id("projects-detail-scroll")
            .size_full()
            .overflow_y_scroll()
            .child(content)
            .into_any_element()
    }
}

impl<R: ProjectRepository + Send + 'static> Render for ProjectsScreen<R> {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("projects-screen")
            .size_full()
            .flex()
            .overflow_hidden()
            .child(self.render_sidebar(None, cx))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .child(self.render_details(cx)),
            )
    }
}

fn detail_line(theme: &Theme, label: &'static str, value: String) -> Div {
    div()
        .w_full()
        .py(px(SpacingScale::S1))
        .flex()
        .items_baseline()
        .gap(px(SpacingScale::S3))
        .child(
            text_style(div(), TypeScale::LABEL)
                .w(px(84.0))
                .flex_none()
                .text_color(theme.colors.text_muted())
                .child(label),
        )
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .flex_1()
                .min_w(px(0.0))
                .text_color(theme.colors.text_primary())
                .child(value),
        )
}

/// Text-only secondary actions avoid the generic square placeholder icon.
fn detail_action(
    theme: &Theme,
    id: &'static str,
    label: &'static str,
    destructive: bool,
) -> Stateful<Div> {
    let button = action_button(theme, id, ButtonKind::Secondary, true).aria_label(label);
    if destructive {
        button.text_color(theme.colors.status_danger()).child(label)
    } else {
        button.child(label)
    }
}

/// The "your search matched nothing" state.
///
/// A separate primitive from the first-run state, on purpose. The empty
/// state tells you what to do next; this one tells you the filter is the reason
/// the list is empty, and that clearing it brings the list back. Reusing the
/// empty state here would make a person think their data was gone.
fn no_matches_state(theme: &Theme, query: &str) -> Stateful<Div> {
    div()
        .id("projects-no-matches")
        .pt(px(SpacingScale::S10))
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .role(Role::Status)
        .aria_label(format!("Nenhum projeto corresponde a {query}"))
        .child(
            text_style(div(), TypeScale::HEADING_2)
                .text_color(theme.colors.text_primary())
                .child("Nada corresponde"),
        )
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_secondary())
                .child(format!(
                    "Nenhum projeto corresponde a “{query}”. Esc limpa a busca."
                )),
        )
}

/// A compact inline alert for validation errors that stay on the screen.
fn inline_alert(theme: &Theme, id: impl Into<ElementId>, message: &str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .role(Role::Alert)
        .aria_label(message.to_string())
        .child(
            div()
                .size(px(8.0))
                .rounded_full()
                .bg(theme.colors.status_danger()),
        )
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_secondary())
                .child(message.to_string()),
        )
}

/// The list loading pattern: a status dot with an accessible description.
fn loading_state(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .child(status_dot(
            theme,
            "projects-loading",
            StatusKind::Info,
            "Carregando projetos",
        ))
}

/// Product-language message for the validation errors; `None` routes storage
/// failures to the full error state instead.
fn inline_message(error: &ProjectError) -> Option<&'static str> {
    match error {
        ProjectError::InvalidLocation => Some("Esse caminho não existe ou não é um diretório."),
        ProjectError::AlreadyRegistered => Some("Esse diretório já está sendo acompanhado."),
        ProjectError::NotFound => Some("Esse projeto não foi encontrado."),
        ProjectError::Storage(_) => None,
    }
}

/// Logs the technical detail and returns the product-language failure surface.
///
/// Only the operation tag and the storage diagnostic are logged; no file
/// content (PRIV-001). The message itself is never rendered in the UI.
fn storage_failure(context: StorageContext, error: &ProjectError) -> StorageFailure {
    if let ProjectError::Storage(detail) = error {
        tracing::error!(
            error = %detail,
            operation = context.operation(),
            "projects storage operation failed"
        );
    }
    context.failure()
}

#[cfg(test)]
mod tests {
    use application::projects::{ProjectError, ProjectRecord, ProjectRepository};
    use std::cell::RefCell;

    use super::{inline_message, storage_failure, StorageContext};

    struct FakeRepository {
        records: RefCell<Vec<ProjectRecord>>,
    }

    impl ProjectRepository for FakeRepository {
        fn insert(&self, record: &ProjectRecord) -> Result<(), ProjectError> {
            self.records.borrow_mut().push(record.clone());
            Ok(())
        }

        fn list(&self) -> Result<Vec<ProjectRecord>, ProjectError> {
            Ok(self.records.borrow().clone())
        }

        fn get(&self, id: &str) -> Result<Option<ProjectRecord>, ProjectError> {
            Ok(self
                .records
                .borrow()
                .iter()
                .find(|record| record.id == id)
                .cloned())
        }

        fn remove(&self, id: &str) -> Result<bool, ProjectError> {
            let mut records = self.records.borrow_mut();
            let before = records.len();
            records.retain(|record| record.id != id);
            Ok(records.len() != before)
        }
    }

    #[test]
    fn storage_errors_do_not_produce_an_inline_message() {
        assert_eq!(inline_message(&ProjectError::Storage("boom".into())), None);
    }

    #[test]
    fn validation_errors_have_product_language() {
        assert!(inline_message(&ProjectError::InvalidLocation).is_some());
        assert!(inline_message(&ProjectError::AlreadyRegistered).is_some());
    }

    #[test]
    fn storage_failure_never_renders_the_technical_detail() {
        let failure = storage_failure(
            StorageContext::Mutate,
            &ProjectError::Storage("disk I/O".into()),
        );
        assert_eq!(failure.title, "Não foi possível concluir a ação");
        assert!(!failure.body.contains("disk"));
        assert!(!failure.recovery.contains("disk"));
    }

    #[test]
    fn fake_repository_is_reachable_from_the_screen_module() {
        // Guards the test double against accidental signature drift.
        let repository = FakeRepository {
            records: RefCell::new(Vec::new()),
        };
        assert!(repository.list().unwrap().is_empty());
    }
}
