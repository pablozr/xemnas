//! Quiet Glass UI layer.
//!
//! Tokens and the theme live in [`tokens`] and [`theme`]; the reusable
//! primitives live in [`glass`], [`controls`] and [`feedback`]. Screen-specific
//! components (NavigationRail, InboxRow, DisclosureRow, ProposalSurface,
//! DiffViewer) are deliberately **not** here: they belong to tickets 06/15/16.

pub mod appearance;
pub mod controls;
pub mod feedback;
pub mod glass;
pub mod icons;
pub mod material;
pub mod motion;
pub mod patterns;
pub mod popup;
pub mod search_edit;
pub mod search_field;
pub mod theme;
pub mod tokens;
pub mod tooltip;
pub mod wallpaper;
