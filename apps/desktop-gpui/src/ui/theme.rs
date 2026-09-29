//! The Quiet Glass theme: the single entry point views use to reach tokens.
//!
//! A [`Theme`] bundles the token groups so primitives receive one typed object
//! instead of reaching for loose values. The only theme today is
//! [`Theme::quiet_glass`].

use gpui::Styled;

use crate::ui::tokens::{
    ColorTokens, MotionTokens, RadiusScale, SpacingScale, TypeScale, TypeToken,
};

/// The Quiet Glass theme.
#[derive(Clone, Copy, Debug, Default)]
pub struct Theme {
    /// Color tokens.
    pub colors: ColorTokens,
    /// Typographic scale.
    pub type_scale: TypeScale,
    /// Spacing scale.
    pub spacing: SpacingScale,
    /// Corner radii.
    pub radius: RadiusScale,
    /// Motion durations and easings.
    pub motion: MotionTokens,
}

impl Theme {
    /// Interface font family registered from the embedded Inter Variable file.
    ///
    /// If registration fails, the GPUI/OS text system resolves a fallback on
    /// its own; that fallback is platform-provided (typically Segoe UI on
    /// Windows) and is not guaranteed by this crate, so there is deliberately
    /// no `FONT_FALLBACK` constant pretending otherwise.
    pub const FONT_INTERFACE: &'static str = "Inter Variable";
    /// Monospace font family for code and technical IDs.
    pub const FONT_MONO: &'static str = "JetBrains Mono";

    /// Builds the approved Quiet Glass theme.
    pub const fn quiet_glass() -> Self {
        Self {
            colors: ColorTokens,
            type_scale: TypeScale,
            spacing: SpacingScale,
            radius: RadiusScale,
            motion: MotionTokens,
        }
    }
}

/// Applies a typographic token with the interface font family.
pub fn text_style<T: Styled>(mut element: T, token: TypeToken) -> T {
    element = element.font_family(Theme::FONT_INTERFACE);
    element = element.font_weight(token.font_weight());
    element = element.text_size(token.size_px());
    element = element.line_height(token.line_height_px());
    element
}

/// Applies a typographic token with the monospace font family (`type.code`).
pub fn code_style<T: Styled>(mut element: T, token: TypeToken) -> T {
    element = element.font_family(Theme::FONT_MONO);
    element = element.font_weight(token.font_weight());
    element = element.text_size(token.size_px());
    element = element.line_height(token.line_height_px());
    element
}
