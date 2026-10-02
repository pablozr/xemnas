//! The motion catalog: every duration and curve the app animates with, and
//! the small entrance/exit helpers built on them.
//!
//! Durations follow how often an interaction happens: a content swap (every
//! selection) stays near 200 ms, a menu opens in 140 ms and leaves in 100 ms,
//! since leaving faster than arriving reads as responsive. GPUI has no scale
//! transform on `div`, so "pop" entrances are a fade plus a few pixels of
//! travel, applied with `relative().top()` so siblings never move.
//!
//! Adapted from Zeron's motion catalog (MIT, © 2026 Wing,
//! github.com/zeronsh/zeron at 27480d99); see `NOTICE`.

use std::time::Duration;

use gpui::prelude::*;
use gpui::{px, Animation, AnimationElement, AnimationExt, ElementId};

use super::curve::{CubicBezier, EASE, EASE_IN, EASE_OUT_EXPO, EASE_OUT_QUINT};

/// One catalog entry: how long and along which curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MotionSpec {
    duration_ms: u64,
    curve: CubicBezier,
}

impl MotionSpec {
    /// A spec of `duration_ms` along `curve`.
    pub const fn new(duration_ms: u64, curve: CubicBezier) -> Self {
        Self { duration_ms, curve }
    }

    /// Wall-clock length.
    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.duration_ms)
    }

    /// Eased progress for raw progress `t` (0..1).
    pub fn progress(&self, t: f32) -> f32 {
        self.curve.eval(t)
    }

    /// Eased progress of a phase that started `elapsed` ago: 1 once done.
    pub fn progress_since(&self, elapsed: Duration) -> f32 {
        let total = self.duration().as_secs_f32();
        if total <= 0.0 {
            return 1.0;
        }
        self.progress((elapsed.as_secs_f32() / total).min(1.0))
    }

    /// A one-shot GPUI animation for this spec.
    pub fn animation(&self) -> Animation {
        let spec = *self;
        Animation::new(spec.duration()).with_easing(move |t| spec.progress(t))
    }
}

/// Content replacing content (a destination, a project, an item): a fade
/// with a 4 px rise.
pub const CONTENT_IN: MotionSpec = MotionSpec::new(220, EASE_OUT_EXPO);
/// A menu, palette or popover arriving.
pub const MENU_IN: MotionSpec = MotionSpec::new(140, EASE);
/// The same leaving: faster than it came.
pub const MENU_OUT: MotionSpec = MotionSpec::new(100, EASE_IN);
/// A larger floating panel (the assistant) arriving.
pub const PANEL_IN: MotionSpec = MotionSpec::new(200, EASE_OUT_EXPO);
/// A selection indicator gliding to its new place.
pub const SLIDE: MotionSpec = MotionSpec::new(180, EASE_OUT_QUINT);
/// A toast arriving at the foot of a surface.
pub const TOAST_IN: MotionSpec = MotionSpec::new(180, EASE_OUT_EXPO);

/// Content arriving: opacity 0 → 1 while settling 4 px up into place.
pub fn content_in<E>(id: impl Into<ElementId>, element: E) -> AnimationElement<E>
where
    E: Styled + IntoElement + 'static,
{
    element.with_animation(id, CONTENT_IN.animation(), |element, t| {
        element.relative().opacity(t).top(px(4.0 * (1.0 - t)))
    })
}

/// A popover arriving from its trigger's side: `from` is the signed starting
/// offset (positive below the resting place for menus that open upward).
/// Starts at a third of its opacity so it never blinks in from nothing.
pub fn menu_in<E>(id: impl Into<ElementId>, from: f32, element: E) -> AnimationElement<E>
where
    E: Styled + IntoElement + 'static,
{
    element.with_animation(id, MENU_IN.animation(), move |element, t| {
        element
            .relative()
            .opacity(0.3 + 0.7 * t)
            .top(px(from * (1.0 - t)))
    })
}

/// A popover leaving toward its trigger. `t` is the exit progress the caller
/// derives from [`crate::ui::popup::Popup::exit_progress`]: an element-keyed
/// clock would restart on a remount and flash the popover back to full
/// opacity; the wall-clock progress cannot go backwards. The animation here
/// only keeps frames coming for the exit's span.
pub fn menu_out<E>(id: impl Into<ElementId>, toward: f32, t: f32, element: E) -> AnimationElement<E>
where
    E: Styled + IntoElement + 'static,
{
    element.with_animation(id, MENU_OUT.animation(), move |element, _| {
        element
            .relative()
            .opacity(1.0 - t)
            .top(px(toward * 0.5 * t))
    })
}

/// A larger panel arriving: fade with an 8 px rise.
pub fn panel_in<E>(id: impl Into<ElementId>, element: E) -> AnimationElement<E>
where
    E: Styled + IntoElement + 'static,
{
    element.with_animation(id, PANEL_IN.animation(), |element, t| {
        element.relative().opacity(t).top(px(8.0 * (1.0 - t)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exits_are_faster_than_entrances() {
        assert!(MENU_OUT.duration() < MENU_IN.duration());
        assert!(MENU_IN.duration() < CONTENT_IN.duration());
    }

    #[test]
    fn progress_since_reaches_one_and_stays() {
        assert_eq!(MENU_OUT.progress_since(Duration::ZERO), 0.0);
        assert_eq!(MENU_OUT.progress_since(Duration::from_millis(100)), 1.0);
        assert_eq!(MENU_OUT.progress_since(Duration::from_secs(5)), 1.0);
        let half = MENU_OUT.progress_since(Duration::from_millis(50));
        assert!(half > 0.0 && half < 1.0);
    }
}
