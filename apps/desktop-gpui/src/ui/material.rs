//! The material of controls: a vertical gradient lit from above, a hairline
//! rim, a one-pixel inner highlight on the top edge and a soft drop beneath.
//!
//! A flat fill reads as a sticker on the surface; these four layers make a
//! button read as a raised, slightly glassy plate without blur (GPUI has no
//! backdrop blur on this revision). Both palettes are dark, so every recipe
//! lifts translucent white off the surface. Colors come from the theme; the
//! amounts here are the recipe.
//!
//! The recipe (gradient, rim, inset highlight, drop) follows Zeron's
//! `glass.rs` plates (MIT, © 2026 Wing, github.com/zeronsh/zeron at
//! 27480d99); see `NOTICE`.

use gpui::prelude::*;
use gpui::{linear_color_stop, linear_gradient, px, Background, BoxShadow, Hsla, Rgba};

use crate::ui::theme::Theme;

/// One surface treatment, applied to any styled element.
pub struct Plate {
    background: Background,
    rim: Hsla,
    shadows: Vec<BoxShadow>,
}

impl Plate {
    /// Applies fill, rim and shadows.
    pub fn apply<E: Styled>(self, element: E) -> E {
        element
            .bg(self.background)
            .border_1()
            .border_color(self.rim)
            .shadow(self.shadows)
    }

    /// The fill alone, for a hover or pressed style that keeps the rim.
    pub fn background(&self) -> Background {
        self.background
    }

    /// The shadows alone (hover styles replace the whole list).
    pub fn shadows(&self) -> Vec<BoxShadow> {
        self.shadows.clone()
    }
}

fn white(theme: &Theme, alpha: f32) -> Hsla {
    Hsla::from(theme.colors.emphasis_highlight()).alpha(alpha)
}

fn black(theme: &Theme, alpha: f32) -> Hsla {
    Hsla::from(theme.colors.shadow_emphasis()).alpha(alpha)
}

fn vertical(top: Hsla, bottom: Hsla) -> Background {
    linear_gradient(
        180.0,
        linear_color_stop(top, 0.0),
        linear_color_stop(bottom, 1.0),
    )
}

fn shadow(color: Hsla, y: f32, blur: f32, inset: bool) -> BoxShadow {
    let shadow = BoxShadow::new(px(0.0), px(y), color).blur_radius(px(blur));
    if inset {
        shadow.inset()
    } else {
        shadow
    }
}

/// `color` lifted toward white by `amount`, keeping its alpha.
fn lift(color: Rgba, amount: f32) -> Hsla {
    let color = Hsla::from(color);
    Hsla {
        l: color.l + (1.0 - color.l) * amount,
        ..color
    }
}

/// The neutral plate of a secondary control. `lit` (0 rest, 1 hover) raises
/// the whole treatment one step.
pub fn neutral_plate(theme: &Theme, lit: f32) -> Plate {
    let top = 0.075 + 0.035 * lit;
    Plate {
        background: vertical(white(theme, top), white(theme, top * 0.55)),
        rim: white(theme, 0.10 + 0.05 * lit),
        shadows: vec![
            shadow(white(theme, 0.07 + 0.03 * lit), 1.0, 0.0, true),
            shadow(black(theme, 0.22), 1.0, 2.0, false),
        ],
    }
}

/// The accent plate of the one primary action. `glow` (0 rest, 1 hover)
/// spreads its lavender halo.
pub fn accent_plate(theme: &Theme, glow: f32) -> Plate {
    let base = theme.colors.accent_emphasis();
    let top = lift(base, 0.10 + 0.06 * glow);
    let halo = Hsla::from(theme.colors.accent_emphasis()).opacity(0.18 + 0.22 * glow);
    Plate {
        background: vertical(top, Hsla::from(base)),
        rim: lift(base, 0.35).opacity(0.55),
        shadows: vec![
            shadow(white(theme, 0.22), 1.0, 0.0, true),
            BoxShadow::new(px(0.0), px(2.0 + 2.0 * glow), halo).blur_radius(px(6.0 + 10.0 * glow)),
        ],
    }
}

/// The accent plate pressed: flatter, no halo, one step deeper.
pub fn accent_pressed(theme: &Theme) -> Plate {
    let base = theme.colors.accent_default();
    Plate {
        background: vertical(Hsla::from(base), Hsla::from(base)),
        rim: lift(base, 0.2).opacity(0.5),
        shadows: vec![shadow(black(theme, 0.18), 1.0, 2.0, true)],
    }
}

/// How high a surface floats. Every floating surface takes one of these
/// instead of a shadow of its own, so depth reads the same across the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elevation {
    /// Tooltips and toasts: just off the surface.
    Hint,
    /// Menus and popovers (theme menu, project panel).
    Floating,
    /// Panels that hold a task (palette, assistant).
    Dialog,
}

/// The shadows of `level`: a wide soft drop, a tight contact shadow and a
/// faint lit top edge, so a floating surface reads as a raised sheet.
pub fn elevation(theme: &Theme, level: Elevation) -> Vec<BoxShadow> {
    let drop = Hsla::from(theme.colors.shadow_emphasis());
    let (y, blur, contact) = match level {
        Elevation::Hint => (4.0, 12.0, 0.18),
        Elevation::Floating => (12.0, 32.0, 0.22),
        Elevation::Dialog => (24.0, 56.0, 0.26),
    };
    vec![
        BoxShadow::new(px(0.0), px(y), drop).blur_radius(px(blur)),
        BoxShadow::new(px(0.0), px(1.0), black(theme, contact)).blur_radius(px(2.0)),
        shadow(white(theme, 0.05), 1.0, 0.0, true),
    ]
}
