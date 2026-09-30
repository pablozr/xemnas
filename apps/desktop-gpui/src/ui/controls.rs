//! Interactive Quiet Glass controls.
//!
//! Every button in the product comes from [`action_button`] or
//! [`icon_action`]; views attach behaviour (`track_focus`, `on_click`) but
//! never re-declare height, radius, colour or the focus ring.

use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, Rgba, Role, Stateful};

use crate::ui::glass::focus_ring;
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
    match kind {
        ButtonKind::Primary if enabled => button
            .bg(colors.accent_emphasis())
            .hover(move |style| style.bg(colors.accent_hover())),
        ButtonKind::Primary => button.bg(colors.surface()),
        ButtonKind::Secondary => button
            .border_1()
            .border_color(colors.hairline_divider())
            .hover(move |style| style.bg(colors.hover_veil())),
        ButtonKind::Ghost => button.hover(move |style| style.bg(colors.hover_veil())),
    }
}

/// A square icon-only ghost action (`control.sm`), for title and list headers.
pub fn icon_action(theme: &Theme, id: impl Into<ElementId>, aria_label: &str) -> Stateful<Div> {
    let hover = theme.colors.hover_veil();
    div()
        .id(id)
        .size(px(ControlSize::SM))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(theme.radius.control())
        .hover(move |style| style.bg(hover))
        .role(Role::Button)
        .aria_label(aria_label.to_string())
        .focus_visible(focus_ring(theme))
        .cursor_pointer()
}
