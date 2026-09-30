//! Small layout patterns shared by every screen.
//!
//! Each one used to be re-typed per screen with slightly different sizes and
//! colours (a count chip existed in four paddings, a selected row in three
//! recipes). Screens compose these instead, so a panel reads the same in
//! Projects, Revisão and Decisões.

use gpui::prelude::*;
use gpui::{
    div, px, Animation, AnimationElement, AnimationExt, Div, ElementId, Rgba, SharedString,
};

use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{MotionTokens, SpacingScale, TypeScale};

/// The quiet title of a side panel: 12 px, muted. The content, not the
/// panel name, carries the weight.
pub fn panel_title(theme: &Theme, label: impl Into<SharedString>) -> Div {
    text_style(div(), TypeScale::LABEL)
        .text_color(theme.colors.text_muted())
        .child(label.into())
}

/// A small numeric chip: tab badges, panel counts, versions.
pub fn count_chip(theme: &Theme, value: impl Into<SharedString>) -> Div {
    text_style(div(), TypeScale::META)
        .flex_none()
        .px(px(6.0))
        .rounded(px(4.0))
        .bg(theme.colors.surface())
        .text_color(theme.colors.text_secondary())
        .child(value.into())
}

/// A status pill: dot plus text, never colour alone.
pub fn status_pill(theme: &Theme, color: Rgba, label: &'static str) -> Div {
    text_style(div(), TypeScale::META)
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(SpacingScale::S2))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(theme.colors.surface())
        .text_color(theme.colors.text_secondary())
        .child(div().size(px(6.0)).flex_none().rounded_full().bg(color))
        .child(label)
}

/// An editorial section label ("ESCOLHA SUGERIDA", "JUSTIFICATIVA").
///
/// Uppercase meta text replaces the old heading-plus-rule pair: sections are
/// separated by space, and the few rules left mark real boundaries.
pub fn section_label(theme: &Theme, label: &str) -> Div {
    text_style(div(), TypeScale::META)
        .text_color(theme.colors.text_muted())
        .child(label.to_uppercase())
}

/// The single selection treatment for list rows: the `selection` fill plus a
/// 2 px lavender bar on the leading edge. The row must be `relative()`.
pub fn mark_selected<E: ParentElement + Styled>(row: E, theme: &Theme, selected: bool) -> E {
    if !selected {
        return row;
    }
    row.bg(theme.colors.selection()).child(
        div()
            .absolute()
            .left_0()
            .top(px(SpacingScale::S2))
            .bottom(px(SpacingScale::S2))
            .w(px(2.0))
            .rounded_full()
            .bg(theme.colors.accent_hover()),
    )
}

/// Fades content in when `key` changes (a new selection, a new destination).
///
/// GPUI has no style transitions, so hover stays instant; this is used where
/// the content itself is replaced, which is where an abrupt swap is felt.
/// `App::reduce_motion` is honoured by GPUI itself.
pub fn fade_in(content: Div, key: impl Into<ElementId>) -> AnimationElement<Div> {
    content.with_animation(
        key,
        Animation::new(MotionTokens::BASE).with_easing(MotionTokens::enter_easing()),
        |content, delta| content.opacity(delta),
    )
}
