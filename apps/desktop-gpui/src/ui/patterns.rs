//! Small layout patterns shared by every screen.
//!
//! Each one used to be re-typed per screen with slightly different sizes and
//! colours (a count chip existed in four paddings, a selected row in three
//! recipes). Screens compose these instead, so a panel reads the same in
//! Projects, Revisão and Decisões.

use gpui::prelude::*;
use gpui::{
    div, px, Animation, AnimationElement, AnimationExt, AnyElement, Div, ElementId, Rgba,
    SharedString, SpringAnimation, Stateful,
};

use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{MotionTokens, SpacingScale, TypeScale, TypeToken};

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

/// Text laid out word by word, so punctuation stays with its word.
///
/// GPUI's line wrapper treats `?` as a break opportunity (for URLs), which left
/// a lone "?" on the last line of a question. Each word here is one flex item,
/// so "captura?" wraps as a unit. `max_lines` clips extra lines.
pub fn word_wrapped(text: &str, token: TypeToken, max_lines: Option<usize>) -> Div {
    let words = text
        .split_whitespace()
        .map(|word| div().flex_none().child(word.to_owned()));
    text_style(div(), token)
        .w_full()
        .min_w(px(0.0))
        .flex()
        .flex_wrap()
        .gap_x(px(token.size * 0.27))
        .when_some(max_lines, |text, lines| {
            text.max_h(px(token.line_height * lines as f32))
                .overflow_hidden()
        })
        .children(words)
}

/// The title of a reading pane: display face, word-wrapped.
pub fn reading_title(text: &str) -> Div {
    // 500 is a named instance of the variable face; an in-between weight such
    // as 560 made the text system fall back to the interface family.
    word_wrapped(text, TypeScale::DISPLAY, None)
        .font_family(Theme::font_display())
        .font_weight(gpui::FontWeight::MEDIUM)
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

/// Records which item is under the pointer. Returns whether it changed, so
/// callers only re-render when the hover target really moved.
pub fn track_hover<T: PartialEq>(slot: &mut Option<T>, key: T, hovered: bool) -> bool {
    if hovered {
        if slot.as_ref() == Some(&key) {
            return false;
        }
        *slot = Some(key);
        true
    } else if slot.as_ref() == Some(&key) {
        *slot = None;
        true
    } else {
        false
    }
}

/// Eases a row's hover tint in and out with a spring instead of snapping.
///
/// GPUI's `.hover()` style swaps instantly; the spring keeps its state under
/// `key`, so the tint animates toward `hovered` on every change. Selected rows
/// keep their own fill (`enabled == false`).
pub fn hover_tint(
    row: Stateful<Div>,
    key: impl Into<ElementId>,
    hovered: bool,
    enabled: bool,
    theme: &Theme,
) -> AnyElement {
    let hover = theme.colors.hover_veil();
    let rest = hover.alpha(0.0);
    row.with_spring(
        key,
        SpringAnimation::new(MotionTokens::HOVER_SPRING).to(hovered && enabled),
        move |row, phase| {
            if enabled {
                row.bg(phase.interpolate_between_clamped(0.0..=1.0, rest, hover))
            } else {
                row
            }
        },
    )
    .into_any_element()
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
