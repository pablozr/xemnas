//! Settings → Language: which language every screen reads in.
//!
//! Like the appearance, this is a preference of the interface
//! (`ui::appearance`), saved as soon as it changes and applied to every
//! window at once.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Context, Render, Role, Toggled, Window};

use super::appearance::radio_mark;
use super::parts::{card, card_body};
use crate::i18n::{self, settings as t, Language};
use crate::ui::appearance;
use crate::ui::controls::focus_ring;
use crate::ui::patterns::mark_selected;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};

/// The language section of Settings.
#[derive(Default)]
pub struct LanguagePanel;

impl LanguagePanel {
    fn row(
        theme: &Theme,
        language: Language,
        selected: bool,
        divided: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = theme.colors;
        let translated = t::language_name(language);
        mark_selected(
            div()
                .id(("settings-language", language as usize))
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
                .aria_label(language.native_name())
                .aria_toggled(if selected {
                    Toggled::True
                } else {
                    Toggled::False
                })
                .focus_visible(focus_ring(theme))
                .on_click(cx.listener(move |_, _, _, cx| {
                    appearance::update(cx, |appearance| appearance.language = language);
                })),
            theme,
            selected,
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .items_baseline()
                .gap(px(SpacingScale::S3))
                .child(text_style(div(), TypeScale::ROW_TITLE).child(language.native_name()))
                // The name in the current language helps whoever landed on
                // an interface they cannot read find their way back.
                .when(translated != language.native_name(), |row| {
                    row.child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .text_color(colors.text_muted())
                            .child(translated),
                    )
                }),
        )
        .child(radio_mark(theme, selected))
        .into_any_element()
    }
}

impl Render for LanguagePanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        let current = i18n::current();
        let mut list = div()
            .flex()
            .flex_col()
            .rounded(RadiusScale.surface())
            .border_1()
            .border_color(theme.colors.glass_border_card())
            .overflow_hidden();
        for (index, language) in Language::ALL.into_iter().enumerate() {
            list = list.child(Self::row(
                &theme,
                language,
                language == current,
                index > 0,
                cx,
            ));
        }
        card(&theme, t::language_title(), t::language_card_body()).child(card_body().child(list))
    }
}
