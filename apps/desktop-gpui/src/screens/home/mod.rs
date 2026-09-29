//! Home screen: the destination when the app opens.
//!
//! Home is **action, not explanation**. It answers one question — "what do I do
//! now?" — with the two things a person can actually do (register a directory,
//! open Projects) plus the state of the machine they are about to rely on. A
//! "how it works" tour, a dashboard of invented metrics and a promotional panel
//! are deliberately absent: every reference app in this category (Linear, Raycast)
//! keeps onboarding out of its landing surface, and the design system asks for
//! "o empty state com o próximo passo real da tela".
//!
//! Home reads the same `Projects` use case as the Projects screen; it never
//! opens a database and never names a storage type (ARCH-001).

use application::projects::{ProjectRepository, Projects};
use domain::projects::ProjectSummary;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, FocusHandle, Render, Role, Stateful, Window,
};

use crate::ui::controls::{primary_button, ControlState};
use crate::ui::feedback::{status_dot, StatusKind};
use crate::ui::glass::{GlassSurface, GlassVariant};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

/// What the recent-projects list is currently showing.
enum RecentState {
    /// A read is in flight on the background executor.
    Loading,
    /// The most recent tracked projects, newest first.
    Ready(Vec<ProjectSummary>),
    /// The read failed; the technical detail is logged, never rendered.
    Failed,
}

/// The Home screen view.
pub struct HomeScreen<R: ProjectRepository + Send + 'static> {
    theme: Theme,
    /// Parked while a background task owns it; returned with the outcome.
    projects: Option<Projects<R>>,
    recent: RecentState,
    /// Focus target for the primary action.
    action_focus: FocusHandle,
    /// Set when Home asks the shell to navigate to Projects; the shell consumes
    /// and clears it. Home never mounts another screen itself, so navigation
    /// stays in one place.
    request_projects: bool,
}

impl<R: ProjectRepository + Send + 'static> HomeScreen<R> {
    /// Builds Home around an already composed `Projects` use case.
    pub fn new(cx: &mut Context<Self>, projects: Projects<R>) -> Self {
        Self {
            theme: Theme::quiet_glass(),
            projects: Some(projects),
            recent: RecentState::Loading,
            action_focus: cx.focus_handle().tab_stop(true),
            request_projects: false,
        }
    }

    /// The focus handle the shell should focus when the window opens.
    pub fn action_focus(&self) -> FocusHandle {
        self.action_focus.clone()
    }

    /// Takes the pending Projects navigation request, if there is one.
    ///
    /// Returning it as an event (rather than Home swapping screens) keeps the
    /// shell the only place that decides what the content area shows.
    pub(crate) fn take_projects_request(&mut self) -> bool {
        std::mem::take(&mut self.request_projects)
    }

    /// Schedules the first read, painting the loading state first.
    pub(crate) fn start(&mut self, cx: &mut Context<Self>) {
        self.reload(cx);
    }

    /// Hands the use case back to the shell so it can mount another screen.
    ///
    /// Returns `None` while a background read owns it; see the note on
    /// [`ProjectsScreen::take_use_case`].
    pub(crate) fn take_use_case(&mut self) -> Option<Projects<R>> {
        self.projects.take()
    }

    /// Reads the tracked projects on the background executor.
    ///
    /// The use case is moved into the task (`take`/`put back`) rather than
    /// shared, because the SQLite connection is `Send` but not `Sync`
    /// (ASYNC-001).
    fn reload(&mut self, cx: &mut Context<Self>) {
        let Some(projects) = self.projects.take() else {
            return;
        };
        self.recent = RecentState::Loading;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let (projects, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = projects.list();
                    (projects, outcome)
                })
                .await;
            let _ = this.update(cx, |screen, cx| {
                screen.projects = Some(projects);
                screen.recent = match outcome {
                    Ok(list) => RecentState::Ready(list),
                    Err(error) => {
                        tracing::error!(
                            error = %error,
                            operation = "home_recent",
                            "could not read tracked projects for the home screen"
                        );
                        RecentState::Failed
                    }
                };
                cx.notify();
            });
        })
        .detach();
    }
}

impl<R: ProjectRepository + Send + 'static> Render for HomeScreen<R> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        // Most recent first. `list()` already orders by registration time
        // ascending, so the newest is the tail of the vector.
        let recent: Vec<ProjectSummary> = match &self.recent {
            RecentState::Ready(list) => list.iter().rev().take(4).cloned().collect(),
            _ => Vec::new(),
        };
        let tracked_total = match &self.recent {
            RecentState::Ready(list) => list.len(),
            _ => 0,
        };

        let greeting = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .child(
                text_style(div(), TypeScale::HEADING_1)
                    .text_color(theme.colors.text_primary())
                    .child("Acompanhe um diretório"),
            )
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(theme.colors.text_secondary())
                    .child("Registre um caminho local e o xemnas passa a observar o que muda nele, sem tocar no conteúdo."),
            );

        // The primary action repeats the Projects screen's register affordance
        // with the same `Glass Emphasis` recipe, so the two screens share one
        // visual language for "the thing you can do".
        let action = GlassSurface::new(GlassVariant::Emphasis)
            .render(&theme)
            .p(px(SpacingScale::S6))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S4))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(
                        text_style(div(), TypeScale::HEADING_2)
                            .text_color(theme.colors.text_primary())
                            .child("Comece por um projeto"),
                    ),
            )
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(theme.colors.text_secondary())
                    .child("O caminho precisa apontar para um diretório que já existe. O xemnas registra o acompanhamento e nunca escreve dentro dele."),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .child(
                        primary_button(&theme, "home-open-projects", ControlState::Rest, "Abrir Projects")
                            .track_focus(&self.action_focus)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_projects = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child("Enter"),
                    ),
            );

        let recent_area: AnyElement = match &self.recent {
            RecentState::Loading => status_dot(
                &theme,
                "home-loading",
                StatusKind::Info,
                "Carregando projetos acompanhados",
            )
            .into_any_element(),
            RecentState::Failed => status_dot(
                &theme,
                "home-recent-failed",
                StatusKind::Warning,
                "Não foi possível ler os projetos acompanhados",
            )
            .into_any_element(),
            RecentState::Ready(list) if list.is_empty() => status_dot(
                &theme,
                "home-no-projects",
                StatusKind::Info,
                "Nenhum diretório acompanhado ainda",
            )
            .into_any_element(),
            RecentState::Ready(_) => div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .children(recent.iter().map(|summary| recent_row(&theme, summary)))
                .into_any_element(),
        };

        let recent_section = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .child(
                        text_style(div(), TypeScale::HEADING_3)
                            .text_color(theme.colors.text_primary())
                            .child("Acompanhados recentemente"),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child(format!("{tracked_total} no total")),
                    ),
            )
            .child(recent_area);

        let footer = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .child(status_dot(
                &theme,
                "home-capture",
                StatusKind::Success,
                "Captura local ativa",
            ))
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child("O wallpaper do Windows atravessa o Mica desta janela."),
            );

        // The content is centred in the available height and capped at a
        // comfortable reading measure, so the window never shows a tall empty
        // band under the content. `justify_center` is what removes the ~560 px
        // of dead space this screen had when everything was pinned to the top;
        // the flex column still scrolls once the content outgrows the viewport.
        div()
            .id("home-screen")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(SpacingScale::S5))
            .px(px(SpacingScale::S6))
            .child(
                div()
                    .w_full()
                    .max_w(px(720.0))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S5))
                    .child(greeting)
                    .child(action)
                    .child(recent_section),
            )
            .child(
                div()
                    .w_full()
                    .max_w(px(720.0))
                    .mt(px(SpacingScale::S6))
                    .child(footer),
            )
    }
}

/// One recent-project row: name, path and registration time.
fn recent_row(theme: &Theme, summary: &ProjectSummary) -> Stateful<Div> {
    let location = summary.location().to_string();
    GlassSurface::new(GlassVariant::Low)
        .render(theme)
        .id((
            ElementId::from("home-recent"),
            summary.id().as_str().to_string(),
        ))
        .px(px(SpacingScale::S4))
        .py(px(SpacingScale::S3))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .role(Role::ListItem)
        .aria_label(location.clone())
        .child(
            div()
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
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_secondary())
                        .truncate()
                        .child(location),
                ),
        )
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_muted())
                .child(format!(
                    "{} níveis",
                    count_path_segments(&summary.location().to_string())
                )),
        )
}

/// Counts the path segments of a location, for the row's trailing metadata.
fn count_path_segments(location: &str) -> usize {
    location
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .count()
}

#[cfg(test)]
mod tests {
    use super::count_path_segments;

    #[test]
    fn windows_and_posix_paths_both_count_segments() {
        assert_eq!(count_path_segments(r"C:\code\xemnas"), 3);
        assert_eq!(count_path_segments("/home/pablo/xemnas"), 3);
        // A bare drive root is one segment, and a trailing slash adds none.
        assert_eq!(count_path_segments(r"C:\"), 1);
        assert_eq!(count_path_segments(r"D:\code\xemnas\"), 3);
    }
}
