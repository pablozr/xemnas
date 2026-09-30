//! Inline SVG icons.
//!
//! The product needs a small, consistent icon family. Rather than pull in a
//! full icon library or register an `AssetSource`, every icon is a byte string
//! rendered with `gpui::svg().data(...)`, which copies the bytes into an `Arc`
//! and keys the sprite cache on their hash (gpui/src/elements/svg.rs:53-59) —
//! the same glyph always lands on the same cache entry.
//!
//! ## Why the icons are alpha masks
//!
//! `paint_svg` rasterises each icon through `render_alpha_mask`
//! (gpui/src/window.rs:4829): the SVG is drawn as an **alpha mask** and then
//! tinted with `style.text.color`. Two consequences, and both are load-bearing:
//!
//! 1. The `stroke="#rrggbb"` inside the markup is **ignored**. Only the alpha
//!    channel survives, so one icon source can be tinted by any state colour.
//! 2. `paint` returns early unless `style.text.color` is `Some`
//!    (gpui/src/elements/svg.rs:150-152) — an SVG with no text colour set on the
//!    element is silently skipped and draws nothing at all. Every helper below
//!    sets `.text_color(..)` on the SVG element itself, and the colour baked
//!    into the markup is an opaque white mask that keeps the source
//!    readable when the file is opened in a browser.
//!
//! Because the tint comes from the element style, an icon *can* change colour
//! between states. The first version of this file got that wrong: it baked the
//! stroke into a `format!` string, which meant the rail's selected item could
//! not brighten its icon. The rail now really does retint on selection.

use gpui::prelude::*;
use gpui::{div, px, Hsla, Rgba, Styled};

use crate::ui::theme::Theme;

/// Renders `data` at `size` logical pixels, tinted to `color`.
fn render(data: &str, size: f32, color: Hsla) -> impl IntoElement {
    div().flex_none().w(px(size)).h(px(size)).child(
        gpui::svg()
            .data(data.as_bytes())
            .size(px(size))
            .text_color(color)
            .into_any_element(),
    )
}

/// Builds the markup for a glyph, given its body.
///
/// The single point of variation between icons is the inner markup; the
/// `viewBox`, stroke width, caps and joins are shared, so every glyph is drawn
/// on the same 24x24 grid with the same 1.75 px stroke and round caps/joins.
/// That shared frame is what makes the family read as one set.
///
fn glyph(body: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"#
    )
}

/// The icon family. One variant per concept, no duplicates.
pub struct Icon;

impl Icon {
    /// Temporal index of preserved documents.
    pub fn list(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="M8 6h12M8 12h12M8 18h12M3 6h1M3 12h1M3 18h1"/>"#),
            size,
            icon_color(theme, true),
        )
    }
    /// Recorded provenance link.
    pub fn link(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="m10 13 4-4M8 16l-1 1a4 4 0 0 1-6-6l4-4a4 4 0 0 1 6 0M16 8l1-1a4 4 0 0 1 6 6l-4 4a4 4 0 0 1-6 0"/>"#,
            ),
            size,
            icon_color(theme, true),
        )
    }
    /// Expand the source reading viewport.
    pub fn expand(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="M9 3H3v6m12-6h6v6M3 15v6h6m12-6v6h-6"/>"#),
            size,
            icon_color(theme, false),
        )
    }
    /// Disclosure chevron with its actual open state.
    pub fn disclosure(theme: &Theme, size: f32, open: bool) -> impl IntoElement {
        render(
            &glyph(if open {
                r#"<path d="m6 9 6 6 6-6"/>"#
            } else {
                r#"<path d="m9 6 6 6-6 6"/>"#
            }),
            size,
            icon_color(theme, true),
        )
    }
    /// Context fields stored alongside a decision.
    pub fn layers(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="m12 3 10 5-10 5L2 8l10-5ZM2 12l10 5 10-5M2 16l10 5 10-5"/>"#),
            size,
            icon_color(theme, true),
        )
    }
    /// Change the list's status filter.
    pub fn filter(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="M3 6h18M6 12h12M9 18h6"/>"#),
            size,
            icon_color(theme, true),
        )
    }
    /// Export a document to a user-chosen file.
    pub fn export(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="M12 3v12m-4-4 4 4 4-4"/><path d="M5 15v5h14v-5"/>"#),
            size,
            icon_color(theme, false),
        )
    }
    /// Copy the recorded source text.
    pub fn copy(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(
                r#"<rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V4H4v12h4"/>"#,
            ),
            size,
            icon_color(theme, false),
        )
    }
    /// Revise a versioned document.
    pub fn edit(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="m15 4 5 5-11 11H4v-5L15 4Zm-2 2 5 5"/>"#),
            size,
            theme.colors.accent_on_emphasis().into(),
        )
    }
    /// A source file, beside its name in the evidence tab strip.
    pub fn file(theme: &Theme, size: f32, muted: bool) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6"/>"#,
            ),
            size,
            icon_color(theme, muted),
        )
    }
    /// Home: a roof over a body.
    pub fn home(theme: &Theme, size: f32, muted: bool) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="M3 10.5 12 3l9 7.5"/><path d="M5.5 9.5V20a1 1 0 0 0 1 1H10v-5.5h4V21h3.5a1 1 0 0 0 1-1V9.5"/>"#,
            ),
            size,
            icon_color(theme, muted),
        )
    }

    /// Projects: a folder, the thing a project tracks.
    pub fn folder(theme: &Theme, size: f32, muted: bool) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.2a1.5 1.5 0 0 1 1.2.6l1 1.4h8.6A1.5 1.5 0 0 1 21 9.5v9A1.5 1.5 0 0 1 19.5 20h-15A1.5 1.5 0 0 1 3 18.5z"/>"#,
            ),
            size,
            icon_color(theme, muted),
        )
    }

    /// A magnifying glass, for the search field.
    pub fn search(theme: &Theme, size: f32, muted: bool) -> impl IntoElement {
        render(
            &glyph(r#"<circle cx="11" cy="11" r="6.5"/><path d="m16 16 4.5 4.5"/>"#),
            size,
            icon_color(theme, muted),
        )
    }

    /// A plus, painted on an emphasis fill so it needs the inverse colour.
    pub fn plus(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="M12 5v14M5 12h14"/>"#),
            size,
            theme.colors.accent_on_emphasis().into(),
        )
    }

    /// A folder with a plus: the "add a project" action.
    pub fn folder_plus(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="M3 7.5A1.5 1.5 0 0 1 4.5 6h4.2a1.5 1.5 0 0 1 1.2.6l1 1.4h8.6A1.5 1.5 0 0 1 21 9.5v9A1.5 1.5 0 0 1 19.5 20h-15A1.5 1.5 0 0 1 3 18.5z"/><path d="M12 11v5M9.5 13.5h5"/>"#,
            ),
            size,
            theme.colors.accent_default().into(),
        )
    }

    /// An eye: the product watches, it does not edit.
    pub fn eye(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12"/><circle cx="12" cy="12" r="3"/>"#,
            ),
            size,
            theme.colors.accent_default().into(),
        )
    }

    /// A clock: when a project was registered.
    pub fn clock(theme: &Theme, size: f32, muted: bool) -> impl IntoElement {
        render(
            &glyph(r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 7.5V12l3 2"/>"#),
            size,
            icon_color(theme, muted),
        )
    }

    /// A trash can, for the quiet remove action.
    pub fn trash(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(
                r#"<path d="M4 7h16M9.5 7V5.5A1.5 1.5 0 0 1 11 4h2a1.5 1.5 0 0 1 1.5 1.5V7"/><path d="M6.5 7l.8 12a1.5 1.5 0 0 0 1.5 1.4h6.4a1.5 1.5 0 0 0 1.5-1.4l.8-12"/>"#,
            ),
            size,
            theme.colors.status_danger().into(),
        )
    }

    /// A chevron, for disclosure and "go on".
    pub fn chevron_right(theme: &Theme, size: f32, muted: bool) -> impl IntoElement {
        render(
            &glyph(r#"<path d="m9.5 5.5 6.5 6.5-6.5 6.5"/>"#),
            size,
            icon_color(theme, muted),
        )
    }

    /// A check, marking a value that has been chosen.
    ///
    /// Painted in `accent_default`, not `status_success`. This mark answers
    /// "is this the one you picked?", which is a fact about the form; a green
    /// would answer "is the system healthy?", and those are different sentences.
    /// On this screen the check appears on every registered folder, so a green
    /// one made every row look like a health badge — the single loudest source
    /// of colour in a screen that is otherwise achromatic.
    pub fn check(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="m5 12.5 4.5 4.5L19 7.5"/>"#),
            size,
            theme.colors.accent_default().into(),
        )
    }

    /// A half-filled disc: switching between the two palettes.
    pub fn contrast(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(
                r#"<circle cx="12" cy="12" r="8.5"/><path d="M12 3.5a8.5 8.5 0 0 1 0 17z" fill="white" stroke="none"/>"#,
            ),
            size,
            theme.colors.text_secondary().into(),
        )
    }

    /// An activity line, for a busy state.
    pub fn activity(theme: &Theme, size: f32) -> impl IntoElement {
        render(
            &glyph(r#"<path d="M3 12h4l2.5-6 4 12 2.5-6h5"/>"#),
            size,
            theme.colors.accent_default().into(),
        )
    }
}

/// The colour an icon draws in, given whether its item is idle.
///
/// Active items draw in `text_primary`, idle items in `text_muted`: the icon
/// carries part of the selected state and the label carries the rest, so the
/// two never have to be read independently.
pub fn icon_color(theme: &Theme, muted: bool) -> Hsla {
    let color: Rgba = if muted {
        theme.colors.text_muted()
    } else {
        theme.colors.text_primary()
    };
    color.into()
}
