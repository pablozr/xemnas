//! Settings → Aparência: the theme, and the background behind the surfaces.
//!
//! Nothing here reaches the application layer: the appearance is a
//! preference of the interface (`ui::appearance`), saved as soon as it
//! changes and applied to every window at once.

use gpui::prelude::*;
use std::sync::Arc;

use gpui::{
    div, img, px, AnyElement, Context, Div, Image, ObjectFit, Render, Role, Toggled, Window,
};

use super::parts::{card, card_body};
use crate::ui::appearance::{self, Level, Wallpaper};
use crate::ui::controls::focus_ring;
use crate::ui::icons::{icon, IconName};
use crate::ui::patterns::{mark_selected, segment_label, segmented};
use crate::ui::theme::{text_style, Theme, ThemeMode};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};
use crate::ui::wallpaper::{self, BUILTINS};

/// The appearance section of Settings.
pub struct AppearancePanel {
    /// Picker thumbnails of the shipped backgrounds, decoded once.
    thumbnails: Vec<Arc<Image>>,
}

impl Default for AppearancePanel {
    fn default() -> Self {
        Self {
            thumbnails: BUILTINS.iter().map(|builtin| builtin.thumbnail()).collect(),
        }
    }
}

/// Width and height of a background tile.
const TILE: (f32, f32) = (164.0, 92.0);

impl AppearancePanel {
    /// One background choice: the picture, its name, a ring when chosen.
    fn tile(
        theme: &Theme,
        id: &'static str,
        title: String,
        picture: AnyElement,
        selected: bool,
        choose: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        div()
            .id(id)
            .w(px(TILE.0))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S2))
            .cursor_pointer()
            .role(Role::RadioButton)
            .aria_label(title.clone())
            .aria_toggled(if selected {
                Toggled::True
            } else {
                Toggled::False
            })
            .focus_visible(focus_ring(theme))
            .on_click(cx.listener(move |this, _, _, cx| choose(this, cx)))
            .child(
                div()
                    .w(px(TILE.0))
                    .h(px(TILE.1))
                    .rounded(RadiusScale.surface())
                    .overflow_hidden()
                    // The ring sits on a border every tile has, so choosing
                    // one never moves the grid.
                    .border_2()
                    .border_color(if selected {
                        colors.accent_emphasis()
                    } else {
                        colors.glass_border_card()
                    })
                    .hover(move |style| style.border_color(colors.glass_border_card_hover()))
                    .child(picture),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .truncate()
                    .text_color(if selected {
                        colors.text_primary()
                    } else {
                        colors.text_secondary()
                    })
                    .child(title),
            )
            .into_any_element()
    }

    fn backgrounds(&mut self, theme: &Theme, current: &Wallpaper, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let mut grid = div().flex().flex_wrap().gap(px(SpacingScale::S4));
        grid = grid.child(Self::tile(
            theme,
            "wallpaper-none",
            "Sem fundo".into(),
            div()
                .size_full()
                .bg(colors.canvas())
                .flex()
                .items_center()
                .justify_center()
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child("Só o tema"),
                )
                .into_any_element(),
            *current == Wallpaper::None,
            |_, cx| appearance::update(cx, |appearance| appearance.wallpaper = Wallpaper::None),
            cx,
        ));
        for (index, builtin) in BUILTINS.iter().enumerate() {
            let id = builtin.id;
            grid = grid.child(Self::tile(
                theme,
                ["wallpaper-0", "wallpaper-1", "wallpaper-2", "wallpaper-3"][index],
                builtin.title.into(),
                img(self.thumbnails[index].clone())
                    .size_full()
                    .object_fit(ObjectFit::Cover)
                    .into_any_element(),
                *current == Wallpaper::Builtin(id.into()),
                move |_, cx| {
                    appearance::update(cx, |appearance| {
                        appearance.wallpaper = Wallpaper::Builtin(id.into())
                    })
                },
                cx,
            ));
        }
        let custom = match current {
            Wallpaper::Custom(path) => Some(path.clone()),
            _ => None,
        };
        let picture = match &custom {
            Some(path) => img(path.clone())
                .size_full()
                .object_fit(ObjectFit::Cover)
                .into_any_element(),
            None => div()
                .size_full()
                .bg(colors.canvas_deep())
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(SpacingScale::S1))
                .child(icon(IconName::Plus, 16.0, colors.text_muted()))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_muted())
                        .child("Escolher imagem"),
                )
                .into_any_element(),
        };
        grid.child(Self::tile(
            theme,
            "wallpaper-custom",
            custom
                .as_ref()
                .map(|path| wallpaper::custom_name(path))
                .unwrap_or_else(|| "Sua imagem".into()),
            picture,
            custom.is_some(),
            |_, cx| pick_image(cx),
            cx,
        ))
    }

    fn level_row(
        &self,
        theme: &Theme,
        name: &'static str,
        labels: [&'static str; 3],
        value: Level,
        set: fn(&mut appearance::Appearance, Level),
        cx: &mut Context<Self>,
    ) -> Div {
        let mut track = segmented(theme).id(name).role(Role::RadioGroup);
        for (level, label) in Level::ALL.into_iter().zip(labels) {
            track = track.child(
                segment_label(theme, (name, level as usize), label, level == value).on_click(
                    cx.listener(move |_, _, _, cx| {
                        appearance::update(cx, |appearance| set(appearance, level))
                    }),
                ),
            );
        }
        div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .child(
                text_style(div(), TypeScale::LABEL)
                    .w(px(120.0))
                    .flex_none()
                    .text_color(theme.colors.text_secondary())
                    .child(name),
            )
            .child(track)
    }

    fn theme_row(
        &self,
        theme: &Theme,
        mode: ThemeMode,
        selected: bool,
        divided: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        mark_selected(
            div()
                .id(("appearance-theme", mode as usize))
                .relative()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S4))
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S3))
                .when(divided, |row| {
                    row.border_t_1().border_color(colors.hairline_divider())
                })
                .when(!selected, |row| {
                    row.hover(move |style| style.bg(colors.glass_fill_medium()))
                        .active(move |style| style.bg(colors.glass_fill_strong()))
                })
                .cursor_pointer()
                .role(Role::RadioButton)
                .aria_label(mode.label())
                .aria_toggled(if selected {
                    Toggled::True
                } else {
                    Toggled::False
                })
                .focus_visible(focus_ring(theme))
                .on_click(cx.listener(move |_, _, _, cx| {
                    appearance::update(cx, |appearance| appearance.theme = mode);
                })),
            theme,
            selected,
        )
        .child(preview(mode))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(text_style(div(), TypeScale::ROW_TITLE).child(mode.label()))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_muted())
                        .child(mode.blurb()),
                ),
        )
        .child(radio_mark(theme, selected))
        .into_any_element()
    }
}

/// A miniature of the app in `mode`: rail, a selected row, a reading line
/// and the primary action, so the choice is seen rather than described.
fn preview(mode: ThemeMode) -> Div {
    let colors = mode.theme().colors;
    let line = |width: f32, color| div().h(px(3.0)).w(px(width)).rounded_full().bg(color);
    div()
        .w(px(84.0))
        .h(px(52.0))
        .flex_none()
        .flex()
        .rounded(px(6.0))
        .overflow_hidden()
        .border_1()
        .border_color(colors.glass_border_card())
        .bg(colors.canvas())
        .child(
            div()
                .w(px(22.0))
                .h_full()
                .bg(colors.rail())
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(4.0))
                .child(
                    div()
                        .h(px(5.0))
                        .rounded(px(1.5))
                        .bg(colors.selection())
                        .border_l_2()
                        .border_color(colors.accent_emphasis()),
                )
                .child(line(10.0, colors.text_muted()))
                .child(line(12.0, colors.text_muted())),
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .p(px(6.0))
                .child(line(34.0, colors.text_primary()))
                .child(line(44.0, colors.text_muted()))
                .child(line(30.0, colors.text_muted()))
                .child(div().flex_1())
                .child(
                    div().flex().justify_end().child(
                        div()
                            .w(px(18.0))
                            .h(px(7.0))
                            .rounded(px(2.0))
                            .bg(colors.accent_emphasis()),
                    ),
                ),
        )
}

fn radio_mark(theme: &Theme, selected: bool) -> Div {
    let colors = theme.colors;
    div()
        .size(px(16.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .border_color(if selected {
            colors.accent_hover()
        } else {
            colors.glass_border_control()
        })
        .when(selected, |mark| {
            mark.child(div().size(px(8.0)).rounded_full().bg(colors.accent_hover()))
        })
}

impl Render for AppearancePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let current = appearance::current(cx);
        let mut themes = div()
            .flex()
            .flex_col()
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(theme.colors.glass_border_card())
            .overflow_hidden();
        for (index, mode) in ThemeMode::ALL.into_iter().enumerate() {
            themes =
                themes.child(self.theme_row(&theme, mode, current.theme == mode, index > 0, cx));
        }
        let backgrounds = self.backgrounds(&theme, &current.wallpaper, cx);
        let failed = wallpaper::failed(cx);
        let mut background = card_body().child(backgrounds);
        if failed {
            background = background.child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .text_color(theme.colors.status_warning())
                    .child("Não foi possível abrir essa imagem. Escolha outra ou volte a um fundo do app."),
            );
        }
        if current.wallpaper != Wallpaper::None {
            background = background
                .child(self.level_row(
                    &theme,
                    "Desfoque",
                    ["Leve", "Médio", "Forte"],
                    current.blur,
                    |appearance, level| appearance.blur = level,
                    cx,
                ))
                .child(self.level_row(
                    &theme,
                    "Escurecer",
                    ["Pouco", "Médio", "Bastante"],
                    current.dim,
                    |appearance, level| appearance.dim = level,
                    cx,
                ))
                .child(self.level_row(
                    &theme,
                    "Superfícies",
                    ["Mais vidro", "Equilibradas", "Mais sólidas"],
                    current.solidity,
                    |appearance, level| appearance.solidity = level,
                    cx,
                ));
        }
        div()
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .child(
                card(
                    &theme,
                    "Tema",
                    "Cores de todo o app. Muda na hora e fica salvo para a próxima vez.",
                )
                .child(card_body().child(themes)),
            )
            .child(
                card(
                    &theme,
                    "Fundo",
                    "Uma imagem atrás das superfícies, desfocada e escurecida para o texto seguir legível. Os fundos do app são inspirados na Organização.",
                )
                .child(background),
            )
    }
}

/// Asks the system for an image and makes it the background.
fn pick_image(cx: &mut Context<AppearancePanel>) {
    cx.spawn(async move |_, cx| {
        let picked = cx
            .background_executor()
            .spawn(async move {
                rfd::FileDialog::new()
                    .set_title("Escolher imagem de fundo")
                    .add_filter("Imagens", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
            })
            .await;
        if let Some(path) = picked {
            cx.update(|cx| {
                appearance::update(cx, |appearance| {
                    appearance.wallpaper = Wallpaper::Custom(path)
                })
            });
        }
    })
    .detach();
}
