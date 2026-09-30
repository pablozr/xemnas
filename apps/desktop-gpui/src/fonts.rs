//! Embedded fonts and imagery for the desktop app.
//!
//! Inter Variable (interface), JetBrains Mono (code / technical IDs) and
//! Bricolage Grotesque (wordmark and reading titles) are compiled into the binary and
//! registered through the GPUI text system, so the app does not depend on fonts
//! installed on the machine. The app mark is embedded the same way, so the
//! window never opens with an empty hole where the icon belongs. The OFL-1.1
//! licence texts ship next to the font files in `assets/fonts/`.

use std::borrow::Cow;
use std::path::PathBuf;

use crate::ui::theme::text_style;
use crate::ui::tokens::TypeToken;
use gpui::prelude::*;

use crate::ui::theme::Theme;
use gpui::{div, img, px, App};

/// Interface font: `Inter Variable` (SIL Open Font License 1.1).
const INTER_VARIABLE: &[u8] = include_bytes!("../assets/fonts/InterVariable.ttf");

/// Monospace font: `JetBrains Mono` (SIL Open Font License 1.1).
const JETBRAINS_MONO: &[u8] = include_bytes!("../assets/fonts/JetBrainsMono-VariableFont_wght.ttf");

/// Display face: `Bricolage Grotesque` (SIL Open Font License 1.1).
///
/// The wordmark and the one reading title per pane (the question a decision
/// answers) use it; everything else stays on Inter, so the product keeps one
/// reading voice and one identity voice. The file is variable, but GPUI picks
/// named instances (opsz 14, weights 200–800) rather than setting axes.
const BRICOLAGE: &[u8] = include_bytes!("../assets/fonts/BricolageGrotesque-Variable.ttf");

/// The app mark: a decision crossing (chosen path unbroken, the alternative
/// interrupted) on a charcoal tile. Source: `assets/brand/xemnas-mark.svg`.
///
/// 256 px is the size the mark is drawn at. Every consumer scales it down, and
/// down-scaling a raster keeps the inner highlight intact where up-scaling
/// would smear it.
const APP_ICON: &[u8] = include_bytes!("../assets/images/app-icon-256.png");

/// Registers the embedded fonts with the application text system.
///
/// Failures are reported and recoverable: the theme's documented fallback
/// (`Segoe UI`) keeps the app usable if a face cannot be installed.
pub fn register_embedded(cx: &App) {
    let inter: &'static [u8] = INTER_VARIABLE;
    let mono: &'static [u8] = JETBRAINS_MONO;
    let display: &'static [u8] = BRICOLAGE;
    let fonts = vec![
        Cow::Borrowed(inter),
        Cow::Borrowed(mono),
        Cow::Borrowed(display),
    ];
    if let Err(error) = cx.text_system().add_fonts(fonts) {
        eprintln!("failed to register embedded fonts: {error}");
    }
    let families = crate::ui::theme::resolve_font_families(&cx.text_system().all_font_names());
    tracing::debug!(
        interface = %families.interface,
        display = %families.display,
        mono = %families.mono,
        "resolved embedded font families"
    );
}

/// Materialises the embedded icon as a file that `img` can load.
///
/// `img` takes an `ImageSource`, and a file path is the only source that needs
/// neither a network fetch nor a registered `AssetSource`. The file is written
/// once and reused for the process lifetime. A write failure is logged (no file
/// content, PRIV-001); the image then simply does not appear, which is a
/// degraded title bar rather than a failed frame.
fn app_icon_path() -> PathBuf {
    // Named after the bytes, so a new mark never reuses a stale cached file.
    let digest = APP_ICON
        .iter()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
        });
    let path = std::env::temp_dir().join(format!("xemnas-mark-{digest:016x}.png"));
    if !path.exists() {
        if let Err(error) = std::fs::write(&path, APP_ICON) {
            tracing::error!(
                error = %error,
                operation = "write_app_icon",
                "could not materialise the app icon for the image cache"
            );
        }
    }
    path
}

/// The app mark at `size` logical pixels.
///
/// The asset already carries its own rounded corners and inner highlight, so
/// this is a plain scaled image with no decorative chrome around it.
pub fn app_icon(size: f32) -> impl IntoElement {
    img(app_icon_path()).w(px(size)).h(px(size)).flex_none()
}

/// The product name as the wordmark spells it.
const WORDMARK: &str = "xemnas";

/// The `xemnas` wordmark at `size` logical pixels.
///
/// Set on the title bar and nowhere else. The design system asks for
/// "tracking quase neutro" and forbids over-spaced titles, and this GPUI
/// revision exposes no letter-spacing at all, so the identity is carried by the
/// face (display optical size, firm weight) rather than by tracking — the same
/// split Linear and Zed use between chrome and wordmark.
pub fn wordmark(theme: &Theme, size: f32, weight: f32) -> impl IntoElement {
    let token = TypeToken::new(size, size * 1.25, weight);
    text_style(div(), token)
        .font_family(Theme::font_display())
        .text_color(theme.colors.text_primary())
        .child(WORDMARK)
}
