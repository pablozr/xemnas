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

use application::projects::{ProjectError, ProjectRepository, Projects};
use domain::projects::ProjectSummary;
use gpui::prelude::*;
use gpui::{
    div, px, AnyElement, Context, Div, ElementId, FocusHandle, KeyDownEvent, Render, Role,
    Stateful, Window,
};

use crate::fonts::FONT_CODE;
use crate::ui::controls::focus_ring;
use crate::ui::feedback::{status_dot, StatusKind};
use crate::ui::icons::Icon;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};

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
    /// The path being typed into the register field.
    path_input: String,
    /// Product-language error for a rejected path, kept on the row.
    inline_error: Option<String>,
    /// Focus target for the folder picker.
    choose_focus: FocusHandle,
    /// Focus target for the register button.
    register_focus: FocusHandle,
    /// Set when Home asks the shell to navigate to Projects; the shell consumes
    /// and clears it. Home never mounts another screen itself, so navigation
    /// stays in one place.
    /// Set while the platform's folder dialog is open.
    ///
    /// The dialog is modal, so the app behind it only needs to know not to
    /// accept another pick. The path arrives when the person answers.
    picking: bool,

    request_projects: bool,
    /// What the title-bar search is filtering on, mirrored from the shell.
    ///
    /// The list arrives whole from `list()`; the filter is applied here rather
    /// than in SQL because a person's search is a local interaction over data
    /// this screen already holds, and pushing it into the repository would
    /// make the shell re-run a background read on every keystroke.
    query: String,
}

impl<R: ProjectRepository + Send + 'static> HomeScreen<R> {
    /// Builds Home around an already composed `Projects` use case.
    pub fn new(cx: &mut Context<Self>, projects: Projects<R>) -> Self {
        Self {
            theme: Theme::quiet_glass(),
            projects: Some(projects),
            recent: RecentState::Loading,
            path_input: String::new(),
            inline_error: None,
            choose_focus: cx.focus_handle().tab_stop(true),
            register_focus: cx.focus_handle().tab_stop(true),
            request_projects: false,
            query: String::new(),
            picking: false,
        }
    }

    /// Sets the search filter and re-renders.
    pub(crate) fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        if self.query == query {
            return;
        }
        self.query = query.to_string();
        cx.notify();
    }

    /// The focus handle the shell should focus when the window opens.
    ///
    /// The folder picker, not a navigation button: Home's first purpose is to
    /// choose a directory, so that is where focus belongs.
    pub fn choose_focus(&self) -> FocusHandle {
        self.choose_focus.clone()
    }

    /// How many projects are registered, for the shell's status bar.
    pub(crate) fn tracked_total(&self) -> usize {
        match &self.recent {
            RecentState::Ready(list) => list.len(),
            _ => 0,
        }
    }

    /// Opens the platform's folder dialog and puts the result in the field.
    ///
    /// The dialog blocks until the person answers, so it runs on the background
    /// executor (ASYNC-001) and the outcome is applied back on the UI thread.
    /// A cancel leaves the field exactly as it was — picking is not a decision
    /// until a directory is chosen.
    pub(crate) fn pick_directory(&mut self, cx: &mut Context<Self>) {
        if self.picking {
            return;
        }
        self.picking = true;
        cx.notify();
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { folder_picker::pick_folder() })
                .await;
            let _ = this.update(cx, |screen, cx| {
                screen.picking = false;
                match outcome {
                    Ok(Ok(Some(path))) => {
                        screen.path_input = path.to_string_lossy().to_string();
                        screen.inline_error = None;
                    }
                    // A cancel, or a dialog that could not be opened. Neither is
                    // an error worth a message: the field is unchanged and the
                    // person can try again.
                    Ok(Ok(None)) | Ok(Err(_)) | Err(_) => {}
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The chosen path is shown, not typed. Hand-typing a path is the old way:
    /// it invites a typo, it asks the person to reproduce a string the shell
    /// already knows, and it is the reason this screen needed a browser bolted
    /// onto it. The path now arrives from the platform picker, and the field is
    /// a receipt for what was chosen rather than an input.
    fn path_receipt(&self, theme: &Theme) -> impl IntoElement {
        let empty = self.path_input.is_empty();
        div()
            .id("home-path")
            .h(px(40.0))
            .flex_1()
            .min_w(px(0.0))
            .px(px(SpacingScale::S3))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .rounded(px(RadiusScale::CONTROL))
            .bg(theme.colors.glass_fill_low())
            .border_1()
            .border_color(theme.colors.glass_border_control())
            .child(if empty {
                Icon::search(theme, 15.0, true).into_any_element()
            } else {
                Icon::check(theme, 15.0).into_any_element()
            })
            .child(
                // Monospace and truncated: a Windows path is long and shares a
                // prefix with every other one, so the tail is what identifies
                // it, and the face has to keep the separators aligned.
                text_style(div(), TypeScale::BODY_SMALL)
                    .font_family(FONT_CODE)
                    .text_color(if empty {
                        theme.colors.text_disabled()
                    } else {
                        theme.colors.text_primary()
                    })
                    .truncate()
                    .child(if empty {
                        "Nenhum diretório escolhido".to_string()
                    } else {
                        self.path_input.clone()
                    }),
            )
    }

    /// The action row: what is chosen, the register button, and the picker.
    ///
    /// "Escolher pasta" is the primary action, and "Registrar" is a plain
    /// control that only becomes meaningful once there is something chosen.
    /// Putting the picker first makes the sequence obvious — you cannot
    /// register a path you have not picked — instead of leaving a text field
    /// that invites a path nobody wants to type.
    fn choose_button(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("home-choose")
            .h(px(40.0))
            .px(px(SpacingScale::S4))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .rounded(px(RadiusScale::CONTROL))
            .bg(theme.colors.glass_fill_emphasis())
            .border_1()
            .border_color(theme.colors.glass_border())
            .text_color(theme.colors.accent_on_emphasis())
            .font_weight(gpui::FontWeight::MEDIUM)
            .role(Role::Button)
            .aria_label("Escolher diretório")
            .cursor(if self.picking {
                gpui::CursorStyle::Wait
            } else {
                gpui::CursorStyle::PointingHand
            })
            .opacity(if self.picking { 0.6 } else { 1.0 })
            .track_focus(&self.choose_focus)
            .focus_visible(focus_ring(theme))
            .on_click(cx.listener(|this, _, _, cx| this.pick_directory(cx)))
            .child(Icon::folder(theme, 15.0))
            .child(if self.picking {
                "Abrindo…"
            } else {
                "Escolher pasta"
            })
    }

    /// Registers the typed directory.
    ///
    /// The same use case and the same background-executor handoff the Projects
    /// screen uses, so the connection is `Send`-but-not-`Sync`-safe in both
    /// places and a register from Home behaves identically to one from Projects.
    fn register(&mut self, cx: &mut Context<Self>) {
        let Some(projects) = self.projects.take() else {
            return;
        };
        let location = self.path_input.trim().to_string();
        cx.spawn(async move |this, cx| {
            let (projects, outcome) = cx
                .background_executor()
                .spawn(async move {
                    let outcome = projects.register(&location).map(|_| ());
                    (projects, outcome)
                })
                .await;
            let _ = this.update(cx, |screen, cx| {
                screen.projects = Some(projects);
                match outcome {
                    Ok(()) => {
                        screen.path_input.clear();
                        screen.inline_error = None;
                        // Reload here, inside the completion handler, and not
                        // after the spawn. The use case has been `take`n for the
                        // task, so a `reload` issued on this line finds
                        // `self.projects == None` and returns early — the
                        // register succeeded and the list silently stayed
                        // empty. Reading the list here is also the only place
                        // the new project can appear.
                        screen.reload(cx);
                    }
                    Err(ProjectError::InvalidLocation) => {
                        screen.inline_error =
                            Some("Esse caminho não existe ou não é um diretório.".to_string());
                    }
                    Err(ProjectError::AlreadyRegistered) => {
                        screen.inline_error =
                            Some("Esse diretório já está sendo acompanhado.".to_string());
                    }
                    Err(error) => {
                        tracing::error!(
                            error = %error,
                            operation = "home_register",
                            "could not register the typed path"
                        );
                        screen.inline_error =
                            Some("Não foi possível registrar agora. Tente de novo.".to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
        self.reload(cx);
    }

    /// Takes the pending Projects navigation request, if there is one.
    ///
    /// Returning it as an event (rather than Home swapping screens) keeps the
    /// shell the only place that decides what the content area shows.
    pub(crate) fn take_projects_request(&mut self) -> bool {
        std::mem::take(&mut self.request_projects)
    }

    /// Schedules the first read, painting the loading state first.
    pub(crate) fn start(&mut self, cx: &mut Context<Self>, query: &str) {
        self.set_query(query, cx);
        if self.browsing.is_none() {
            self.browse(Self::default_browse_root(), cx);
        }
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
                screen.suggestions = match &outcome {
                    Ok(list) => sibling_directories(list),
                    Err(_) => Vec::new(),
                };
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        // Most recent first. `list()` already orders by registration time
        // ascending, so the newest is the tail of the vector.
        let recent: Vec<ProjectSummary> = match &self.recent {
            RecentState::Ready(list) => list
                .iter()
                .rev()
                .filter(|summary| matches_query(summary, &self.query))
                .take(5)
                .cloned()
                .collect(),
            _ => Vec::new(),
        };
        let tracked_total = match &self.recent {
            RecentState::Ready(list) => list.len(),
            _ => 0,
        };

        // The screen has one job: let the person register a directory. That is
        // why the form is here and not two clicks away on another screen.
        //
        // The previous version was a marketing composition — a tagline, two
        // feature cards, a section that restated the card above it, and a
        // button whose only job was to navigate away and do the same thing. It
        // read as generated because it described the product instead of
        // operating it. A real desktop landing surface is a work surface.
        let field_focused = self.path_focus.is_focused(window);
        let can_register = !self.path_input.trim().is_empty() && self.projects.is_some();

        let register = div()
            .id("home-register")
            .h(px(40.0))
            .px(px(SpacingScale::S4))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .rounded(px(RadiusScale::CONTROL))
            .bg(theme.colors.glass_fill_emphasis())
            .border_1()
            .border_color(theme.colors.glass_border())
            .text_color(theme.colors.accent_on_emphasis())
            .font_weight(gpui::FontWeight::MEDIUM)
            .role(Role::Button)
            .aria_label("Registrar projeto")
            .cursor(if can_register {
                gpui::CursorStyle::PointingHand
            } else {
                gpui::CursorStyle::default()
            })
            .opacity(if can_register { 1.0 } else { 0.5 })
            .track_focus(&self.register_focus)
            .focus_visible(focus_ring(&theme))
            // Always clickable: `register` ignores an empty field, and a
            // control that stops accepting clicks the moment a field empties
            // feels broken to the pointer even when it looks right.
            .on_click(cx.listener(|this, _, _, cx| this.register(cx)))
            .child(Icon::plus(&theme, 15.0))
            .child("Registrar");

        let register_row = div()
            .flex()
            .gap(px(SpacingScale::S2))
            .child(self.path_receipt(&theme))
            .child(register)
            .child(self.choose_button(&theme, cx));

        // Validation stays on the row, under the field, in the same voice the
        // Projects screen uses — one place that owns the sentence, so the two
        // screens never disagree about what went wrong.
        let inline: AnyElement = match &self.inline_error {
            Some(message) => div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .size(px(6.0))
                        .rounded_full()
                        .bg(theme.colors.status_danger()),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.status_danger())
                        .child(message.clone()),
                )
                .into_any_element(),
            None => div().into_any_element(),
        };

        let form = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(register_row)
            .child(inline);

        let list_area: AnyElement = match &self.recent {
            RecentState::Loading => div()
                .py(px(SpacingScale::S4))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child("Carregando projetos acompanhados"),
                )
                .into_any_element(),
            RecentState::Failed => status_dot(
                &theme,
                "home-recent-failed",
                StatusKind::Warning,
                "Não foi possível ler os projetos acompanhados",
            )
            .into_any_element(),
            RecentState::Ready(list) if list.is_empty() => div().into_any_element(),
            RecentState::Ready(_) if recent.is_empty() => {
                no_matches(&theme, &self.query).into_any_element()
            }
            RecentState::Ready(_) => div()
                .flex()
                .flex_col()
                .children(recent.iter().map(|summary| recent_row(&theme, summary)))
                .into_any_element(),
        };

        // The sidebar always has a heading. With nothing registered it says so
        // in one line instead of being omitted: the column is part of the
        // layout, and an empty column is a hole in the window.
        let list_section: AnyElement = match &self.recent {
            RecentState::Ready(list) if list.is_empty() => div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    text_style(div(), TypeScale::LABEL)
                        .text_color(theme.colors.text_muted())
                        .child("Acompanhados"),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_disabled())
                        .child("nenhum"),
                )
                .into_any_element(),
            RecentState::Ready(_) if recent.is_empty() => div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    text_style(div(), TypeScale::LABEL)
                        .text_color(theme.colors.text_muted())
                        .child("Acompanhados"),
                )
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(theme.colors.text_disabled())
                        .child("0"),
                )
                .into_any_element(),
            _ => div()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(
                            text_style(div(), TypeScale::LABEL)
                                .text_color(theme.colors.text_muted())
                                .child("Acompanhados"),
                        )
                        .child(
                            text_style(div(), TypeScale::META)
                                .text_color(theme.colors.text_disabled())
                                .child(match_count_label(recent.len(), tracked_total)),
                        ),
                )
                .child(list_area)
                .into_any_element(),
        };

        // A heading, not a tagline. It names the screen the way a window title
        // does and stops there — the sentence under it used to explain what the
        // app does, which the two screens already explain by being usable.
        let heading = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .child(
                text_style(div(), TypeScale::HEADING_1)
                    .text_color(theme.colors.text_primary())
                    .child("Novo projeto"),
            )
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_muted())
                    .child("Aponte para um diretório. O xemnas só registra o acompanhamento."),
            );

        div()
            .id("home-screen")
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .px(px(SpacingScale::S6))
            .child(
                // Top-aligned, not vertically centred: this is a work surface,
                // and a form that floats in the middle of a window moves under
                // the pointer as the window resizes.
                div()
                    .w(px(620.0))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S6))
                    .mt(px(SpacingScale::S10))
                    .child(heading)
                    .child(form)
                    .child(list_section),
            )
    }
}

/// Whether a project matches the search text.
///
/// Matches on name and location, case-insensitively. Substring rather than
/// prefix, because people search for a folder name they remember from the
/// middle of a path.
fn matches_query(summary: &ProjectSummary, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    let needle = query.trim().to_lowercase();
    summary.name().to_lowercase().contains(&needle)
        || summary
            .location()
            .to_string()
            .to_lowercase()
            .contains(&needle)
}

/// The "no match for your search" note.
///
/// One line, sitting where the rows would be, with no card of its own: it is a
/// note about a filter, not a panel.
fn no_matches(theme: &Theme, query: &str) -> Stateful<Div> {
    div()
        .id("home-no-matches")
        .px(px(SpacingScale::S3))
        .py(px(SpacingScale::S4))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .role(Role::Status)
        .aria_label(format!("Nenhum projeto corresponde a {query}"))
        .child(Icon::search(theme, 14.0, true))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child(format!("Nada corresponde a \"{query}\"")),
        )
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_disabled())
                .child("Esc limpa"),
        )
}

/// One registered project: its name, its path, and how to remove it.
///
/// The path is monospace and truncates from the left. Windows paths share a
/// long prefix, so the informative end is the tail; truncating the tail would
/// hide the folder the person is actually looking for.
fn recent_row(theme: &Theme, summary: &ProjectSummary) -> Stateful<Div> {
    let location = summary.location().to_string();
    div()
        .id((
            ElementId::from("home-row"),
            summary.id().as_str().to_string(),
        ))
        .px(px(SpacingScale::S3))
        .py(px(SpacingScale::S2))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .rounded(px(RadiusScale::CONTROL))
        .border_1()
        .border_color(theme.colors.glass_border_card())
        .bg(theme.colors.glass_fill_card())
        .role(Role::ListItem)
        .aria_label(location.clone())
        .child(Icon::folder(theme, 15.0, true))
        .child(
            text_style(div(), TypeScale::BODY)
                .text_color(theme.colors.text_primary())
                .child(summary.name().to_string()),
        )
        .child(
            div().flex_1().min_w(px(0.0)).child(
                text_style(div(), TypeScale::META)
                    .font_family(FONT_CODE)
                    .text_color(theme.colors.text_disabled())
                    .truncate()
                    .child(location),
            ),
        )
        .child(
            div()
                .id((
                    ElementId::from("home-remove"),
                    summary.id().as_str().to_string(),
                ))
                .size(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.0))
                .role(Role::Button)
                .aria_label(format!("Remover {}", summary.name()))
                .cursor_pointer()
                .child(Icon::trash(theme, 14.0)),
        )
}

/// Counts the path segments of a location.
///
/// Kept because the test covers both path separators. It is no longer rendered:
/// a count of path separators is not information a person can act on.
#[allow(dead_code)]
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
