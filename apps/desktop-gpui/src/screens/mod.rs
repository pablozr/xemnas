//! Product screens.
//!
//! Each screen is a GPUI view that calls the application use cases and renders
//! Quiet Glass primitives. Screen-specific components live beside their screen
//! (for example [`projects`]) instead of in [`crate::ui`].

mod evidence;
pub mod inbox;
pub mod projects;
mod review_editor;
