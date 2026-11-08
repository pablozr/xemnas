//! The review ticket: one decision per screen. The ticket card carries the
//! question and the change; a progress row above it says where the cycle
//! stands; a floating pill holds the four actions. The queue is not a column:
//! it opens over the ticket. The proof (sources) and the effect (the full
//! reading) replace the ticket while open.

use std::cell::Cell;

use gpui::{canvas, App, FontWeight};

use super::*;
use crate::app::{OpenProof, ToggleQueue};
use crate::i18n::inbox_ticket as tt;
use crate::screens::capture_health::CaptureHealth;
use crate::ui::material::{elevation, Elevation};
use crate::ui::night_field::{full_columns, strip_dims, Clearings};
use crate::ui::theme::code_style;
use crate::ui::tokens::TypeToken;
use crate::ui::tokens::{mix, tint};

/// Width of the ticket card and of the column above and below it.
const TICKET_WIDTH: f32 = 760.0;
/// Height of the ticket card.
const TICKET_HEIGHT: f32 = 296.0;
/// Width of the stub (confidence) at the right of the card.
const STUB_WIDTH: f32 = 196.0;
/// Diameter of the cutouts where the perforation meets the card edge.
const CUTOUT: f32 = 18.0;
/// Dashes of the progress row.
const DASHES: usize = 10;
/// Gap between the bottom of the surface and the action pill.
const PILL_BOTTOM: f32 = 40.0;
/// Characters of the reason kept on the ticket.
const REASON_CHARS: usize = 220;
/// Height of the colophon line at the bottom.
const COLOPHON_HEIGHT: f32 = 28.0;
/// Gap between the "see the proof" line and the night under it.
const NIGHT_GAP: f32 = 20.0;
/// Columns and rows of the strip before the room for it is measured.
const NIGHT_FIRST: (usize, usize) = (198, 18);
/// Widest the "next" line gets: the width of the quiet hole behind it.
const NEXT_WIDTH: f32 = 440.0;
/// Blocks of the confidence meter.
const METER_BLOCKS: usize = 10;
/// The "%" of the stub, relative to the number.
const PERCENT_SCALE: f32 = 0.55;

/// The ticket's own tokens: Bricolage sizes of the mockup.
const BIG_VALUE: TypeToken = TypeToken::new(46.0, 52.0, 500.0);
/// The proposed value when nothing came before it.
const NEW_VALUE: TypeToken = TypeToken::new(34.0, 40.0, 600.0);
const MID_VALUE: TypeToken = TypeToken::new(32.0, 38.0, 500.0);
const SMALL_VALUE: TypeToken = TypeToken::new(28.0, 34.0, 600.0);
const STUB_VALUE: TypeToken = TypeToken::new(52.0, 58.0, 500.0);
const REASON: TypeToken = TypeToken::new(17.0, 26.0, 400.0);
/// Small captions above a value (11 / 500).
const CAPTION: TypeToken = TypeToken::new(11.0, 16.0, 500.0);
/// The file where the choice lives (mono 13).
const WHERE_FILE: TypeToken = TypeToken::new(13.0, 19.0, 400.0);

/// What the ticket knows of the room under it: the grid that fits it as the
/// layout last measured it, and the one the last frame was drawn with.
#[derive(Default)]
pub(super) struct NightSlot {
    measured: Cell<(usize, usize)>,
    drawn: Cell<(usize, usize)>,
    /// Columns of the full lake that fit the empty queue's pane (0 before
    /// the first measure).
    full_columns: Cell<usize>,
    /// Where "next" (0) and the pill (1) were last painted, in the window:
    /// the lake keeps them clear.
    clearings: Clearings,
}

/// What fills the screen for the selected candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pane {
    /// The ticket.
    Ticket,
    /// The sources that support it.
    Proof,
    /// The full reading: what confirming changes.
    Effect,
}

/// A request from a key to change what the screen shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReviewView {
    /// `↵`.
    Proof,
    /// `E`.
    Effect,
    /// `Tab`.
    Queue,
    /// `L`.
    Accepted,
    /// `D`.
    Today,
    /// `Esc`.
    Close,
    /// `←`.
    PrevSource,
    /// `→`.
    NextSource,
}

/// Asks the shell to open Decisões, from the empty queue.
pub struct OpenDecisions;

impl<S: InboxStore + ReviewExceptionStore + Send + 'static> EventEmitter<OpenDecisions>
    for InboxScreen<S>
{
}

/// Whether a word is a chip: a path or a quoted word.
fn is_chip(word: &str) -> bool {
    let trimmed = word.trim_matches(|c: char| ",.;:!?()".contains(c));
    if trimmed.len() < 2 {
        return false;
    }
    let quoted = ['`', '"', '“', '\'', '‘']
        .iter()
        .any(|quote| trimmed.starts_with(*quote));
    let path = trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.rsplit_once('.').is_some_and(|(stem, extension)| {
            stem.len() >= 2
                && (1..=5).contains(&extension.len())
                && extension.chars().all(|c| c.is_ascii_alphanumeric())
                && extension.chars().any(|c| c.is_ascii_lowercase())
                && stem
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "_-./".contains(c))
        });
    quoted || path
}

/// The first sentence of a reason, kept short.
fn one_sentence(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let end = flat
        .char_indices()
        .find(|&(index, c)| {
            matches!(c, '.' | '!' | '?')
                && flat[index + c.len_utf8()..]
                    .chars()
                    .next()
                    .is_none_or(char::is_whitespace)
        })
        .map_or(flat.len(), |(index, c)| index + c.len_utf8());
    let sentence = &flat[..end];
    if sentence.chars().count() <= REASON_CHARS {
        return sentence.to_owned();
    }
    let cut: String = sentence.chars().take(REASON_CHARS).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{cut}…")
}

/// Confidence band of a value: high, medium or low.
fn confidence_level(value: f64) -> &'static str {
    if value >= 0.75 {
        tt::confidence_high()
    } else if value >= 0.5 {
        tt::confidence_medium()
    } else {
        tt::confidence_low()
    }
}

/// Characters a choice may have and still be set at the full size.
const FULL_VALUE_CHARS: usize = 60;

/// The type token for a value of `len` characters, so a long choice still fits
/// in three lines.
fn value_token(len: usize) -> TypeToken {
    if len <= FULL_VALUE_CHARS {
        NEW_VALUE
    } else {
        SMALL_VALUE
    }
}

/// The file a candidate touched, without its folders: where the choice lives.
fn where_file(detail: &CandidateDetail) -> Option<String> {
    detail
        .diff_summary
        .files
        .first()
        .and_then(|path| path.rsplit(['/', '\\']).find(|name| !name.is_empty()))
        .map(str::to_owned)
}

impl<S: InboxStore + ReviewExceptionStore + Send + 'static> InboxScreen<S> {
    /// What the AI accepted by itself and nobody put back.
    fn accepted_by_ai(&self) -> usize {
        self.ledger
            .iter()
            .filter(|entry| {
                entry.kind == ItemKind::Candidate
                    && entry.verdict == Verdict::Accepted
                    && entry.by == By::Ai
                    && entry.undone_at.is_none()
            })
            .count()
    }

    /// Changes the view from a key. `false` when the key has nothing to do
    /// here, so it goes on to whatever else listens to it.
    pub fn run_view(
        &mut self,
        view: ReviewView,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.editor.is_some() {
            return false;
        }
        let has_ticket = self.detail.is_some() && !self.conflict_open();
        let handled = match view {
            ReviewView::Proof if has_ticket && self.pane == Pane::Ticket && !self.queue_open => {
                self.pane = Pane::Proof;
                true
            }
            ReviewView::Effect if has_ticket && self.pane == Pane::Ticket && !self.queue_open => {
                self.pane = Pane::Effect;
                true
            }
            ReviewView::Queue => {
                self.queue_open = !self.queue_open;
                true
            }
            ReviewView::Accepted if !self.ledger.is_empty() || self.approvals.is_some() => {
                self.queue_open = true;
                self.ledger_open = true;
                true
            }
            ReviewView::Today if self.rows.is_empty() && self.loaded => {
                cx.emit(OpenDecisions);
                true
            }
            ReviewView::Close if self.queue_open => {
                self.queue_open = false;
                true
            }
            ReviewView::Close if self.pane != Pane::Ticket => {
                self.pane = Pane::Ticket;
                true
            }
            ReviewView::PrevSource | ReviewView::NextSource if self.pane == Pane::Proof => {
                let count = self.source_focus.len();
                if count > 0 {
                    self.source_index = if view == ReviewView::NextSource {
                        (self.source_index + 1) % count
                    } else {
                        (self.source_index + count - 1) % count
                    };
                }
                true
            }
            _ => false,
        };
        if handled {
            // The screen keeps the focus, so Tab and Enter stay its own.
            if !self.queue_open {
                window.focus(&self.root_focus, cx);
            }
            cx.notify();
        }
        handled
    }

    /// The whole review surface: the ticket (or the empty queue), the queue
    /// over it when open, and the editor or the conflict when they are.
    pub(super) fn review_surface(
        &self,
        theme: &Theme,
        visible: &[&CandidateSummary],
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let home = self.root_focus.is_focused(window);
        let empty = self.detail_visible().is_none()
            && self.loaded
            && self.rows.is_empty()
            && !self.busy
            && self.editor.is_none();
        let ticket = self.editor.is_none()
            && self.detail_visible().is_some()
            && !self.conflict_open()
            && self.pane == Pane::Ticket;
        let content = if let Some(editor) = self.editor.as_ref().filter(|_| {
            self.detail
                .as_ref()
                .is_some_and(|detail| matches_query(&detail.summary, &self.query))
        }) {
            editor.clone().into_any_element()
        } else if self.detail_visible().is_none() {
            if self.loaded && self.rows.is_empty() && !self.busy {
                self.empty_queue(theme, cx)
            } else {
                self.reading_pane(cx)
            }
        } else if self.conflict_open() {
            self.reading_pane(cx)
        } else {
            match self.pane {
                Pane::Ticket => self.ticket(theme, visible, cx),
                Pane::Proof => self.proof(theme, cx),
                Pane::Effect => self.effect(theme, cx),
            }
        };
        div()
            .id("inbox-surface")
            .flex_1()
            .min_h(px(0.0))
            .relative()
            .track_focus(&self.root_focus)
            .when(home, |surface| surface.key_context("ReviewHome"))
            .on_action(cx.listener(|this, _: &ToggleQueue, window, cx| {
                this.run_view(ReviewView::Queue, window, cx);
            }))
            .on_action(cx.listener(|this, _: &OpenProof, window, cx| {
                if !this.run_view(ReviewView::Proof, window, cx) {
                    cx.propagate();
                }
            }))
            .when(empty, |surface| surface.child(self.night(cx)))
            .child(content)
            .when(empty || ticket, |surface| {
                surface.child(self.colophon(theme, empty))
            })
            .when(self.queue_open, |surface| {
                surface.child(self.queue_overlay(theme, visible, cx))
            })
            .into_any_element()
    }

    /// Puts the keyboard on the review surface, so Tab and Enter reach it.
    pub fn focus_surface(&self, window: &mut Window, cx: &mut App) {
        window.focus(&self.root_focus, cx);
    }

    /// The selected candidate, when the search still shows it.
    fn detail_visible(&self) -> Option<&CandidateDetail> {
        self.detail
            .as_ref()
            .filter(|detail| matches_query(&detail.summary, &self.query))
    }

    /// The queue over a scrim: a click outside it, or `Esc`, closes it.
    fn queue_overlay(
        &self,
        theme: &Theme,
        visible: &[&CandidateSummary],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("inbox-queue-overlay")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .bg(theme.colors.canvas_deep().alpha(0.55))
            .role(Role::Dialog)
            .aria_label(tt::queue_overlay_aria())
            .on_click(cx.listener(|this, _, _, cx| {
                this.queue_open = false;
                cx.notify();
            }))
            .child(
                self.queue_column(theme, visible, cx)
                    .id("inbox-queue")
                    .shadow(elevation(theme, Elevation::Dialog))
                    .on_click(|_, _, cx| cx.stop_propagation()),
            )
            .into_any_element()
    }

    /// The day's cycle as (accepted by the AI, done, total), at most
    /// `DASHES` long: the dashes share it.
    fn cycle(&self) -> (usize, usize, usize) {
        let ai = self.accepted_by_ai();
        let total = (ai + self.decided + self.rows.len()).clamp(2, DASHES);
        (ai.min(total), (ai + self.decided).min(total), total)
    }

    /// The night over the whole empty queue, behind everything and deaf to
    /// the mouse.
    fn night(&self, cx: &mut Context<Self>) -> AnyElement {
        let clearings = self.night_slot.clearings.clone();
        let mascot = !self.status_bar;
        let columns = self.night_slot.full_columns.get();
        self.night_full.update(cx, |field, _| {
            field.set_waiting(0);
            field.set_clearings(&clearings);
            field.set_mascot(mascot);
            if columns > 0 {
                field.set_columns(columns);
            }
        });
        let (width, height) = self.night_full.read(cx).size();
        let slot = self.night_slot.clone();
        let screen = cx.entity_id();
        // A pane narrower than the lake would cut the cliff and the pier:
        // measure it and repaint once with the columns that fit.
        let measure = canvas(
            move |bounds, _, cx| {
                let fits = full_columns(f32::from(bounds.size.width));
                if slot.full_columns.replace(fits) != fits {
                    cx.defer(move |cx| cx.notify(screen));
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(COLOPHON_HEIGHT))
            .h(px(height))
            .w_full()
            .flex()
            .justify_center()
            .overflow_hidden()
            .child(measure)
            .child(
                div()
                    .w(px(width))
                    .flex_none()
                    .child(self.night_full.clone()),
            )
            .into_any_element()
    }

    /// The night under the ticket: the room between the "see the proof" line
    /// (20 px below it) and the colophon, with a strip as tall as fits in it,
    /// resting on the bottom. The room is measured while painting; when the
    /// grid that fits changes, the screen repaints once with it.
    fn night_strip(&self, cx: &mut Context<Self>) -> Div {
        let measured = self.night_slot.measured.get();
        let (columns, rows) = if measured == (0, 0) {
            NIGHT_FIRST
        } else {
            measured
        };
        self.night_slot.drawn.set((columns, rows));
        let slot = self.night_slot.clone();
        let screen = cx.entity_id();
        let measure = canvas(
            move |bounds, _, cx| {
                let size = (f32::from(bounds.size.width), f32::from(bounds.size.height));
                let fits = strip_dims(size.0, size.1);
                if fits != slot.measured.replace(fits) && fits != slot.drawn.get() {
                    cx.defer(move |cx| cx.notify(screen));
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let room = div()
            .relative()
            .w_full()
            .flex_1()
            .min_h(px(0.0))
            .mt(px(NIGHT_GAP))
            .mb(px(COLOPHON_HEIGHT))
            .overflow_hidden()
            .child(measure);
        if rows == 0 {
            return room;
        }
        let waiting = self.rows.len();
        let clearings = self.night_slot.clearings.clone();
        self.night.update(cx, |field, _| {
            field.set_waiting(waiting);
            field.set_clearings(&clearings);
            field.set_size(columns, rows);
        });
        let (width, height) = self.night.read(cx).size();
        room.child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(height))
                .flex()
                .justify_center()
                .child(div().w(px(width)).flex_none().child(self.night.clone())),
        )
    }

    /// The colophon in mono: the build on the left, what the night is on the
    /// right.
    fn colophon(&self, theme: &Theme, closed: bool) -> Div {
        let colors = theme.colors;
        code_style(
            div()
                .absolute()
                .left_0()
                .right_0()
                .bottom_0()
                .h(px(COLOPHON_HEIGHT))
                .px(px(SpacingScale::S6))
                .flex()
                .items_center()
                .justify_between()
                .text_color(colors.text_disabled()),
            TypeScale::META,
        )
        .child(tt::colophon_build(
            env!("CARGO_PKG_VERSION"),
            env!("XEMNAS_COMMIT"),
            env!("CARGO_PKG_LICENSE"),
        ))
        .child(if closed {
            tt::colophon_closed()
        } else {
            tt::colophon_night()
        })
    }

    /// Dashes of the cycle: the AI's accepted ones (green, faint), what the
    /// person decided, the one being read and what is left.
    fn dashes(&self, theme: &Theme) -> Div {
        let (ai, done, total) = self.cycle();
        let colors = theme.colors;
        div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .children((0..total).map(move |index| {
                let color = if index < ai {
                    colors.status_success().alpha(0.3)
                } else if index < done {
                    colors.accent_default()
                } else if index == done {
                    colors.accent_emphasis()
                } else {
                    colors.glass_fill_strong()
                };
                div().w(px(18.0)).h(px(3.0)).rounded_full().bg(color)
            }))
    }

    /// Where the cycle stands, with the mode: Manual or Automatic.
    fn progress_row(&self, theme: &Theme, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let automatic = self.status.is_some_and(|status| status.automatic);
        let ai = self.accepted_by_ai();
        let mine = self.decided + self.rows.len();
        let position = (self.decided + 1).min(mine.max(1));
        let muted = |text: String| {
            text_style(div(), TypeScale::LABEL)
                .flex_none()
                .text_color(colors.text_muted())
                .child(text)
        };
        let mode = self.mode_track(theme, cx);
        let see_accepted = automatic.then(|| {
            self.view_hint(
                theme,
                "progress-accepted",
                "L",
                tt::see(),
                ReviewView::Accepted,
                cx,
            )
        });
        let queue = self.view_hint(
            theme,
            "progress-queue",
            "Tab",
            tt::queue_hint(),
            ReviewView::Queue,
            cx,
        );
        div()
            .w_full()
            .h(px(28.0))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .children(mode)
            .when(automatic, |row| {
                row.child(
                    text_style(div(), TypeScale::LABEL)
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .text_color(colors.text_muted())
                        .child(tt::ai_accepted_before())
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(colors.text_primary())
                                .child(ai.to_string()),
                        )
                        .child(tt::ai_accepted_after()),
                )
                .children(see_accepted)
            })
            .when(!automatic && self.status.is_some(), |row| {
                row.child(muted(tt::you_decide_all().to_owned()))
            })
            .child(self.dashes(theme))
            .child(muted(if automatic {
                tt::for_you_of(position, mine.max(1))
            } else {
                tt::position(position, mine.max(1))
            }))
            .child(div().flex_1())
            .child(queue)
    }

    /// The ticket: the card, the reason in one sentence and the hints.
    fn ticket(
        &self,
        theme: &Theme,
        visible: &[&CandidateSummary],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(detail) = self.detail_visible() else {
            return div().into_any_element();
        };
        let next = self
            .selected
            .as_ref()
            .and_then(|id| visible.iter().position(|row| &row.id == id))
            .and_then(|index| visible.get(index + 1))
            .map(|row| row.question.clone());
        let hints = div()
            .w_full()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S5))
            .child(self.view_hint(
                theme,
                "ticket-proof",
                "↵",
                tt::see_proof(),
                ReviewView::Proof,
                cx,
            ))
            .child(self.view_hint(
                theme,
                "ticket-effect",
                "E",
                tt::see_effect(),
                ReviewView::Effect,
                cx,
            ));
        let column = div()
            .w_full()
            .max_w(px(TICKET_WIDTH))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .child(self.progress_row(theme, cx))
            .child(self.card(theme, detail))
            .child(self.reason_flow(
                theme,
                &one_sentence(&detail.rationale),
                where_file(detail).as_deref(),
            ))
            .child(hints);
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .w_full()
                    .flex_none()
                    .flex()
                    .justify_center()
                    .px(px(SpacingScale::S8))
                    .pt(px(56.0))
                    .child(fade_in(
                        column,
                        ElementId::Name(format!("ticket-{}", detail.summary.id).into()),
                    )),
            )
            .child(self.night_strip(cx))
            .child(self.action_pill(theme, next, cx))
            .into_any_element()
    }

    /// A clickable hint that opens a view: key, then what it opens.
    fn view_hint(
        &self,
        theme: &Theme,
        id: &'static str,
        key: &'static str,
        label: &'static str,
        view: ReviewView,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = theme.colors;
        text_style(div(), TypeScale::LABEL)
            .id(id)
            .flex_none()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .cursor_pointer()
            .text_color(colors.text_muted())
            .role(Role::Button)
            .aria_label(label)
            .focus_visible(focus_ring(theme))
            .hover(move |style| style.text_color(colors.text_secondary()))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.run_view(view, window, cx);
            }))
            .child(kbd(colors.text_muted(), key))
            .child(label)
    }

    /// The reason in a sentence of words and chips (files and quotes). A
    /// sentence with no chip of its own ends with the file of the capture,
    /// when there is one.
    fn reason_flow(&self, theme: &Theme, sentence: &str, file: Option<&str>) -> Div {
        let colors = theme.colors;
        let chip = |word: String| {
            code_style(div(), TypeScale::CODE)
                .flex_none()
                .px(px(6.0))
                .rounded(px(5.0))
                .bg(colors.glass_fill_medium())
                .text_color(colors.text_primary())
                .child(word)
        };
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let own_chip = words.iter().any(|word| is_chip(word));
        let closing = file.filter(|_| !own_chip && !words.is_empty());
        let last = words.len().saturating_sub(1);
        code_style(
            div()
                .w_full()
                .min_w(px(0.0))
                .flex()
                .flex_wrap()
                .items_center(),
            REASON,
        )
        .font_family(Theme::font_interface())
        .text_color(colors.text_secondary())
        .gap_x(px(REASON.size * 0.27))
        .gap_y(px(4.0))
        .children(words.into_iter().enumerate().map(|(index, word)| {
            let piece = if is_chip(word) {
                chip(word.to_owned())
            } else {
                div().flex_none().child(word.to_owned())
            };
            match closing {
                // The file of the capture stays with the last word: it never wraps alone.
                Some(name) if index == last => div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_x(px(REASON.size * 0.27))
                    .child(piece)
                    .child(chip(name.to_owned())),
                _ => piece,
            }
        }))
    }

    /// The ticket card: the left side reads, the stub says how sure it is.
    /// Nothing is shown that the model does not have: with no earlier value
    /// the choice stands alone under "new decision", and a field with no data
    /// is left out.
    fn card(&self, theme: &Theme, detail: &CandidateDetail) -> Div {
        let colors = theme.colors;
        let summary = &detail.summary;
        let value = &summary.choice;
        let token = value_token(value.chars().count());
        let origin = [
            summary.adapter.clone().unwrap_or_default(),
            short_date(&summary.received_at),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
        let id: String = summary
            .id
            .chars()
            .skip(summary.id.chars().count().saturating_sub(4))
            .collect();
        let caption = |text: &'static str| {
            text_style(div(), CAPTION)
                .text_color(colors.text_disabled())
                .child(text)
        };
        let scope = detail
            .qualifiers
            .iter()
            .find(|item| item.kind == application::qualifiers::QualifierKind::Scope)
            .map(|item| item.text.clone());
        let field = |label: &'static str, value: String, mono: bool| {
            let text = if mono {
                code_style(div(), WHERE_FILE)
            } else {
                text_style(div(), TypeScale::BODY_SMALL)
            };
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(caption(label))
                .child(
                    text.text_color(colors.text_secondary())
                        .truncate()
                        .child(value),
                )
        };
        let kind = if summary.kind == CandidateKind::Rule {
            t::kind_rule()
        } else {
            t::kind_decision()
        };
        let left = div()
            .flex_1()
            .min_w(px(0.0))
            .h_full()
            .p(px(28.0))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S3))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(px(SpacingScale::S2))
                    .child(
                        code_style(div(), TypeScale::META)
                            .flex_none()
                            .text_color(colors.text_disabled())
                            .child(id),
                    )
                    .child(
                        text_style(div(), TypeScale::LABEL)
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(colors.text_muted())
                            .child(origin),
                    ),
            )
            .children(self.left_for_you(&summary.id).map(|entry| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S2))
                    .child(
                        div()
                            .size(px(6.0))
                            .flex_none()
                            .rounded_full()
                            .bg(colors.status_warning()),
                    )
                    .child(
                        text_style(div(), TypeScale::META)
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(colors.status_warning())
                            .child(t::left_for_you_reason(&entry.reason)),
                    )
            }))
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(colors.text_secondary())
                    .max_h(px(44.0))
                    .overflow_hidden()
                    .child(summary.question.clone()),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(caption(tt::new_decision()))
                    .child(
                        text_style(div(), token)
                            .font_family(Theme::font_display())
                            .text_color(colors.text_primary())
                            .max_h(px(token.line_height * 3.0))
                            .overflow_hidden()
                            .child(value.clone()),
                    ),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .gap(px(SpacingScale::S4))
                    .children(where_file(detail).map(|file| field(tt::field_where(), file, true)))
                    .children(scope.map(|scope| field(tt::field_scope(), scope, false)))
                    .child(field(tt::field_applies(), kind.to_owned(), false)),
            );
        let filled = (summary.confidence * 10.0).round().clamp(0.0, 10.0) as usize;
        let stub = div()
            .w(px(STUB_WIDTH))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(SpacingScale::S2))
            .child(caption(tt::confidence_label()))
            .child(
                text_style(div(), STUB_VALUE)
                    .font_family(Theme::font_display())
                    .flex()
                    .items_baseline()
                    .text_color(colors.text_primary())
                    .child((summary.confidence * 100.0).round().to_string())
                    .child(
                        div()
                            .text_size(px(STUB_VALUE.size * PERCENT_SCALE))
                            .text_color(colors.text_muted())
                            .child("%"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap(px(3.0))
                    .children((0..METER_BLOCKS).map(|index| {
                        div()
                            .w(px(8.0))
                            .h(px(10.0))
                            .rounded(px(2.0))
                            .bg(if index < filled {
                                colors.accent_emphasis()
                            } else {
                                colors.surface()
                            })
                    })),
            )
            .child(
                text_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .child(tt::confidence_line(
                        confidence_level(summary.confidence),
                        detail.artifacts.len(),
                    )),
            );
        let perforation = div()
            .relative()
            .flex_none()
            .w(px(1.0))
            .h_full()
            .py(px(CUTOUT))
            .flex()
            .flex_col()
            .justify_between()
            .children((0..24).map(|_| div().w(px(1.0)).h(px(5.0)).bg(colors.hairline_divider())))
            .child(self.cutout(theme, true))
            .child(self.cutout(theme, false));
        div()
            .relative()
            .w_full()
            .h(px(TICKET_HEIGHT))
            .flex_none()
            .flex()
            .rounded(px(16.0))
            .overflow_hidden()
            .bg(colors.canvas_raised())
            .border_1()
            .border_color(colors.glass_border_card())
            .shadow(elevation(theme, Elevation::Floating))
            .child(left)
            .child(perforation)
            .child(stub)
    }

    /// A notch of the perforation where it meets the card edge.
    fn cutout(&self, theme: &Theme, top: bool) -> Div {
        let notch = div()
            .absolute()
            .left(px(-CUTOUT / 2.0))
            .size(px(CUTOUT))
            .rounded_full()
            .bg(theme.colors.canvas())
            .border_1()
            .border_color(theme.colors.hairline_divider());
        if top {
            notch.top(px(-CUTOUT / 2.0))
        } else {
            notch.bottom(px(-CUTOUT / 2.0))
        }
    }

    /// The four actions in a floating pill, with what comes next above it.
    fn action_pill(&self, theme: &Theme, next: Option<String>, cx: &mut Context<Self>) -> Div {
        let colors = theme.colors;
        let Some(detail) = self.detail_visible() else {
            return div();
        };
        let labels = [
            t::review_reject(),
            if detail.summary.status == CandidateStatus::Snoozed {
                t::review_resume()
            } else {
                t::review_snooze()
            },
            t::review_adjust(),
            t::review_confirm(),
        ];
        let pill = div()
            .flex_none()
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .p(px(6.0))
            .rounded_full()
            .bg(colors.floating())
            .border_1()
            .border_color(colors.glass_border())
            .shadow(elevation(theme, Elevation::Floating))
            .when(self.busy, |pill| {
                pill.child(
                    text_style(div(), TypeScale::LABEL)
                        .px(px(SpacingScale::S3))
                        .text_color(colors.text_muted())
                        .child(t::review_recording()),
                )
            })
            .children(
                [
                    ReviewAction::Reject,
                    ReviewAction::Snooze,
                    ReviewAction::Edit,
                    ReviewAction::Confirm,
                ]
                .into_iter()
                .enumerate()
                .map(|(index, action)| {
                    let kind = if index == 3 {
                        ButtonKind::Primary
                    } else {
                        ButtonKind::Ghost
                    };
                    action_button(theme, ("review-action", index), kind, !self.busy)
                        .px(px(SpacingScale::S4))
                        .rounded_full()
                        .aria_label(labels[index])
                        .track_focus(&self.action_focus[index])
                        .on_click(
                            cx.listener(move |this, _, window, cx| this.review(action, window, cx)),
                        )
                        .on_key_down(cx.listener(
                            move |this, event: &gpui::KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    this.review(action, window, cx);
                                    cx.stop_propagation();
                                }
                            },
                        ))
                        .child(labels[index])
                        .child(
                            text_style(div(), TypeScale::META)
                                .flex_none()
                                .text_color(if self.busy {
                                    colors.text_disabled()
                                } else if index == 3 {
                                    colors.accent_on_emphasis()
                                } else {
                                    colors.text_muted()
                                })
                                .child(["R", "S", "A", "C"][index]),
                        )
                }),
            );
        let screen = cx.entity_id();
        // The lake reads these while it paints, in the same frame: the
        // prepaint of the whole tree runs before any paint.
        let measure = |slot: usize| {
            let clearings = self.night_slot.clearings.clone();
            canvas(
                move |bounds, _, cx| {
                    let before = clearings.borrow_mut()[slot].replace(bounds);
                    if before != Some(bounds) {
                        cx.defer(move |cx| cx.notify(screen));
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .size_full()
        };
        if next.is_none() {
            self.night_slot.clearings.borrow_mut()[0] = None;
        }
        div()
            .absolute()
            .left_0()
            .right_0()
            .bottom(px(PILL_BOTTOM))
            .flex()
            .flex_col()
            .items_center()
            .gap(px(SpacingScale::S3))
            .children(next.map(|question| {
                text_style(div(), TypeScale::LABEL)
                    .relative()
                    .max_w(px(NEXT_WIDTH))
                    .truncate()
                    .text_color(colors.text_muted())
                    .child(tt::next_up(&question))
                    .child(measure(0))
            }))
            .child(pill.relative().child(measure(1)))
    }

    /// The proof: where the choice came from, read in the sources.
    fn proof(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let Some(detail) = self.detail_visible() else {
            return div().into_any_element();
        };
        let source_detail = evidence_detail(detail, self.member.as_ref());
        let why = one_sentence(&detail.rationale);
        let column = div()
            .w_full()
            .max_w(px(TICKET_WIDTH))
            .flex()
            .flex_col()
            .gap(px(SpacingScale::S5))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S4))
                    .child(panel_title(theme, tt::proof_title()))
                    .child(div().flex_1())
                    .child(self.view_hint(
                        theme,
                        "proof-close",
                        "Esc",
                        tt::close_hint(),
                        ReviewView::Close,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S1))
                    .child(
                        text_style(div(), TypeScale::BODY)
                            .text_color(colors.text_secondary())
                            .child(detail.summary.question.clone()),
                    )
                    .child(
                        text_style(div(), MID_VALUE)
                            .font_family(Theme::font_display())
                            .text_color(colors.text_primary())
                            .child(detail.summary.choice.clone()),
                    ),
            )
            .child(self.evidence_block(theme, source_detail, cx))
            .when(!why.is_empty(), |page| {
                page.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(SpacingScale::S2))
                        .child(section_label(theme, tt::proof_why()))
                        .child(self.reason_flow(theme, &why, where_file(detail).as_deref())),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S5))
                    .child(self.view_hint(
                        theme,
                        "proof-prev",
                        "←",
                        tt::other_source_hint(),
                        ReviewView::PrevSource,
                        cx,
                    ))
                    .child(self.view_hint(
                        theme,
                        "proof-next",
                        "→",
                        tt::other_source_hint(),
                        ReviewView::NextSource,
                        cx,
                    )),
            );
        div()
            .id("inbox-proof")
            .size_full()
            .overflow_y_scroll()
            .px(px(SpacingScale::S8))
            .py(px(SpacingScale::S8))
            .flex()
            .flex_col()
            .items_center()
            .child(column)
            .into_any_element()
    }

    /// What confirming changes: the full reading of the candidate.
    fn effect(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex_none()
                    .px(px(SpacingScale::S8))
                    .pt(px(SpacingScale::S4))
                    .flex()
                    .items_center()
                    .gap(px(SpacingScale::S4))
                    .child(panel_title(theme, tt::effect_title()))
                    .child(div().flex_1())
                    .child(self.view_hint(
                        theme,
                        "effect-close",
                        "Esc",
                        tt::close_hint(),
                        ReviewView::Close,
                        cx,
                    )),
            )
            .child(div().flex_1().min_h(px(0.0)).child(self.reading_pane(cx)))
            .child(self.review_actions(cx))
            .into_any_element()
    }

    /// The empty queue: the day in one line, whether captures still arrive,
    /// and the ways on.
    fn empty_queue(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let ai = self.accepted_by_ai();
        let you = self.decided;
        let view = self.capture_view();
        let parts = empty_parts(ai, you, view.as_ref());
        let capture = self.capture_block(theme, view.as_ref(), parts, cx);
        let mut shortcut =
            |id: &'static str, key: &'static str, label: &'static str, target: ReviewView| {
                self.view_hint(theme, id, key, label, target, cx)
            };
        let shortcuts = (parts.decided || parts.accepted).then(|| {
            div()
                .flex()
                .gap(px(SpacingScale::S6))
                .when(parts.decided, |row| {
                    row.child(shortcut(
                        "zero-today",
                        "D",
                        tt::zero_decisions(),
                        ReviewView::Today,
                    ))
                })
                .when(parts.accepted, |row| {
                    row.child(shortcut(
                        "zero-accepted",
                        "L",
                        tt::zero_accepted(),
                        ReviewView::Accepted,
                    ))
                })
        });
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(SpacingScale::S4))
            .child(
                text_style(div(), BIG_VALUE)
                    .font_family(Theme::font_display())
                    .text_color(colors.text_primary())
                    .child(if parts.decided {
                        tt::zero_title()
                    } else {
                        tt::empty_title_sentence()
                    }),
            )
            .when(parts.decided, |column| {
                column.child(
                    text_style(div(), TypeScale::BODY)
                        .text_color(colors.text_secondary())
                        .child(tt::zero_summary(ai + you, ai, you)),
                )
            })
            .children(capture)
            .children(shortcuts)
            .children(self.activity_line(theme, cx))
            .into_any_element()
    }

    /// Whether captures reach the project: a quiet line when they do (or in
    /// the demo), an opaque warning card when they stopped or nothing arrives,
    /// and a line for failed analyses. Nothing before the status is known.
    fn capture_block(
        &self,
        theme: &Theme,
        view: Option<&CaptureView>,
        parts: EmptyParts,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let view = view?;
        let block = parts.block?;
        let colors = theme.colors;
        let dot = |color| div().size(px(DOT)).flex_none().rounded_full().bg(color);
        let main = match block {
            BlockKind::Line => {
                let (color, text) = match view.health {
                    CaptureHealth::Demo => {
                        (colors.status_info(), tt::capture_demo_line().to_owned())
                    }
                    CaptureHealth::Waiting => {
                        (colors.status_success(), tt::capture_none_title().to_owned())
                    }
                    _ => (
                        colors.status_success(),
                        tt::capture_ok(&view.last_relative()),
                    ),
                };
                div()
                    .max_w(px(CAPTURE_WIDTH))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(SpacingScale::S1))
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .text_color(colors.text_secondary())
                            .child(dot(color))
                            .child(text),
                    )
                    .when(view.health == CaptureHealth::Waiting, |line| {
                        line.child(
                            text_style(div(), TypeScale::BODY_SMALL)
                                .text_color(colors.text_muted())
                                .child(tt::capture_none_body()),
                        )
                    })
                    .into_any_element()
            }
            BlockKind::Warning => {
                let (title, body) = view.warning()?;
                let warning = colors.status_warning();
                div()
                    .w(px(CAPTURE_WIDTH))
                    .p(px(SpacingScale::S4))
                    .flex()
                    .flex_col()
                    .gap(px(SpacingScale::S2))
                    .rounded(theme.radius.surface())
                    .border_1()
                    .border_color(tint(warning, 0.28))
                    // Opaque: the night field must not show through the card.
                    .bg(mix(colors.canvas(), warning, 0.08))
                    .child(
                        text_style(div(), TypeScale::ROW_TITLE)
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .text_color(colors.text_primary())
                            .child(dot(warning))
                            .child(title),
                    )
                    .child(
                        text_style(div(), TypeScale::BODY_SMALL)
                            // Aligned with the title, past the dot and the gap.
                            .pl(px(DOT + SpacingScale::S2))
                            .text_color(colors.text_secondary())
                            .child(body),
                    )
                    .child(
                        div().flex().justify_end().child(
                            action_button(
                                theme,
                                "review-capture-diagnostics",
                                ButtonKind::Secondary,
                                true,
                            )
                            .aria_label(tt::open_diagnostics())
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(OpenDiagnostics)))
                            .child(tt::open_diagnostics()),
                        ),
                    )
                    .into_any_element()
            }
        };
        Some(
            div()
                .flex()
                .flex_col()
                .items_center()
                .gap(px(SpacingScale::S3))
                .child(main)
                .when(parts.failures, |column| {
                    column.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(SpacingScale::S2))
                            .child(dot(colors.status_danger()))
                            .child(
                                text_style(div(), TypeScale::BODY_SMALL)
                                    .text_color(colors.status_danger())
                                    .child(tt::capture_not_analysed(view.failed)),
                            )
                            .child(
                                action_button(
                                    theme,
                                    "review-capture-failures",
                                    ButtonKind::Ghost,
                                    true,
                                )
                                .aria_label(tt::open_diagnostics())
                                .on_click(cx.listener(|_, _, _, cx| cx.emit(OpenDiagnostics)))
                                .child(tt::open_diagnostics()),
                            ),
                    )
                })
                .into_any_element(),
        )
    }
}

/// Widest the capture line and card get under the empty queue.
const CAPTURE_WIDTH: f32 = 520.0;

/// The status dot of the capture line, card and failures.
const DOT: f32 = 8.0;

/// How the capture shows on the empty queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockKind {
    /// One quiet line: active, waiting for a first capture, or the demo.
    Line,
    /// A warning card: the capture stopped or nothing arrives.
    Warning,
}

/// Which pieces the empty queue shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EmptyParts {
    /// Something was decided today: "Queue cleared.", the summary and the
    /// D shortcut, instead of "Nothing waiting for review.".
    decided: bool,
    /// The AI accepted something today: the L shortcut.
    accepted: bool,
    block: Option<BlockKind>,
    /// A line for the captures whose analysis failed.
    failures: bool,
}

/// Decides the empty queue from what the AI (`ai`) and the user (`you`)
/// decided today and the capture state: a day with no decision does not
/// celebrate, and failures add a line only when the capture is not stopped
/// nor the demo.
fn empty_parts(ai: usize, you: usize, view: Option<&CaptureView>) -> EmptyParts {
    let block = view.map(|view| match view.health {
        CaptureHealth::Stale | CaptureHealth::Stopped => BlockKind::Warning,
        CaptureHealth::Active | CaptureHealth::Waiting | CaptureHealth::Demo => BlockKind::Line,
    });
    let failures = view.is_some_and(|view| {
        view.failed > 0
            && matches!(
                view.health,
                CaptureHealth::Active | CaptureHealth::Waiting | CaptureHealth::Stale
            )
    });
    EmptyParts {
        decided: ai + you > 0,
        accepted: ai > 0,
        block,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_quoted_words_are_chips() {
        for word in [
            "src/index/cache.rs",
            "`ttl`",
            "“cache”,",
            "cache.rs.",
            "a\\b",
        ] {
            assert!(is_chip(word), "{word}");
        }
        for word in ["cache", "the", "3.5", "e.g.", "a", "version."] {
            assert!(!is_chip(word), "{word}");
        }
    }

    #[test]
    fn a_reason_is_cut_at_its_first_sentence() {
        assert_eq!(one_sentence("Uses a TTL.  It is faster."), "Uses a TTL.");
        assert_eq!(one_sentence("no stop"), "no stop");
        assert_eq!(one_sentence("v1.2 is out. Next"), "v1.2 is out.");
        let long = "word ".repeat(100);
        let cut = one_sentence(&long);
        assert!(cut.chars().count() <= REASON_CHARS + 1);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn long_choices_use_smaller_values() {
        assert_eq!(value_token(5), NEW_VALUE);
        assert_eq!(value_token(52), NEW_VALUE);
        assert_eq!(value_token(60), NEW_VALUE);
        assert_eq!(value_token(80), SMALL_VALUE);
    }

    fn view(health: CaptureHealth, failed: usize) -> CaptureView {
        CaptureView {
            health,
            last: Some("2026-10-02T11:43:00Z".into()),
            failed,
        }
    }

    #[test]
    fn zero_day_does_not_celebrate() {
        let parts = empty_parts(0, 0, None);
        assert!(!parts.decided);
        assert!(!parts.accepted);
        assert_eq!(parts.block, None);
        assert!(!parts.failures);
    }

    #[test]
    fn ai_only_shows_l_and_d() {
        let parts = empty_parts(3, 0, None);
        assert!(parts.decided && parts.accepted);
        let you_only = empty_parts(0, 2, None);
        assert!(you_only.decided && !you_only.accepted);
    }

    #[test]
    fn stopped_shows_warning_card() {
        let parts = empty_parts(0, 0, Some(&view(CaptureHealth::Stopped, 2)));
        assert_eq!(parts.block, Some(BlockKind::Warning));
        assert!(!parts.failures);
    }

    #[test]
    fn stale_shows_warning_card() {
        let parts = empty_parts(0, 0, Some(&view(CaptureHealth::Stale, 1)));
        assert_eq!(parts.block, Some(BlockKind::Warning));
        assert!(parts.failures);
    }

    #[test]
    fn failures_add_a_line_to_active() {
        let parts = empty_parts(1, 0, Some(&view(CaptureHealth::Active, 2)));
        assert_eq!(parts.block, Some(BlockKind::Line));
        assert!(parts.failures);
        let clean = empty_parts(1, 0, Some(&view(CaptureHealth::Active, 0)));
        assert!(!clean.failures);
    }

    #[test]
    fn demo_shows_demo_line() {
        let parts = empty_parts(0, 0, Some(&view(CaptureHealth::Demo, 3)));
        assert_eq!(parts.block, Some(BlockKind::Line));
        assert!(!parts.failures);
    }
}
