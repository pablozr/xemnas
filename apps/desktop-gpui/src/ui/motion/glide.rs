//! A selection indicator that glides between items instead of jumping.
//!
//! The row of items reports where each child landed
//! (`on_children_prepainted`); the indicator is drawn behind the row,
//! absolutely, at the selected child's place. When the selection changes it
//! travels from wherever it is now — even mid-glide — to the new place along
//! [`super::spec::SLIDE`]. The first placement, a resize and reduced motion
//! all land without travel.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use gpui::{App, Bounds, EntityId, Pixels, Window};

use super::spec::SLIDE;

/// Horizontal place of one item: left edge and width, relative to the row.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Span {
    left: f32,
    width: f32,
}

impl Span {
    fn lerp(self, to: Span, t: f32) -> Span {
        Span {
            left: self.left + (to.left - self.left) * t,
            width: self.width + (to.width - self.width) * t,
        }
    }
}

/// Where the indicator is this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IndicatorFrame {
    /// Left edge relative to the row.
    pub left: f32,
    /// Width.
    pub width: f32,
    /// Whether it is still travelling (ask for another frame).
    pub moving: bool,
}

/// The state of one gliding indicator; keep it on the view.
#[derive(Default)]
pub struct SlideIndicator {
    measured: Rc<RefCell<Vec<Span>>>,
    selected: Option<usize>,
    from: Option<Span>,
    to: Option<Span>,
    started: Option<Instant>,
}

impl SlideIndicator {
    /// The listener for the row's `on_children_prepainted`: records where
    /// each item is and repaints `view` when that changed.
    pub fn measure(&self, view: EntityId) -> impl Fn(Vec<Bounds<Pixels>>, &mut Window, &mut App) {
        let measured = self.measured.clone();
        move |bounds, _, cx| {
            let Some(first) = bounds.first() else {
                return;
            };
            let origin = f32::from(first.origin.x);
            let spans: Vec<Span> = bounds
                .iter()
                .map(|bound| Span {
                    left: f32::from(bound.origin.x) - origin,
                    width: f32::from(bound.size.width),
                })
                .collect();
            if *measured.borrow() != spans {
                *measured.borrow_mut() = spans;
                cx.notify(view);
            }
        }
    }

    /// The indicator for `selected` this frame; `None` until the row has
    /// been measured once.
    pub fn frame(&mut self, selected: usize, reduce_motion: bool) -> Option<IndicatorFrame> {
        let target = *self.measured.borrow().get(selected)?;
        let now = Instant::now();
        let current = self.current(now);
        let reselected = self.selected != Some(selected);
        if reselected && self.selected.is_some() && !reduce_motion {
            // Travel from wherever it is now.
            self.from = current.or(Some(target));
            self.started = Some(now);
        } else if reselected || self.to != Some(target) && self.started.is_none() {
            // First placement, a resize at rest, or reduced motion: land.
            self.from = Some(target);
            self.started = None;
        }
        self.selected = Some(selected);
        self.to = Some(target);
        let span = self.current(now).unwrap_or(target);
        let moving = self.started.is_some();
        Some(IndicatorFrame {
            left: span.left,
            width: span.width,
            moving,
        })
    }

    fn current(&mut self, now: Instant) -> Option<Span> {
        let (from, to) = (self.from?, self.to?);
        let Some(started) = self.started else {
            return Some(to);
        };
        let t = SLIDE.progress_since(now.duration_since(started));
        if t >= 1.0 {
            self.started = None;
            self.from = Some(to);
            return Some(to);
        }
        Some(from.lerp(to, t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured(indicator: &SlideIndicator, spans: &[(f32, f32)]) {
        *indicator.measured.borrow_mut() = spans
            .iter()
            .map(|(left, width)| Span {
                left: *left,
                width: *width,
            })
            .collect();
    }

    #[test]
    fn lands_first_then_glides_on_a_new_selection() {
        let mut indicator = SlideIndicator::default();
        assert_eq!(indicator.frame(0, false), None, "not measured yet");
        measured(&indicator, &[(0.0, 60.0), (64.0, 90.0)]);
        let first = indicator.frame(0, false).expect("frame");
        assert_eq!((first.left, first.width, first.moving), (0.0, 60.0, false));
        let start = indicator.frame(1, false).expect("frame");
        assert!(start.moving);
        assert!(start.left < 64.0, "starts from where it was");
    }

    #[test]
    fn reduced_motion_lands_at_once() {
        let mut indicator = SlideIndicator::default();
        measured(&indicator, &[(0.0, 60.0), (64.0, 90.0)]);
        indicator.frame(0, true);
        let jump = indicator.frame(1, true).expect("frame");
        assert_eq!((jump.left, jump.width, jump.moving), (64.0, 90.0, false));
    }
}
