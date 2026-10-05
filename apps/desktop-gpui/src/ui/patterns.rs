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

use crate::i18n::common as t;
use crate::ui::icons::{icon, IconName};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{
    ControlSize, MotionTokens, RadiusScale, SpacingScale, TypeScale, TypeToken,
};

/// The quiet title of a side panel: 12 px, muted. The content, not the
/// panel name, carries the weight.
pub fn panel_title(theme: &Theme, label: impl Into<SharedString>) -> Div {
    text_style(div(), TypeScale::LABEL)
        .text_color(theme.colors.text_muted())
        .child(label.into())
}

/// Figures in equal-width digits (`tnum`), so a count that changes from 9
/// to 10 or from 1 to 8 never shifts what follows it.
pub fn tabular<E: Styled>(element: E) -> E {
    element.font_features(gpui::FontFeatures(std::sync::Arc::new(vec![(
        "tnum".into(),
        1,
    )])))
}

/// A figure that counts up to `value` when it first appears (and again when
/// it changes), in equal-width digits.
pub fn count_up(id: impl Into<SharedString>, value: usize, element: Div) -> AnimationElement<Div> {
    let key = ElementId::Name(format!("{}-{value}", id.into()).into());
    tabular(element).with_animation(
        key,
        Animation::new(MotionTokens::SLOW * 2).with_easing(MotionTokens::enter_easing()),
        move |element, t| element.child(((value as f32 * t).round() as usize).to_string()),
    )
}

/// A sentence with some words in the strong face: `(text, strong)` parts,
/// laid out word by word (like [`word_wrapped`]) so it wraps cleanly. Used to
/// say what a suggestion is about with the names that matter standing out.
pub fn rich_sentence(theme: &Theme, parts: &[(String, bool)], token: TypeToken) -> Div {
    let colors = theme.colors;
    // Words with their face; punctuation that opens a part sticks to the
    // word before it instead of standing apart.
    let mut flow: Vec<(String, bool)> = Vec::new();
    for (text, strong) in parts {
        for (position, word) in text.split_whitespace().enumerate() {
            let sticks = position == 0
                && !flow.is_empty()
                && word.starts_with(['.', ',', ';', ':', ')', '?', '!']);
            if sticks {
                if let Some((last, _)) = flow.last_mut() {
                    last.push_str(word);
                }
            } else {
                flow.push((word.to_owned(), *strong));
            }
        }
    }
    let words = flow.into_iter().map(|(word, strong)| {
        let word = div().flex_none().child(word);
        if strong {
            word.font_weight(gpui::FontWeight::MEDIUM)
                .text_color(colors.text_primary())
        } else {
            word.text_color(colors.text_secondary())
        }
    });
    text_style(div(), token)
        .w_full()
        .min_w(px(0.0))
        .flex()
        .flex_wrap()
        .gap_x(px(token.size * 0.27))
        .children(words)
}

/// A suggestion the person can accept or reject, written to be understood
/// without opening anything else: what it is (`lead`, a sentence), the text
/// that gave rise to it, and what confirming changes. The buttons come from
/// the screen (`actions`), which owns their focus and what they do.
pub fn suggestion_card(
    theme: &Theme,
    lead: impl IntoElement,
    evidence: Option<String>,
    effect: &str,
    actions: impl IntoElement,
) -> Div {
    let colors = theme.colors;
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S3))
        .p(px(SpacingScale::S4))
        .rounded(RadiusScale.surface())
        .border_1()
        .border_color(colors.glass_border_card())
        .bg(colors.glass_fill_card())
        .child(lead)
        .when_some(evidence, |card, quote| {
            card.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(
                        text_style(div(), TypeScale::META)
                            .text_color(colors.text_muted())
                            .child(t::suggestion_source()),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .pl(px(SpacingScale::S3))
                            .border_l_2()
                            .border_color(colors.hairline_divider())
                            .text_color(colors.text_secondary())
                            .child(format!("“{quote}”")),
                    ),
            )
        })
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .mt(px(6.0))
                        .size(px(6.0))
                        .flex_none()
                        .rounded_full()
                        .bg(colors.accent_default()),
                )
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .flex_1()
                        .min_w(px(0.0))
                        .text_color(colors.text_secondary())
                        .child(t::on_confirm(effect)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(SpacingScale::S2))
                .child(actions),
        )
}

/// A section of suggestions: its label, a sentence saying what the section
/// is and what confirming does there, then the cards.
pub fn suggestion_section(theme: &Theme, label: &str, intro: &str, cards: Div) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S3))
        .child(section_label(theme, label))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child(intro.to_owned()),
        )
        .child(cards)
}

/// A small numeric chip: tab badges, panel counts, versions.
pub fn count_chip(theme: &Theme, value: impl Into<SharedString>) -> Div {
    tabular(text_style(div(), TypeScale::META))
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
    toast_with(theme, message, bottom, None)
}

/// A [`toast`] with one action at its end ("Desfazer"), for what a person
/// can take back: the confirmation says what happened and the way out is one
/// press away, so nothing has to ask "tem certeza?" first.
pub fn toast_with(
    theme: &Theme,
    message: &str,
    bottom: f32,
    action: Option<AnyElement>,
) -> AnyElement {
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
        .shadow(crate::ui::material::elevation(
            theme,
            crate::ui::material::Elevation::Hint,
        ))
        .child(icon(
            IconName::CheckCircle,
            14.0,
            theme.colors.status_success(),
        ))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_primary())
                .child(message.to_owned()),
        )
        .children(action);
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
        .aria_label(t::loading())
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
    empty_panel_with(
        theme,
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
        eyebrow,
        title,
        body,
        None,
    )
}

/// [`empty_panel`] with the actions that get out of the state, left-aligned
/// under the text: the blocking surface that has no screen behind it (the
/// startup failure), where "nothing to show" still needs a real way forward.
pub fn empty_panel_actions(
    theme: &Theme,
    glyph: IconName,
    eyebrow: &str,
    title: &str,
    body: &str,
    actions: impl IntoElement,
) -> Div {
    empty_panel_with(
        theme,
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
        eyebrow,
        title,
        body,
        Some(actions.into_any_element()),
    )
}

/// [`empty_panel`] with the mascot in the glyph's place: the resting pose
/// (eyes closed) when there is simply nothing to do, the lit one when the
/// surface waits for the person's first step.
pub fn empty_panel_mascot(
    theme: &Theme,
    figure: std::sync::Arc<gpui::Image>,
    eyebrow: &str,
    title: &str,
    body: &str,
) -> Div {
    empty_panel_with(
        theme,
        gpui::img(figure).size(px(96.0)).ml(px(-SpacingScale::S3)),
        eyebrow,
        title,
        body,
        None,
    )
}

fn empty_panel_with(
    theme: &Theme,
    figure: impl IntoElement,
    eyebrow: &str,
    title: &str,
    body: &str,
    actions: Option<AnyElement>,
) -> Div {
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
                .child(figure)
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
                )
                .children(actions.map(|actions| div().mt(px(SpacingScale::S2)).child(actions))),
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

/// Tallest a [`menu_panel`] grows before its options scroll.
pub const MENU_MAX_HEIGHT: f32 = 320.0;

/// A dropdown menu under the control that opened it: the floating surface
/// of the quick theme menu (`floating`, card border, floating elevation),
/// with the options scrolling past [`MENU_MAX_HEIGHT`]. The caller places it
/// (absolute, under its trigger), adds the click-outside and key handlers,
/// wraps it in `menu_in`/`menu_out` and fills it with [`menu_item`]s.
pub fn menu_panel(theme: &Theme, id: impl Into<ElementId>, label: &str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .p(px(SpacingScale::S1))
        .max_h(px(MENU_MAX_HEIGHT))
        .overflow_y_scroll()
        .rounded(theme.radius.surface())
        .border_1()
        .border_color(theme.colors.glass_border_card())
        .bg(theme.colors.floating())
        .shadow(crate::ui::material::elevation(
            theme,
            crate::ui::material::Elevation::Floating,
        ))
        .occlude()
        .role(Role::Menu)
        .aria_label(label.to_owned())
}

/// One exclusive option of a [`menu_panel`]: glyph, label, an optional count
/// and the check of the chosen one. Hover tints it; the chosen option reads
/// in the primary text with a lavender check (no second selection fill in a
/// floating surface). The caller adds focus, click and key handlers.
pub fn menu_item(
    theme: &Theme,
    id: impl Into<ElementId>,
    glyph: IconName,
    label: &str,
    count: Option<usize>,
    selected: bool,
) -> Stateful<Div> {
    let colors = theme.colors;
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(SpacingScale::S3))
        .h(px(ControlSize::MD))
        .px(px(SpacingScale::S2))
        .rounded(theme.radius.control())
        .cursor_pointer()
        .hover(move |style| style.bg(colors.glass_fill_medium()))
        .active(move |style| style.bg(colors.glass_fill_strong()))
        .focus_visible(crate::ui::controls::focus_ring(theme))
        .role(Role::MenuItemRadio)
        .aria_label(match count {
            Some(count) => format!("{label}, {count}"),
            None => label.to_owned(),
        })
        .aria_toggled(if selected {
            Toggled::True
        } else {
            Toggled::False
        })
        .child(icon(
            glyph,
            14.0,
            if selected {
                colors.text_primary()
            } else {
                colors.text_muted()
            },
        ))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .flex_1()
                .min_w(px(0.0))
                .truncate()
                .text_color(if selected {
                    colors.text_primary()
                } else {
                    colors.text_secondary()
                })
                .child(label.to_owned()),
        )
        .children(count.map(|count| count_chip(theme, count.to_string())))
        .child(div().size(px(14.0)).flex_none().when(selected, |mark| {
            mark.child(icon(IconName::Check, 14.0, colors.accent_hover()))
        }))
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

/// A [`segmented`] option that is only a word (levels, sizes), where a
/// glyph would say nothing.
pub fn segment_label(
    theme: &Theme,
    id: impl Into<ElementId>,
    label: &'static str,
    selected: bool,
) -> Stateful<Div> {
    let colors = theme.colors;
    let hover = colors.glass_fill_low();
    text_style(div(), TypeScale::LABEL)
        .id(id)
        .flex()
        .items_center()
        .h(px(26.0))
        .px(px(SpacingScale::S3))
        .rounded(px(6.0))
        .cursor_pointer()
        .text_color(if selected {
            colors.text_primary()
        } else {
            colors.text_muted()
        })
        .when(selected, |item| item.bg(colors.glass_fill_medium()))
        .when(!selected, |item| item.hover(move |style| style.bg(hover)))
        .role(gpui::Role::RadioButton)
        .aria_label(label)
        .focus_visible(crate::ui::controls::focus_ring(theme))
        .child(label)
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

// ---------------------------------------------------------------------------
// Forms for figures: a number worth showing is worth drawing. Each is a few
// pixels of canvas, so a list can carry one per row at no cost, and each
// needs the number or a sentence beside it (or an `aria_label` on its parent)
// for whoever does not see it.
// ---------------------------------------------------------------------------

/// Share of `value` in `of`, kept in 0 to 1 (an empty whole is 0).
pub fn share(value: usize, of: usize) -> f32 {
    if of == 0 {
        0.0
    } else {
        (value as f32 / of as f32).clamp(0.0, 1.0)
    }
}

/// A thin bar filled to `fraction` of its width: a quantity against a limit.
pub fn meter(theme: &Theme, fraction: f32, color: Rgba) -> Div {
    div()
        .h(px(3.0))
        .w_full()
        .rounded_full()
        .overflow_hidden()
        .bg(theme.colors.glass_fill_medium())
        .child(
            div()
                .h_full()
                .w(gpui::relative(fraction.clamp(0.0, 1.0)))
                .rounded_full()
                .bg(color),
        )
}

/// A thin bar split in parts by weight: what a whole is made of. Parts with
/// no weight take no room.
pub fn meter_stack(theme: &Theme, parts: &[(usize, Rgba)]) -> Div {
    let total: usize = parts.iter().map(|(weight, _)| weight).sum();
    let bar = div()
        .h(px(6.0))
        .w_full()
        .flex()
        .rounded_full()
        .overflow_hidden()
        .bg(theme.colors.glass_fill_medium());
    parts
        .iter()
        .filter(|(weight, _)| *weight > 0)
        .fold(bar, |bar, (weight, color)| {
            bar.child(
                div()
                    .h_full()
                    .w(gpui::relative(share(*weight, total)))
                    .border_r_1()
                    .border_color(theme.colors.canvas_raised())
                    .bg(*color),
            )
        })
}

/// Where the points of a sparkline sit in a `width` by `height` box: evenly
/// spaced, the largest value at the top, zero at the bottom (a flat series of
/// zeros lies on the bottom).
pub fn spark_points(values: &[f32], width: f32, height: f32) -> Vec<(f32, f32)> {
    let top = values
        .iter()
        .copied()
        .fold(0.0_f32, f32::max)
        .max(f32::EPSILON);
    let last = values.len().saturating_sub(1).max(1) as f32;
    values
        .iter()
        .enumerate()
        .map(|(at, value)| {
            (
                width * at as f32 / last,
                height - (height - 2.0) * (value / top).clamp(0.0, 1.0) - 1.0,
            )
        })
        .collect()
}

/// A line of values in a small box with the area under it tinted and a dot on
/// the last one: a trend, not a chart (no axes). `values` are oldest first.
pub fn sparkline(values: &[f32], width: f32, height: f32, color: Rgba) -> impl IntoElement {
    let points = spark_points(values, width, height);
    let color: gpui::Hsla = color.into();
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let at = |(x, y): (f32, f32)| gpui::point(px(ox + x), px(oy + y));
            let Some(&last) = points.last() else {
                return;
            };
            if points.len() > 1 {
                let mut area = gpui::PathBuilder::fill();
                let mut outline: Vec<_> = points.iter().map(|point| at(*point)).collect();
                outline.push(at((last.0, height)));
                outline.push(at((0.0, height)));
                area.add_polygon(&outline, true);
                if let Ok(path) = area.build() {
                    window.paint_path(path, color.opacity(0.14));
                }
                let mut line = gpui::PathBuilder::stroke(px(1.5));
                line.move_to(at(points[0]));
                for point in &points[1..] {
                    line.line_to(at(*point));
                }
                if let Ok(path) = line.build() {
                    window.paint_path(path, color);
                }
            }
            let dot: Vec<_> = (0..12)
                .map(|step| {
                    let angle = step as f32 / 12.0 * std::f32::consts::TAU;
                    at((last.0 + angle.cos() * 2.4, last.1 + angle.sin() * 2.4))
                })
                .collect();
            let mut head = gpui::PathBuilder::fill();
            head.add_polygon(&dot, true);
            if let Ok(path) = head.build() {
                window.paint_path(path, color);
            }
        },
    )
    .w(px(width))
    .h(px(height))
}

/// The points of an arc that starts at the top and runs clockwise over
/// `fraction` of a circle of `radius` around (`cx`, `cy`).
pub fn arc_points(cx: f32, cy: f32, radius: f32, fraction: f32) -> Vec<(f32, f32)> {
    let sweep = fraction.clamp(0.0, 1.0) * std::f32::consts::TAU;
    let steps = ((sweep / std::f32::consts::TAU) * 48.0).ceil().max(1.0) as usize;
    (0..=steps)
        .map(|step| {
            let angle = -std::f32::consts::FRAC_PI_2 + sweep * step as f32 / steps as f32;
            (cx + radius * angle.cos(), cy + radius * angle.sin())
        })
        .collect()
}

/// A ring filled clockwise to `fraction`: one quantity against a limit where
/// a bar would be too long. `side` is its outer size in pixels.
pub fn ring(theme: &Theme, fraction: f32, side: f32, color: Rgba) -> impl IntoElement {
    let track: gpui::Hsla = theme.colors.surface().into();
    let color: gpui::Hsla = color.into();
    let stroke = (side / 7.0).max(2.5);
    let radius = (side - stroke) / 2.0;
    gpui::canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let centre = side / 2.0;
            let draw = |fraction: f32, colour: gpui::Hsla, window: &mut gpui::Window| {
                let points = arc_points(centre, centre, radius, fraction);
                let mut path = gpui::PathBuilder::stroke(px(stroke));
                if let Some(&(x, y)) = points.first() {
                    path.move_to(gpui::point(px(ox + x), px(oy + y)));
                }
                for (x, y) in points.iter().skip(1) {
                    path.line_to(gpui::point(px(ox + x), px(oy + y)));
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, colour);
                }
            };
            draw(1.0, track, window);
            if fraction > 0.0 {
                draw(fraction, color, window);
            }
        },
    )
    .w(px(side))
    .h(px(side))
}

#[cfg(test)]
mod form_tests {
    use super::*;

    #[test]
    fn share_is_kept_between_zero_and_one() {
        assert_eq!(share(1, 4), 0.25);
        assert_eq!(share(9, 4), 1.0);
        assert_eq!(share(3, 0), 0.0);
    }

    #[test]
    fn a_sparkline_puts_the_largest_value_on_top_and_zero_at_the_bottom() {
        let points = spark_points(&[0.0, 5.0, 10.0], 60.0, 20.0);
        assert_eq!(points.len(), 3);
        assert!(points[2].1 < points[1].1 && points[1].1 < points[0].1);
        assert_eq!((points[0].0, points[2].0), (0.0, 60.0));
        assert!(points.iter().all(|(_, y)| (0.0..=20.0).contains(y)));
        // All zeros lie flat on the bottom; one value does not divide by zero.
        assert!(spark_points(&[0.0, 0.0], 10.0, 10.0)
            .iter()
            .all(|(_, y)| *y > 8.0));
        assert_eq!(spark_points(&[3.0], 10.0, 10.0).len(), 1);
        assert!(spark_points(&[], 10.0, 10.0).is_empty());
    }

    #[test]
    fn a_ring_starts_at_the_top_and_closes_at_a_full_turn() {
        let quarter = arc_points(10.0, 10.0, 8.0, 0.25);
        let (x, y) = quarter[0];
        assert!((x - 10.0).abs() < 0.01 && (y - 2.0).abs() < 0.01, "top");
        let (x, y) = *quarter.last().unwrap();
        assert!((x - 18.0).abs() < 0.01 && (y - 10.0).abs() < 0.01, "right");
        let full = arc_points(10.0, 10.0, 8.0, 1.0);
        let (a, b) = (full[0], *full.last().unwrap());
        assert!((a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01);
    }
}
