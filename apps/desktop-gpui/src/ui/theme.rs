//! Themes: the single entry point views use to reach tokens.
//!
//! A [`Theme`] bundles the token groups so primitives receive one typed object
//! instead of reaching for loose values. Both palettes apply to every screen;
//! navigation never changes the user's selected mode.

use gpui::{App, Global, Styled};

/// One palette for the entire application, independent of navigation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    /// Original blue graphite palette.
    QuietGlass,
    /// Neutral charcoal palette.
    Charcoal,
}
impl Global for ThemeMode {}
impl ThemeMode {
    /// Selects the other supported palette.
    pub fn toggled(self) -> Self {
        match self {
            Self::QuietGlass => Self::Charcoal,
            Self::Charcoal => Self::QuietGlass,
        }
    }
    /// Label shown in the global theme control.
    pub fn label(self) -> &'static str {
        match self {
            Self::QuietGlass => "Quiet Glass",
            Self::Charcoal => "Carvão",
        }
    }
    /// Resolves the complete token set for this mode.
    pub fn theme(self) -> Theme {
        match self {
            Self::QuietGlass => Theme::quiet_glass(),
            Self::Charcoal => Theme::charcoal(),
        }
    }
}

use crate::ui::tokens::{
    ColorTokens, MotionTokens, RadiusScale, SpacingScale, TypeScale, TypeToken,
};

/// Shared theme tokens.
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
    /// Reads the palette selected by the global theme control.
    pub fn current(cx: &App) -> Self {
        cx.try_global::<ThemeMode>()
            .copied()
            .unwrap_or_default()
            .theme()
    }
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
            colors: ColorTokens::quiet_glass(),
            type_scale: TypeScale,
            spacing: SpacingScale,
            radius: RadiusScale,
            motion: MotionTokens,
        }
    }
    /// Editorial charcoal theme; shares typography, spacing and control geometry.
    pub const fn charcoal() -> Self {
        Self {
            colors: ColorTokens::charcoal(),
            ..Self::quiet_glass()
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
