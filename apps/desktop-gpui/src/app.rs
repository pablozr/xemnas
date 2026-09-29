//! Desktop navigation between tracked projects and the candidate reading Inbox.

use application::inbox::{Inbox, InboxStore};
use application::projects::{ProjectRepository, Projects};
use gpui::prelude::*;
use gpui::{
    actions, div, px, App, Context, Entity, FocusHandle, Focusable, Render, Role, Subscription,
    Window, WindowControlArea,
};

use crate::fonts::{app_icon, wordmark};
use crate::screens::inbox::InboxScreen;
use crate::screens::projects::ProjectsScreen;
use crate::ui::feedback::error_state;
use crate::ui::glass::focus_ring;
use crate::ui::search_field::{SearchChanged, SearchField};
use crate::ui::theme::{text_style, Theme};
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
pub struct Shell<R: ProjectRepository + InboxStore + Send + 'static> {
    theme: Theme,
    focus: FocusHandle,
    search: Entity<SearchField>,
    _search_subscription: Subscription,
    projects: Option<Entity<ProjectsScreen<R>>>,
    inbox: Option<Entity<InboxScreen<R>>>,
    in_inbox: bool,
    destination_focus: [FocusHandle; 2],
}

impl<R: ProjectRepository + InboxStore + Send + 'static> Shell<R> {
    /// Mounts both use cases once, retaining their state across navigation.
    pub fn new(
        cx: &mut Context<Self>,
        projects: Result<Projects<R>, String>,
        inbox: Option<Inbox<R>>,
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
        let search_subscription = cx.subscribe(&search, |shell, _, event: &SearchChanged, cx| {
            if shell.in_inbox {
                if let Some(screen) = &shell.inbox {
                    screen.update(cx, |screen, cx| screen.set_query(&event.0, cx));
                }
            } else if let Some(screen) = &shell.projects {
                screen.update(cx, |screen, cx| screen.set_query(&event.0, cx));
            }
        });
        Self {
            theme: Theme::quiet_glass(),
            focus: cx.focus_handle(),
            search,
            _search_subscription: search_subscription,
            projects: screen,
            inbox: inbox.map(|inbox| cx.new(|cx| InboxScreen::new(cx, inbox))),
            in_inbox: false,
            destination_focus: [
                cx.focus_handle().tab_stop(true),
                cx.focus_handle().tab_stop(true),
            ],
        }
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
        window.focus(&self.search.read(cx).focus_handle(cx), cx);
    }

    fn switch_destination(&mut self, inbox: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.in_inbox = inbox;
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
        let selected = self.in_inbox == inbox;
        text_style(div(), TypeScale::BODY_SMALL)
            .id(if inbox { "nav-inbox" } else { "nav-projects" })
            .h(px(36.0))
            .px(px(SpacingScale::S3))
            .flex()
            .items_center()
            .rounded(theme.radius.control())
            .role(Role::Button)
            .aria_label(if inbox { "Inbox" } else { "Projetos" })
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
            .child(if inbox { "Inbox" } else { "Projetos" })
    }
}

impl<R: ProjectRepository + InboxStore + Send + 'static> Render for Shell<R> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        window.set_window_title(if self.in_inbox {
            "xemnas — Inbox"
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
            .child(self.destination(false, cx))
            .child(self.destination(true, cx))
            .child(div().ml_auto().child(self.search.clone()))
            .child(window_controls(&theme, window.is_maximized()));

        let mounted = if self.in_inbox {
            self.inbox
                .as_ref()
                .map(|screen| screen.clone().into_any_element())
        } else {
            self.projects
                .as_ref()
                .map(|screen| screen.clone().into_any_element())
        };
        let body: gpui::AnyElement = match mounted {
            Some(screen) => screen,
            None => error_state(
                &theme,
                "startup-error",
                "Não foi possível abrir o banco de dados",
                "Os projetos acompanhados não puderam ser carregados.",
                "Feche e abra o app novamente. Se o erro persistir, verifique o espaço em disco.",
            )
            .into_any_element(),
        };

        div()
            .id("xemnas-shell")
            .role(Role::Application)
            .aria_label("xemnas")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.layer_fill())
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
