//! xemnas desktop application crate.
//!
//! Library surface shared by the `xemnas` binary and the `quiet-glass-gallery`
//! evidence binary. UI tokens and reusable Quiet Glass primitives live under
//! [`ui`]; product screens live under [`screens`] and the window shell under
//! [`app`]. The embedded fonts are registered through [`fonts`].
#![warn(missing_docs)]

pub mod app;
pub mod fonts;
pub mod palette;
pub mod screens;
pub mod ui;
