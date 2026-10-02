//! Interactive Quiet Glass controls.
//!
//! Every button in the product comes from [`action_button`] or
//! [`icon_action`]; views attach behaviour (`track_focus`, `on_click`) but
//! never re-declare height, radius, colour or the focus ring.

use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, Rgba, Role, Stateful};

pub use crate::ui::glass::focus_ring;
use crate::ui::material::{accent_plate, accent_pressed, neutral_plate};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{ControlSize, SpacingScale, TypeScale};

/// The three product button weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// The one real primary action in a region: solid lavender.
    Primary,
    /// A peer action that needs an edge to read as a control.
    Secondary,
    /// Toolbar and inline actions: no container until hovered or selected.
    Ghost,
}

/// The text (and icon) colour of a product button, so icons placed inside it
/// always match the label, including the disabled state.
pub fn button_foreground(theme: &Theme, kind: ButtonKind, enabled: bool) -> Rgba {
    let colors = theme.colors;
    match (kind, enabled) {
        (_, false) => colors.text_disabled(),
        (ButtonKind::Primary, true) => colors.accent_on_emphasis(),
        (ButtonKind::Secondary, true) => colors.text_primary(),
        (ButtonKind::Ghost, true) => colors.text_secondary(),
    }
}

/// The product button recipe shared by every screen.
///
/// Screens attach focus, handlers and children; height, radius, colours,
/// hover and the focus ring come from here so a toolbar mixing kinds keeps
/// one baseline. `enabled == false` keeps the control focusable but greys its
/// content, which is how the screens already signal a running operation.
pub fn action_button(
    theme: &Theme,
    id: impl Into<ElementId>,
    kind: ButtonKind,
    enabled: bool,
) -> Stateful<Div> {
    let colors = theme.colors;
    let button = text_style(div(), TypeScale::BODY_SMALL)
        .id(id)
        .h(px(ControlSize::MD))
        .px(px(SpacingScale::S3))
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.0))
        .rounded(theme.radius.control())
        .role(Role::Button)
        .focus_visible(focus_ring(theme))
        .cursor_pointer();
    let button = button.text_color(button_foreground(theme, kind, enabled));
    // GPUI has no style transitions, so each state has to read on its own:
    // hover lifts the fill and the edge, press settles one step deeper, and a
    // disabled control ignores both so it never looks clickable.
    match (kind, enabled) {
        (ButtonKind::Primary, true) => {
            // The material: lit gradient, rim, top highlight and a lavender
            // halo that spreads on hover (`ui::material`).
            let hover = accent_plate(theme, 1.0);
            let (hover_bg, hover_shadows) = (hover.background(), hover.shadows());
            let pressed = accent_pressed(theme);
            let (pressed_bg, pressed_shadows) = (pressed.background(), pressed.shadows());
            accent_plate(theme, 0.0)
                .apply(button)
                .hover(move |style| style.bg(hover_bg).shadow(hover_shadows))
                .active(move |style| style.bg(pressed_bg).shadow(pressed_shadows))
        }
        (ButtonKind::Primary, false) => button.bg(colors.surface()),
        (ButtonKind::Secondary, true) => {
            let hover = neutral_plate(theme, 1.0);
            let (hover_bg, hover_shadows) = (hover.background(), hover.shadows());
            neutral_plate(theme, 0.0)
                .apply(button)
                .hover(move |style| {
                    style
                        .bg(hover_bg)
                        .shadow(hover_shadows)
                        .border_color(colors.glass_border_card_hover())
                        .text_color(colors.text_primary())
                })
                .active(move |style| style.bg(colors.glass_fill_strong()).shadow(vec![]))
        }
        (ButtonKind::Secondary, false) => button.border_1().border_color(colors.hairline_divider()),
        (ButtonKind::Ghost, true) => button
            .hover(move |style| {
                style
                    .bg(colors.glass_fill_medium())
                    .text_color(colors.text_primary())
            })
            .active(move |style| style.bg(colors.glass_fill_strong())),
        (ButtonKind::Ghost, false) => button,
    }
}

/// A square icon-only ghost action (`control.sm`), for title and list headers.
pub fn icon_action(theme: &Theme, id: impl Into<ElementId>, aria_label: &str) -> Stateful<Div> {
    let hover = theme.colors.glass_fill_medium();
    let pressed = theme.colors.glass_fill_strong();
    div()
        .id(id)
        .size(px(ControlSize::SM))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(theme.radius.control())
        .hover(move |style| style.bg(hover))
        .active(move |style| style.bg(pressed))
        .role(Role::Button)
        .aria_label(aria_label.to_string())
        .focus_visible(focus_ring(theme))
        .cursor_pointer()
}
