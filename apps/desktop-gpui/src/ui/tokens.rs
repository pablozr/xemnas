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

use gpui::{px, rgb, Pixels, Rgba};

/// A colour as written in the design system: `0xRRGGBB` plus an alpha.
///
/// Keeping hex and alpha apart avoids the `rgba(0xRRGGBB)` trap: GPUI's
/// `rgba` reads eight hex digits, so a six-digit white silently became cyan.
#[derive(Clone, Copy, Debug)]
struct Tone(u32, f32);

impl Tone {
    const fn solid(hex: u32) -> Self {
        Self(hex, 1.0)
    }

    fn rgba(self) -> Rgba {
        rgb(self.0).alpha(self.1)
    }
}

/// The tokens that differ between palettes. Everything else is shared.
#[derive(Debug)]
struct Palette {
    canvas: Tone,
    canvas_raised: Tone,
    canvas_deep: Tone,
    rail: Tone,
    surface: Tone,
    surface_hover: Tone,
    selection: Tone,
    glass_surface_lavender: Tone,
    glass_edge_lavender: Tone,
    hairline_divider: Tone,
    layer_fill: Tone,
    text_primary: Tone,
    text_secondary: Tone,
    text_muted: Tone,
    status_danger: Tone,
}

/// Blue graphite: the original Quiet Glass palette.
const QUIET_GLASS: Palette = Palette {
    canvas: Tone::solid(0x0D111A),
    canvas_raised: Tone::solid(0x111622),
    canvas_deep: Tone::solid(0x090D15),
    rail: Tone::solid(0x0A0E17),
    surface: Tone::solid(0x181E2A),
    surface_hover: Tone::solid(0x202634),
    selection: Tone::solid(0x1C1A28),
    glass_surface_lavender: Tone::solid(0x1C1A28),
    glass_edge_lavender: Tone::solid(0x514A63),
    hairline_divider: Tone(0xCDC7DC, 0.10),
    layer_fill: Tone(0x0D111A, 0.72),
    text_primary: Tone::solid(0xECEEF4),
    text_secondary: Tone::solid(0xBEC3D0),
    text_muted: Tone::solid(0x858C9D),
    status_danger: Tone::solid(0xD96776),
};

/// Neutral charcoal: the editorial palette.
const CHARCOAL: Palette = Palette {
    canvas: Tone::solid(0x202024),
    canvas_raised: Tone::solid(0x26262C),
    canvas_deep: Tone::solid(0x1B1B1F),
    rail: Tone::solid(0x1B1B1F),
    surface: Tone::solid(0x26262C),
    surface_hover: Tone::solid(0x302B39),
    selection: Tone::solid(0x302B39),
    glass_surface_lavender: Tone::solid(0x26262C),
    glass_edge_lavender: Tone::solid(0x49434F),
    hairline_divider: Tone::solid(0x35353D),
    layer_fill: Tone::solid(0x202024),
    text_primary: Tone::solid(0xEDEDF0),
    text_secondary: Tone::solid(0xC0C0CA),
    text_muted: Tone::solid(0xA09FAB),
    status_danger: Tone::solid(0xE27F8D),
};

/// Color tokens from the Quiet Glass design system.
///
/// Palette-specific values live in one [`Palette`] table per theme; the
/// methods below are the only way views read them.
#[derive(Clone, Copy, Debug)]
pub struct ColorTokens {
    palette: &'static Palette,
}

impl Default for ColorTokens {
    fn default() -> Self {
        Self::quiet_glass()
    }
}

impl ColorTokens {
    /// Original Quiet Glass palette.
    pub const fn quiet_glass() -> Self {
        Self {
            palette: &QUIET_GLASS,
        }
    }

    /// Neutral charcoal palette.
    pub const fn charcoal() -> Self {
        Self { palette: &CHARCOAL }
    }

    /// `color.canvas` — main continuous background.
    pub fn canvas(&self) -> Rgba {
        self.palette.canvas.rgba()
    }

    /// `color.canvas-raised` — regions with slight elevation.
    pub fn canvas_raised(&self) -> Rgba {
        self.palette.canvas_raised.rgba()
    }

    /// `color.canvas-deep` — recessed areas such as code wells.
    pub fn canvas_deep(&self) -> Rgba {
        self.palette.canvas_deep.rgba()
    }

    /// `color.rail` — side lists, one step behind the canvas.
    pub fn rail(&self) -> Rgba {
        self.palette.rail.rgba()
    }

    /// `color.surface` — auxiliary solid surface: chips, counters, layers.
    pub fn surface(&self) -> Rgba {
        self.palette.surface.rgba()
    }

    /// `color.surface-hover` — hover without glass.
    pub fn surface_hover(&self) -> Rgba {
        self.palette.surface_hover.rgba()
    }

    /// `color.selection` — the selected row, tab or index item.
    pub fn selection(&self) -> Rgba {
        self.palette.selection.rgba()
    }

    /// `glass.surface-lavender` — controlled opaque base for featured panes.
    pub fn glass_surface_lavender(&self) -> Rgba {
        self.palette.glass_surface_lavender.rgba()
    }

    /// `glass.edge-lavender` — mineral-lavender edge over the dark base.
    pub fn glass_edge_lavender(&self) -> Rgba {
        self.palette.glass_edge_lavender.rgba()
    }

    /// `hairline.divider` — 1 px separation inside a continuous surface.
    pub fn hairline_divider(&self) -> Rgba {
        self.palette.hairline_divider.rgba()
    }

    /// `layer.fill` — the content layer painted on top of a Mica backdrop.
    pub fn layer_fill(&self) -> Rgba {
        self.palette.layer_fill.rgba()
    }

    /// `text.primary` — titles and primary content.
    pub fn text_primary(&self) -> Rgba {
        self.palette.text_primary.rgba()
    }

    /// `text.secondary` — descriptions and secondary body.
    pub fn text_secondary(&self) -> Rgba {
        self.palette.text_secondary.rgba()
    }

    /// `text.muted` — metadata.
    pub fn text_muted(&self) -> Rgba {
        self.palette.text_muted.rgba()
    }

    /// `status.danger` — rejection and error.
    pub fn status_danger(&self) -> Rgba {
        self.palette.status_danger.rgba()
    }

    // --- shared by both palettes ---

    /// `glass.fill-low` — rail and wide surfaces.
    pub fn glass_fill_low(&self) -> Rgba {
        rgb(0x9790AC).alpha(0.055)
    }

    /// `glass.fill-medium` — selection and proposal surface.
    pub fn glass_fill_medium(&self) -> Rgba {
        rgb(0x9790AC).alpha(0.095)
    }

    /// `glass.fill-strong` — primary control.
    pub fn glass_fill_strong(&self) -> Rgba {
        rgb(0xA69EBB).alpha(0.16)
    }

    /// `glass.fill-emphasis` — the saturated fill of a small primary control,
    /// never a large surface.
    pub fn glass_fill_emphasis(&self) -> Rgba {
        rgb(0xC3BADD).alpha(0.64)
    }

    /// `glass.fill-card` — white at 3% over the canvas: lifts a large surface
    /// without tinting it.
    pub fn glass_fill_card(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.030)
    }

    /// `glass.fill-card-hover` — the same surface under the pointer.
    pub fn glass_fill_card_hover(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.055)
    }

    /// `glass.surface-raised` — a card sitting above another card.
    pub fn glass_surface_raised(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.045)
    }

    /// `glass.border` — general outline.
    pub fn glass_border(&self) -> Rgba {
        rgb(0xCDC7DC).alpha(0.13)
    }

    /// `glass.border-top` — inner top highlight.
    pub fn glass_border_top(&self) -> Rgba {
        rgb(0xEAE6F1).alpha(0.20)
    }

    /// `glass.border-bottom` — bottom depth.
    pub fn glass_border_bottom(&self) -> Rgba {
        rgb(0x474354).alpha(0.28)
    }

    /// `glass.border-card` — the outline of a large surface.
    pub fn glass_border_card(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.09)
    }

    /// `glass.border-card-hover` — a card outline under the pointer.
    pub fn glass_border_card_hover(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.16)
    }

    /// `glass.border-control` — the outline of a field or control at rest.
    pub fn glass_border_control(&self) -> Rgba {
        rgb(0xFFFFFF).alpha(0.10)
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

    /// `accent.hover` — the primary action under the pointer and the document
    /// marker: one step lighter than `accent.emphasis`.
    pub fn accent_hover(&self) -> Rgba {
        rgb(0xB3A5CB)
    }

    /// `accent.on-emphasis` — text over light lavender.
    pub fn accent_on_emphasis(&self) -> Rgba {
        rgb(0x15141B)
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

    /// `status.info` — neutral information.
    pub fn status_info(&self) -> Rgba {
        rgb(0x839BBE)
    }

    /// Shadow color of `Glass Low`.
    pub fn shadow_low(&self) -> Rgba {
        rgb(0x000000).alpha(0.16)
    }

    /// Shadow color of `Glass Selected`.
    pub fn shadow_selected(&self) -> Rgba {
        rgb(0x000000).alpha(0.18)
    }

    /// Shadow color of `Glass Emphasis`.
    pub fn shadow_emphasis(&self) -> Rgba {
        rgb(0x000000).alpha(0.22)
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
        rgb(0x181E2A).alpha(0.98)
    }

    /// `color.hover-veil` — hover for rows, tabs and ghost controls.
    pub fn hover_veil(&self) -> Rgba {
        rgb(0x9790AC).alpha(0.07)
    }

    /// `scrollbar.thumb` — the scroll indicator.
    pub fn scrollbar_thumb(&self) -> Rgba {
        rgb(0x9790AC).alpha(0.22)
    }

    /// `diff.added` — row tint behind an added line in a diff hunk.
    pub fn diff_added(&self) -> Rgba {
        rgb(0x83C59A).alpha(0.10)
    }

    /// `diff.removed` — row tint behind a removed line in a diff hunk.
    pub fn diff_removed(&self) -> Rgba {
        rgb(0xD96776).alpha(0.10)
    }

    /// `glow.warm` — a wide, very low-alpha aura behind a featured surface.
    pub fn glow_warm(&self) -> Rgba {
        rgb(0xD7C9AF).alpha(0.05)
    }

    /// `glow.lavender` — diffuse edge light, never a saturated fill.
    pub fn glow_lavender(&self) -> Rgba {
        rgb(0xA89EBA).alpha(0.08)
    }

    /// The inner top highlight, as an inset shadow colour.
    pub fn inset_highlight(&self) -> Rgba {
        rgb(0xEAE6F1).alpha(0.20)
    }

    /// The inner bottom depth, as an inset shadow colour.
    pub fn inset_depth(&self) -> Rgba {
        rgb(0x474354).alpha(0.28)
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

/// The typographic scale.
///
/// Weights are named instances (400/500/600) only: GPUI selects a face by
/// weight and does not set the variable `wght` axis, so 520 or 560 silently
/// snapped to the nearest instance.
///
/// Stepped down one notch from the first pass (body 15 → 14, headings
/// 24/18/15 → 20/16/14): at 15 px the product read like a document editor,
/// while the reference tools (Linear, Zed) set their chrome at 13 px and let
/// only the reading surface grow.
#[derive(Clone, Copy, Debug, Default)]
pub struct TypeScale;

impl TypeScale {
    /// `type.display` — 26 / 34, weight 500: the decision title.
    pub const DISPLAY: TypeToken = TypeToken::new(26.0, 34.0, 500.0);
    /// `type.heading-1` — 20 / 28, weight 600: the title of a reading pane.
    pub const HEADING_1: TypeToken = TypeToken::new(20.0, 28.0, 600.0);
    /// `type.heading-2` — 16 / 24, weight 500: a proposed choice, empty states.
    pub const HEADING_2: TypeToken = TypeToken::new(16.0, 24.0, 500.0);
    /// `type.heading-3` — 14 / 20, weight 600: section headings.
    pub const HEADING_3: TypeToken = TypeToken::new(14.0, 20.0, 600.0);
    /// `type.body` — 14 / 22, weight 400: reading text.
    pub const BODY: TypeToken = TypeToken::new(14.0, 22.0, 400.0);
    /// `type.body-small` — 13 / 19, weight 400: chrome and list text.
    pub const BODY_SMALL: TypeToken = TypeToken::new(13.0, 19.0, 400.0);
    /// `type.row-title` — 13 / 19, weight 500: the name in a list row.
    pub const ROW_TITLE: TypeToken = TypeToken::new(13.0, 19.0, 500.0);
    /// `type.label` — 12 / 16, weight 500: panel titles and field labels.
    pub const LABEL: TypeToken = TypeToken::new(12.0, 16.0, 500.0);
    /// `type.meta` — 11 / 16, weight 400: dates, counts, paths.
    pub const META: TypeToken = TypeToken::new(11.0, 16.0, 400.0);
    /// `type.code` — 12.5 / 20, weight 400.
    pub const CODE: TypeToken = TypeToken::new(12.5, 20.0, 400.0);
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

/// Control heights. Every button, tab and field picks one of these instead of
/// a local literal, so rows of mixed controls share a baseline.
#[derive(Clone, Copy, Debug, Default)]
pub struct ControlSize;

impl ControlSize {
    /// `control.sm` = 28: title-bar and inline icon actions.
    pub const SM: f32 = 28.0;
    /// `control.md` = 32: tabs, toolbar buttons, fields and review actions.
    pub const MD: f32 = 32.0;
}

/// Corner radii (`radius.control` … `radius.round`).
#[derive(Clone, Copy, Debug, Default)]
pub struct RadiusScale;

impl RadiusScale {
    /// `radius.control` = 6. Matches the 28–32 px control heights; 8 px read as
    /// a pill at that size and diverged from the 6 px the Decisions surface used.
    pub const CONTROL: f32 = 6.0;
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

    /// Hover spring: critically damped (no overshoot), settles in ~200 ms, and
    /// keeps its velocity when the pointer leaves mid-way, so a fast sweep
    /// across a list reads as one soft wave instead of flickering rows.
    pub const HOVER_SPRING: gpui::SpringConfig = gpui::SpringConfig::new(500.0, 44.7, 1.0);

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
        for colors in [ColorTokens::quiet_glass(), ColorTokens::charcoal()] {
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
}
