//! Glass primitives.
//!
//! [`GlassSurface`] centralises every glass recipe (fill, strokes, top
//! highlight, selection edge, shadow and the solid fallback) so views only pick
//! a variant. It never depends on backdrop blur; the translucent fill over the
//! controlled canvas *is* the documented fallback.
//!
//! [`FocusRing`] is the single source of the focus-visible treatment.

use gpui::prelude::*;
use gpui::{px, BoxShadow, Div, Rgba, StyleRefinement};

use crate::ui::theme::Theme;

/// The three approved glass variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlassVariant {
    /// Rail and wide secondary surfaces.
    Low,
    /// Selected item and proposal surface.
    Selected,
    /// Primary action; never large panels.
    Emphasis,
}

/// A glass surface built from approved tokens.
#[derive(Clone, Copy, Debug)]
pub struct GlassSurface {
    variant: GlassVariant,
    solid: bool,
}

impl GlassSurface {
    /// Creates a glass surface for the given variant.
    pub fn new(variant: GlassVariant) -> Self {
        Self {
            variant,
            solid: false,
        }
    }

    /// Uses the opaque fallback recipe instead of the translucent one.
    ///
    /// The design system forbids depending on blur; this is the documented
    /// solid path used when translucency would hurt legibility.
    pub fn solid(mut self) -> Self {
        self.solid = true;
        self
    }

    /// The variant this surface was built with.
    pub fn variant(&self) -> GlassVariant {
        self.variant
    }

    /// Foreground color appropriate for content placed on this surface.
    pub fn foreground(&self, theme: &Theme) -> Rgba {
        match self.variant {
            GlassVariant::Emphasis => theme.colors.accent_on_emphasis(),
            GlassVariant::Low | GlassVariant::Selected => theme.colors.text_primary(),
        }
    }

    /// Renders the surface. Additional children are appended by the caller.
    pub fn render(&self, theme: &Theme) -> Div {
        let colors = &theme.colors;
        let radius = theme.radius.surface();

        let (fill, border, highlight, shadow_color, shadow_blur) = match self.variant {
            GlassVariant::Low => (
                colors.glass_fill_low(),
                colors.glass_border(),
                colors.glass_border_top(),
                colors.shadow_low(),
                28.0,
            ),
            GlassVariant::Selected => (
                colors.glass_fill_medium(),
                colors.glass_border(),
                colors.glass_border_top(),
                colors.shadow_selected(),
                32.0,
            ),
            GlassVariant::Emphasis => (
                colors.glass_fill_emphasis(),
                colors.glass_border(),
                colors.emphasis_highlight(),
                colors.shadow_emphasis(),
                22.0,
            ),
        };

        let fill = if self.solid {
            match self.variant {
                GlassVariant::Emphasis => colors.accent_emphasis(),
                GlassVariant::Low | GlassVariant::Selected => colors.glass_fallback(),
            }
        } else {
            fill
        };

        // The top inner highlight used to be a hand-placed absolute `div`
        // inset by the corner radius, because GPUI clips children to a
        // rectangular mask and a full-bleed 1 px line painted straight across
        // the corner arcs. `BoxShadow::inset` is drawn by the element's own
        // shader inside its rounded bounds, so the highlight now follows the
        // curve on all four sides — which is what makes a surface read as lit
        // glass rather than as a bordered rectangle. The absolute div is gone.
        let surface = gpui::div()
            .relative()
            .rounded(radius)
            .border_1()
            .border_color(border)
            .bg(fill)
            .shadow(vec![
                BoxShadow::new(px(0.0), px(8.0), shadow_color.into()).blur_radius(px(shadow_blur)),
                BoxShadow::new(px(0.0), px(-1.0), highlight.into())
                    .blur_radius(px(0.5))
                    .inset(),
            ]);

        match self.variant {
            GlassVariant::Selected => surface.child(
                // 2 px selection edge on the leading side, inset vertically so
                // it does not cross the rounded top/bottom corners.
                gpui::div()
                    .absolute()
                    .top(radius)
                    .bottom(radius)
                    .left_0()
                    .w(px(2.0))
                    .bg(colors.accent_subtle()),
            ),
            GlassVariant::Emphasis => surface.child(
                // Bottom edge for a pressed/embossed feel, inset the same way
                // as the top highlight.
                gpui::div()
                    .absolute()
                    .bottom_0()
                    .left(radius)
                    .right(radius)
                    .h(px(1.0))
                    .bg(colors.emphasis_bottom()),
            ),
            GlassVariant::Low => surface,
        }
    }
}

/// Focus-visible ring: 2 px `accent.emphasis`.
///
/// GPUI resolves focus through a border rather than an offset outline, so the
/// documented 2 px offset is recorded as a fidelity limitation in the ticket.
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
    move |style: StyleRefinement| style.border_2().border_color(color)
}
