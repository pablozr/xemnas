//! Quiet Glass design tokens.
//!
//! This module is the **only** place in the desktop app allowed to contain raw
//! color values. It transcribes `docs/design-system-quiet-glass.md` verbatim:
//! color tokens, the typographic scale, spacing, radii and motion. Views and
//! primitives consume these tokens through [`crate::ui::theme::Theme`] and must
//! never write a color literal themselves (enforced by the architecture guard
//! in `tests/architecture`).
#![allow(clippy::unreadable_literal)]

use std::time::Duration;

use gpui::{px, rgb, rgba, Pixels, Rgba};

/// Color tokens from the Quiet Glass design system.
///
/// Each method returns the token's [`Rgba`] value. `rgba(0xRRGGBBFF)` builds an
/// opaque color and [`Rgba::alpha`] applies the documented alpha.
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorTokens;

impl ColorTokens {
    /// Graphite document canvas used by the Decisions destination.
    pub fn decision_canvas(&self) -> Rgba {
        rgb(0x17171B)
    }
    /// Quiet index rail.
    pub fn decision_rail(&self) -> Rgba {
        rgb(0x111114)
    }
    /// Auxiliary document layer.
    pub fn decision_layer(&self) -> Rgba {
        rgb(0x1D1D23)
    }
    /// Selected index item and compact active tab.
    pub fn decision_selected(&self) -> Rgba {
        rgb(0x24222D)
    }
    /// Neutral separator.
    pub fn decision_line(&self) -> Rgba {
        rgb(0x2A2A32)
    }
    /// Mineral lavender document marker.
    pub fn decision_accent(&self) -> Rgba {
        rgb(0xB3A5CB)
    }
    /// Confirmed decision indicator.
    pub fn decision_confirmed(&self) -> Rgba {
        rgb(0x8CB69A)
    }
    /// `color.canvas` — main continuous background.
    pub fn canvas(&self) -> Rgba {
        rgb(0x0D111A)
    }

    /// `color.canvas-raised` — regions with slight elevation.
    pub fn canvas_raised(&self) -> Rgba {
        rgb(0x111622)
    }

    /// `color.canvas-deep` — rail and recessed areas.
    pub fn canvas_deep(&self) -> Rgba {
        rgb(0x090D15)
    }

    /// `color.surface` — auxiliary solid surface.
    pub fn surface(&self) -> Rgba {
        rgb(0x181E2A)
    }

    /// `color.surface-hover` — hover without glass.
    pub fn surface_hover(&self) -> Rgba {
        rgb(0x202634)
    }

    /// `glass.fill-low` — rail and wide surfaces.
    pub fn glass_fill_low(&self) -> Rgba {
        rgba(0x9790ACFF).alpha(0.055)
    }

    /// `glass.fill-medium` — selection and proposal surface.
    pub fn glass_fill_medium(&self) -> Rgba {
        rgba(0x9790ACFF).alpha(0.095)
    }

    /// `glass.fill-strong` — primary control.
    pub fn glass_fill_strong(&self) -> Rgba {
        rgba(0xA69EBBFF).alpha(0.16)
    }

    /// `glass.fill-emphasis` — the saturated fill of a small primary control.
    ///
    /// This is a **button** fill, not a surface. It used to be a near-opaque
    /// lilac, and using it as a 440 px card is what made the Home screen read
    /// as a lilac slab: a large area of high-alpha lavender dominates every
    /// other surface and flattens the hierarchy. Reference systems keep large
    /// surfaces at `rgba(255, 255, 255, 0.02)`-`0.05` and reserve saturation
    /// for the one small element that is the primary action.
    ///
    /// Kept at 0.64: still the brightest thing on the canvas, still carrying
    /// `accent.on-emphasis` above WCAG AA (the contrast test locks this in),
    /// but a 40 px control rather than a full card.
    pub fn glass_fill_emphasis(&self) -> Rgba {
        rgba(0xC3BADDFF).alpha(0.64)
    }

    /// `glass.fill-card` — the fill of a large surface: a card, a panel.
    ///
    /// New in this pass. The old stack had no token for "a big quiet
    /// rectangle", so the only available recipe was Emphasis, and every large
    /// surface inherited a saturated fill. This is the Linear/Notion value:
    /// white at 3% over the canvas, which lifts the surface without tinting it.
    pub fn glass_fill_card(&self) -> Rgba {
        rgba(0xFFFFFF).alpha(0.030)
    }

    /// `glass.fill-card-hover` — the same surface under the pointer.
    pub fn glass_fill_card_hover(&self) -> Rgba {
        rgba(0xFFFFFF).alpha(0.055)
    }

    /// `glass.surface-raised` — a card sitting above another card.
    pub fn glass_surface_raised(&self) -> Rgba {
        rgba(0xFFFFFF).alpha(0.045)
    }

    /// `glass.surface-lavender` — controlled opaque base for featured panes.
    /// Avoids the teal cast observed on the previous translucent controls.
    pub fn glass_surface_lavender(&self) -> Rgba {
        rgb(0x1C1A28)
    }

    /// `glass.edge-lavender` — mineral-lavender edge over the dark base.
    pub fn glass_edge_lavender(&self) -> Rgba {
        rgb(0x514A63)
    }

    /// `glass.border` — general outline.
    pub fn glass_border(&self) -> Rgba {
        rgba(0xCDC7DCFF).alpha(0.13)
    }

    /// `glass.border-top` — inner top highlight.
    pub fn glass_border_top(&self) -> Rgba {
        rgba(0xEAE6F1FF).alpha(0.20)
    }

    /// `glass.border-bottom` — bottom depth.
    pub fn glass_border_bottom(&self) -> Rgba {
        rgba(0x474354FF).alpha(0.28)
    }

    /// `accent.subtle` — selection indicator and edge.
    pub fn accent_subtle(&self) -> Rgba {
        rgb(0x82799B)
    }

    /// `accent.default` — focus and active icons.
    pub fn accent_default(&self) -> Rgba {
        rgb(0x9188A8)
    }

    /// `accent.emphasis` — primary action and strong focus.
    pub fn accent_emphasis(&self) -> Rgba {
        rgb(0xA89EBA)
    }

    /// `accent.on-emphasis` — text over light lavender.
    pub fn accent_on_emphasis(&self) -> Rgba {
        rgb(0x15141B)
    }

    /// `text.primary` — titles and primary content.
    pub fn text_primary(&self) -> Rgba {
        rgb(0xECEEF4)
    }

    /// `text.secondary` — descriptions and secondary body.
    pub fn text_secondary(&self) -> Rgba {
        rgb(0xBEC3D0)
    }

    /// `text.muted` — metadata.
    pub fn text_muted(&self) -> Rgba {
        rgb(0x858C9D)
    }

    /// `text.disabled` — unavailable state.
    pub fn text_disabled(&self) -> Rgba {
        rgb(0x5E6574)
    }

    /// `text.inverse` — light controls.
    pub fn text_inverse(&self) -> Rgba {
        rgb(0x15141B)
    }

    /// `status.success` — confirmed or healthy.
    pub fn status_success(&self) -> Rgba {
        rgb(0x83C59A)
    }

    /// `status.warning` — pending or attention.
    pub fn status_warning(&self) -> Rgba {
        rgb(0xD4B56E)
    }

    /// `status.danger` — rejection and error.
    pub fn status_danger(&self) -> Rgba {
        rgb(0xD96776)
    }

    /// `status.info` — neutral information.
    pub fn status_info(&self) -> Rgba {
        rgb(0x839BBE)
    }

    /// Shadow color of `Glass Low`: `rgba(0, 0, 0, 0.16)`.
    pub fn shadow_low(&self) -> Rgba {
        rgba(0x000000FF).alpha(0.16)
    }

    /// Shadow color of `Glass Selected`: `rgba(0, 0, 0, 0.18)`.
    pub fn shadow_selected(&self) -> Rgba {
        rgba(0x000000FF).alpha(0.18)
    }

    /// Shadow color of `Glass Emphasis`: `rgba(0, 0, 0, 0.22)`.
    pub fn shadow_emphasis(&self) -> Rgba {
        rgba(0x000000FF).alpha(0.22)
    }

    /// Inner highlight of `Glass Emphasis`: white at 16%.
    pub fn emphasis_highlight(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.16)
    }

    /// Bottom edge of `Glass Emphasis`: black at 16%.
    pub fn emphasis_bottom(&self) -> Rgba {
        rgb(0x000000).alpha(0.16)
    }

    /// A stronger translucent fill used by the solid (non-blur) fallback.
    pub fn glass_fallback(&self) -> Rgba {
        rgba(0x181E2AFF).alpha(0.98)
    }

    // --- additions required by the window shell (ticket: densidade + material) ---

    /// `color.rail` — the navigation rail, one step behind the canvas.
    ///
    /// Sits between `color.canvas-deep` (#090D15) and `color.canvas` (#0D111A):
    /// it recedes the rail without reading as a stain on the window.
    pub fn rail(&self) -> Rgba {
        rgb(0x0A0E17)
    }

    /// `hairline.divider` — 1 px separation inside a continuous surface.
    ///
    /// `glass.border` at 13% is too loud for a full-length rule; internal
    /// separators (rail edge, header, status bar) use this quieter 10%.
    pub fn hairline_divider(&self) -> Rgba {
        rgba(0xCDC7DCFF).alpha(0.10)
    }

    /// `glass.border-card` — the outline of a large surface.
    ///
    /// `glass.border` (13%) was doing two jobs: a full-length divider and a
    /// card outline. At 13% a 1400 px rule reads as a hard line, while a card
    /// needs a visible edge to exist at all. These are now separate tokens —
    /// 8% for rules, 9% for card outlines — so each can be tuned on its own.
    pub fn glass_border_card(&self) -> Rgba {
        rgba(0xFFFFFF).alpha(0.09)
    }

    /// `glass.border-card-hover` — a card outline under the pointer.
    pub fn glass_border_card_hover(&self) -> Rgba {
        rgba(0xFFFFFF).alpha(0.16)
    }

    /// `glass.border-control` — the outline of a control at rest.
    pub fn glass_border_control(&self) -> Rgba {
        rgba(0xFFFFFF).alpha(0.10)
    }

    /// `color.hover-veil` — hover for rows and rail items.
    ///
    /// Lighter than `color.surface-hover` (#202634, an opaque solid) because it
    /// is a large-area tint that must not compete with the content on top of it.
    pub fn hover_veil(&self) -> Rgba {
        rgba(0x9790ACFF).alpha(0.045)
    }

    /// `layer.fill` — the content layer painted on top of a Mica backdrop.
    ///
    /// Windows 11's `LayerFillColorDefaultBrush`: a low-opacity solid that lets
    /// the material behind it read through while keeping text on an even base.
    pub fn layer_fill(&self) -> Rgba {
        rgba(0x0D111AFF).alpha(0.72)
    }

    /// `scrollbar.thumb` — the scroll indicator, visible only while scrolling.
    pub fn scrollbar_thumb(&self) -> Rgba {
        rgba(0x9790ACFF).alpha(0.22)
    }

    /// `glow.warm` — a wide, very low-alpha aura behind a featured surface.
    ///
    /// Borrowed from the reference systems' "warm glow" (`rgba(215, 201, 175,
    /// 0.05)`, 20 px blur). It is what stops a dark card from reading as a hole
    /// cut in the canvas: the surface has a faint light of its own instead of
    /// only a border. Deliberately almost invisible — it is felt, not seen.
    pub fn glow_warm(&self) -> Rgba {
        rgba(0xD7C9AF).alpha(0.05)
    }

    /// `glow.lavender` — diffuse edge light, never a saturated fill.
    pub fn glow_lavender(&self) -> Rgba {
        rgba(0xA89EBAFF).alpha(0.08)
    }

    /// The inner top highlight, as an inset shadow colour.
    ///
    /// Replaces the hand-placed absolute `div` the glass used to draw its top
    /// edge: an inset shadow follows the rounded corner on all four sides, so
    /// the highlight reads as a lit rim instead of a straight line between arcs.
    pub fn inset_highlight(&self) -> Rgba {
        rgba(0xEAE6F1FF).alpha(0.20)
    }

    /// The inner bottom depth, as an inset shadow colour.
    pub fn inset_depth(&self) -> Rgba {
        rgba(0x474354FF).alpha(0.28)
    }
}

/// A single typographic token: size, line height and weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeToken {
    /// Font size in logical pixels.
    pub size: f32,
    /// Line height in logical pixels.
    pub line_height: f32,
    /// Font weight (variable-font axis value).
    pub weight: f32,
}

impl TypeToken {
    /// Builds a typographic token.
    pub const fn new(size: f32, line_height: f32, weight: f32) -> Self {
        Self {
            size,
            line_height,
            weight,
        }
    }

    /// Font size as [`Pixels`].
    pub fn size_px(&self) -> Pixels {
        px(self.size)
    }

    /// Line height as [`Pixels`].
    pub fn line_height_px(&self) -> Pixels {
        px(self.line_height)
    }

    /// Font weight as [`gpui::FontWeight`].
    pub fn font_weight(&self) -> gpui::FontWeight {
        gpui::FontWeight(self.weight)
    }
}

/// The nine typographic tokens from the design system.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScale;

impl TypeScale {
    /// `type.display` — 32 / 40, weight 560.
    pub const DISPLAY: TypeToken = TypeToken::new(32.0, 40.0, 560.0);
    /// `type.heading-1` — 24 / 32, weight 600.
    pub const HEADING_1: TypeToken = TypeToken::new(24.0, 32.0, 600.0);
    /// `type.heading-2` — 18 / 26, weight 560.
    pub const HEADING_2: TypeToken = TypeToken::new(18.0, 26.0, 560.0);
    /// `type.heading-3` — 15 / 22, weight 560.
    pub const HEADING_3: TypeToken = TypeToken::new(15.0, 22.0, 560.0);
    /// `type.body` — 15 / 23, weight 400.
    pub const BODY: TypeToken = TypeToken::new(15.0, 23.0, 400.0);
    /// `type.body-small` — 13 / 19, weight 400.
    pub const BODY_SMALL: TypeToken = TypeToken::new(13.0, 19.0, 400.0);
    /// `type.label` — 12 / 16, weight 520.
    pub const LABEL: TypeToken = TypeToken::new(12.0, 16.0, 520.0);
    /// `type.meta` — 11 / 16, weight 450.
    pub const META: TypeToken = TypeToken::new(11.0, 16.0, 450.0);
    /// `type.code` — 13 / 20, weight 400.
    pub const CODE: TypeToken = TypeToken::new(13.0, 20.0, 400.0);
}

/// The `4 px` spacing scale (`space.1` … `space.16`).
#[derive(Clone, Copy, Debug, Default)]
pub struct SpacingScale;

impl SpacingScale {
    /// `space.1` = 4.
    pub const S1: f32 = 4.0;
    /// `space.2` = 8.
    pub const S2: f32 = 8.0;
    /// `space.3` = 12.
    pub const S3: f32 = 12.0;
    /// `space.4` = 16.
    pub const S4: f32 = 16.0;
    /// `space.5` = 20.
    pub const S5: f32 = 20.0;
    /// `space.6` = 24.
    pub const S6: f32 = 24.0;
    /// `space.8` = 32.
    pub const S8: f32 = 32.0;
    /// `space.10` = 40.
    pub const S10: f32 = 40.0;
    /// `space.12` = 48.
    pub const S12: f32 = 48.0;
    /// `space.16` = 64.
    pub const S16: f32 = 64.0;
}

/// Corner radii (`radius.control` … `radius.round`).
#[derive(Clone, Copy, Debug, Default)]
pub struct RadiusScale;

impl RadiusScale {
    /// `radius.control` = 8.
    pub const CONTROL: f32 = 8.0;
    /// `radius.surface` = 10.
    pub const SURFACE: f32 = 10.0;
    /// `radius.dialog` = 12.
    pub const DIALOG: f32 = 12.0;
    /// `radius.round` = 999.
    pub const ROUND: f32 = 999.0;

    /// `radius.control` as [`Pixels`].
    pub fn control(&self) -> Pixels {
        px(Self::CONTROL)
    }

    /// `radius.surface` as [`Pixels`].
    pub fn surface(&self) -> Pixels {
        px(Self::SURFACE)
    }

    /// `radius.dialog` as [`Pixels`].
    pub fn dialog(&self) -> Pixels {
        px(Self::DIALOG)
    }

    /// `radius.round` as [`Pixels`].
    pub fn round(&self) -> Pixels {
        px(Self::ROUND)
    }
}

/// Motion durations and easing curves.
#[derive(Clone, Copy, Debug, Default)]
pub struct MotionTokens;

impl MotionTokens {
    /// `motion.fast` = 100 ms (hover and pressed).
    pub const FAST: Duration = Duration::from_millis(100);
    /// `motion.base` = 160 ms (selection and disclosure).
    pub const BASE: Duration = Duration::from_millis(160);
    /// `motion.slow` = 240 ms (pane and overlay).
    pub const SLOW: Duration = Duration::from_millis(240);
    /// `easing.enter` = cubic-bezier(0.16, 1, 0.3, 1).
    pub const EASING_ENTER: [f32; 4] = [0.16, 1.0, 0.3, 1.0];
    /// `easing.exit` = cubic-bezier(0.4, 0, 1, 1).
    pub const EASING_EXIT: [f32; 4] = [0.4, 0.0, 1.0, 1.0];

    /// The documented `easing.enter` as a GPUI easing function.
    ///
    /// The design system specifies `cubic-bezier(0.16, 1, 0.3, 1)`, but this
    /// revision of GPUI ships only `linear`, `quadratic`, `ease_in_out` and
    /// `ease_out_quint` (gpui/src/elements/animation.rs:502-528) — no general
    /// cubic-bezier evaluator. `ease_out_quint` is the honest approximation:
    /// fast start, long settle, no overshoot, so a control never appears to
    /// bounce past its resting state. Swap the body for a Newton solve on
    /// [`Self::EASING_ENTER`] if a future GPUI exposes one.
    pub fn enter_easing() -> impl Fn(f32) -> f32 {
        gpui::ease_out_quint()
    }

    /// The documented `easing.exit` as a GPUI easing function.
    ///
    /// Mirrors [`Self::enter_easing`]'s reasoning; `ease_in_out` matches the
    /// shape of the documented curve (slow start, fast middle, slow end).
    pub fn exit_easing() -> impl Fn(f32) -> f32 {
        gpui::ease_in_out
    }
}

#[cfg(test)]
mod tests {
    use gpui::Rgba;

    use super::ColorTokens;

    /// An opaque RGB color with channels in `0.0..=1.0`.
    type Opaque = (f64, f64, f64);

    /// Converts an sRGB channel in `0.0..=1.0` to linear light.
    fn linear(channel: f64) -> f64 {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    /// WCAG relative luminance of an opaque color.
    fn relative_luminance(color: Opaque) -> f64 {
        let (r, g, b) = color;
        0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
    }

    /// Alpha-composites `foreground` over an opaque `background`.
    fn composite_over(foreground: Rgba, background: Rgba) -> Opaque {
        let alpha = foreground.a as f64;
        let mix = |fg: f32, bg: f32| fg as f64 * alpha + bg as f64 * (1.0 - alpha);
        (
            mix(foreground.r, background.r),
            mix(foreground.g, background.g),
            mix(foreground.b, background.b),
        )
    }

    /// WCAG contrast ratio between two opaque colors.
    fn contrast_ratio(a: Opaque, b: Opaque) -> f64 {
        let la = relative_luminance(a);
        let lb = relative_luminance(b);
        let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
        (hi + 0.05) / (lo + 0.05)
    }

    fn opaque(color: Rgba) -> Opaque {
        (color.r as f64, color.g as f64, color.b as f64)
    }

    #[test]
    fn rendered_text_surfaces_meet_wcag_aa() {
        let colors = ColorTokens;
        let canvas = colors.canvas();

        // Every text/surface pair the gallery (and the primitives) actually
        // render. Surfaces are alpha-composited onto `color.canvas`, which is
        // what the window paints behind them.
        let text_pairs: [(&str, Opaque, Opaque); 11] = [
            (
                "text.primary / color.canvas",
                opaque(colors.text_primary()),
                opaque(canvas),
            ),
            (
                "text.secondary / color.canvas",
                opaque(colors.text_secondary()),
                opaque(canvas),
            ),
            (
                "text.muted / color.canvas",
                opaque(colors.text_muted()),
                opaque(canvas),
            ),
            (
                "text.secondary / color.surface-hover",
                opaque(colors.text_secondary()),
                opaque(colors.surface_hover()),
            ),
            (
                "text.primary / glass.fill-low",
                opaque(colors.text_primary()),
                composite_over(colors.glass_fill_low(), canvas),
            ),
            (
                "text.secondary / glass.fill-low",
                opaque(colors.text_secondary()),
                composite_over(colors.glass_fill_low(), canvas),
            ),
            (
                "text.muted / glass.fill-low",
                opaque(colors.text_muted()),
                composite_over(colors.glass_fill_low(), canvas),
            ),
            (
                "status.danger / glass.fill-low",
                opaque(colors.status_danger()),
                composite_over(colors.glass_fill_low(), canvas),
            ),
            (
                "text.primary / glass.fill-medium",
                opaque(colors.text_primary()),
                composite_over(colors.glass_fill_medium(), canvas),
            ),
            (
                "text.muted / glass.fill-medium",
                opaque(colors.text_muted()),
                composite_over(colors.glass_fill_medium(), canvas),
            ),
            (
                "accent.on-emphasis / glass.fill-emphasis",
                opaque(colors.accent_on_emphasis()),
                composite_over(colors.glass_fill_emphasis(), canvas),
            ),
        ];

        for (name, foreground, background) in text_pairs {
            let ratio = contrast_ratio(foreground, background);
            eprintln!("WCAG {name} = {ratio:.2}:1");
            assert!(
                ratio >= 4.5,
                "{name} is {ratio:.2}:1, below the WCAG AA text minimum of 4.5:1"
            );
        }

        // Essential icons only need 3:1; `status.danger` is used as an icon on
        // the quiet danger action, never as text over the hover tint.
        let icon_ratio = contrast_ratio(
            opaque(colors.status_danger()),
            opaque(colors.surface_hover()),
        );
        eprintln!("WCAG status.danger icon / color.surface-hover = {icon_ratio:.2}:1");
        assert!(
            icon_ratio >= 3.0,
            "status.danger icon over color.surface-hover is {icon_ratio:.2}:1, below 3:1"
        );

        // text.disabled is intentionally not asserted: WCAG 1.4.3 exempts
        // inactive user-interface components from the contrast minimum.
    }
}
