//! Desktop navigation between tracked projects and the candidate reading Inbox.

use application::decisions::{DecisionStore, Decisions};
use application::export::Export;
use application::inbox::{Inbox, InboxStore};
use application::projects::{ProjectRepository, Projects};
use gpui::prelude::*;
use gpui::{
    actions, div, px, App, Context, ElementId, Entity, FocusHandle, Focusable, Render, Role,
    Subscription, Window, WindowControlArea,
};

use crate::fonts::{app_icon, wordmark};
use crate::screens::decisions::DecisionsScreen;
use crate::screens::inbox::InboxScreen;
use crate::screens::projects::{ProjectChanged, ProjectsScreen};
use crate::screens::settings::{AiBackend, CloseSettings, SettingsScreen};
use crate::ui::controls::icon_action;
use crate::ui::feedback::error_state;
use crate::ui::glass::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{count_chip, fade_in};
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme, ThemeMode};
use crate::ui::tokens::{ControlSize, SpacingScale, TypeScale};

actions!(
    xemnas,
    [
        /// Focuses the next control.
        TabNext,
        /// Focuses the previous control.
        TabPrev,
        /// Focuses the current destination's search field.
        FocusSearch
    ]
);

/// Height of the custom title bar (brand, theme and caption controls).
const TITLE_BAR_HEIGHT: f32 = 40.0;
/// Height of the project bar that carries the breadcrumb and destinations.
const PROJECT_BAR_HEIGHT: f32 = 44.0;

/// The three places a selected project can be read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Destination {
    Review,
    Decisions,
    Details,
}

impl Destination {
    fn index(self) -> usize {
        match self {
            Self::Details => 0,
            Self::Review => 1,
            Self::Decisions => 2,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Review => "nav-inbox",
            Self::Decisions => "nav-decisions",
            Self::Details => "nav-projects",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Review => "Revisão",
            Self::Decisions => "Decisões",
            Self::Details => "Detalhes",
        }
    }
}

/// Persistent product screens with contextual search and native window controls.
pub struct Shell<R: ProjectRepository + InboxStore + DecisionStore + Send + 'static> {
    theme: Theme,
    focus: FocusHandle,
    search: Entity<SearchField>,
    _search_subscription: Subscription,
    _project_subscription: Option<Subscription>,
    _inbox_subscription: Option<Subscription>,
    projects: Option<Entity<ProjectsScreen<R>>>,
    inbox: Option<Entity<InboxScreen<R>>>,
    decisions: Option<Entity<DecisionsScreen<R>>>,
    settings: Option<Entity<SettingsScreen>>,
    _settings_subscription: Option<Subscription>,
    settings_open: bool,
    settings_focus: FocusHandle,
    destination: Destination,
    destination_focus: [FocusHandle; 3],
    theme_focus: FocusHandle,
    demo: bool,
}

impl<R: ProjectRepository + InboxStore + DecisionStore + Send + 'static> Shell<R> {
    /// Mounts both use cases once, retaining their state across navigation.
    pub fn new(
        cx: &mut Context<Self>,
        projects: Result<Projects<R>, String>,
        inbox: Option<Inbox<R>>,
        decisions: Option<(Decisions<R>, Export<R>)>,
        ai: Option<Box<dyn AiBackend>>,
    ) -> Self {
        let screen = match projects {
            Ok(projects) => {
                let screen = cx.new(|cx| ProjectsScreen::new(cx, projects));
                screen.update(cx, |screen, cx| screen.start(cx, ""));
                Some(screen)
            }
            Err(detail) => {
                tracing::error!(error = %detail, operation = "open_database", "could not open projects database");
                None
            }
        };
        let search = cx.new(SearchField::new);
        search.update(cx, |search, cx| {
            search.set_context("Filtrar candidatos carregados", cx)
        });
        let search_subscription = cx.subscribe(&search, |shell, _, event: &SearchChanged, cx| {
            if shell.destination == Destination::Review {
                if let Some(screen) = &shell.inbox {
                    screen.update(cx, |screen, cx| screen.set_query(&event.0, cx));
                }
            }
        });
        let project_subscription = screen.as_ref().map(|screen| {
            cx.subscribe(screen, |shell, _, event: &ProjectChanged, cx| {
                if let Some(inbox) = &shell.inbox {
                    inbox.update(cx, |inbox, cx| {
                        inbox.set_project(
                            event
                                .0
                                .as_ref()
                                .map(|project| project.id().as_str().to_owned()),
                            cx,
                        );
                    });
                }
                if let Some(decisions) = &shell.decisions {
                    decisions.update(cx, |screen, cx| {
                        screen.set_project(
                            event
                                .0
                                .as_ref()
                                .map(|project| project.id().as_str().to_owned()),
                            cx,
                        )
                    });
                }
                if shell.destination == Destination::Review {
                    shell.search.update(cx, |search, cx| {
                        search.set_context("Filtrar candidatos carregados", cx)
                    });
                }
                cx.notify();
            })
        });
        let inbox = inbox.map(|inbox| {
            cx.new(|cx| {
                let mut screen = InboxScreen::new(cx, inbox);
                screen.attach_search(search.clone());
                screen
            })
        });
        let inbox_subscription = inbox
            .as_ref()
            .map(|screen| cx.observe(screen, |_, _, cx| cx.notify()));
        let decisions = decisions
            .map(|(decisions, export)| cx.new(|cx| DecisionsScreen::new(cx, decisions, export)));
        let settings = ai.map(|backend| cx.new(|cx| SettingsScreen::new(cx, backend)));
        let settings_subscription = settings.as_ref().map(|screen| {
            cx.subscribe(screen, |shell, _, _: &CloseSettings, cx| {
                shell.settings_open = false;
                cx.notify();
            })
        });
        Self {
            theme: Theme::quiet_glass(),
            focus: cx.focus_handle(),
            search: search.clone(),
            _search_subscription: search_subscription,
            _project_subscription: project_subscription,
            _inbox_subscription: inbox_subscription,
            projects: screen,
            inbox,
            decisions,
            settings,
            _settings_subscription: settings_subscription,
            settings_open: false,
            settings_focus: cx.focus_handle().tab_stop(true),
            destination: Destination::Review,
            destination_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
            theme_focus: cx.focus_handle().tab_stop(true),
            demo: false,
        }
    }

    /// Identifies opt-in sample data visibly, without changing navigation.
    pub fn set_demo(&mut self, demo: bool) {
        self.demo = demo;
    }

    /// Focuses the actual next action rather than an inert path field.
    pub fn initial_focus(&self, cx: &App) -> FocusHandle {
        self.projects
            .as_ref()
            .map(|screen| screen.read(cx).initial_focus())
            .unwrap_or_else(|| self.focus.clone())
    }

    fn on_tab_next(&mut self, _: &TabNext, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
    }

    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
    }

    fn on_focus_search(&mut self, _: &FocusSearch, window: &mut Window, cx: &mut Context<Self>) {
        if self.settings_open {
            return;
        }
        if self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project())
            .is_none()
        {
            window.focus(&self.initial_focus(cx), cx);
            return;
        }
        if self.destination == Destination::Decisions {
            if let Some(screen) = &self.decisions {
                screen.update(cx, |screen, cx| screen.focus_search(window, cx));
            }
            return;
        }
        if self.destination != Destination::Review {
            self.switch_to(Destination::Review, window, cx);
        }
        window.focus(&self.search.read(cx).focus_handle(cx), cx);
    }

    fn switch_to(&mut self, destination: Destination, window: &mut Window, cx: &mut Context<Self>) {
        self.destination = destination;
        window.focus(&self.destination_focus[destination.index()], cx);
        match destination {
            Destination::Review => {
                self.search.update(cx, |search, cx| {
                    search.set_context("Filtrar candidatos carregados", cx)
                });
                if let Some(screen) = &self.inbox {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
            Destination::Decisions => {
                if let Some(screen) = &self.decisions {
                    screen.update(cx, |screen, cx| screen.refresh(cx));
                }
            }
            Destination::Details => self
                .search
                .update(cx, |search, cx| search.set_context("Buscar projetos", cx)),
        }
        cx.notify();
    }

    /// One tab recipe for every project destination: quiet at rest, a soft
    /// filled pill when selected, no underline or outline competing with it.
    fn nav_tab(&self, destination: Destination, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let selected = self.destination == destination;
        let foreground = if selected {
            theme.colors.text_primary()
        } else {
            theme.colors.text_muted()
        };
        let glyph = icon(
            match destination {
                Destination::Review => IconName::List,
                Destination::Decisions => IconName::File,
                Destination::Details => IconName::Info,
            },
            14.0,
            foreground,
        );
        let count = (destination == Destination::Review).then(|| {
            self.inbox
                .as_ref()
                .and_then(|screen| screen.read(cx).total_count())
                .map(|count| count.to_string())
                .unwrap_or_else(|| "…".into())
        });
        text_style(div(), TypeScale::BODY_SMALL)
            .id(destination.id())
            .h(px(ControlSize::SM))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .rounded(theme.radius.control())
            .role(Role::Tab)
            .aria_label(destination.label())
            .aria_selected(selected)
            .track_focus(&self.destination_focus[destination.index()])
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .text_color(if selected {
                theme.colors.text_primary()
            } else {
                theme.colors.text_muted()
            })
            .when(selected, |tab| tab.bg(theme.colors.selection()))
            .when(!selected, |tab| {
                tab.hover(move |style| {
                    style
                        .bg(theme.colors.hover_veil())
                        .text_color(theme.colors.text_secondary())
                })
            })
            .on_click(
                cx.listener(move |this, _, window, cx| this.switch_to(destination, window, cx)),
            )
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.switch_to(destination, window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .child(glyph)
            .child(destination.label())
            .children(count.map(|count| count_chip(&theme, count)))
    }

    fn toggle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mode = cx
            .try_global::<ThemeMode>()
            .copied()
            .unwrap_or_default()
            .toggled();
        cx.set_global(mode);
        window.refresh();
        cx.notify();
    }

    /// A quiet icon action in the title bar; the palette name lives in the
    /// accessible label instead of a permanent text chip beside the brand.
    fn theme_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mode = cx.try_global::<ThemeMode>().copied().unwrap_or_default();
        let label = format!(
            "Tema: {}. Alternar para {}",
            mode.label(),
            mode.toggled().label()
        );
        icon_action(&theme, "theme-switch", &label)
            .mr(px(SpacingScale::S2))
            .track_focus(&self.theme_focus)
            .on_click(cx.listener(Self::on_theme_click))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_theme(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(icon(
                IconName::Contrast,
                16.0,
                theme.colors.text_secondary(),
            ))
    }

    /// Opens the app-level settings page, or returns to the projects.
    fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(screen) = &self.settings else {
            return;
        };
        self.settings_open = !self.settings_open;
        if self.settings_open {
            screen.update(cx, |screen, cx| screen.open(cx));
        } else {
            window.focus(&self.initial_focus(cx), cx);
        }
        cx.notify();
    }

    fn settings_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let open = self.settings_open;
        icon_action(
            &theme,
            "settings-open",
            if open {
                "Fechar configurações"
            } else {
                "Configurações"
            },
        )
        .mr(px(SpacingScale::S1))
        .track_focus(&self.settings_focus)
        .aria_selected(open)
        .when(open, |button| button.bg(theme.colors.selection()))
        .on_click(cx.listener(|this, _, window, cx| this.toggle_settings(window, cx)))
        .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                this.toggle_settings(window, cx);
                cx.stop_propagation();
            }
        }))
        .child(icon(
            IconName::Settings,
            16.0,
            if open {
                theme.colors.text_primary()
            } else {
                theme.colors.text_secondary()
            },
        ))
    }

    fn on_theme_click(
        &mut self,
        _: &gpui::ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_theme(window, cx);
    }
}

impl<R: ProjectRepository + InboxStore + DecisionStore + Send + 'static> Render for Shell<R> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme = Theme::current(cx);
        let theme = self.theme;
        window.set_window_title(match self.destination {
            _ if self.settings_open => "xemnas — Configurações",
            Destination::Decisions => "xemnas — Decisões",
            Destination::Review => "xemnas — Revisão",
            Destination::Details => "xemnas — Projetos",
        });

        let title = div()
            .id("title-bar")
            .h(px(TITLE_BAR_HEIGHT))
            .flex_none()
            .pl(px(SpacingScale::S4))
            .flex()
            .items_center()
            .bg(theme.colors.rail())
            .border_b_1()
            .border_color(theme.colors.hairline_divider())
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .window_control_area(WindowControlArea::Drag)
                    .child(app_icon(18.0))
                    .child(wordmark(&theme, 14.0, 600.0))
                    .when(self.demo, |brand| {
                        brand.child(
                            text_style(div(), TypeScale::META)
                                .ml(px(SpacingScale::S2))
                                .px(px(SpacingScale::S2))
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(theme.colors.hairline_divider())
                                .text_color(theme.colors.text_muted())
                                .child("Demonstração · dados fictícios"),
                        )
                    }),
            )
            .when(self.settings.is_some(), |title| {
                title.child(self.settings_button(cx))
            })
            .child(self.theme_button(cx))
            .child(window_controls(&theme, window.is_maximized()));

        let selected = self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project());
        let settings = self.settings.clone().filter(|_| self.settings_open);
        let body: gpui::AnyElement = if let Some(settings) = settings {
            fade_in(
                div().size_full().child(settings),
                ElementId::Name("content-settings".into()),
            )
            .into_any_element()
        } else if let Some(projects) = self.projects.clone() {
            let sidebar = projects.update(cx, |screen, cx| screen.render_sidebar(cx));
            let content = if selected.is_some() && self.destination == Destination::Decisions {
                self.decisions
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else if selected.is_some() && self.destination == Destination::Review {
                self.inbox
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else {
                projects.update(cx, |screen, cx| screen.render_details(cx))
            };
            div()
                .size_full()
                .flex()
                .child(sidebar)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .h_full()
                        .flex()
                        .flex_col()
                        // One bar: the project as breadcrumb, then its destinations.
                        // The path already lives in the sidebar row and in Detalhes.
                        .children(selected.as_ref().map(|project| {
                            div()
                                .h(px(PROJECT_BAR_HEIGHT))
                                .flex_none()
                                .px(px(SpacingScale::S4))
                                .flex()
                                .items_center()
                                .gap(px(SpacingScale::S2))
                                .border_b_1()
                                .border_color(theme.colors.hairline_divider())
                                .child(
                                    text_style(div(), TypeScale::ROW_TITLE)
                                        .max_w(px(280.0))
                                        .truncate()
                                        .child(project.name().to_owned()),
                                )
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .text_color(theme.colors.text_disabled())
                                        .child("/"),
                                )
                                .child(
                                    div()
                                        .id("project-destinations")
                                        .flex()
                                        .gap(px(SpacingScale::S1))
                                        .role(Role::TabList)
                                        .child(self.nav_tab(Destination::Review, cx))
                                        .child(self.nav_tab(Destination::Decisions, cx))
                                        .child(self.nav_tab(Destination::Details, cx)),
                                )
                        }))
                        .child(fade_in(
                            div().flex_1().min_h(px(0.0)).child(content),
                            ElementId::Name(
                                format!(
                                    "content-{:?}-{}",
                                    self.destination,
                                    selected
                                        .as_ref()
                                        .map(|project| project.id().as_str())
                                        .unwrap_or("none")
                                )
                                .into(),
                            ),
                        )),
                )
                .into_any_element()
        } else {
            error_state(
                &theme,
                "startup-error",
                "Não foi possível abrir o banco de dados",
                "Os projetos acompanhados não puderam ser carregados.",
                "Feche e abra o app novamente. Se o erro persistir, verifique o espaço em disco.",
            )
            .into_any_element()
        };

        div()
            .id("xemnas-shell")
            .role(Role::Application)
            .aria_label("xemnas")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.canvas())
            .font_family(Theme::FONT_INTERFACE)
            .text_color(theme.colors.text_primary())
            .track_focus(&self.focus)
            .key_context("xemnas")
            .on_action(cx.listener(Self::on_tab_next))
            .on_action(cx.listener(Self::on_tab_prev))
            .on_action(cx.listener(Self::on_focus_search))
            .child(title)
            .child(div().flex_1().min_h(px(0.0)).overflow_hidden().child(body))
    }
}

/// Client-side Windows caption controls; GPUI forwards their hitboxes to the OS.
/// The system performs minimize, maximize/restore and close, including Snap Layouts.
fn window_controls(theme: &Theme, maximized: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_none()
        .h_full()
        .font_family("Segoe Fluent Icons")
        .child(caption_button(
            theme,
            "window-minimize",
            "Minimizar",
            "\u{e921}",
            WindowControlArea::Min,
            false,
        ))
        .child(caption_button(
            theme,
            "window-maximize",
            if maximized { "Restaurar" } else { "Maximizar" },
            if maximized { "\u{e923}" } else { "\u{e922}" },
            WindowControlArea::Max,
            false,
        ))
        .child(caption_button(
            theme,
            "window-close",
            "Fechar",
            "\u{e8bb}",
            WindowControlArea::Close,
            true,
        ))
}

fn caption_button(
    theme: &Theme,
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    area: WindowControlArea,
    destructive: bool,
) -> impl IntoElement {
    let hover = if destructive {
        theme.colors.status_danger()
    } else {
        theme.colors.hover_veil()
    };
    div()
        .id(id)
        .w(px(46.0))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.0))
        .text_color(theme.colors.text_secondary())
        .hover(move |style| style.bg(hover).text_color(theme.colors.text_primary()))
        .window_control_area(area)
        .role(Role::Button)
        .aria_label(label)
        .child(glyph)
}
