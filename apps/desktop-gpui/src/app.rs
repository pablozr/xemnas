//! Application shell: the title bar and the content area that hosts a screen.
//!
//! The shell is the only GPUI view the composition root mounts in the window.
//! It is generic over the repository port so the concrete store is chosen in
//! `main.rs` and never named here (ARCH-001: the UI depends on the application
//! port, not on SQLite). If the database cannot be opened the shell paints the
//! documented error state instead of panicking (MVP-SPEC §14).

use application::projects::{ProjectRepository, Projects};
use gpui::prelude::*;
use gpui::{
    actions, div, px, Animation, AnimationExt, App, Context, Div, ElementId, Entity, FocusHandle,
    IntoElement, Render, Role, Stateful, Window,
};

use crate::fonts::{app_icon, wordmark};
use crate::screens::home::HomeScreen;
use crate::screens::projects::ProjectsScreen;
use crate::ui::feedback::error_state;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{MotionTokens, RadiusScale, SpacingScale, TypeScale};

actions!(
    xemnas,
    [
        /// Move focus to the next tab stop.
        TabNext,
        /// Move focus to the previous tab stop.
        TabPrev
    ]
);

/// The destinations the rail can reach.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Destination {
    /// The opening screen: what to do next.
    Home,
    /// Register, list and remove tracked directories.
    Projects,
}

/// What the shell shows in the content area.
enum Content<R: ProjectRepository + Send + 'static> {
    /// A screen over the shared `Projects` use case.
    Screen {
        /// The mounted screen, kept so the shell can read its navigation and
        /// focus requests without owning its own copy of the use case.
        entity: AnyScreen<R>,
    },
    /// The database could not be opened; only product copy is rendered.
    Failure,
}

/// A mounted screen the shell can drive without knowing its concrete type.
enum AnyScreen<R: ProjectRepository + Send + 'static> {
    /// The Home screen.
    Home(Entity<HomeScreen<R>>),
    /// The Projects screen.
    Projects(Entity<ProjectsScreen<R>>),
}

impl<R: ProjectRepository + Send + 'static> AnyScreen<R> {
    /// The focus handle this screen wants focused when it is mounted.
    fn initial_focus(&self, cx: &App) -> FocusHandle {
        match self {
            Self::Home(screen) => screen.read(cx).action_focus(),
            Self::Projects(screen) => screen.read(cx).field_focus(),
        }
    }
}

/// The xemnas window shell.
pub struct Shell<R: ProjectRepository + Send + 'static> {
    theme: Theme,
    focus: FocusHandle,
    content: Content<R>,
    /// Which rail item is active. The only copy: `Content::Screen` does not
    /// repeat it, so the rail and the breadcrumb can never disagree.
    destination: Destination,
}

impl<R: ProjectRepository + Send + 'static> Shell<R> {
    /// Builds the shell around a ready use case or a startup failure detail.
    ///
    /// The startup detail is logged (no file content, PRIV-001) and the window
    /// paints a product-language error state; the raw error never reaches the
    /// UI.
    pub fn new(cx: &mut Context<Self>, projects: Result<Projects<R>, String>) -> Self {
        // The app opens on Home: with no project registered, Home is the only
        // surface that can say what to do next. The use case moves into the
        // mounted screen, so the shell itself never holds it.
        let content = match projects {
            Ok(projects) => {
                let screen = cx.new(|cx| HomeScreen::new(cx, projects));
                screen.update(cx, |screen, cx| screen.start(cx));
                Content::Screen {
                    entity: AnyScreen::Home(screen),
                }
            }
            Err(detail) => {
                tracing::error!(
                    error = %detail,
                    operation = "open_database",
                    "could not open the projects database"
                );
                Content::Failure
            }
        };
        Self {
            theme: Theme::quiet_glass(),
            focus: cx.focus_handle(),
            content,
            destination: Destination::Home,
        }
    }

    /// The focus handle the window should focus when it opens.
    pub fn initial_focus(&self, cx: &App) -> FocusHandle {
        match &self.content {
            Content::Screen { entity } => entity.initial_focus(cx),
            Content::Failure => self.focus.clone(),
        }
    }

    /// Mounts `destination`, taking the use case back from the screen that
    /// currently holds it.
    ///
    /// The use case is `Send` but not `Sync` (a single SQLite connection), so it
    /// can only be in one place at a time. A screen that is mid-read keeps
    /// holding it and the navigation is skipped: the rail simply does not
    /// change, and the user can try again a moment later. That is preferable
    /// to tearing down a task mid-query.
    fn mount(&mut self, destination: Destination, cx: &mut Context<Self>) {
        if destination == self.destination {
            return;
        }
        let Some(projects) = self.take_use_case(cx) else {
            return;
        };
        let screen: AnyScreen<R> = match destination {
            Destination::Home => {
                let screen = cx.new(|cx| HomeScreen::new(cx, projects));
                screen.update(cx, |screen, cx| screen.start(cx));
                AnyScreen::Home(screen)
            }
            Destination::Projects => {
                let screen = cx.new(|cx| ProjectsScreen::new(cx, projects));
                screen.update(cx, |screen, cx| screen.start(cx));
                AnyScreen::Projects(screen)
            }
        };
        self.content = Content::Screen { entity: screen };
        self.destination = destination;
        cx.notify();
    }

    /// Reclaims the use case from the mounted screen, if it is idle.
    fn take_use_case(&mut self, cx: &mut Context<Self>) -> Option<Projects<R>> {
        let Content::Screen { entity } = &self.content else {
            return None;
        };
        match entity {
            AnyScreen::Home(screen) => screen.update(cx, |screen, _| screen.take_use_case()),
            AnyScreen::Projects(screen) => screen.update(cx, |screen, _| screen.take_use_case()),
        }
    }

    /// Consumes a screen's request to navigate, if it made one.
    fn handle_screen_requests(&mut self, cx: &mut Context<Self>) {
        let Content::Screen { entity } = &self.content else {
            return;
        };
        let wants_projects = match entity {
            AnyScreen::Home(screen) => {
                screen.update(cx, |screen, _| screen.take_projects_request())
            }
            AnyScreen::Projects(_) => false,
        };
        if wants_projects {
            self.mount(Destination::Projects, cx);
        }
    }

    /// Moves focus forward through the rendered tab stops.
    fn on_tab_next(&mut self, _: &TabNext, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_next(cx);
    }

    /// Moves focus backward through the rendered tab stops.
    fn on_tab_prev(&mut self, _: &TabPrev, window: &mut Window, cx: &mut Context<Self>) {
        window.focus_prev(cx);
    }
}

impl<R: ProjectRepository + Send + 'static> Render for Shell<R> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Home asks to navigate when its primary action fires; consuming it here
        // keeps navigation in the shell instead of inside a screen.
        self.handle_screen_requests(cx);
        let theme = self.theme;

        // The OS title bar is part of the chrome, so it must not keep claiming a
        // screen the user is no longer on. It used to be hardcoded to
        // "Projects" in `main.rs`, which disagreed with the in-app breadcrumb
        // the moment the user opened Home. This is the one place with a real
        // `Window` handle, so the title is corrected here rather than guessed.
        window.set_window_title(&format!("xemnas — {}", destination_label(self.destination)));

        // The title bar is a commanding surface: it carries the app wordmark and
        // sits on the Mica Alt backdrop, so it needs no background of its own —
        // the material is the background. Only a 1 px hairline separates it from
        // the content below.
        // The title bar carries the identity on the trailing side of the left
        // group and the search affordance pinned right. The wordmark uses its
        // own face (`fonts::wordmark`) so the product has a voice distinct from
        // the interface it sits on.
        let title_bar = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .h(px(48.0))
            .px(px(SpacingScale::S6))
            .border_b_1()
            .border_color(theme.colors.hairline_divider())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S3))
                    .child(app_icon(20.0))
                    .child(wordmark(&theme, 15.0, 700.0))
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child("—"),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_secondary())
                            .child(destination_label(self.destination)),
                    ),
            )
            // `ml_auto` pins search to the trailing edge instead of letting it
            // sit next to the wordmark. The margin lives on the element, not on
            // a spacer sibling: a sibling would also absorb the free space but
            // would let the field keep its own width from collapsing.
            .child(search_affordance(&theme).ml_auto());

        let body: gpui::AnyElement = match &self.content {
            Content::Screen { entity } => match entity {
                AnyScreen::Home(screen) => screen.clone().into_any_element(),
                AnyScreen::Projects(screen) => screen.clone().into_any_element(),
            },
            Content::Failure => error_state(
                &theme,
                "startup-error",
                "Não foi possível abrir o banco de dados",
                "Os projetos acompanhados não puderam ser carregados.",
                "Feche e abra o app novamente. Se o erro persistir, verifique o espaço em disco.",
            )
            .into_any_element(),
        };

        // The rail carries the two real destinations. The active item is a
        // `Glass Selected` well so it reads as recessed into the material
        // rather than painted on top of it, and its settle is animated on
        // `motion.fast` so the surface comes alive under the pointer without a
        // decorative pulse.
        //
        // The animation wraps the item rather than the row it is built from:
        // `with_animation` requires `IntoElement`, which `Stateful<Div>` (the
        // result of `.id()`) does not implement — only the plain `Div` does
        // (gpui/src/elements/div.rs:2091). Building the `Div` first and
        // animating it is the ordering that compiles.
        let rail_item = |label: &'static str, active: bool| {
            let base = div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(SpacingScale::S1))
                .w(px(64.0))
                .h(px(56.0))
                .rounded(px(RadiusScale::SURFACE))
                .border_1()
                .child(
                    text_style(div(), TypeScale::LABEL)
                        .text_color(if active {
                            theme.colors.text_primary()
                        } else {
                            theme.colors.text_secondary()
                        })
                        .child(label),
                );
            // Only the active well carries a fill; an inactive item stays flat
            // so the rail reads as one destination lit, not two competing.
            if active {
                base.bg(theme.colors.glass_fill_medium())
                    .border_color(theme.colors.glass_border())
                    .with_animation(
                        "rail-active-anim",
                        Animation::new(MotionTokens::FAST)
                            .with_easing(MotionTokens::enter_easing()),
                        |this, progress| this.opacity(0.74 + 0.26 * progress),
                    )
            } else {
                base.border_color(theme.colors.hairline_divider())
                    .with_animation(
                        "rail-idle-anim",
                        Animation::new(MotionTokens::FAST)
                            .with_easing(MotionTokens::enter_easing()),
                        |this, progress| this.opacity(0.62 + 0.38 * progress),
                    )
            }
        };

        // Each item is wrapped so it keeps its `Role::Button` and accessible
        // name while still being animated.
        let rail_button = |label: &'static str, destination: Destination, active: bool| {
            div()
                .id((ElementId::from("rail-item"), label))
                .role(Role::Button)
                .aria_label(label)
                .cursor_pointer()
                .on_click(cx.listener(move |shell, _, _, cx| {
                    shell.mount(destination, cx);
                }))
                .child(rail_item(label, active))
        };

        // The rail is a fixed 88 px column that fills the window height.
        //
        // No `flex_1()` here. This rail used to end with `.flex_1()`, which
        // sets `flex_grow: 1` and `flex_basis: auto` and therefore overrides
        // both the `w(88px)` and the `flex_basis(px(88))` above it: the column
        // measured 687 px and the item floated in the middle of it. The row
        // already gives the rail its full height, so the rail itself only needs
        // a fixed width.
        let rail = div()
            .id("navigation-rail")
            .flex()
            .flex_col()
            .items_center()
            .gap(px(SpacingScale::S2))
            .w(px(88.0))
            .h_full()
            .flex_shrink_0()
            .py(px(SpacingScale::S4))
            .bg(theme.colors.rail())
            .border_r_1()
            .border_color(theme.colors.hairline_divider())
            .child(app_icon(28.0))
            .child(rail_button(
                "Home",
                Destination::Home,
                self.destination == Destination::Home,
            ))
            .child(rail_button(
                "Projects",
                Destination::Projects,
                self.destination == Destination::Projects,
            ));

        // The status bar is dense by design: it is the only place the window
        // reports runtime state, and the design system asks for metadata to
        // stay at `type.meta` so it never competes with the content above.
        let status_bar = div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .h(px(28.0))
            .px(px(SpacingScale::S6))
            .border_t_1()
            .border_color(theme.colors.hairline_divider())
            .child(
                div()
                    .size(px(6.0))
                    .rounded_full()
                    .bg(theme.colors.status_success()),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child("Captura local ativa"),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_disabled())
                    .child("·"),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child("0 projetos"),
            )
            .child(
                // Right-aligned keyboard affordance; `ml_auto` pins it to the
                // trailing edge instead of leaving it next to the status text.
                div()
                    .ml_auto()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_disabled())
                            .child("Tab"),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child("navegar"),
                    ),
            );

        div()
            .id("xemnas-shell")
            .role(Role::Application)
            .aria_label("xemnas")
            .size_full()
            .flex()
            .flex_col()
            // `layer.fill`, not the opaque canvas: this is the content layer of
            // the Mica Alt backdrop declared in `main.rs`. Translucent rather
            // than solid is what lets the user's wallpaper tint the window
            // without costing the text its contrast — the same
            // `LayerFillColorDefaultBrush` rule the Fluent material spec sets.
            .bg(theme.colors.layer_fill())
            .font_family(Theme::FONT_INTERFACE)
            .text_color(theme.colors.text_primary())
            .track_focus(&self.focus)
            .key_context("xemnas")
            .on_action(cx.listener(Self::on_tab_next))
            .on_action(cx.listener(Self::on_tab_prev))
            .child(title_bar)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .overflow_hidden()
                    .child(
                        // The row that holds the rail and the content column.
                        //
                        // It must not be `size_full()`: that sets a `100%` flex
                        // basis, so both children were laid out against the full
                        // window width and the rail's `w(88px)` was ignored —
                        // measured 392 px with the item never laid out. This row
                        // simply fills the space the parent already gave it, and
                        // the rail's explicit basis is then honoured.
                        div()
                            .flex()
                            .flex_1()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .overflow_hidden()
                            .child(rail)
                            .child(
                                div()
                                    .flex_1()
                                    .min_w(px(0.0))
                                    .min_h(px(0.0))
                                    .overflow_hidden()
                                    .p(px(SpacingScale::S8))
                                    .child(body),
                            ),
                    ),
            )
            .child(status_bar)
    }
}

/// The rail/breadcrumb label for a destination.
fn destination_label(destination: Destination) -> &'static str {
    match destination {
        Destination::Home => "Home",
        Destination::Projects => "Projects",
    }
}

/// The trailing search affordance in the title bar.
///
/// It is a real `Glass Low` well with a placeholder and the `Ctrl K` hint, not
/// a label: the field exists so the affordance is discoverable without a
/// tutorial, which is the same reason the reference apps put a persistent
/// search box in the chrome. Command execution is not part of this milestone, so
/// the field is presentational and carries no editable text yet.
fn search_affordance(theme: &Theme) -> Stateful<Div> {
    div()
        .id("title-search")
        .h(px(28.0))
        .w(px(240.0))
        .px(px(SpacingScale::S3))
        .flex()
        .items_center()
        .justify_between()
        .gap(px(SpacingScale::S2))
        .rounded(px(RadiusScale::CONTROL))
        .bg(theme.colors.glass_fill_low())
        .border_1()
        .border_color(theme.colors.hairline_divider())
        .role(Role::TextInput)
        .aria_label("Buscar")
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child("Buscar"),
        )
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_disabled())
                .child("Ctrl K"),
        )
}
