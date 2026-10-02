//! Themes: the single entry point views use to reach tokens.
//!
//! A [`Theme`] bundles the token groups so primitives receive one typed object
//! instead of reaching for loose values. Every palette applies to every screen;
//! navigation never changes the user's selected mode.

use std::sync::OnceLock;

use gpui::{App, Global, SharedString, Styled};

/// One palette for the entire application, independent of navigation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    /// Original blue graphite palette.
    QuietGlass,
    /// Neutral charcoal palette.
    Charcoal,
    /// Near-black and cold silver: the Organization.
    Organization,
    /// Green ink and sage.
    Moss,
    /// Navy and a muted cold cyan.
    Midnight,
}
impl Global for ThemeMode {}
impl ThemeMode {
    /// Every theme, in the order the picker lists them.
    pub const ALL: [Self; 5] = [
        Self::QuietGlass,
        Self::Charcoal,
        Self::Organization,
        Self::Moss,
        Self::Midnight,
    ];

    /// The next theme in [`Self::ALL`] (the palette command cycles).
    pub fn toggled(self) -> Self {
        let index = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(index + 1) % Self::ALL.len()]
    }

    /// Label shown in the theme picker.
    pub fn label(self) -> &'static str {
        match self {
            Self::QuietGlass => "Quiet Glass",
            Self::Charcoal => "Carvão",
            Self::Organization => "Organização",
            Self::Moss => "Musgo",
            Self::Midnight => "Meia-noite",
        }
    }

    /// One line on what the theme feels like.
    pub fn blurb(self) -> &'static str {
        match self {
            Self::QuietGlass => "Grafite azulado com lavanda. O original.",
            Self::Charcoal => "Carvão neutro, editorial, com lavanda.",
            Self::Organization => "Preto profundo e prata fria, como os casacos.",
            Self::Moss => "Verde-tinta com sálvia. Calmo e orgânico.",
            Self::Midnight => "Azul-marinho com um ciano frio e discreto.",
        }
    }

    /// Stable identifier for the saved preference.
    pub fn id(self) -> &'static str {
        match self {
            Self::QuietGlass => "quiet-glass",
            Self::Charcoal => "charcoal",
            Self::Organization => "organization",
            Self::Moss => "moss",
            Self::Midnight => "midnight",
        }
    }

    /// The theme saved as `id`, if it still exists.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.id() == id)
    }

    /// Resolves the complete token set for this mode.
    pub fn theme(self) -> Theme {
        let colors = match self {
            Self::QuietGlass => ColorTokens::quiet_glass(),
            Self::Charcoal => ColorTokens::charcoal(),
            Self::Organization => ColorTokens::organization(),
            Self::Moss => ColorTokens::moss(),
            Self::Midnight => ColorTokens::midnight(),
        };
        Theme {
            colors,
            ..Theme::quiet_glass()
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
        let mut theme = cx
            .try_global::<ThemeMode>()
            .copied()
            .unwrap_or_default()
            .theme();
        let glass = cx
            .try_global::<Backdrop>()
            .is_some_and(|backdrop| backdrop.0);
        theme.colors = theme.colors.with_glass(glass);
        // A background image turns every frame surface into glass over it;
        // "surfaces" sets how much of the image shows through.
        let appearance = crate::ui::appearance::current(cx);
        if appearance.wallpaper != crate::ui::appearance::Wallpaper::None
            && !crate::ui::wallpaper::failed(cx)
        {
            let (chrome, content) = match appearance.solidity {
                crate::ui::appearance::Level::Low => (0.34, 0.56),
                crate::ui::appearance::Level::Medium => (0.48, 0.72),
                crate::ui::appearance::Level::High => (0.66, 0.88),
            };
            theme.colors = theme.colors.with_veil(chrome, content);
        }
        theme
    }
    /// Interface family (embedded Inter Variable), as the platform named it.
    pub fn font_interface() -> SharedString {
        families().interface.clone()
    }
    /// Display family for reading titles and the wordmark (embedded
    /// Bricolage Grotesque). Every control, list and paragraph stays on Inter.
    pub fn font_display() -> SharedString {
        families().display.clone()
    }
    /// Monospace family for code and technical IDs (embedded JetBrains Mono).
    pub fn font_mono() -> SharedString {
        families().mono.clone()
    }

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

/// Whether the window material currently shows through the chrome. The shell
/// sets it every frame from the backdrop in use and the system appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Backdrop(pub bool);
impl Global for Backdrop {}

/// Family names as the platform text system registered the embedded files.
///
/// The names differ per platform: DirectWrite groups variable-font instances
/// by optical size, so Windows registers "Inter Variable Text" and
/// "Bricolage Grotesque 14pt", while other platforms use the typographic family
/// names. Asking for a name that does not exist silently falls back to the
/// system face, which is how the app ended up in Segoe UI.
#[derive(Clone, Debug)]
pub struct FontFamilies {
    /// Interface family.
    pub interface: SharedString,
    /// Display family.
    pub display: SharedString,
    /// Monospace family.
    pub mono: SharedString,
}

/// Candidates tried in order; the first registered one wins.
const INTERFACE_NAMES: &[&str] = &["Inter Variable Text", "Inter Variable", "Inter"];
const DISPLAY_NAMES: &[&str] = &["Bricolage Grotesque 14pt", "Bricolage Grotesque"];
const MONO_NAMES: &[&str] = &["JetBrains Mono", "JetBrains Mono Regular"];

static FAMILIES: OnceLock<FontFamilies> = OnceLock::new();

fn families() -> &'static FontFamilies {
    FAMILIES.get_or_init(|| FontFamilies {
        interface: INTERFACE_NAMES[0].into(),
        display: DISPLAY_NAMES[0].into(),
        mono: MONO_NAMES[0].into(),
    })
}

fn pick(available: &[String], candidates: &[&str]) -> SharedString {
    candidates
        .iter()
        .find(|name| available.iter().any(|family| family == *name))
        .copied()
        .unwrap_or(candidates[0])
        .into()
}

/// Resolves the families once, after the embedded fonts are registered.
/// Returns the chosen names so the caller can log them.
pub fn resolve_font_families(available: &[String]) -> FontFamilies {
    let resolved = FontFamilies {
        interface: pick(available, INTERFACE_NAMES),
        display: pick(available, DISPLAY_NAMES),
        mono: pick(available, MONO_NAMES),
    };
    let _ = FAMILIES.set(resolved.clone());
    resolved
}

/// Applies a typographic token with the interface font family.
pub fn text_style<T: Styled>(mut element: T, token: TypeToken) -> T {
    element = element.font_family(Theme::font_interface());
    element = element.font_weight(token.font_weight());
    element = element.text_size(token.size_px());
    element = element.line_height(token.line_height_px());
    element
}

/// Applies a typographic token with the monospace font family (`type.code`).
pub fn code_style<T: Styled>(mut element: T, token: TypeToken) -> T {
    element = element.font_family(Theme::font_mono());
    element = element.font_weight(token.font_weight());
    element = element.text_size(token.size_px());
    element = element.line_height(token.line_height_px());
    element
}
