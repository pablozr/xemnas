//! Focus treatment.
//!
//! [`FocusRing`] is the single source of the focus-visible ring. The glass
//! material itself is the window backdrop behind `color.chrome` (see
//! `ColorTokens::chrome`); GPUI has no per-element blur.

use gpui::prelude::*;
use gpui::{px, BoxShadow, Rgba, StyleRefinement};

use crate::ui::theme::Theme;

/// Focus-visible ring: 2 px `accent.emphasis`.
///
/// Drawn as an inset shadow rather than a border: a `border_2` grew every
/// borderless row and tab by 2 px on each side, so content jumped when focus
/// arrived. The shadow follows the element's corner radius and leaves layout
/// untouched. GPUI has no offset outline, so the documented 2 px offset
/// remains a recorded fidelity limitation.
#[derive(Clone, Copy, Debug)]
pub struct FocusRing;

impl FocusRing {
    /// Ring width in logical pixels.
    pub const WIDTH: f32 = 2.0;
    /// Documented offset between the control and the ring.
    pub const OFFSET: f32 = 2.0;

    /// The ring color (`accent.emphasis`).
    pub fn color(theme: &Theme) -> Rgba {
        theme.colors.accent_emphasis()
    }
}

/// Returns the focus-visible style closure. The ring is never removed for
/// aesthetics (design system rule).
pub fn focus_ring(theme: &Theme) -> impl FnOnce(StyleRefinement) -> StyleRefinement {
    let color = FocusRing::color(theme);
    move |style: StyleRefinement| {
        style.shadow(vec![BoxShadow::new(px(0.0), px(0.0), color.into())
            .spread_radius(px(FocusRing::WIDTH))
            .inset()])
    }
}
