//! Inline SVG icons.
//!
//! The product needs a small, consistent icon family. Rather than pull in a
//! full icon library or register an `AssetSource`, every icon is a static byte
//! string rendered with `gpui::svg().data(...)`, which keys the sprite cache
//! on the bytes' hash (gpui/src/elements/svg.rs) — the same glyph always lands
//! on the same cache entry.
//!
//! ## Why the icons are alpha masks
//!
//! `paint_svg` rasterises each icon as an **alpha mask** and tints it with
//! `style.text.color`. Two consequences, and both are load-bearing:
//!
//! 1. The `stroke="white"` inside the markup only feeds the alpha channel, so
//!    one source can be tinted by any state colour.
//! 2. `paint` returns early unless `style.text.color` is `Some` — an SVG with
//!    no colour on the element itself draws nothing. [`icon`] therefore always
//!    takes the colour explicitly.
//!
//! ## Colour is the caller's
//!
//! Glyphs used to carry their own colour (a dark `edit` that vanished on a
//! disabled button, a lavender `activity` beside muted siblings). An icon now
//! takes the colour of the text it sits next to:
//! [`crate::ui::controls::button_foreground`] for buttons, the label colour
//! for tabs and rows.

use gpui::prelude::*;
use gpui::{div, px, AnyElement, Hsla};

/// Shared frame: 24×24 grid, 1.75 px round stroke. Keeping it in one place is
/// what makes the family read as one set.
macro_rules! glyph {
    ($body:literal) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">"#,
            $body,
            "</svg>"
        )
    };
}

/// The icon family. One glyph per concept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconName {
    /// The review queue.
    List,
    /// A decision document or a source file.
    File,
    /// Project properties.
    Info,
    /// A tracked project folder.
    Folder,
    /// Registering a new project folder.
    FolderPlus,
    /// Adding something new, on a primary fill.
    Plus,
    /// Searching a list.
    Search,
    /// Palette switch.
    Contrast,
    /// Version history.
    Clock,
    /// Context of a decision (the section).
    Layers,
    /// Scope: what the decision covers.
    Target,
    /// Assumptions held as true.
    CheckCircle,
    /// Consequences that follow.
    Activity,
    /// Conditions to reconsider the decision.
    Rotate,
    /// Evidence and provenance links.
    Link,
    /// Status filter.
    Filter,
    /// Export to a file.
    Export,
    /// Copy source text.
    Copy,
    /// Revise a document.
    Edit,
    /// Enlarge the source viewport.
    Expand,
    /// Collapsed disclosure.
    ChevronRight,
    /// Open disclosure.
    ChevronDown,
    /// Application settings.
    Settings,
    /// AI and privacy settings.
    Shield,
    /// Leaving a page back to where the user came from.
    ArrowLeft,
    /// A pending requirement (the met one is [`IconName::CheckCircle`]).
    Circle,
    /// Work done on this machine.
    Cpu,
    /// A remote provider.
    Cloud,
    /// A stored credential.
    Key,
    /// What becomes visible to a third party.
    Eye,
    /// A confirmed step or a selected option.
    Check,
    /// A signed-in account (the ChatGPT plan).
    User,
    /// A link that opens outside the app, in the browser.
    ArrowUpRight,
    /// The project map: entities and what ties them together.
    Graph,
}

impl IconName {
    fn markup(self) -> &'static str {
        match self {
            Self::List => {
                glyph!(r#"<path d="M8 6h12M8 12h12M8 18h12M3.5 6h.5M3.5 12h.5M3.5 18h.5"/>"#)
            }
            Self::File => glyph!(
                r#"<path d="M14 2.5H6.5a2 2 0 0 0-2 2v15a2 2 0 0 0 2 2h11a2 2 0 0 0 2-2V8z"/><path d="M14 2.5V8h5.5"/>"#
            ),
            Self::Info => {
                glyph!(r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 11v5.5M12 7.75h.01"/>"#)
            }
            Self::Folder => glyph!(
                r#"<path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.2a1.5 1.5 0 0 1 1.2.6l1 1.4h8.6A1.5 1.5 0 0 1 21 9.5v9A1.5 1.5 0 0 1 19.5 20h-15A1.5 1.5 0 0 1 3 18.5z"/>"#
            ),
            Self::FolderPlus => glyph!(
                r#"<path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.2a1.5 1.5 0 0 1 1.2.6l1 1.4h8.6A1.5 1.5 0 0 1 21 9.5v9A1.5 1.5 0 0 1 19.5 20h-15A1.5 1.5 0 0 1 3 18.5z"/><path d="M12 11v5M9.5 13.5h5"/>"#
            ),
            Self::Plus => glyph!(r#"<path d="M12 5v14M5 12h14"/>"#),
            Self::Search => {
                glyph!(r#"<circle cx="11" cy="11" r="6.5"/><path d="m16 16 4.5 4.5"/>"#)
            }
            Self::Contrast => glyph!(
                r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="white" stroke="none"/>"#
            ),
            Self::Clock => glyph!(r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#),
            Self::Layers => glyph!(
                r#"<path d="m12 3 9 4.5-9 4.5-9-4.5z"/><path d="m3 12 9 4.5 9-4.5M3 16.5 12 21l9-4.5"/>"#
            ),
            Self::Target => glyph!(
                r#"<circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="4.5"/><circle cx="12" cy="12" r=".75"/>"#
            ),
            Self::CheckCircle => {
                glyph!(r#"<circle cx="12" cy="12" r="8.5"/><path d="m8.5 12.25 2.5 2.5 4.5-5"/>"#)
            }
            Self::Activity => glyph!(r#"<path d="M3 12h4l2.5-6 4 12 2.5-6h5"/>"#),
            Self::Rotate => glyph!(
                r#"<path d="M4 12a8 8 0 0 1 13.66-5.66L20 8.5"/><path d="M20 4v4.5h-4.5"/><path d="M20 12a8 8 0 0 1-13.66 5.66L4 15.5"/><path d="M4 20v-4.5h4.5"/>"#
            ),
            Self::Link => glyph!(
                r#"<path d="M10 13.5a4.5 4.5 0 0 0 6.8.5l2.7-2.7a4.5 4.5 0 0 0-6.4-6.4l-1.5 1.5"/><path d="M14 10.5a4.5 4.5 0 0 0-6.8-.5l-2.7 2.7a4.5 4.5 0 0 0 6.4 6.4l1.5-1.5"/>"#
            ),
            Self::Filter => glyph!(r#"<path d="M4 6.5h16M7 12h10M10 17.5h4"/>"#),
            Self::Export => {
                glyph!(r#"<path d="M12 3.5v11m-4-4 4 4 4-4"/><path d="M5 15.5v4h14v-4"/>"#)
            }
            Self::Copy => glyph!(
                r#"<rect x="8.5" y="8.5" width="11.5" height="11.5" rx="2"/><path d="M15.5 8.5V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v7.5a2 2 0 0 0 2 2h2.5"/>"#
            ),
            Self::Edit => {
                glyph!(r#"<path d="M15 4.5 19.5 9 9 19.5H4.5V15z"/><path d="m13 6.5 4.5 4.5"/>"#)
            }
            Self::Expand => glyph!(r#"<path d="M9 4H4v5M15 4h5v5M4 15v5h5M20 15v5h-5"/>"#),
            Self::ChevronRight => glyph!(r#"<path d="m9.5 6 6 6-6 6"/>"#),
            Self::ChevronDown => glyph!(r#"<path d="m6 9.5 6 6 6-6"/>"#),
            Self::Settings => glyph!(
                r#"<circle cx="12" cy="12" r="3"/><path d="M12 2.75v2.5M12 18.75v2.5M2.75 12h2.5M18.75 12h2.5M5.46 5.46l1.77 1.77M16.77 16.77l1.77 1.77M5.46 18.54l1.77-1.77M16.77 7.23l1.77-1.77"/><circle cx="12" cy="12" r="6.75"/>"#
            ),
            Self::Shield => {
                glyph!(r#"<path d="M12 3 5 6v5.5c0 4.2 2.9 7.8 7 9.5 4.1-1.7 7-5.3 7-9.5V6z"/>"#)
            }
            Self::ArrowLeft => glyph!(r#"<path d="M19 12H5m6-6-6 6 6 6"/>"#),
            Self::Circle => glyph!(r#"<circle cx="12" cy="12" r="8.5"/>"#),
            Self::Cpu => glyph!(
                r#"<rect x="6" y="6" width="12" height="12" rx="2"/><rect x="9.5" y="9.5" width="5" height="5" rx=".5"/><path d="M9.5 2.5V6M14.5 2.5V6M9.5 18v3.5M14.5 18v3.5M2.5 9.5H6M2.5 14.5H6M18 9.5h3.5M18 14.5h3.5"/>"#
            ),
            Self::Cloud => {
                glyph!(
                    r#"<path d="M7 18.5a4.5 4.5 0 0 1-.6-8.96A6 6 0 0 1 18 9a4.75 4.75 0 0 1-.5 9.5z"/>"#
                )
            }
            Self::Key => glyph!(
                r#"<circle cx="8" cy="15.5" r="4"/><path d="m10.9 12.6 8.6-8.6M16.5 7l2.5 2.5M14 9.5l2 2"/>"#
            ),
            Self::Eye => glyph!(
                r#"<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12z"/><circle cx="12" cy="12" r="3"/>"#
            ),
            Self::Check => glyph!(r#"<path d="m5 12.5 4.5 4.5L19 7.5"/>"#),
            Self::User => glyph!(
                r#"<circle cx="12" cy="8.5" r="3.75"/><path d="M4.75 20a7.25 7.25 0 0 1 14.5 0"/>"#
            ),
            Self::ArrowUpRight => glyph!(r#"<path d="M7 17 17 7M8.5 7H17v8.5"/>"#),
            Self::Graph => glyph!(
                r#"<circle cx="6" cy="6" r="2.5"/><circle cx="18" cy="8" r="2.5"/><circle cx="9" cy="18" r="2.5"/><path d="M8.3 7.2 15.6 7.7M7 8.4 8.4 15.6M16.6 10.1 10.7 16.2"/>"#
            ),
        }
    }
}

/// Renders `name` at `size` logical pixels, tinted to `color`.
pub fn icon(name: IconName, size: f32, color: impl Into<Hsla>) -> AnyElement {
    div()
        .flex_none()
        .size(px(size))
        .child(
            gpui::svg()
                .data(name.markup().as_bytes())
                .size(px(size))
                .text_color(color.into()),
        )
        .into_any_element()
}
