//! Hover tooltips: a label and, when there is one, its keyboard shortcut.
//!
//! Icon-only controls carry their name only in the accessibility tree; the
//! tooltip gives pointer users the same name. GPUI builds tooltips as views,
//! so this is a tiny view instead of a style.

use gpui::prelude::*;
use gpui::{div, px, AnyView, App, Context, SharedString, Window};

use crate::ui::patterns::kbd;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

/// The tooltip content.
pub struct Tooltip {
    label: SharedString,
    shortcut: Option<&'static str>,
}

impl Render for Tooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::current(cx);
        text_style(div(), TypeScale::META)
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .px(px(SpacingScale::S2))
            .py(px(SpacingScale::S1))
            .rounded(theme.radius.control())
            .border_1()
            .border_color(theme.colors.hairline_divider())
            .bg(theme.colors.surface())
            .text_color(theme.colors.text_primary())
            .shadow(vec![gpui::BoxShadow::new(
                px(0.0),
                px(4.0),
                theme.colors.shadow_emphasis().into(),
            )
            .blur_radius(px(12.0))])
            .child(self.label.clone())
            .children(
                self.shortcut
                    .map(|key| kbd(theme.colors.text_secondary(), key)),
            )
    }
}

/// Builds the tooltip callback GPUI's `.tooltip(...)` expects.
pub fn tooltip(
    label: impl Into<SharedString>,
    shortcut: Option<&'static str>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let label = label.into();
    move |_, cx| {
        cx.new(|_| Tooltip {
            label: label.clone(),
            shortcut,
        })
        .into()
    }
}
