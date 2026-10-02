//! CSS `cubic-bezier()` timing functions.
//!
//! GPUI ships only a few fixed easings, so the design system's curves
//! (`easing.enter` = `cubic-bezier(0.16, 1, 0.3, 1)`) were approximated by
//! `ease_out_quint`. This evaluates the real curve: solve x(t) = progress by
//! Newton iteration with a bisection fallback, then read y(t).
//!
//! Adapted from Zeron's `crates/ui/src/motion.rs` (MIT, © 2026 Wing,
//! github.com/zeronsh/zeron at 27480d99); see `NOTICE` at the repository root.

/// A `cubic-bezier(x1, y1, x2, y2)` with endpoints fixed at (0,0) and (1,1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    /// The curve through control points (x1, y1) and (x2, y2).
    pub const fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
    }

    fn coefficients(a: f32, b: f32) -> (f32, f32, f32) {
        let c = 3.0 * a;
        let bb = 3.0 * (b - a) - c;
        (1.0 - c - bb, bb, c)
    }

    fn sample(t: f32, p1: f32, p2: f32) -> f32 {
        let (a, b, c) = Self::coefficients(p1, p2);
        ((a * t + b) * t + c) * t
    }

    fn slope(&self, t: f32) -> f32 {
        let (a, b, c) = Self::coefficients(self.x1, self.x2);
        (3.0 * a * t + 2.0 * b) * t + c
    }

    fn solve(&self, x: f32) -> f32 {
        let mut t = x;
        for _ in 0..8 {
            let error = Self::sample(t, self.x1, self.x2) - x;
            if error.abs() < 1e-6 {
                return t;
            }
            let slope = self.slope(t);
            if slope.abs() < 1e-6 {
                break;
            }
            t -= error / slope;
        }
        let (mut low, mut high) = (0.0_f32, 1.0_f32);
        for _ in 0..32 {
            let middle = (low + high) / 2.0;
            if Self::sample(middle, self.x1, self.x2) < x {
                low = middle;
            } else {
                high = middle;
            }
        }
        (low + high) / 2.0
    }

    /// Eased output for progress `x`, clamped to 0..1 (GPUI asserts its
    /// animation delta stays inside that range).
    pub fn eval(&self, x: f32) -> f32 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }
        Self::sample(self.solve(x), self.y1, self.y2).clamp(0.0, 1.0)
    }

    /// This curve as a GPUI easing function.
    pub fn easing(self) -> impl Fn(f32) -> f32 + 'static {
        move |x| self.eval(x)
    }
}

/// `easing.enter`: fast start, long settle. Entrances and content swaps.
pub const EASE_OUT_EXPO: CubicBezier = CubicBezier::new(0.16, 1.0, 0.3, 1.0);
/// CSS `ease`: menus and dialogs.
pub const EASE: CubicBezier = CubicBezier::new(0.25, 0.1, 0.25, 1.0);
/// `easeOutQuint`: an indicator gliding to a new place.
pub const EASE_OUT_QUINT: CubicBezier = CubicBezier::new(0.22, 1.0, 0.36, 1.0);
/// `easing.exit`: things leaving accelerate away.
pub const EASE_IN: CubicBezier = CubicBezier::new(0.4, 0.0, 1.0, 1.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_are_fixed_and_the_curve_is_monotonic() {
        for curve in [EASE_OUT_EXPO, EASE, EASE_OUT_QUINT, EASE_IN] {
            assert_eq!(curve.eval(0.0), 0.0);
            assert_eq!(curve.eval(1.0), 1.0);
            let mut last = 0.0;
            for step in 1..=100 {
                let value = curve.eval(step as f32 / 100.0);
                assert!(value >= last - 1e-5, "{curve:?} dips at {step}");
                assert!((0.0..=1.0).contains(&value));
                last = value;
            }
        }
    }

    #[test]
    fn expo_out_is_mostly_done_early() {
        // The point of the enter curve: most of the travel in the first third.
        assert!(EASE_OUT_EXPO.eval(0.3) > 0.8);
        assert!(EASE_IN.eval(0.3) < 0.2);
    }

    #[test]
    fn linear_bezier_is_identity() {
        let linear = CubicBezier::new(0.0, 0.0, 1.0, 1.0);
        for step in 0..=10 {
            let x = step as f32 / 10.0;
            assert!((linear.eval(x) - x).abs() < 1e-4);
        }
    }
}
