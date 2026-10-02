//! Small layout patterns shared by every screen.
//!
//! Each one used to be re-typed per screen with slightly different sizes and
//! colours (a count chip existed in four paddings, a selected row in three
//! recipes). Screens compose these instead, so a panel reads the same in
//! Projects, Revisão and Decisões.

use gpui::prelude::*;
use gpui::{
    deferred, div, px, Animation, AnimationElement, AnimationExt, AnyElement, Div, ElementId, Rgba,
    Role, SharedString, SpringAnimation, Stateful, Toggled,
};

use crate::ui::icons::{icon, IconName};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{MotionTokens, RadiusScale, SpacingScale, TypeScale, TypeToken};

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

/// The reading column width shared by Revisão, Decisões and the editors.
pub const READING_WIDTH: f32 = 760.0;

/// A scrollable page with the reading column centred in it.
pub fn reading_page(id: impl Into<ElementId>, column: impl IntoElement) -> Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .px(px(SpacingScale::S8))
        .py(px(SpacingScale::S8))
        .child(
            div()
                .w_full()
                .max_w(px(READING_WIDTH))
                .mx_auto()
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S6))
                .child(column),
        )
}

/// A labelled form field: section label above the control, optional hint.
pub fn form_field(
    theme: &Theme,
    label: &str,
    hint: Option<&str>,
    control: impl IntoElement,
) -> Div {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .child(section_label(theme, label))
        .child(control)
        .when_some(hint, |field, hint| {
            field.child(
                text_style(div(), TypeScale::META)
                    .text_color(theme.colors.text_muted())
                    .child(hint.to_owned()),
            )
        })
}

/// The bar that holds a surface's actions, pinned under its scrolling
/// content: Revisão's review actions and both editors share it. A message on
/// the left explains why the primary action is unavailable.
pub fn action_footer(theme: &Theme, message: Option<(&str, bool)>) -> Div {
    div()
        .flex_none()
        .px(px(SpacingScale::S8))
        .py(px(SpacingScale::S3))
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(SpacingScale::S2))
        .border_t_1()
        .border_color(theme.colors.hairline_divider())
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .flex_1()
                .min_w(px(0.0))
                .when_some(message, |text, (message, danger)| {
                    text.text_color(if danger {
                        theme.colors.status_danger()
                    } else {
                        theme.colors.text_muted()
                    })
                    .child(message.to_owned())
                }),
        )
}

/// How long a confirmation toast stays on screen.
pub const TOAST_DURATION: std::time::Duration = std::time::Duration::from_millis(3500);

/// A confirmation that floats over the bottom of a surface and leaves on its
/// own. The parent must be `relative()`; `bottom` clears its action footer.
pub fn toast(theme: &Theme, message: &str, bottom: f32) -> AnyElement {
    let pill = div()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .px(px(SpacingScale::S4))
        .py(px(SpacingScale::S2))
        .rounded(theme.radius.surface())
        .border_1()
        .border_color(theme.colors.hairline_divider())
        .bg(theme.colors.floating())
        .shadow(vec![gpui::BoxShadow::new(
            px(0.0),
            px(8.0),
            theme.colors.shadow_emphasis().into(),
        )
        .blur_radius(px(24.0))])
        .child(icon(
            IconName::CheckCircle,
            14.0,
            theme.colors.status_success(),
        ))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_primary())
                .child(message.to_owned()),
        );
    deferred(
        div()
            .id("toast")
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(bottom))
            .flex()
            .justify_center()
            .role(gpui::Role::Status)
            .aria_label(message.to_owned())
            .child(fade_in(
                pill,
                ElementId::Name(format!("toast-{message}").into()),
            )),
    )
    .with_priority(2)
    .into_any_element()
}

/// A recoverable failure pinned to the top of a surface. The caller appends
/// its retry action; the message is product language, never the raw error.
pub fn error_banner(theme: &Theme, message: &str) -> Div {
    div()
        .flex_none()
        .px(px(SpacingScale::S4))
        .py(px(SpacingScale::S2))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .bg(theme.colors.danger_tint())
        .border_b_1()
        .border_color(theme.colors.hairline_divider())
        .child(
            div()
                .size(px(6.0))
                .flex_none()
                .rounded_full()
                .bg(theme.colors.status_danger()),
        )
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .flex_1()
                .min_w(px(0.0))
                .text_color(theme.colors.text_primary())
                .child(message.to_owned()),
        )
}

/// Placeholder rows while a list loads: the shape of what is coming, softly
/// breathing, instead of the word "Carregando". Honours reduced motion
/// through GPUI's animation element.
pub fn skeleton_list(theme: &Theme, id: &'static str, rows: usize) -> AnyElement {
    let bar = |width: f32, height: f32| {
        div()
            .h(px(height))
            .w(gpui::relative(width))
            .rounded(px(3.0))
            .bg(theme.colors.surface())
    };
    div()
        .id(id)
        .w_full()
        .flex()
        .flex_col()
        .role(gpui::Role::Status)
        .aria_label("Carregando")
        .children((0..rows).map(|row| {
            let wide = [0.86, 0.72, 0.8, 0.64][row % 4];
            div()
                .px(px(SpacingScale::S4))
                .py(px(SpacingScale::S3))
                .flex()
                .flex_col()
                .gap(px(SpacingScale::S2))
                .child(bar(0.28, 8.0))
                .child(bar(wide, 10.0))
                .child(bar(wide - 0.2, 8.0))
        }))
        .with_animation(
            ElementId::NamedInteger(id.into(), 1),
            Animation::new(std::time::Duration::from_millis(1400))
                .repeat()
                .with_easing(gpui::ease_in_out),
            |list, delta| list.opacity(0.55 + 0.45 * (1.0 - (2.0 * delta - 1.0).abs())),
        )
        .into_any_element()
}

/// The empty state of a reading surface: a quiet mark, what this place is,
/// and what makes it fill. No card, no glow.
pub fn empty_panel(theme: &Theme, glyph: IconName, eyebrow: &str, title: &str, body: &str) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .max_w(px(420.0))
                .px(px(SpacingScale::S6))
                .flex()
                .flex_col()
                .items_start()
                .gap(px(SpacingScale::S3))
                .child(
                    div()
                        .size(px(40.0))
                        .mb(px(SpacingScale::S2))
                        .rounded(theme.radius.surface())
                        .border_1()
                        .border_color(theme.colors.hairline_divider())
                        .bg(theme.colors.surface())
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(icon(glyph, 20.0, theme.colors.text_secondary())),
                )
                .child(section_label(theme, eyebrow))
                .child(
                    text_style(div(), TypeScale::HEADING_1)
                        .text_color(theme.colors.text_primary())
                        .child(title.to_owned()),
                )
                .child(
                    text_style(div(), TypeScale::BODY)
                        .text_color(theme.colors.text_secondary())
                        .child(body.to_owned()),
                ),
        )
}

/// A keyboard shortcut hint placed inside a button, tinted like its label.
pub fn kbd(color: Rgba, key: &'static str) -> Div {
    text_style(div(), TypeScale::META)
        .flex_none()
        .min_w(px(16.0))
        .px(px(4.0))
        .rounded(px(3.0))
        .border_1()
        .border_color(color.alpha(0.28))
        .text_color(color.alpha(0.72))
        .flex()
        .justify_center()
        .child(key)
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

/// A kind or a verb ("Regra", "Restrição", "depende de"): the pill's shape
/// without the dot. The dot belongs to [`status_pill`] and means a state.
pub fn tag(theme: &Theme, label: impl Into<SharedString>) -> Div {
    text_style(div(), TypeScale::META)
        .flex_none()
        .px(px(SpacingScale::S2))
        .py(px(2.0))
        .rounded(px(4.0))
        .bg(theme.colors.surface())
        .text_color(theme.colors.text_secondary())
        .child(label.into())
}

/// A list of exclusive options: rows divided by hairlines inside one
/// bordered list. Options are never cards side by side.
pub fn radio_list(theme: &Theme, id: impl Into<ElementId>) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .rounded(RadiusScale.surface())
        .border_1()
        .border_color(theme.colors.glass_border_card())
        .overflow_hidden()
        .role(Role::RadioGroup)
}

/// One option of a [`radio_list`]: glyph, title, one line of description and
/// the radio mark at the right; the selected row takes `mark_selected`. The
/// caller adds the id, focus and click handler.
pub fn radio_row(
    theme: &Theme,
    id: impl Into<ElementId>,
    selected: bool,
    divided: bool,
    glyph: IconName,
    title: &'static str,
    body: &'static str,
) -> Stateful<Div> {
    let colors = theme.colors;
    mark_selected(
        div()
            .id(id)
            .relative()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S3))
            .px(px(SpacingScale::S4))
            .py(px(SpacingScale::S3))
            .when(divided, |row| {
                row.border_t_1().border_color(colors.hairline_divider())
            })
            .cursor_pointer()
            .role(Role::RadioButton)
            .aria_label(title)
            .aria_toggled(if selected {
                Toggled::True
            } else {
                Toggled::False
            }),
        theme,
        selected,
    )
    .child(icon(
        glyph,
        16.0,
        if selected {
            colors.text_primary()
        } else {
            colors.text_muted()
        },
    ))
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
                    .text_color(colors.text_muted())
                    .child(body),
            ),
    )
    .child(
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
            }),
    )
}

/// Width of the index rail beside a reading column (Mapa, Contexto).
pub const INDEX_WIDTH: f32 = 296.0;

/// The index rail: a fixed-width pane at the left of a destination, with a
/// hairline on its right edge.
pub fn index_rail(theme: &Theme) -> Div {
    div()
        .w(px(INDEX_WIDTH))
        .flex_none()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme.colors.pane())
        .border_r_1()
        .border_color(theme.colors.hairline_divider())
}

/// One entry of an [`index_rail`]: glyph, label and an optional count,
/// selected with [`mark_selected`]. The caller adds focus and handlers.
pub fn index_row(
    theme: &Theme,
    id: impl Into<ElementId>,
    selected: bool,
    glyph: IconName,
    label: &'static str,
    badge: Option<String>,
) -> Stateful<Div> {
    let colors = theme.colors;
    let row = div()
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S3))
        .px(px(SpacingScale::S3))
        .py(px(SpacingScale::S2))
        .rounded(theme.radius.control())
        .cursor_pointer()
        .role(gpui::Role::Button)
        .aria_label(label)
        .focus_visible(crate::ui::controls::focus_ring(theme))
        .when(!selected, |row| {
            row.hover(move |style| style.bg(colors.glass_fill_medium()))
                .active(move |style| style.bg(colors.glass_fill_strong()))
        })
        .child(icon(
            glyph,
            16.0,
            if selected {
                colors.text_primary()
            } else {
                colors.text_muted()
            },
        ))
        .child(
            text_style(div(), TypeScale::ROW_TITLE)
                .flex_1()
                .text_color(if selected {
                    colors.text_primary()
                } else {
                    colors.text_secondary()
                })
                .child(label),
        )
        .children(badge.map(|badge| count_chip(theme, badge)));
    mark_selected(row, theme, selected)
}

/// The track of a segmented switch between views of the same content
/// ("Blocos | Grafo"): a recessed pill holding [`segment`]s.
pub fn segmented(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_none()
        .p(px(2.0))
        .gap(px(2.0))
        .rounded(px(8.0))
        .bg(theme.colors.canvas_deep())
        .border_1()
        .border_color(theme.colors.hairline_divider())
}

/// One option of a [`segmented`] switch: glyph and label; the chosen one is
/// raised on `glass_fill_medium`, the others stay quiet until hovered. The
/// caller adds focus tracking and the press handlers.
pub fn segment(
    theme: &Theme,
    id: impl Into<ElementId>,
    glyph: IconName,
    label: &'static str,
    selected: bool,
) -> Stateful<Div> {
    let colors = theme.colors;
    let hover = colors.glass_fill_low();
    let text = if selected {
        colors.text_primary()
    } else {
        colors.text_muted()
    };
    text_style(div(), TypeScale::LABEL)
        .id(id)
        .flex()
        .items_center()
        .gap(px(6.0))
        .h(px(26.0))
        .px(px(SpacingScale::S3))
        .rounded(px(6.0))
        .cursor_pointer()
        .text_color(text)
        .when(selected, |item| item.bg(colors.glass_fill_medium()))
        .when(!selected, |item| item.hover(move |style| style.bg(hover)))
        .role(gpui::Role::Tab)
        .aria_label(label)
        .focus_visible(crate::ui::controls::focus_ring(theme))
        .child(icon(glyph, 13.0, text))
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

/// The heading of a section in a reading page: the uppercase label and, at
/// the right, whatever the caller adds (a count, one quiet action). Every
/// section of every reading page uses this one shape; there is no second,
/// bolder heading with an icon.
pub fn section_header(theme: &Theme, label: &str) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .min_h(px(20.0))
        .child(section_label(theme, label).flex_1())
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
    crate::ui::motion::content_in(key, content)
}
