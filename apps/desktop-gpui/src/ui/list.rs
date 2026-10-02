//! Lists that stay light at any size.
//!
//! Two pieces, one per way the app avoids showing everything at once:
//!
//! - **Virtual lists** (`gpui::list` with a [`ListState`]): only the rows in
//!   view are built, so the cost of scrolling follows the window, not the
//!   data. [`ScrollMemory`] and [`scroll_thumb`] give them a slim thumb that
//!   appears while the list moves and fades when it rests, so a long list
//!   shows how long it is without a permanent bar.
//! - **Reveal footers** ([`reveal_footer`]) for lists that grow on demand: a
//!   hairline, "N de M" in tabular figures, a thin progress line and the
//!   actions (more, all). One shape for every list that is cut.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::prelude::*;
use gpui::{canvas, div, px, relative, Div, IntoElement, ListState, SharedString};

use crate::ui::patterns::tabular;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{SpacingScale, TypeScale};

/// How long the thumb stays at full strength after the list last moved.
const THUMB_HOLD: Duration = Duration::from_millis(650);
/// How long it then takes to fade.
const THUMB_FADE: Duration = Duration::from_millis(350);
/// Shortest the thumb is drawn, so it stays easy to see on a long list.
const THUMB_MIN: f32 = 28.0;
/// Width of the thumb.
const THUMB_WIDTH: f32 = 3.0;

/// When a virtual list last moved, for the thumb to fade by.
#[derive(Clone, Default)]
pub struct ScrollMemory(Rc<Cell<Option<Instant>>>);

impl ScrollMemory {
    /// Starts noting scrolls of `state`.
    pub fn attach(&self, state: &ListState) {
        let moved = self.0.clone();
        state.set_scroll_handler(move |_, _, _| moved.set(Some(Instant::now())));
    }

    /// Strength of the thumb now, 0 to 1, and whether it is still changing.
    fn level(&self) -> (f32, bool) {
        let Some(at) = self.0.get() else {
            return (0.0, false);
        };
        let elapsed = at.elapsed();
        if elapsed <= THUMB_HOLD {
            return (1.0, true);
        }
        let fade = (elapsed - THUMB_HOLD).as_secs_f32() / THUMB_FADE.as_secs_f32();
        if fade >= 1.0 {
            (0.0, false)
        } else {
            (1.0 - fade * fade, true)
        }
    }
}

/// The thumb for a virtual list, laid over it (the parent is `relative`).
/// It is not a control: it shows position and length, nothing more.
pub fn scroll_thumb(theme: &Theme, state: &ListState, memory: &ScrollMemory) -> Div {
    let viewport = f32::from(state.viewport_bounds().size.height);
    let max = f32::from(state.max_offset_for_scrollbar().y);
    let (level, animating) = memory.level();
    let holder = div()
        .absolute()
        .top_0()
        .right(px(2.0))
        .bottom_0()
        .w(px(THUMB_WIDTH + 2.0));
    if viewport <= 0.0 || max <= 0.5 || level <= 0.0 {
        return holder;
    }
    let offset = (-f32::from(state.scroll_px_offset_for_scrollbar().y)).clamp(0.0, max);
    let content = viewport + max;
    let length = (viewport * viewport / content).clamp(THUMB_MIN.min(viewport), viewport);
    let top = (viewport - length) * offset / max;
    holder
        // Asks for the next frame only while the thumb is visible or fading.
        .child(
            canvas(
                |_, _, _| (),
                move |_, _, window, _| {
                    if animating {
                        window.request_animation_frame();
                    }
                },
            )
            .absolute()
            .size_0(),
        )
        .child(
            div()
                .absolute()
                .top(px(top))
                .left_0()
                .w(px(THUMB_WIDTH))
                .h(px(length))
                .rounded_full()
                .bg(theme.colors.text_muted().alpha(0.55 * level)),
        )
}

/// The footer of a list cut at `shown` of `total`: how far along it is, and
/// the actions to go further. `more` and `all` are buttons built by the
/// screen (it owns their focus); `all` is for lists with a page of their own.
pub fn reveal_footer(
    theme: &Theme,
    shown: usize,
    total: usize,
    more: Option<impl IntoElement>,
    all: Option<impl IntoElement>,
) -> Div {
    let colors = theme.colors;
    let fraction = if total == 0 {
        1.0
    } else {
        (shown as f32 / total as f32).clamp(0.0, 1.0)
    };
    let progress = SharedString::from(format!("{shown} de {total}"));
    div()
        .flex()
        .flex_col()
        .gap(px(SpacingScale::S2))
        .px(px(SpacingScale::S2))
        .pt(px(SpacingScale::S2))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S3))
                .child(
                    tabular(text_style(div(), TypeScale::META))
                        .flex_none()
                        .text_color(colors.text_muted())
                        .child(progress),
                )
                .child(
                    div()
                        .flex_1()
                        .h(px(2.0))
                        .rounded_full()
                        .bg(colors.hairline_divider())
                        .child(
                            div()
                                .h_full()
                                .w(relative(fraction))
                                .rounded_full()
                                .bg(colors.accent_default().alpha(0.7)),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S2))
                .children(more)
                .children(all),
        )
}
