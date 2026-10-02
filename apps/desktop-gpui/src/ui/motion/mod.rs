//! Motion: the curves, the catalog of durations, entrance and exit helpers,
//! and the shared clock for things that move on their own.
//!
//! Rules: a duration comes from [`spec`], never a literal in a screen;
//! continuous motion goes through [`clock`], never a repeating
//! `with_animation`; reduced motion is honoured by GPUI for one-shot
//! animations and by [`clock`] for loops.

pub mod clock;
pub mod curve;
pub mod glide;
pub mod spec;

pub use spec::{cascade, content_in, menu_in, menu_out, panel_in, MotionSpec};
