//! Interactive Quiet Glass controls.
//!
//! Every control is a reusable recipe over [`crate::ui::glass`]; views attach
//! behaviour (`track_focus`, `on_click`) but never re-declare glass or color.
//! The `state` parameter renders each documented state explicitly so the
//! gallery can capture evidence for all of them. Real keyboard focus always
//! shows the focus-visible ring; the [`ControlState::FocusVisible`] state forces
//! the same ring statically for screenshots.

use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, Rgba, Role, Stateful};

use crate::ui::glass::{focus_ring as glass_focus_ring, GlassSurface, GlassVariant};
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{ControlSize, RadiusScale, SpacingScale, TypeScale};

/// The keyboard focus ring, re-exported so views can apply it to a control they
/// build themselves.
///
/// Home builds its own register button because it needs an icon inside the
/// emphasis fill, which `primary_button` does not offer; the ring is not
/// re-declared there, it is the same recipe.
pub fn focus_ring(theme: &Theme) -> impl FnOnce(gpui::StyleRefinement) -> gpui::StyleRefinement {
    glass_focus_ring(theme)
}

/// The interaction states every control implements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlState {
    /// At rest.
    Rest,
    /// Pointer hover.
    Hover,
    /// Keyboard focus, drawn with the focus-visible ring.
    FocusVisible,
    /// Pressed.
    Pressed,
    /// Unavailable.
    Disabled,
    /// Busy; only some controls support it.
    Loading,
}

impl ControlState {
    /// Whether the control responds to pointer interaction.
    pub fn is_interactive(&self) -> bool {
        !matches!(self, Self::Disabled | Self::Loading)
    }
}

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
    let button = match kind {
        ButtonKind::Primary if enabled => button
            .bg(colors.accent_emphasis())
            .text_color(colors.accent_on_emphasis())
            .hover(move |style| style.bg(colors.accent_hover())),
        ButtonKind::Primary => button
            .bg(colors.surface())
            .text_color(colors.text_disabled()),
        ButtonKind::Secondary => button
            .border_1()
            .border_color(colors.hairline_divider())
            .text_color(colors.text_primary())
            .hover(move |style| style.bg(colors.hover_veil())),
        ButtonKind::Ghost => button
            .text_color(colors.text_secondary())
            .hover(move |style| {
                style
                    .bg(colors.hover_veil())
                    .text_color(colors.text_primary())
            }),
    };
    if enabled || kind == ButtonKind::Primary {
        button
    } else {
        button.text_color(colors.text_disabled())
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

/// A 16 px outline icon placeholder.
///
/// The product has not chosen an icon family yet (design system: "escolher uma
/// única biblioteca e não misturar famílias"), so this neutral geometric shape
/// stands in until that decision; it intentionally is not an emoji.
fn icon_placeholder(color: Rgba) -> Div {
    div()
        .size(px(16.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(color)
}

fn state_text_color(theme: &Theme, state: ControlState) -> Rgba {
    match state {
        ControlState::Disabled => theme.colors.text_disabled(),
        _ => theme.colors.text_secondary(),
    }
}

/// The one primary action per screen (`Glass Emphasis`).
///
/// Height 44 px, horizontal padding 20 px. `loading` replaces the label with a
/// static busy marker (no perpetual shimmer).
pub fn primary_button(
    theme: &Theme,
    id: impl Into<ElementId>,
    state: ControlState,
    label: &str,
) -> Stateful<Div> {
    let surface = GlassSurface::new(GlassVariant::Emphasis);
    let foreground = surface.foreground(theme);
    let mut button = surface
        .render(theme)
        .id(id)
        .h(px(44.0))
        .px(px(SpacingScale::S5))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(SpacingScale::S2))
        .role(Role::Button)
        .aria_label(label.to_string())
        .focus_visible(focus_ring(theme))
        .cursor_pointer();

    button = match state {
        ControlState::Rest => button.hover(move |style| style.opacity(0.94)),
        ControlState::Hover => button.border_color(theme.colors.accent_subtle()),
        ControlState::Pressed => button.shadow_none().opacity(0.92),
        ControlState::FocusVisible => button
            .border_2()
            .border_color(theme.colors.accent_emphasis()),
        ControlState::Disabled => button.opacity(0.45),
        ControlState::Loading => button.opacity(0.7),
    };

    if state == ControlState::Disabled {
        button = button.text_color(theme.colors.text_disabled());
    } else {
        button = button.text_color(foreground);
    }

    let body = if state == ControlState::Loading {
        div()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .child(icon_placeholder(foreground))
            .child(text_style(div(), TypeScale::LABEL).child("Trabalhando…"))
    } else {
        text_style(div(), TypeScale::LABEL).child(label.to_string())
    };

    button.child(body)
}

/// A quiet action: icon + label, no visible container at rest.
///
/// Hover gets a light tint; `danger` only colors the content on hover.
pub fn quiet_button(
    theme: &Theme,
    id: impl Into<ElementId>,
    state: ControlState,
    label: &str,
    danger: bool,
) -> Stateful<Div> {
    let label_color = state_text_color(theme, state);
    // `status.danger` over `color.surface-hover` is ≈4.4:1, below the 4.5:1
    // text minimum, so danger only tints the icon (essential icons need 3:1).
    let danger_active = danger && matches!(state, ControlState::Hover | ControlState::Pressed);
    let icon_color = if danger_active {
        theme.colors.status_danger()
    } else {
        label_color
    };

    let mut button = div()
        .id(id)
        .h(px(SpacingScale::S10))
        .px(px(SpacingScale::S3))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .rounded(px(RadiusScale::CONTROL))
        .role(Role::Button)
        .aria_label(label.to_string())
        .text_color(label_color)
        .focus_visible(focus_ring(theme))
        .cursor_pointer();

    button = match state {
        ControlState::Hover | ControlState::Pressed => button.bg(theme.colors.surface_hover()),
        ControlState::FocusVisible => button
            .border_2()
            .border_color(theme.colors.accent_emphasis()),
        ControlState::Disabled => button.opacity(0.45),
        ControlState::Loading => button.opacity(0.7),
        ControlState::Rest => button.hover(|style| style.bg(theme.colors.surface_hover())),
    };

    button
        .child(icon_placeholder(icon_color))
        .child(text_style(div(), TypeScale::LABEL).child(label.to_string()))
}

/// A 40×40 icon-only button with an accessible name.
pub fn icon_button(
    theme: &Theme,
    id: impl Into<ElementId>,
    state: ControlState,
    aria_label: &str,
) -> Stateful<Div> {
    let color = state_text_color(theme, state);
    let mut button = div()
        .id(id)
        .size(px(SpacingScale::S10))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(RadiusScale::CONTROL))
        .role(Role::Button)
        .aria_label(aria_label.to_string())
        .focus_visible(focus_ring(theme))
        .cursor_pointer();

    button = match state {
        ControlState::Hover | ControlState::Pressed => button.bg(theme.colors.surface_hover()),
        ControlState::FocusVisible => button
            .border_2()
            .border_color(theme.colors.accent_emphasis()),
        ControlState::Disabled => button.opacity(0.45),
        ControlState::Loading => button.opacity(0.7),
        ControlState::Rest => button.hover(|style| style.bg(theme.colors.surface_hover())),
    };

    button.child(icon_placeholder(color))
}

/// A `Glass Low` search field; the border turns lavender only on focus.
pub fn search_field(
    theme: &Theme,
    id: impl Into<ElementId>,
    state: ControlState,
    placeholder: &str,
) -> Stateful<Div> {
    let mut field = GlassSurface::new(GlassVariant::Low)
        .render(theme)
        .id(id)
        .h(px(SpacingScale::S10))
        .px(px(SpacingScale::S3))
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .role(Role::TextInput)
        .aria_label(placeholder.to_string())
        .focus_visible({
            let color = theme.colors.accent_default();
            move |style| style.border_color(color).border_2()
        });

    field = match state {
        ControlState::FocusVisible => field.border_color(theme.colors.accent_default()).border_2(),
        ControlState::Disabled => field.opacity(0.45),
        ControlState::Hover => field.border_color(theme.colors.glass_border_top()),
        ControlState::Rest | ControlState::Pressed | ControlState::Loading => field,
    };

    field
        .child(icon_placeholder(theme.colors.text_muted()))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_muted())
                .child(placeholder.to_string()),
        )
}
