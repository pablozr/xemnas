//! Desktop navigation between tracked projects and the candidate reading Inbox.

use application::decisions::{DecisionStore, Decisions};
use application::export::Export;
use application::inbox::{Inbox, InboxStore};
use application::projects::{ProjectRepository, Projects};
use gpui::prelude::*;
use gpui::{
    actions, div, px, App, Context, Entity, FocusHandle, Focusable, Render, Role, Subscription,
    Window, WindowControlArea,
};

use crate::fonts::{app_icon, wordmark};
use crate::screens::decisions::DecisionsScreen;
use crate::screens::inbox::InboxScreen;
use crate::screens::projects::{ProjectChanged, ProjectsScreen};
use crate::ui::feedback::error_state;
use crate::ui::glass::focus_ring;
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme, ThemeMode};
use crate::ui::tokens::{SpacingScale, TypeScale};

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
    in_decisions: bool,
    in_inbox: bool,
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
            if shell.in_inbox {
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
                if shell.in_inbox {
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
            in_decisions: false,
            in_inbox: true,
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
        if self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project())
            .is_none()
        {
            window.focus(&self.initial_focus(cx), cx);
            return;
        }
        if self.in_decisions {
            if let Some(screen) = &self.decisions {
                screen.update(cx, |screen, cx| screen.focus_search(window, cx));
            }
            return;
        }
        if !self.in_inbox {
            self.switch_destination(true, window, cx);
        }
        window.focus(&self.search.read(cx).focus_handle(cx), cx);
    }

    fn switch_destination(&mut self, inbox: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.in_inbox = inbox;
        self.in_decisions = false;
        window.focus(&self.destination_focus[usize::from(inbox)], cx);
        self.search.update(cx, |search, cx| {
            search.set_context(
                if inbox {
                    "Filtrar candidatos carregados"
                } else {
                    "Buscar projetos"
                },
                cx,
            )
        });
        if inbox {
            if let Some(screen) = &self.inbox {
                screen.update(cx, |screen, cx| screen.refresh(cx));
            }
        }
        cx.notify();
    }

    fn destination(&self, inbox: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let selected = !self.in_decisions && self.in_inbox == inbox;
        text_style(div(), TypeScale::BODY_SMALL)
            .id(if inbox { "nav-inbox" } else { "nav-projects" })
            .h(px(36.0))
            .px(px(SpacingScale::S3))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .border_b_2()
            .border_color(if selected {
                theme.colors.accent_subtle()
            } else {
                theme.colors.layer_fill()
            })
            .role(Role::Button)
            .aria_label(if inbox { "Revisão" } else { "Detalhes" })
            .aria_selected(selected)
            .track_focus(&self.destination_focus[usize::from(inbox)])
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .text_color(if selected {
                theme.colors.text_primary()
            } else {
                theme.colors.text_muted()
            })
            .bg(if selected {
                theme.colors.glass_surface_lavender()
            } else {
                theme.colors.layer_fill()
            })
            .hover(move |style| {
                style.bg(if selected {
                    theme.colors.glass_surface_lavender()
                } else {
                    theme.colors.hover_veil()
                })
            })
            .on_click(
                cx.listener(move |this, _, window, cx| this.switch_destination(inbox, window, cx)),
            )
            .on_key_down(
                cx.listener(move |this, event: &gpui::KeyDownEvent, window, cx| {
                    if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                        this.switch_destination(inbox, window, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .child(if inbox {
                crate::ui::icons::Icon::list(&theme, 14.0).into_any_element()
            } else {
                crate::ui::icons::Icon::layers(&theme, 14.0).into_any_element()
            })
            .child(if inbox { "Revisão" } else { "Detalhes" })
            .when(inbox, |tab| {
                tab.child(
                    text_style(div(), TypeScale::META)
                        .px(px(6.0))
                        .rounded(px(4.0))
                        .bg(theme.colors.glass_fill_medium())
                        .child(
                            self.inbox
                                .as_ref()
                                .and_then(|screen| screen.read(cx).total_count())
                                .map(|count| count.to_string())
                                .unwrap_or_else(|| "…".into()),
                        ),
                )
            })
    }

    fn decisions_destination(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let selected = self.in_decisions;
        text_style(div(), TypeScale::BODY_SMALL)
            .id("nav-decisions")
            .h(px(36.0))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(7.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(if selected {
                theme.colors.glass_edge_lavender()
            } else {
                theme.colors.canvas()
            })
            .bg(if selected {
                theme.colors.decision_selected()
            } else {
                theme.colors.layer_fill()
            })
            .text_color(if selected {
                theme.colors.text_primary()
            } else {
                theme.colors.text_muted()
            })
            .role(Role::Button)
            .aria_label("Decisões")
            .aria_selected(selected)
            .track_focus(&self.destination_focus[2])
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, window, cx| this.switch_decisions(window, cx)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.switch_decisions(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(crate::ui::icons::Icon::file(&theme, 14.0, !selected))
            .child("Decisões")
    }

    fn switch_decisions(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.in_decisions = true;
        self.in_inbox = false;
        window.focus(&self.destination_focus[2], cx);
        if let Some(screen) = &self.decisions {
            screen.update(cx, |screen, cx| screen.refresh(cx));
        }
        cx.notify();
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

    fn theme_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mode = cx.try_global::<ThemeMode>().copied().unwrap_or_default();
        text_style(div(), TypeScale::BODY_SMALL)
            .id("theme-switch")
            .h(px(30.0))
            .px(px(10.0))
            .mr(px(12.0))
            .flex()
            .items_center()
            .gap(px(7.0))
            .rounded(px(6.0))
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .bg(theme.colors.surface())
            .text_color(theme.colors.text_secondary())
            .hover(move |style| style.bg(theme.colors.surface_hover()))
            .role(Role::Button)
            .aria_label(format!(
                "Tema: {}. Alternar para {}",
                mode.label(),
                mode.toggled().label()
            ))
            .track_focus(&self.theme_focus)
            .focus_visible(focus_ring(&theme))
            .cursor_pointer()
            .on_click(cx.listener(Self::on_theme_click))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, window, cx| {
                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.toggle_theme(window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(crate::ui::icons::Icon::layers(&theme, 14.0))
            .child(format!("Tema · {}", mode.label()))
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
        window.set_window_title(if self.in_decisions {
            "xemnas — Decisões"
        } else if self.in_inbox {
            "xemnas — Revisão"
        } else {
            "xemnas — Projetos"
        });

        let title = div()
            .id("title-bar")
            .h(px(48.0))
            .pl(px(SpacingScale::S5))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .border_b_1()
            .border_color(theme.colors.hairline_divider())
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .window_control_area(WindowControlArea::Drag)
                    .child(app_icon(22.0))
                    .child(wordmark(&theme, 15.0, 600.0)),
            )
            .child(self.theme_button(cx))
            .child(window_controls(&theme, window.is_maximized()));

        let selected = self
            .projects
            .as_ref()
            .and_then(|screen| screen.read(cx).selected_project());
        let body: gpui::AnyElement = if let Some(projects) = self.projects.clone() {
            let sidebar = projects.update(cx, |screen, cx| screen.render_sidebar(cx));
            let content = if selected.is_some() && self.in_decisions {
                self.decisions
                    .as_ref()
                    .map(|screen| screen.clone().into_any_element())
                    .unwrap_or_else(|| div().into_any_element())
            } else if selected.is_some() && self.in_inbox {
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
                        .children(selected.as_ref().map(|project| {
                            div()
                                .px(px(SpacingScale::S5))
                                .pt(px(SpacingScale::S5))
                                .flex()
                                .flex_col()
                                .gap(px(SpacingScale::S2))
                                .border_b_1()
                                .border_color(theme.colors.hairline_divider())
                                .when(self.in_decisions, |header| {
                                    header.bg(theme.colors.decision_canvas())
                                })
                                .child(
                                    text_style(div(), TypeScale::HEADING_2)
                                        .truncate()
                                        .child(project.name().to_owned()),
                                )
                                .child(
                                    text_style(div(), TypeScale::BODY_SMALL)
                                        .truncate()
                                        .text_color(theme.colors.text_muted())
                                        .child(project.location().to_string()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(SpacingScale::S3))
                                        .child(self.destination(true, cx))
                                        .child(self.decisions_destination(cx))
                                        .child(self.destination(false, cx)),
                                )
                        }))
                        .child(div().flex_1().min_h(px(0.0)).child(content)),
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
            .child(
                div()
                    .h(px(28.0))
                    .px(px(SpacingScale::S6))
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(theme.colors.hairline_divider())
                    .when(self.demo, |footer| {
                        footer.child(
                            text_style(div(), TypeScale::META)
                                .text_color(theme.colors.text_muted())
                                .child("Demonstração · dados fictícios"),
                        )
                    })
                    .child(
                        text_style(div(), TypeScale::META)
                            .ml_auto()
                            .text_color(theme.colors.text_muted())
                            .child("Ctrl K · buscar"),
                    ),
            )
    }
}

/// Client-side Windows caption controls; GPUI forwards their hitboxes to the OS.
/// The system performs minimize, maximize/restore and close, including Snap Layouts.
fn window_controls(theme: &Theme, maximized: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_none()
        .h(px(48.0))
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
