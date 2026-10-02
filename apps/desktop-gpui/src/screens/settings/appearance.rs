//! Settings → Aparência: the theme, and the background behind the surfaces.
//!
//! Nothing here reaches the application layer: the appearance is a
//! preference of the interface (`ui::appearance`), saved as soon as it
//! changes and applied to every window at once.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Div, Render, Role, Toggled, Window};

use super::parts::{card, card_body};
use crate::ui::appearance;
use crate::ui::controls::focus_ring;
use crate::ui::patterns::mark_selected;
use crate::ui::theme::{text_style, Theme, ThemeMode};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};

/// The appearance section of Settings.
#[derive(Default)]
pub struct AppearancePanel;

impl AppearancePanel {
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
        div().flex().flex_col().gap(px(SpacingScale::S5)).child(
            card(
                &theme,
                "Tema",
                "Cores de todo o app. Muda na hora e fica salvo para a próxima vez.",
            )
            .child(card_body().child(themes)),
        )
    }
}
