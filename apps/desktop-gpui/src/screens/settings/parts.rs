//! Visual pieces shared by the settings sections: hairline panels, the inline
//! status line, labelled fields, checklist steps, figures and key/value rows.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Div, Rgba, Role, Stateful};

use crate::i18n::settings as t;
use crate::ui::icons::{icon, IconName};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{RadiusScale, SpacingScale, TypeScale};

/// A settings group: a hairline panel whose header is the title and one
/// line of purpose, like a Linear or Zed settings section. No glyph: the
/// section navigation already carries the icon.
pub(super) fn card(theme: &Theme, title: &'static str, description: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(RadiusScale.surface())
        .border_1()
        .border_color(theme.colors.glass_border_card())
        .bg(theme.colors.glass_fill_card())
        .overflow_hidden()
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .px(px(SpacingScale::S5))
                .pt(px(SpacingScale::S4))
                .child(text_style(div(), TypeScale::HEADING_3).child(title))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .min_w(px(0.0))
                        .text_color(theme.colors.text_muted())
                        .child(description),
                ),
        )
}

pub(super) fn card_body() -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S4))
        .p(px(SpacingScale::S5))
}

pub(super) fn card_footer(theme: &Theme) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .px(px(SpacingScale::S5))
        .py(px(SpacingScale::S3))
        .border_t_1()
        .border_color(theme.colors.hairline_divider())
        .bg(theme.colors.glass_fill_low())
}

pub(super) fn field_row(theme: &Theme, label: &str, field: AnyElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            text_style(div(), TypeScale::LABEL)
                .text_color(theme.colors.text_secondary())
                .child(label.to_owned()),
        )
        .child(field)
}

/// One requirement before consent, as a checklist row: a check when met,
/// an empty circle while pending.
pub(super) fn step(
    theme: &Theme,
    index: usize,
    done: bool,
    title: &'static str,
    hint: &'static str,
) -> Stateful<Div> {
    let colors = theme.colors;
    div()
        .id(("settings-step", index))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .py(px(SpacingScale::S2))
        .when(index > 0, |row| {
            row.border_t_1().border_color(colors.hairline_divider())
        })
        .role(Role::ListItem)
        .aria_label(format!(
            "{title}: {}",
            if done {
                t::step_done()
            } else {
                t::step_pending()
            }
        ))
        .child(if done {
            icon(IconName::CheckCircle, 16.0, colors.status_success())
        } else {
            icon(IconName::Circle, 16.0, colors.text_muted())
        })
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .flex_1()
                .min_w(px(0.0))
                .text_color(if done {
                    colors.text_primary()
                } else {
                    colors.text_secondary()
                })
                .child(title),
        )
        .child(
            text_style(div(), TypeScale::META)
                .text_color(colors.text_muted())
                .child(hint),
        )
}

/// The state of a section at a glance: a status dot, the state and one
/// sentence, inline under the page title (no tinted box).
pub(super) fn status_hero(
    theme: &Theme,
    id: &'static str,
    color: Rgba,
    title: &'static str,
    body: String,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_start()
        .gap(px(SpacingScale::S3))
        .role(Role::Status)
        .aria_label(format!("{title}. {body}"))
        .child(
            div()
                .mt(px(6.0))
                .size(px(8.0))
                .flex_none()
                .rounded_full()
                .bg(color),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(text_style(div(), TypeScale::ROW_TITLE).child(title))
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_muted())
                        .child(body),
                ),
        )
}

/// A number with its label, flat; an accent colours the number when it
/// matters. The surrounding panel draws the frame.
pub(super) fn stat_tile(
    theme: &Theme,
    label: &'static str,
    value: String,
    accent: Option<Rgba>,
) -> Div {
    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(4.0))
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_muted())
                .child(label),
        )
        .child(
            text_style(div(), TypeScale::HEADING_2)
                .truncate()
                .text_color(accent.unwrap_or(theme.colors.text_primary()))
                .child(value),
        )
}

/// A label and its value on one line; `mono` for paths and addresses.
pub(super) fn kv_row(
    theme: &Theme,
    label: &'static str,
    value: String,
    mono: bool,
    divider: bool,
) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S4))
        .py(px(SpacingScale::S2))
        .when(divider, |row| {
            row.border_t_1()
                .border_color(theme.colors.hairline_divider())
        })
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .w(px(180.0))
                .flex_none()
                .text_color(theme.colors.text_muted())
                .child(label),
        )
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .when(mono, |text| text.font_family(Theme::font_mono()))
                .child(value),
        )
}
