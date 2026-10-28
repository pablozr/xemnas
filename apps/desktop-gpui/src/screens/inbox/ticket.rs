//! The review ticket: one decision per screen. The ticket card carries the
//! question and the change; a progress row above it says where the cycle
//! stands; a floating pill holds the four actions. The queue is not a column:
//! it opens over the ticket. The proof (sources) and the effect (the full
//! reading) replace the ticket while open.

use gpui::FontWeight;

use super::*;
use crate::app::{OpenProof, ToggleQueue};
use crate::i18n::inbox_ticket as tt;
use crate::ui::material::{elevation, Elevation};
use crate::ui::theme::code_style;
use crate::ui::tokens::TypeToken;

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

/// The ticket's own tokens: Bricolage sizes of the mockup.
const BIG_VALUE: TypeToken = TypeToken::new(46.0, 52.0, 500.0);
const MID_VALUE: TypeToken = TypeToken::new(32.0, 38.0, 500.0);
const SMALL_VALUE: TypeToken = TypeToken::new(22.0, 28.0, 500.0);
const STUB_VALUE: TypeToken = TypeToken::new(52.0, 58.0, 500.0);
const REASON: TypeToken = TypeToken::new(17.0, 26.0, 400.0);

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

/// Words of `text` as parts of a sentence: a path or a quoted word is a chip.
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

/// "{level} · {n} sources" for the stub.
fn confidence_level(value: f64) -> &'static str {
    if value >= 0.75 {
        tt::confidence_high()
    } else if value >= 0.5 {
        tt::confidence_medium()
    } else {
        tt::confidence_low()
    }
}

/// The type token for a value of `len` characters, so a long choice still fits.
fn value_token(len: usize) -> TypeToken {
    match len {
        0..=14 => BIG_VALUE,
        15..=30 => MID_VALUE,
        _ => SMALL_VALUE,
    }
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
            .when(empty || ticket, |surface| {
                surface.child(self.night(empty, cx))
            })
            .child(content)
            .when(empty || ticket, |surface| {
                surface.child(self.colophon(theme, empty))
            })
            .when(self.queue_open, |surface| {
                surface.child(self.queue_overlay(theme, visible, cx))
            })
            .into_any_element()
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
    /// `DASHES` long: the dashes and the stars of the night share it.
    fn cycle(&self) -> (usize, usize, usize) {
        let ai = self.accepted_by_ai();
        let total = (ai + self.decided + self.rows.len()).clamp(2, DASHES);
        (ai.min(total), (ai + self.decided).min(total), total)
    }

    /// The night under the ticket (a strip) or over the whole empty queue,
    /// behind everything and deaf to the mouse.
    fn night(&self, full: bool, cx: &mut Context<Self>) -> AnyElement {
        let (_, done, total) = self.cycle();
        let field = if full { &self.night_full } else { &self.night };
        field.update(cx, |field, _| field.set_today(done, total));
        let (width, height) = field.read(cx).size();
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
            .child(div().w(px(width)).flex_none().child(field.clone()))
            .into_any_element()
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
        let hint = |key: &'static str, label: &'static str| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(px(SpacingScale::S1))
                .child(kbd(colors.text_muted(), key))
                .child(muted(label.to_owned()))
        };
        div()
            .w_full()
            .h(px(28.0))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S4))
            .children(self.mode_track(theme, cx))
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
                .child(hint("L", tt::see()))
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
            .child(hint("Tab", tt::queue_hint()))
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
            .child(self.reason_flow(theme, &one_sentence(&detail.rationale)))
            .child(hints);
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .px(px(SpacingScale::S8))
            .pt(px(56.0))
            .child(fade_in(
                column,
                ElementId::Name(format!("ticket-{}", detail.summary.id).into()),
            ))
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

    /// The reason in a sentence of words and chips (files and quotes).
    fn reason_flow(&self, theme: &Theme, sentence: &str) -> Div {
        let colors = theme.colors;
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
        .children(sentence.split_whitespace().map(|word| {
            if is_chip(word) {
                code_style(div(), TypeScale::CODE)
                    .flex_none()
                    .px(px(6.0))
                    .rounded(px(5.0))
                    .bg(colors.glass_fill_medium())
                    .text_color(colors.text_primary())
                    .child(word.to_owned())
            } else {
                div().flex_none().child(word.to_owned())
            }
        }))
    }

    /// The ticket card: the left side reads, the stub says how sure it is.
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
        let id = summary
            .id
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<String>();
        let caption = |text: &'static str| {
            text_style(div(), TypeScale::META)
                .text_color(colors.text_muted())
                .child(text)
        };
        let big = |text: String, color: gpui::Rgba| {
            text_style(div(), token)
                .font_family(Theme::font_display())
                .text_color(color)
                .max_h(px(token.line_height * 2.0))
                .overflow_hidden()
                .child(text)
        };
        let scope = detail
            .qualifiers
            .iter()
            .find(|item| item.kind == application::qualifiers::QualifierKind::Scope)
            .map(|item| item.text.clone());
        let field = |label: &'static str, value: Option<String>| {
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    text_style(div(), TypeScale::META)
                        .text_color(colors.text_disabled())
                        .child(label),
                )
                .child(
                    text_style(div(), TypeScale::BODY_SMALL)
                        .text_color(colors.text_secondary())
                        .truncate()
                        .child(value.unwrap_or_else(|| tt::field_none().to_owned())),
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
                code_style(div(), TypeScale::META)
                    .text_color(colors.text_muted())
                    .truncate()
                    .child(format!("{id}  {origin}")),
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
                    .items_end()
                    .gap(px(SpacingScale::S5))
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .flex_col()
                            .child(big(tt::field_none().to_owned(), colors.text_disabled()))
                            .child(caption(tt::before())),
                    )
                    .child(
                        text_style(div(), MID_VALUE)
                            .flex_none()
                            .pb(px(SpacingScale::S5))
                            .text_color(colors.text_muted())
                            .child("→"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(big(value.clone(), colors.text_primary()))
                            .child(caption(tt::now())),
                    ),
            )
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .gap(px(SpacingScale::S4))
                    .child(field(tt::field_scope(), scope))
                    .child(field(
                        tt::field_where(),
                        Some(project_label(&summary.project_location).to_owned()),
                    ))
                    .child(field(tt::field_applies(), Some(kind.to_owned())))
                    .child(field(tt::field_replaces(), None)),
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
            .gap(px(SpacingScale::S3))
            .child(
                text_style(div(), STUB_VALUE)
                    .font_family(Theme::font_display())
                    .text_color(colors.text_primary())
                    .child(format!("{}%", (summary.confidence * 100.0).round() as i32)),
            )
            .child(div().flex().gap(px(3.0)).children((0..10).map(|index| {
                div()
                    .w(px(10.0))
                    .h(px(6.0))
                    .rounded(px(2.0))
                    .bg(if index < filled {
                        colors.accent_default()
                    } else {
                        colors.glass_fill_strong()
                    })
            })))
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
                        .child(kbd(
                            button_foreground(theme, kind, !self.busy),
                            ["R", "S", "A", "C"][index],
                        ))
                }),
            );
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
                    .max_w(px(TICKET_WIDTH))
                    .truncate()
                    .text_color(colors.text_muted())
                    .child(tt::next_up(&question))
            }))
            .child(pill)
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
                        .child(self.reason_flow(theme, &why)),
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

    /// The empty queue: the day in one line and the ways on.
    fn empty_queue(&self, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let colors = theme.colors;
        let ai = self.accepted_by_ai();
        let you = self.decided;
        let mut shortcut =
            |id: &'static str, key: &'static str, label: &'static str, view: ReviewView| {
                self.view_hint(theme, id, key, label, view, cx)
            };
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
                    .child(tt::zero_title()),
            )
            .child(
                text_style(div(), TypeScale::BODY)
                    .text_color(colors.text_secondary())
                    .child(tt::zero_summary(ai + you, ai, you)),
            )
            .child(
                div()
                    .flex()
                    .gap(px(SpacingScale::S6))
                    .child(shortcut(
                        "zero-today",
                        "D",
                        tt::zero_decisions(),
                        ReviewView::Today,
                    ))
                    .child(shortcut(
                        "zero-accepted",
                        "L",
                        tt::zero_accepted(),
                        ReviewView::Accepted,
                    )),
            )
            .children(self.activity_line(theme, cx))
            .into_any_element()
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
        assert_eq!(value_token(5), BIG_VALUE);
        assert_eq!(value_token(20), MID_VALUE);
        assert_eq!(value_token(80), SMALL_VALUE);
    }
}
