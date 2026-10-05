//! The person's appearance choices: theme, background and interface
//! language, saved beside the other settings (`settings/appearance.json` in
//! the data folder) and read before the first window opens, so the app never
//! flashes the default.
//!
//! This is a preference of the interface, not product data: it lives in the
//! desktop app, as a JSON file the composition root points at.

use std::path::{Path, PathBuf};

use gpui::{App, Global};

use crate::i18n::{self, Language};
use crate::ui::theme::ThemeMode;

/// What sits behind the window's surfaces.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Wallpaper {
    /// The theme's own solid canvas.
    #[default]
    None,
    /// One of the backgrounds shipped with the app, by id.
    Builtin(String),
    /// An image the person picked.
    Custom(PathBuf),
}

/// Three steps for each background control: the picker offers no sliders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    /// Light.
    Low,
    /// The default.
    #[default]
    Medium,
    /// Strong.
    High,
}

impl Level {
    /// Every level, in order.
    pub const ALL: [Self; 3] = [Self::Low, Self::Medium, Self::High];

    fn id(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.id() == id)
    }
}

/// The saved appearance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Appearance {
    /// The palette.
    pub theme: ThemeMode,
    /// The background image, if any.
    pub wallpaper: Wallpaper,
    /// How blurred the background is.
    pub blur: Level,
    /// How much the background is darkened.
    pub dim: Level,
    /// How solid the surfaces over it are.
    pub solidity: Level,
    /// The interface language; English until the person picks another.
    pub language: Language,
}

impl Global for Appearance {}

/// Where the appearance is saved; absent in tests and when no data folder.
#[derive(Clone, Debug)]
pub struct AppearanceFile(pub PathBuf);

impl Global for AppearanceFile {}

impl Appearance {
    /// Reads `path`; anything missing or unreadable falls back to defaults.
    pub fn load(path: &Path) -> Self {
        let Ok(text) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
            return Self::default();
        };
        let field = |name: &str| value.get(name).and_then(|field| field.as_str());
        let level = |name: &str| field(name).and_then(Level::from_id).unwrap_or_default();
        let wallpaper = match (field("wallpaper"), field("wallpaper_path")) {
            (Some("builtin"), Some(id)) => Wallpaper::Builtin(id.to_owned()),
            (Some("custom"), Some(path)) => Wallpaper::Custom(PathBuf::from(path)),
            _ => Wallpaper::None,
        };
        Self {
            theme: field("theme")
                .and_then(ThemeMode::from_id)
                .unwrap_or_default(),
            wallpaper,
            blur: level("blur"),
            dim: level("dim"),
            solidity: level("solidity"),
            language: field("language")
                .and_then(Language::from_id)
                .unwrap_or_default(),
        }
    }

    fn to_json(&self) -> serde_json::Value {
        let (kind, path) = match &self.wallpaper {
            Wallpaper::None => ("none", String::new()),
            Wallpaper::Builtin(id) => ("builtin", id.clone()),
            Wallpaper::Custom(path) => ("custom", path.to_string_lossy().into_owned()),
        };
        serde_json::json!({
            "theme": self.theme.id(),
            "wallpaper": kind,
            "wallpaper_path": path,
            "blur": self.blur.id(),
            "dim": self.dim.id(),
            "solidity": self.solidity.id(),
            "language": self.language.id(),
        })
    }

    /// Writes the appearance to `path`, creating its folder.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = serde_json::to_string_pretty(&self.to_json()).map_err(std::io::Error::other)?;
        std::fs::write(path, text)
    }
}

/// The appearance in effect.
pub fn current(cx: &App) -> Appearance {
    cx.try_global::<Appearance>().cloned().unwrap_or_default()
}

/// Makes the theme and the language follow `appearance`, without saving.
pub fn apply(cx: &mut App, appearance: Appearance) {
    cx.set_global(appearance.theme);
    i18n::set(appearance.language);
    cx.set_global(appearance);
}

/// Applies `change`, makes the theme and language follow it, saves it and
/// repaints every window. A failed save is logged; the change still applies
/// for this run.
pub fn update(cx: &mut App, change: impl FnOnce(&mut Appearance)) {
    let mut appearance = current(cx);
    change(&mut appearance);
    if let Some(file) = cx.try_global::<AppearanceFile>() {
        if let Err(error) = appearance.save(&file.0) {
            tracing::warn!(error = %error, operation = "save_appearance", "could not save");
        }
    }
    apply(cx, appearance);
    cx.refresh_windows();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_the_file_and_tolerates_garbage() {
        let dir = std::env::temp_dir().join(format!("xemnas-appearance-{}", std::process::id()));
        let path = dir.join("appearance.json");
        let appearance = Appearance {
            theme: ThemeMode::Organization,
            wallpaper: Wallpaper::Builtin("never".into()),
            blur: Level::High,
            dim: Level::Low,
            solidity: Level::Medium,
            language: Language::Japanese,
        };
        appearance.save(&path).expect("save");
        assert_eq!(Appearance::load(&path), appearance);
        std::fs::write(&path, "not json").expect("write");
        assert_eq!(Appearance::load(&path), Appearance::default());
        assert_eq!(
            Appearance::load(&dir.join("missing.json")),
            Appearance::default()
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}
