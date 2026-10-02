//! The open → closing → closed lifecycle of a popover.
//!
//! GPUI unmounts an element the frame its state drops, so a popover that just
//! disappears cannot play an exit. A [`Popup`] keeps the state mounted while
//! it leaves: logic asks [`Popup::is_open`] / [`Popup::as_open`] (a closing
//! popup already reads as closed and takes no input), rendering asks
//! [`Popup::get`] and [`Popup::exit_progress`]. [`reap`] drops the state once
//! the exit has played; a popup reopened meanwhile is left alone.
//!
//! It also settles the trigger that toggles its own popup: the popup's
//! "click outside" closes it on mouse-down, so by the trigger's click it
//! already reads as closed and a plain toggle would reopen it. The trigger
//! notes on mouse-down whether the popup was mounted
//! ([`Popup::note_trigger_press`]) and the click consults it.
//!
//! Adapted from Zeron's `Popup` (MIT, © 2026 Wing,
//! github.com/zeronsh/zeron at 27480d99); see `NOTICE`.

use std::time::{Duration, Instant};

use gpui::Context;

use crate::ui::motion::spec::MENU_OUT;

/// A popover's state, held through its exit.
pub struct Popup<T> {
    /// The state while mounted, with the instant the exit began.
    inner: Option<(T, Option<Instant>)>,
    pressed_while_open: bool,
}

impl<T> Default for Popup<T> {
    fn default() -> Self {
        Self {
            inner: None,
            pressed_while_open: false,
        }
    }
}

impl<T> Popup<T> {
    /// Opens (or reopens mid-exit) with `value`.
    pub fn open(&mut self, value: T) {
        self.inner = Some((value, None));
    }

    /// Open and interactive.
    pub fn is_open(&self) -> bool {
        matches!(self.inner, Some((_, None)))
    }

    /// The state while mounted, open or leaving: what rendering reads.
    pub fn get(&self) -> Option<&T> {
        self.inner.as_ref().map(|(value, _)| value)
    }

    /// The state only while open: what logic reads.
    pub fn as_open(&self) -> Option<&T> {
        match &self.inner {
            Some((value, None)) => Some(value),
            _ => None,
        }
    }

    /// Mutable state only while open.
    pub fn open_mut(&mut self) -> Option<&mut T> {
        match &mut self.inner {
            Some((value, None)) => Some(value),
            _ => None,
        }
    }

    /// Eased exit progress (0 → 1) while leaving; `None` when open or gone.
    pub fn exit_progress(&self) -> Option<f32> {
        match &self.inner {
            Some((_, Some(since))) => Some(MENU_OUT.progress_since(since.elapsed())),
            _ => None,
        }
    }

    /// Starts the exit. `true` when this call started it: the caller then
    /// schedules [`reap`].
    pub fn begin_close(&mut self) -> bool {
        match &mut self.inner {
            Some((_, closing @ None)) => {
                *closing = Some(Instant::now());
                true
            }
            _ => false,
        }
    }

    /// Drops the state if the exit has played out.
    pub fn finish_close(&mut self) {
        if let Some((_, Some(since))) = &self.inner {
            if since.elapsed() >= MENU_OUT.duration() {
                self.inner = None;
            }
        }
    }

    /// From the trigger's mouse-down: whether the popup is mounted now.
    pub fn note_trigger_press(&mut self) {
        self.pressed_while_open = self.inner.is_some();
    }

    /// From the trigger's click: whether that press found the popup mounted
    /// (it closed it; the click must not reopen it). Consumes the note.
    pub fn take_press_was_open(&mut self) -> bool {
        std::mem::take(&mut self.pressed_while_open)
    }
}

/// After a [`Popup::begin_close`], drops the state once the exit has played
/// and repaints. `popup` borrows the field from the view.
pub fn reap<V: 'static, T: 'static>(
    cx: &mut Context<V>,
    popup: impl Fn(&mut V) -> &mut Popup<T> + 'static,
) {
    cx.spawn(async move |view, cx| {
        cx.background_executor()
            .timer(MENU_OUT.duration() + Duration::from_millis(16))
            .await;
        let _ = view.update(cx, |view, cx| {
            popup(view).finish_close();
            cx.notify();
        });
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closing_popup_is_mounted_but_not_open() {
        let mut popup = Popup::default();
        popup.open(7);
        assert!(popup.is_open());
        assert!(popup.begin_close());
        assert!(!popup.begin_close(), "only the first call starts the exit");
        assert!(!popup.is_open());
        assert_eq!(popup.as_open(), None);
        assert_eq!(popup.get(), Some(&7));
        assert!(popup.exit_progress().is_some());
    }

    #[test]
    fn reopening_mid_exit_survives_the_old_reap() {
        let mut popup = Popup::default();
        popup.open(1);
        popup.begin_close();
        popup.open(2);
        popup.finish_close();
        assert_eq!(popup.as_open(), Some(&2));
    }

    #[test]
    fn the_press_that_dismissed_does_not_reopen() {
        let mut popup = Popup::default();
        popup.open(());
        popup.note_trigger_press();
        popup.begin_close(); // the outside-click handler on the same press
        assert!(popup.take_press_was_open());
        assert!(!popup.take_press_was_open(), "the note is consumed");
        let mut closed: Popup<()> = Popup::default();
        closed.note_trigger_press();
        assert!(!closed.take_press_was_open());
    }
}
