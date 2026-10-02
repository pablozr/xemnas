//! Quiet Glass gallery.
//!
//! Evidence binary that renders the product recipes (buttons, tabs, pills,
//! lists, feedback, type) exactly as the screens use them, so
//! `tools/capture-quiet-glass.ps1` can screenshot them at 1440×1024. It is
//! intentionally separate from the product binary (`xemnas`).

use gpui::prelude::*;
use gpui::{
    actions, div, px, size, App, Bounds, Context, Div, Entity, FocusHandle, IntoElement,
    KeyBinding, Render, Role, TitlebarOptions, Window, WindowBounds, WindowOptions,
};
use gpui_platform::application;

use xemnas_desktop::ui::controls::{action_button, button_foreground, icon_action, ButtonKind};
use xemnas_desktop::ui::icons::{icon, IconName};
use xemnas_desktop::ui::patterns::{
    count_chip, empty_panel, error_banner, kbd, mark_selected, panel_title, reading_title,
    section_label, skeleton_list, status_pill,
};
use xemnas_desktop::ui::search_field::SearchField;
use xemnas_desktop::ui::theme::{text_style, Theme};
use xemnas_desktop::ui::tokens::{SpacingScale, TypeScale};

actions!(quiet_glass_gallery, [TabNext]);

const GLYPHS: [IconName; 23] = [
    IconName::List,
    IconName::File,
    IconName::Info,
    IconName::Folder,
    IconName::FolderPlus,
    IconName::Plus,
    IconName::Search,
    IconName::Contrast,
    IconName::Clock,
    IconName::Layers,
    IconName::Target,
    IconName::CheckCircle,
    IconName::Activity,
    IconName::Rotate,
    IconName::Link,
    IconName::Filter,
    IconName::Export,
    IconName::Copy,
    IconName::Edit,
    IconName::Expand,
    IconName::ChevronRight,
    IconName::ChevronDown,
    IconName::ChevronUp,
];

struct Gallery {
    focus: FocusHandle,
    /// Real tab stops, so a keyboard Tab paints the focus-visible ring on a
    /// live control.
    tab_stops: Vec<FocusHandle>,
    /// Next tab stop to focus; advanced by [`Gallery::on_tab_next`].
    tab_index: usize,
    search: Entity<SearchField>,
}

impl Gallery {
    fn new(cx: &mut Context<Self>) -> Self {
        let tab_stops = (0..3).map(|_| cx.focus_handle().tab_stop(true)).collect();
        let search = cx.new(|cx| {
            let mut field = SearchField::new(cx);
            field.set_context("Buscar decisões", cx);
            field
        });
        Self {
            focus: cx.focus_handle(),
            tab_stops,
            tab_index: 0,
            search,
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
}

fn section(theme: &Theme, title: &str, content: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S3))
        .child(section_label(theme, title))
        .child(content)
}

fn row() -> Div {
    div()
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(SpacingScale::S2))
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);

        let buttons = row()
            .child(
                action_button(&theme, "g-confirm", ButtonKind::Primary, true)
                    .aria_label("Confirmar decisão")
                    .track_focus(&self.tab_stops[0])
                    .child("Confirmar")
                    .child(kbd(
                        button_foreground(&theme, ButtonKind::Primary, true),
                        "C",
                    )),
            )
            .child(
                action_button(&theme, "g-adjust", ButtonKind::Secondary, true)
                    .aria_label("Ajustar")
                    .track_focus(&self.tab_stops[1])
                    .child("Ajustar")
                    .child(kbd(
                        button_foreground(&theme, ButtonKind::Secondary, true),
                        "A",
                    )),
            )
            .child(
                action_button(&theme, "g-reject", ButtonKind::Secondary, true)
                    .aria_label("Rejeitar")
                    .child("Rejeitar"),
            )
            .child(
                action_button(&theme, "g-ghost", ButtonKind::Ghost, true)
                    .aria_label("Atualizar")
                    .child("Atualizar"),
            )
            .child(
                action_button(&theme, "g-disabled", ButtonKind::Primary, false)
                    .aria_label("Salvar indisponível")
                    .child("Salvar nova versão"),
            )
            .child(
                icon_action(&theme, "g-copy", "Copiar trecho")
                    .track_focus(&self.tab_stops[2])
                    .child(icon(IconName::Copy, 16.0, theme.colors.text_secondary())),
            );

        let tabs = row()
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .h(px(28.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .rounded(theme.radius.control())
                    .bg(theme.colors.selection())
                    .child(icon(IconName::List, 14.0, theme.colors.text_primary()))
                    .child("Revisão")
                    .child(count_chip(&theme, "5")),
            )
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .h(px(28.0))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .text_color(theme.colors.text_muted())
                    .child(icon(IconName::File, 14.0, theme.colors.text_muted()))
                    .child("Decisões"),
            )
            .child(div().w(px(24.0)))
            .child(status_pill(
                &theme,
                theme.colors.status_warning(),
                "Pendente",
            ))
            .child(status_pill(&theme, theme.colors.status_info(), "Adiado"))
            .child(status_pill(
                &theme,
                theme.colors.status_success(),
                "Confirmado",
            ))
            .child(status_pill(
                &theme,
                theme.colors.status_danger(),
                "Rejeitado",
            ))
            .child(count_chip(&theme, "v2"))
            .child(kbd(theme.colors.text_secondary(), "Ctrl K"));

        let glyphs = row().children(GLYPHS.iter().map(|glyph| {
            div()
                .size(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(theme.radius.control())
                .border_1()
                .border_color(theme.colors.hairline_divider())
                .child(icon(*glyph, 16.0, theme.colors.text_secondary()))
        }));

        let list = div()
            .w(px(320.0))
            .flex()
            .flex_col()
            .rounded(theme.radius.surface())
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .bg(theme.colors.rail())
            .overflow_hidden()
            .child(
                div()
                    .px(px(SpacingScale::S4))
                    .pt(px(SpacingScale::S3))
                    .pb(px(SpacingScale::S2))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(panel_title(&theme, "Aguardando revisão"))
                    .child(count_chip(&theme, "2")),
            )
            .children(
                [
                    ("Como garantir uma única decisão por captura?", true),
                    ("Onde armazenar credenciais do provedor?", false),
                ]
                .into_iter()
                .map(|(question, selected)| {
                    mark_selected(
                        div()
                            .relative()
                            .px(px(SpacingScale::S4))
                            .py(px(SpacingScale::S3))
                            .flex()
                            .flex_col()
                            .gap(px(SpacingScale::S1))
                            .child(
                                text_style(div(), TypeScale::META)
                                    .text_color(theme.colors.text_muted())
                                    .child("29 set 2026"),
                            )
                            .child(text_style(div(), TypeScale::ROW_TITLE).child(question)),
                        &theme,
                        selected,
                    )
                }),
            );

        let feedback = div()
            .w(px(420.0))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                div()
                    .rounded(theme.radius.surface())
                    .overflow_hidden()
                    .border_1()
                    .border_color(theme.colors.hairline_divider())
                    .child(
                        error_banner(&theme, "Não foi possível carregar a fila.").child(
                            action_button(&theme, "g-retry", ButtonKind::Secondary, true)
                                .aria_label("Tentar novamente")
                                .child("Tentar novamente"),
                        ),
                    ),
            )
            .child(
                div()
                    .rounded(theme.radius.surface())
                    .border_1()
                    .border_color(theme.colors.hairline_divider())
                    .bg(theme.colors.rail())
                    .child(skeleton_list(&theme, "g-skeleton", 2)),
            );

        let empty = div()
            .w(px(460.0))
            .h(px(240.0))
            .rounded(theme.radius.surface())
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .child(empty_panel(
                &theme,
                IconName::File,
                "Decisões",
                "Nenhuma decisão ainda",
                "Confirme uma escolha na Revisão para preservá-la aqui.",
            ));

        let type_scale = div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .child(reading_title("Como versionar decisões revisadas?"))
            .child(text_style(div(), TypeScale::HEADING_2).child("Heading 2 · 16 / 24"))
            .child(text_style(div(), TypeScale::BODY).child("Body · 14 / 22 · leitura"))
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.text_secondary())
                    .child("Body small · 13 / 19 · interface"),
            )
            .child(
                xemnas_desktop::ui::theme::code_style(div(), TypeScale::CODE)
                    .text_color(theme.colors.text_secondary())
                    .child("history.append(previous_revision);"),
            );

        div()
            .id("quiet-glass-gallery")
            .role(Role::Application)
            .aria_label("xemnas Quiet Glass gallery")
            .track_focus(&self.focus)
            .key_context("gallery")
            .on_action(cx.listener(Self::on_tab_next))
            .size_full()
            .overflow_hidden()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S6))
            .p(px(SpacingScale::S8))
            .bg(theme.colors.canvas())
            .font_family(Theme::font_interface())
            .text_color(theme.colors.text_primary())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(reading_title("Quiet Glass"))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(theme.colors.text_muted())
                            .child("Receitas do produto · Gate 1 · 1440×1024"),
                    ),
            )
            .child(section(&theme, "Ações", buttons))
            .child(section(
                &theme,
                "Busca",
                div().w(px(320.0)).child(self.search.clone()),
            ))
            .child(section(&theme, "Abas, selos e atalhos", tabs))
            .child(section(&theme, "Ícones", glyphs))
            .child(
                div()
                    .flex()
                    .gap(px(SpacingScale::S8))
                    .child(section(&theme, "Lista", list))
                    .child(section(&theme, "Erro e carregamento", feedback))
                    .child(section(&theme, "Tipografia", type_scale)),
            )
            .child(section(&theme, "Estado vazio", empty))
    }
}

fn main() {
    if let Err(error) = telemetry::init() {
        eprintln!("telemetry init failed: {error}");
    }

    application().run(|cx: &mut App| {
        xemnas_desktop::fonts::register_embedded(cx);
        cx.bind_keys([KeyBinding::new("tab", TabNext, None)]);

        let bounds = Bounds::centered(None, size(px(1440.0), px(1024.0)), cx);
        let view = cx.new(Gallery::new);

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
