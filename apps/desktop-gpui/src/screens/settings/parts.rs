//! Visual pieces shared by the settings sections: glyph tiles, grouped cards,
//! banners, labelled fields and numbered steps.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Div, Rgba, Role, Stateful};

use crate::ui::icons::{icon, IconName};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{tint, SpacingScale, TypeScale};

/// A rounded square holding a glyph in its own soft tint.
pub(super) fn icon_tile(theme: &Theme, glyph: IconName, color: Rgba, size: f32) -> Div {
    div()
        .size(px(size))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px((size * 0.28).round()))
        .bg(tint(color, 0.12))
        .border_1()
        .border_color(tint(color, 0.22))
        .child(icon(glyph, (size * 0.5).round(), color))
        .text_color(theme.colors.text_primary())
}

/// A settings group: header with glyph, title and one-line purpose.
pub(super) fn card(
    theme: &Theme,
    glyph: IconName,
    title: &'static str,
    description: &'static str,
) -> Div {
    div()
        .flex()
        .flex_col()
        .rounded(px(12.0))
        .border_1()
        .border_color(theme.colors.glass_border_card())
        .bg(theme.colors.glass_fill_card())
        .overflow_hidden()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .px(px(SpacingScale::S5))
                .pt(px(SpacingScale::S5))
                .child(icon_tile(theme, glyph, theme.colors.text_secondary(), 28.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(text_style(div(), TypeScale::HEADING_3).child(title))
                        .child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(theme.colors.text_muted())
                                .child(description),
                        ),
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

pub(super) fn banner(
    theme: &Theme,
    id: &'static str,
    color: Rgba,
    message: String,
) -> Stateful<Div> {
    text_style(div(), TypeScale::BODY_SMALL)
        .id(id)
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .px(px(SpacingScale::S3))
        .py(px(SpacingScale::S2))
        .rounded(theme.radius.control())
        .bg(tint(color, 0.08))
        .border_1()
        .border_color(tint(color, 0.3))
        .text_color(theme.colors.text_primary())
        .child(div().size(px(6.0)).flex_none().rounded_full().bg(color))
        .child(message)
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

/// One consent step: a numbered (or checked) node, its label and the
/// connector to the next step.
pub(super) fn step(
    theme: &Theme,
    index: usize,
    done: bool,
    title: &'static str,
    hint: &'static str,
    connector: bool,
) -> Stateful<Div> {
    let colors = theme.colors;
    let node = div()
        .size(px(24.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .border_1()
        .map(|node| {
            if done {
                node.bg(colors.status_success())
                    .border_color(colors.status_success())
                    .child(icon(IconName::Check, 14.0, colors.accent_on_emphasis()))
            } else {
                node.border_color(colors.glass_border_card_hover()).child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_secondary())
                        .child((index + 1).to_string()),
                )
            }
        });
    div()
        .id(("settings-step", index))
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .role(Role::ListItem)
        .aria_label(format!(
            "{title}: {}",
            if done { "concluído" } else { "pendente" }
        ))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .child(node)
                .when(connector, |row| {
                    row.child(
                        div()
                            .flex_1()
                            .h(px(1.0))
                            .mr(px(SpacingScale::S2))
                            .bg(if done {
                                tint(colors.status_success(), 0.5)
                            } else {
                                colors.hairline_divider()
                            }),
                    )
                }),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    text_style(div(), TypeScale::ROW_TITLE)
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
                ),
        )
}

/// The state of a section at a glance: tinted by its status colour, with a
/// glyph tile, a title, a pill and one sentence of explanation.
pub(super) fn status_hero(
    theme: &Theme,
    id: &'static str,
    color: Rgba,
    glyph: IconName,
    title: &'static str,
    pill: &'static str,
    body: String,
) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(SpacingScale::S4))
        .p(px(SpacingScale::S4))
        .rounded(px(10.0))
        .border_1()
        .border_color(tint(color, 0.28))
        .bg(tint(color, 0.07))
        .role(Role::Status)
        .aria_label(format!("{title}. {body}"))
        .child(icon_tile(theme, glyph, color, 40.0))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(SpacingScale::S2))
                        .child(text_style(div(), TypeScale::HEADING_3).child(title))
                        .child(crate::ui::patterns::status_pill(theme, color, pill)),
                )
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(theme.colors.text_secondary())
                        .child(body),
                ),
        )
}

/// A number with its label; an accent colours the number when it matters.
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
        .p(px(SpacingScale::S3))
        .rounded(theme.radius.control())
        .bg(theme.colors.glass_fill_low())
        .border_1()
        .border_color(theme.colors.hairline_divider())
        .child(
            text_style(div(), TypeScale::META)
                .text_color(theme.colors.text_muted())
                .child(label.to_uppercase()),
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
