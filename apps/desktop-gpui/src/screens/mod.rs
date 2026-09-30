//! Product screens.
//!
//! Each screen is a GPUI view that calls the application use cases and renders
//! Quiet Glass primitives. Screen-specific components live beside their screen
//! (for example [`projects`]) instead of in [`crate::ui`].

mod decision_editor;
pub mod decisions;
mod evidence;
mod format;
pub mod inbox;
pub mod projects;
mod review_editor;
