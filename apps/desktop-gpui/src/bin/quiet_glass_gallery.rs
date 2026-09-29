//! Quiet Glass gallery.
//!
//! Evidence binary that renders the reusable primitives in every documented
//! state so `tools/capture-quiet-glass.ps1` can screenshot them at 1440×1024.
//! It is intentionally separate from the product binary (`xemnas`).

use gpui::prelude::*;
use gpui::{
    actions, div, px, size, App, Bounds, Context, Div, FocusHandle, IntoElement, KeyBinding,
    Render, Role, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_platform::application;

use xemnas_desktop::ui::controls::{
    icon_button, primary_button, quiet_button, search_field, ControlState,
};
use xemnas_desktop::ui::feedback::{empty_state, error_state, status_dot, StatusKind};
use xemnas_desktop::ui::glass::{GlassSurface, GlassVariant};
use xemnas_desktop::ui::theme::{text_style, Theme};
use xemnas_desktop::ui::tokens::{SpacingScale, TypeScale};

actions!(quiet_glass_gallery, [TabNext]);

const STATES: [(&str, ControlState); 5] = [
    ("normal", ControlState::Rest),
    ("hover", ControlState::Hover),
    ("foco", ControlState::FocusVisible),
    ("disabled", ControlState::Disabled),
    ("loading", ControlState::Loading),
];

struct Gallery {
    theme: Theme,
    focus: FocusHandle,
    /// Real tab stops, so a keyboard Tab paints the focus-visible ring on a
    /// live control (the "foco" column shows the same treatment statically).
    tab_stops: Vec<FocusHandle>,
    /// Next tab stop to focus; advanced by [`Gallery::on_tab_next`].
    tab_index: usize,
}

impl Gallery {
    fn new(cx: &mut Context<Self>, theme: Theme) -> Self {
        let tab_stops = (0..3).map(|_| cx.focus_handle().tab_stop(true)).collect();
        Self {
            theme,
            focus: cx.focus_handle(),
            tab_stops,
            tab_index: 0,
        }
    }

    fn on_tab_next(&mut self, _: &TabNext, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab_stops.is_empty() {
            return;
        }
        let handle = self.tab_stops[self.tab_index].clone();
        self.tab_index = (self.tab_index + 1) % self.tab_stops.len();
        window.focus(&handle, cx);
        cx.notify();
    }

    fn render_controls(&self) -> Vec<Div> {
        let theme = &self.theme;
        let mut cols = Vec::new();
        for (index, (caption, state)) in STATES.iter().enumerate() {
            let state = *state;
            let mut primary = primary_button(
                theme,
                ("gallery-primary", index),
                state,
                "Confirmar decisão",
            );
            let mut adjust = quiet_button(theme, ("gallery-quiet", index), state, "Ajustar", false);
            let mut copy = icon_button(theme, ("gallery-icon", index), state, "Copiar");
            if index == 0 {
                primary = primary.track_focus(&self.tab_stops[0]);
                adjust = adjust.track_focus(&self.tab_stops[1]);
                copy = copy.track_focus(&self.tab_stops[2]);
            }
            cols.push(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(caption_label(theme, caption))
                    .child(primary)
                    .child(adjust)
                    .child(quiet_button(
                        theme,
                        ("gallery-danger", index),
                        state,
                        "Rejeitar",
                        true,
                    ))
                    .child(copy),
            );
        }
        cols
    }
}

fn caption_label(theme: &Theme, caption: &str) -> Div {
    text_style(div(), TypeScale::META)
        .text_color(theme.colors.text_muted())
        .child(caption.to_string())
}

fn section(theme: &Theme, title: &str, content: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S3))
        .child(
            text_style(div(), TypeScale::HEADING_3)
                .text_color(theme.colors.text_secondary())
                .child(title.to_string()),
        )
        .child(content)
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;

        let button_rows = div()
            .flex()
            .gap(px(SpacingScale::S8))
            .items_start()
            .children(self.render_controls());

        let search_row = div()
            .flex()
            .gap(px(SpacingScale::S4))
            .items_end()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(caption_label(&theme, "normal"))
                    .child(search_field(
                        &theme,
                        "gallery-search-rest",
                        ControlState::Rest,
                        "Buscar decisões",
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(caption_label(&theme, "foco"))
                    .child(search_field(
                        &theme,
                        "gallery-search-focus",
                        ControlState::FocusVisible,
                        "Buscar decisões",
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .child(caption_label(&theme, "disabled"))
                    .child(search_field(
                        &theme,
                        "gallery-search-disabled",
                        ControlState::Disabled,
                        "Buscar decisões",
                    )),
            );

        let glass_row = div()
            .flex()
            .gap(px(SpacingScale::S5))
            .items_end()
            .child(glass_card(&theme, GlassVariant::Low, "Glass Low"))
            .child(glass_card(&theme, GlassVariant::Selected, "Glass Selected"))
            .child(glass_card(&theme, GlassVariant::Emphasis, "Glass Emphasis"))
            .child(glass_card_solid(&theme, "Fallback sólido"));

        let status_row = div()
            .flex()
            .gap(px(SpacingScale::S6))
            .items_center()
            .child(status_dot(
                &theme,
                "gallery-status-success",
                StatusKind::Success,
                "Build verde",
            ))
            .child(status_dot(
                &theme,
                "gallery-status-warning",
                StatusKind::Warning,
                "Evidência pendente",
            ))
            .child(status_dot(
                &theme,
                "gallery-status-danger",
                StatusKind::Danger,
                "Job falhou",
            ))
            .child(status_dot(
                &theme,
                "gallery-status-info",
                StatusKind::Info,
                "Somente leitura",
            ));

        let states_row = div()
            .flex()
            .gap(px(SpacingScale::S5))
            .items_start()
            .child(div().w(px(430.0)).child(empty_state(
                &theme,
                "gallery-empty",
                "Nenhuma decisão ainda",
                "Capture o trabalho real para começar a formar candidatas.",
                "A Inbox aparece quando houver evidência.",
            )))
            .child(div().w(px(430.0)).child(error_state(
                &theme,
                "gallery-error",
                "Não foi possível abrir o projeto",
                "O caminho configurado não existe mais no disco.",
                "Verifique o caminho e tente novamente.",
            )));

        let header = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S1))
            .child(
                text_style(div(), TypeScale::DISPLAY)
                    .text_color(theme.colors.text_primary())
                    .child("Quiet Glass"),
            )
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(theme.colors.text_muted())
                    .child("Primitivas reutilizáveis · Gate 1 · 1440×1024"),
            );

        div()
            .id("quiet-glass-gallery")
            .role(Role::Application)
            .aria_label("xemnas Quiet Glass gallery")
            .track_focus(&self.focus)
            .key_context("gallery")
            .on_action(cx.listener(Self::on_tab_next))
            .size_full()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .p(px(SpacingScale::S8))
            .bg(theme.colors.canvas())
            .font_family(Theme::FONT_INTERFACE)
            .text_color(theme.colors.text_primary())
            .child(header)
            .child(section(&theme, "Ações", button_rows))
            .child(section(&theme, "Busca", search_row))
            .child(section(&theme, "Superfícies de vidro", glass_row))
            .child(section(&theme, "Status", status_row))
            .child(section(&theme, "Estados vazio e erro", states_row))
    }
}

fn glass_card(theme: &Theme, variant: GlassVariant, label: &str) -> Div {
    let surface = GlassSurface::new(variant);
    let title_color = surface.foreground(theme);
    // On Glass Emphasis the surface is near-opaque lavender: the only legible
    // text token is accent.on-emphasis (text.muted over it is ≈1.2:1).
    let caption_color = if variant == GlassVariant::Emphasis {
        theme.colors.accent_on_emphasis()
    } else {
        theme.colors.text_muted()
    };
    surface
        .render(theme)
        .w(px(210.0))
        .h(px(88.0))
        .p(px(SpacingScale::S4))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(SpacingScale::S1))
        .child(
            text_style(div(), TypeScale::HEADING_3)
                .text_color(title_color)
                .child(label.to_string()),
        )
        .child(
            text_style(div(), TypeScale::META)
                .text_color(caption_color)
                .child("fill · borda · highlight"),
        )
}

fn glass_card_solid(theme: &Theme, label: &str) -> Div {
    GlassSurface::new(GlassVariant::Low)
        .solid()
        .render(theme)
        .w(px(210.0))
        .h(px(88.0))
        .p(px(SpacingScale::S4))
        .flex()
        .flex_col()
        .justify_center()
        .gap(px(SpacingScale::S1))
        .child(
            text_style(div(), TypeScale::HEADING_3)
                .text_color(theme.colors.text_primary())
                .child(label.to_string()),
        )
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_muted())
                .child("sem dependência de blur"),
        )
}

fn main() {
    if let Err(error) = telemetry::init() {
        eprintln!("telemetry init failed: {error}");
    }

    application().run(|cx: &mut App| {
        xemnas_desktop::fonts::register_embedded(cx);
        cx.bind_keys([KeyBinding::new("tab", TabNext, None)]);

        let bounds = Bounds::centered(None, size(px(1440.0), px(1024.0)), cx);
        let view = cx.new(|cx| Gallery::new(cx, Theme::quiet_glass()));

        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("xemnas — Quiet Glass Gallery".into()),
                    ..Default::default()
                }),
                app_id: Some("com.xemnas.gallery".into()),
                window_min_size: Some(size(px(1180.0), px(760.0))),
                ..Default::default()
            },
            |window, cx| {
                let focus = view.read(cx).focus.clone();
                window.focus(&focus, cx);
                view.clone()
            },
        );
        if let Err(error) = opened {
            eprintln!("failed to open gallery window: {error}");
        }

        cx.activate(true);
    });
}
