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

use application::projects::{ProjectError, ProjectRepository, Projects};
use domain::projects::ProjectSummary;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, FocusHandle, KeyDownEvent, Render, Role,
    Stateful, Window,
};

use crate::ui::controls::{primary_button, quiet_button, ControlState};
use crate::ui::feedback::{error_state, status_dot, StatusKind};
use crate::ui::glass::{focus_ring, GlassSurface, GlassVariant};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

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
    /// Registering the path in the field.
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
}

/// The Projects screen view.
///
/// `R` is the repository port the composition root mounted; the screen is
/// generic over it so no concrete storage type leaks into the UI layer. The
/// `Send` bound is what lets the use case cross into the background executor.
pub struct ProjectsScreen<R: ProjectRepository + Send + 'static> {
    theme: Theme,
    /// Parked here while no operation runs; `take()`n for the duration of a
    /// background task and put back with the outcome.
    projects: Option<Projects<R>>,
    list: ListState,
    in_flight: InFlight,
    path_input: String,
    /// Product-language error for `InvalidLocation`/`AlreadyRegistered`.
    inline_error: Option<String>,
    /// Identifier of the row awaiting an inline removal confirmation.
    pending_removal: Option<String>,
    field_focus: FocusHandle,
    register_focus: FocusHandle,
    confirm_focus: FocusHandle,
    cancel_focus: FocusHandle,
    /// One focus handle per listed row, kept stable across frames for Tab order.
    row_focus: Vec<(String, FocusHandle)>,
}

impl<R: ProjectRepository + Send + 'static> ProjectsScreen<R> {
    /// Builds the screen around an already composed `Projects` use case.
    pub fn new(cx: &mut Context<Self>, projects: Projects<R>) -> Self {
        Self {
            theme: Theme::quiet_glass(),
            projects: Some(projects),
            list: ListState::Loading,
            in_flight: InFlight::None,
            path_input: String::new(),
            inline_error: None,
            pending_removal: None,
            field_focus: cx.focus_handle().tab_stop(true),
            register_focus: cx.focus_handle().tab_stop(true),
            confirm_focus: cx.focus_handle().tab_stop(true),
            cancel_focus: cx.focus_handle().tab_stop(true),
            row_focus: Vec::new(),
        }
    }

    /// The focus handle the shell should focus when the window opens.
    pub fn field_focus(&self) -> FocusHandle {
        self.field_focus.clone()
    }

    /// Schedules the first list load, painting the loading state first.
    pub(crate) fn start(&mut self, cx: &mut Context<Self>) {
        self.reload(cx);
    }

    /// Hands the use case back to the shell so it can mount another screen.
    ///
    /// Returns `None` while a background task owns the use case. The shell
    /// must then leave the screen mounted: the connection is `Send` but not
    /// `Sync`, so it cannot be read while a task is writing (ASYNC-001).
    pub(crate) fn take_use_case(&mut self) -> Option<Projects<R>> {
        self.projects.take()
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
        match outcome {
            IoOutcome::Listed(Ok(list)) => {
                self.sync_row_focus(&list, cx);
                self.list = ListState::Ready(list);
            }
            IoOutcome::Listed(Err(error)) => {
                self.list = ListState::Error(storage_failure(StorageContext::Load, &error));
            }
            IoOutcome::Registered(Ok(())) => {
                self.path_input.clear();
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

    /// Registers the path currently typed in the field.
    pub(crate) fn register_from_field(&mut self, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            return;
        }
        let input = self.path_input.trim().to_string();
        if input.is_empty() {
            self.inline_error = Some("Informe o caminho de um diretório.".to_string());
            cx.notify();
            return;
        }
        self.inline_error = None;
        self.in_flight = InFlight::Registering;
        cx.notify();
        self.spawn_io(cx, move |projects| {
            IoOutcome::Registered(projects.register(&input).map(|_| ()))
        });
    }

    /// Opens the inline confirmation for a row.
    fn request_remove(&mut self, id: String, cx: &mut Context<Self>) {
        self.pending_removal = Some(id);
        self.inline_error = None;
        cx.notify();
    }

    /// Dismisses the inline confirmation.
    fn cancel_remove(&mut self, cx: &mut Context<Self>) {
        self.pending_removal = None;
        cx.notify();
    }

    /// Removes the tracking row; the directory on disk is left untouched.
    fn confirm_remove(&mut self, id: String, cx: &mut Context<Self>) {
        if self.projects.is_none() {
            return;
        }
        self.pending_removal = None;
        self.in_flight = InFlight::Removing;
        cx.notify();
        self.spawn_io(cx, move |projects| IoOutcome::Removed(projects.remove(&id)));
    }

    /// Handles typing, backspace and Enter in the path field.
    fn on_field_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.control || keystroke.modifiers.alt || keystroke.modifiers.platform {
            return;
        }
        match keystroke.key.as_str() {
            "backspace" => {
                self.path_input.pop();
            }
            "enter" => {
                self.register_from_field(cx);
                return;
            }
            "escape" => {
                self.path_input.clear();
                self.inline_error = None;
            }
            "space" => {
                self.path_input.push(' ');
            }
            _ => {
                if let Some(character) = &keystroke.key_char {
                    self.path_input.push_str(character);
                }
            }
        }
        cx.notify();
    }

    /// Builds one list row, including the inline removal confirmation.
    fn render_row(&self, theme: &Theme, summary: &ProjectSummary, cx: &mut Context<Self>) -> Div {
        let id = summary.id().as_str().to_string();
        let pending = self.pending_removal.as_deref() == Some(id.as_str());
        let full_location = summary.location().to_string();

        let details = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .gap(px(SpacingScale::S1))
            .child(
                text_style(div(), TypeScale::HEADING_3)
                    .text_color(theme.colors.text_primary())
                    .child(summary.name().to_string()),
            )
            .child(
                // The path is the disambiguating value; it is truncated for
                // layout but the accessible name keeps the whole string.
                div()
                    .id((ElementId::from("projects-location"), id.clone()))
                    .aria_label(full_location.clone())
                    .w_full()
                    .overflow_hidden()
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(theme.colors.text_secondary())
                            .truncate()
                            .child(full_location),
                    ),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child(format!(
                        "Registrado em {}",
                        format_registered_at(summary.registered_at())
                    )),
            );

        let surface = GlassSurface::new(GlassVariant::Low)
            .render(theme)
            .p(px(SpacingScale::S4))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .child(details);

        if pending {
            let confirm_id = id.clone();
            surface
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_secondary())
                        .child("Remover acompanhamento?"),
                )
                .child(
                    quiet_button(
                        theme,
                        (ElementId::from("projects-confirm"), confirm_id.clone()),
                        ControlState::Rest,
                        "Confirmar",
                        true,
                    )
                    .track_focus(&self.confirm_focus)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.confirm_remove(confirm_id.clone(), cx);
                    })),
                )
                .child(
                    quiet_button(
                        theme,
                        (ElementId::from("projects-cancel"), id.clone()),
                        ControlState::Rest,
                        "Cancelar",
                        false,
                    )
                    .track_focus(&self.cancel_focus)
                    .on_click(cx.listener(|this, _, _, cx| this.cancel_remove(cx))),
                )
        } else {
            let handle = self
                .row_focus
                .iter()
                .find(|(existing, _)| existing == &id)
                .map(|(_, handle)| handle.clone())
                .unwrap_or_else(|| self.confirm_focus.clone());
            surface.child(
                quiet_button(
                    theme,
                    (ElementId::from("projects-remove"), id.clone()),
                    ControlState::Rest,
                    "Remover",
                    true,
                )
                .track_focus(&handle)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.request_remove(id.clone(), cx);
                })),
            )
        }
    }
}

impl<R: ProjectRepository + Send + 'static> Render for ProjectsScreen<R> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let field_focused = self.field_focus.is_focused(window);
        let input_text = self.path_input.clone();

        let header = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .child(
                text_style(div(), TypeScale::HEADING_1)
                    .text_color(theme.colors.text_primary())
                    .child("Projects"),
            )
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(theme.colors.text_secondary())
                    .child("Acompanhe diretórios locais sem alterar o conteúdo deles."),
            );

        let field = GlassSurface::new(GlassVariant::Low)
            .render(&theme)
            .id(ElementId::from("projects-path"))
            .h(px(44.0))
            .px(px(SpacingScale::S3))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S1))
            .track_focus(&self.field_focus)
            .role(Role::TextInput)
            .aria_label("Caminho do diretório")
            .focus_visible(focus_ring(&theme))
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.field_focus, cx);
            }))
            .on_key_down(cx.listener(Self::on_field_key_down))
            .child(if input_text.is_empty() {
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_muted())
                    .child("C:\\caminho\\do\\diretório")
            } else {
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_primary())
                    .child(input_text)
            })
            .child(if field_focused {
                div()
                    .w(px(1.0))
                    .h(px(16.0))
                    .bg(theme.colors.accent_default())
            } else {
                div()
            });

        let register_state = match self.in_flight {
            InFlight::Registering => ControlState::Loading,
            InFlight::None => ControlState::Rest,
            InFlight::Listing | InFlight::Removing => ControlState::Disabled,
        };

        let register_row = div()
            .flex()
            .items_start()
            .gap(px(SpacingScale::S3))
            .child(div().flex_1().min_w(px(0.0)).child(field))
            .child(
                primary_button(&theme, "projects-register", register_state, "Registrar")
                    .track_focus(&self.register_focus)
                    .on_click(cx.listener(|this, _, _, cx| this.register_from_field(cx))),
            );

        let inline = self
            .inline_error
            .clone()
            .map(|message| inline_alert(&theme, "projects-inline-error", &message));

        let list_area: AnyElement = match &self.list {
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
                projects_empty_state(&theme).into_any_element()
            }
            ListState::Ready(list) => {
                let rows: Vec<AnyElement> = list
                    .iter()
                    .map(|summary| self.render_row(&theme, summary, cx).into_any_element())
                    .collect();
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .children(rows)
                    .into_any_element()
            }
        };

        div()
            .id("projects-screen")
            .size_full()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .child(header)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S3))
                    .child(register_row)
                    .children(inline),
            )
            // A long tracked-project list scrolls instead of overflowing the
            // window; dozens of rows stay reachable.
            .child(
                div()
                    .id("projects-list-scroll")
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_y_scroll()
                    .child(list_area),
            )
    }
}

/// The Projects empty state.
///
/// The old version was a title, a sentence and a reassurance inside one glass
/// well, which left the register field above it doing the real work with no
/// pointer between them. This version names the next step and repeats the
/// commit the user is about to make ("só o acompanhamento é registrado"), so
/// the empty state explains the action instead of describing the absence.
///
/// It is still the same `Glass Low` recipe and the same tokens as every other
/// surface: an empty state is not a special visual mode.
fn projects_empty_state(theme: &Theme) -> Stateful<Div> {
    div()
        .id("projects-empty")
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .items_center()
        .justify_center()
        .p(px(SpacingScale::S6))
        .child(
            GlassSurface::new(GlassVariant::Low)
                .render(theme)
                .id("projects-empty-card")
                .w(px(440.0))
                .p(px(SpacingScale::S8))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S3))
                .role(Role::Status)
                .aria_label("Nenhum projeto acompanhado")
                .child(
                    text_style(div(), TypeScale::HEADING_2)
                        .text_color(theme.colors.text_primary())
                        .child("Nenhum projeto acompanhado"),
                )
                .child(
                    text_style(div(), TypeScale::BODY)
                        .text_color(theme.colors.text_secondary())
                        .child("Informe o caminho de um diretório acima e clique em Registrar. O xemnas passa a observar o que muda nele."),
                )
                .child(
                    div()
                        .mt(px(SpacingScale::S2))
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(
                            div()
                                .size(px(6.0))
                                .rounded_full()
                                .bg(theme.colors.status_success()),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(theme.colors.text_muted())
                                .child("Só o acompanhamento é registrado; o conteúdo permanece no disco."),
                        ),
                ),
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

/// Formats an RFC 3339 `registered_at` as `YYYY-MM-DD HH:MM` for display.
fn format_registered_at(value: &str) -> String {
    match (value.get(..10), value.get(11..16)) {
        (Some(date), Some(time)) => format!("{date} {time}"),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use application::projects::{ProjectError, ProjectRecord, ProjectRepository};
    use std::cell::RefCell;

    use super::{format_registered_at, inline_message, storage_failure, StorageContext};

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
    fn registered_at_is_trimmed_to_minutes() {
        assert_eq!(
            format_registered_at("2026-01-02T03:04:05Z"),
            "2026-01-02 03:04"
        );
        assert_eq!(format_registered_at("short"), "short");
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
