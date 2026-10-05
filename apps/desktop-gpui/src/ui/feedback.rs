//! Status, empty and error patterns.
//!
//! These are the primitives that answer the "vazio" and "erro" states of the
//! ticket. Color never carries state alone: every status renders a dot **and**
//! a text description.

use gpui::prelude::*;
use gpui::{div, px, Div, ElementId, Role, Stateful};

use crate::i18n::common as t;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

/// The four semantic statuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusKind {
    /// Confirmed or healthy.
    Success,
    /// Pending or attention.
    Warning,
    /// Rejection and error.
    Danger,
    /// Neutral information.
    Info,
}

impl StatusKind {
    /// The token color for this status.
    pub fn color(&self, theme: &Theme) -> gpui::Rgba {
        match self {
            Self::Success => theme.colors.status_success(),
            Self::Warning => theme.colors.status_warning(),
            Self::Danger => theme.colors.status_danger(),
            Self::Info => theme.colors.status_info(),
        }
    }

    /// The human-readable label that accompanies the color.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Success => t::status_success(),
            Self::Warning => t::status_warning(),
            Self::Danger => t::status_danger(),
            Self::Info => t::status_info(),
        }
    }
}

/// A status dot plus an accessible text description.
pub fn status_dot(
    theme: &Theme,
    id: impl Into<ElementId>,
    kind: StatusKind,
    description: &str,
) -> Stateful<Div> {
    let color = kind.color(theme);
    div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(SpacingScale::S2))
        .role(Role::Status)
        .aria_label(format!("{}: {description}", kind.label()))
        .child(div().size(px(8.0)).rounded_full().bg(color))
        .child(
            text_style(div(), TypeScale::BODY_SMALL)
                .text_color(theme.colors.text_secondary())
                .child(format!("{}: {description}", kind.label())),
        )
}

/// The error-state pattern: danger accent, explanation and a recovery hint.
///
/// It carries no button of its own: a retry control only belongs where the
/// caller wires it to a real operation, never as decoration.
pub fn error_state(
    theme: &Theme,
    id: impl Into<ElementId>,
    title: &str,
    body: &str,
    recovery: &str,
) -> Stateful<Div> {
    div()
        .id(id)
        .max_w(px(560.0))
        .m(px(SpacingScale::S8))
        .rounded(theme.radius.surface())
        .border_1()
        .border_color(theme.colors.hairline_divider())
        .bg(theme.colors.surface())
        .p(px(SpacingScale::S6))
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .role(Role::Alert)
        .aria_label(title.to_string())
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .child(
                    div()
                        .size(px(8.0))
                        .rounded_full()
                        .bg(theme.colors.status_danger()),
                )
                .child(
                    text_style(div(), TypeScale::HEADING_2)
                        .text_color(theme.colors.text_primary())
                        .child(title.to_string()),
                ),
        )
        .child(
            text_style(div(), TypeScale::BODY)
                .text_color(theme.colors.text_secondary())
                .child(body.to_string()),
        )
        .child(
            text_style(div(), TypeScale::LABEL)
                .text_color(theme.colors.status_danger())
                .child(recovery.to_string()),
        )
}
