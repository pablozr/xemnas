//! Compact, editable GPUI search field with OS text input, selection and IME.
//!
//! The outer shell owns the search destination; this reusable view owns the
//! editing mechanics. `Role::TextInput` alone does not make a `div` editable.

use std::ops::Range;
use std::sync::Arc;

use gpui::prelude::*;
use gpui::{
    actions, div, fill, point, px, relative, size, App, Bounds, ClipboardItem, Context, Element,
    ElementId, ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle,
    Focusable, GlobalElementId, LayoutId, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, PaintQuad, Pixels, Point, Render, Role, ShapedLine, Style, TextAlign, TextRun,
    UTF16Selection, UnderlineStyle, Window,
};

use crate::i18n::common as t;
use crate::ui::icons::{icon, IconName};
use crate::ui::search_edit::SearchEdit;
use crate::ui::theme::{text_style, Theme};
use crate::ui::tokens::{ControlSize, SpacingScale, TypeScale};

actions!(
    search_field,
    [
        /// Deletes the grapheme before the caret or the selection.
        Backspace,
        /// Deletes the grapheme after the caret or the selection.
        Delete,
        /// Moves the caret left.
        Left,
        /// Moves the caret right.
        Right,
        /// Extends the selection left.
        SelectLeft,
        /// Extends the selection right.
        SelectRight,
        /// Selects all search text.
        SelectAll,
        /// Moves the caret to the start.
        Home,
        /// Moves the caret to the end.
        End,
        /// Pastes clipboard text into the search field.
        Paste,
        /// Copies the selected text.
        Copy,
        /// Cuts the selected text.
        Cut,
        /// Clears the search text.
        Clear
    ]
);

/// Broadcast when editing changes the filter text.
pub struct SearchChanged(pub String);

/// Single-line input for filtering projects; GPUI forwards its IME to Windows.
pub struct SearchField {
    theme: Theme,
    placeholder: &'static str,
    focus: FocusHandle,
    edit: SearchEdit,
    layout: Option<ShapedLine>,
    bounds: Option<Bounds<Pixels>>,
    scroll_x: Pixels,
    selecting: bool,
    width: f32,
    show_shortcut: bool,
    fill_width: bool,
    multiline_height: Option<f32>,
    wrapped: Vec<VisualLine>,
    scroll_y: Pixels,
    last_caret: Option<usize>,
    secret: bool,
}

/// Glyph painted in place of each character of a masked field.
const MASK: char = '•';

/// Byte offset in the masked text for a byte offset in `text`.
fn masked_offset(text: &str, offset: usize) -> usize {
    text[..offset].chars().count() * MASK.len_utf8()
}

/// Byte offset in `text` for a byte offset in its masked form.
fn unmasked_offset(text: &str, offset: usize) -> usize {
    text.char_indices()
        .nth(offset / MASK.len_utf8())
        .map_or(text.len(), |(index, _)| index)
}

impl EventEmitter<SearchChanged> for SearchField {}

impl SearchField {
    /// Creates an unfocused, empty search field.
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            theme: Theme::current(cx),
            placeholder: t::search_projects(),
            focus: cx.focus_handle().tab_stop(true),
            edit: SearchEdit::default(),
            layout: None,
            bounds: None,
            scroll_x: px(0.0),
            selecting: false,
            width: 280.0,
            show_shortcut: true,
            fill_width: false,
            multiline_height: None,
            wrapped: Vec::new(),
            scroll_y: px(0.0),
            last_caret: None,
            secret: false,
        }
    }

    /// Fits the same text editor into a narrower navigation rail.
    pub fn set_width(&mut self, width: f32) {
        self.width = width;
        self.show_shortcut = false;
    }
    /// Uses the parent's width when composing a labelled editing form.
    pub fn stretch(&mut self) {
        self.fill_width = true;
        self.show_shortcut = false;
    }

    /// Enables wrapped, scrollable document editing with native selection and IME.
    pub fn multiline(&mut self, height: f32) {
        self.stretch();
        self.multiline_height = Some(height);
        self.edit.multiline = true;
    }

    /// Masks the value for a credential: paints one dot per character and
    /// refuses copy and cut, so the text only leaves through [`Self::value`].
    pub fn secret(&mut self) {
        self.stretch();
        self.secret = true;
    }

    /// Text as painted: the value itself, or one mask glyph per character.
    fn display_text(&self) -> String {
        if self.secret {
            self.edit.text.chars().map(|_| MASK).collect()
        } else {
            self.edit.text.clone()
        }
    }

    /// Maps a byte offset in the value to the painted text.
    fn display_offset(&self, offset: usize) -> usize {
        if self.secret {
            masked_offset(&self.edit.text, offset)
        } else {
            offset
        }
    }

    /// Maps a byte offset in the painted text back to the value.
    fn value_offset(&self, offset: usize) -> usize {
        if self.secret {
            unmasked_offset(&self.edit.text, offset)
        } else {
            offset
        }
    }

    /// Seeds a single-line value while preserving all normal editing mechanics.
    pub fn set_value(&mut self, value: &str, cx: &mut Context<Self>) {
        self.edit = SearchEdit::default();
        self.edit.multiline = self.multiline_height.is_some();
        self.edit.replace(None, value);
        self.changed(cx);
    }

    /// Current edited text, for a form submission.
    pub fn value(&self) -> &str {
        &self.edit.text
    }

    /// Changes the search destination and clears the previous destination's query.
    pub fn set_context(&mut self, placeholder: &'static str, cx: &mut Context<Self>) {
        self.placeholder = placeholder;
        self.edit = SearchEdit::default();
        self.edit.multiline = self.multiline_height.is_some();
        self.changed(cx);
    }

    fn changed(&mut self, cx: &mut Context<Self>) {
        self.layout = None;
        self.wrapped.clear();
        self.last_caret = None;
        cx.emit(SearchChanged(self.edit.text.clone()));
        cx.notify();
    }

    fn index_at(&self, position: Point<Pixels>) -> usize {
        if self.multiline_height.is_some() {
            let Some(bounds) = self.bounds else {
                return 0;
            };
            let position = position - bounds.origin + point(px(0.0), self.scroll_y);
            return wrapped_index(&self.wrapped, position).min(self.edit.text.len());
        }
        match (self.layout.as_ref(), self.bounds.as_ref()) {
            (Some(line), Some(bounds)) => self
                .value_offset(line.closest_index_for_x(position.x - bounds.left() + self.scroll_x)),
            _ => 0,
        }
    }

    fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.focus, cx);
        self.selecting = true;
        let index = self.index_at(event.position);
        if event.modifiers.shift {
            self.edit.select_to(index);
        } else {
            self.edit.move_to(index);
        }
        cx.notify();
    }

    fn on_mouse_move(&mut self, event: &MouseMoveEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.selecting {
            let index = self.index_at(event.position);
            self.edit.select_to(index);
            cx.notify();
        }
    }

    fn on_mouse_up(&mut self, _: &MouseUpEvent, _: &mut Window, _: &mut Context<Self>) {
        self.selecting = false;
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.backspace();
        self.changed(cx);
    }
    fn delete(&mut self, _: &Delete, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.delete();
        self.changed(cx);
    }
    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.edit.selection.is_empty() {
            self.edit.previous_boundary(self.edit.caret())
        } else {
            self.edit.selection.start
        };
        self.edit.move_to(offset);
        cx.notify();
    }
    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let offset = if self.edit.selection.is_empty() {
            self.edit.next_boundary(self.edit.caret())
        } else {
            self.edit.selection.end
        };
        self.edit.move_to(offset);
        cx.notify();
    }
    fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.edit.previous_boundary(self.edit.caret());
        self.edit.select_to(offset);
        cx.notify();
    }
    fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.edit.next_boundary(self.edit.caret());
        self.edit.select_to(offset);
        cx.notify();
    }
    fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.move_to(0);
        self.edit.select_to(self.edit.text.len());
        cx.notify();
    }
    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.move_to(0);
        cx.notify();
    }
    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.move_to(self.edit.text.len());
        cx.notify();
    }
    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.edit.replace(None, &text);
            self.changed(cx);
        }
    }
    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.secret && !self.edit.selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.edit.text[self.edit.selection.clone()].to_string(),
            ));
        }
    }
    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) {
        if !self.secret && !self.edit.selection.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.edit.text[self.edit.selection.clone()].to_string(),
            ));
            self.edit.replace(None, "");
            self.changed(cx);
        }
    }
    fn clear(&mut self, _: &Clear, _: &mut Window, cx: &mut Context<Self>) {
        if self.fill_width {
            return;
        }
        self.edit = SearchEdit::default();
        self.changed(cx);
    }
}

impl Focusable for SearchField {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl EntityInputHandler for SearchField {
    fn text_for_range(
        &mut self,
        range: Range<usize>,
        actual: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let start = self.edit.from_utf16(range.start);
        let end = self.edit.from_utf16(range.end);
        *actual = Some(self.edit.to_utf16(start)..self.edit.to_utf16(end));
        Some(self.edit.text.get(start..end)?.to_string())
    }
    fn selected_text_range(
        &mut self,
        _: bool,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.edit.to_utf16(self.edit.selection.start)
                ..self.edit.to_utf16(self.edit.selection.end),
            reversed: self.edit.reversed,
        })
    }
    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.edit
            .marked
            .as_ref()
            .map(|range| self.edit.to_utf16(range.start)..self.edit.to_utf16(range.end))
    }
    fn unmark_text(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.edit.marked = None;
        cx.notify();
    }
    fn replace_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range =
            range.map(|range| self.edit.from_utf16(range.start)..self.edit.from_utf16(range.end));
        self.edit.replace(range, text);
        self.changed(cx);
    }
    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range =
            range.map(|range| self.edit.from_utf16(range.start)..self.edit.from_utf16(range.end));
        self.edit.replace_and_mark(range, text, selected);
        self.changed(cx);
    }
    fn bounds_for_range(
        &mut self,
        range: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        if self.multiline_height.is_some() {
            let position = wrapped_position(&self.wrapped, self.edit.from_utf16(range.start));
            return Some(Bounds::new(
                bounds.origin + position - point(px(0.0), self.scroll_y),
                size(px(1.0), px(19.0)),
            ));
        }
        let line = self.layout.as_ref()?;
        let start = self.display_offset(self.edit.from_utf16(range.start));
        let end = self.display_offset(self.edit.from_utf16(range.end));
        Some(Bounds::from_corners(
            point(
                bounds.left() + line.x_for_index(start) - self.scroll_x,
                bounds.top(),
            ),
            point(
                bounds.left() + line.x_for_index(end) - self.scroll_x,
                bounds.bottom(),
            ),
        ))
    }
    fn character_index_for_point(
        &mut self,
        position: Point<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<usize> {
        Some(self.edit.to_utf16(self.index_at(position)))
    }
    fn set_selected_text_range(
        &mut self,
        range: Range<usize>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit.selection = self.edit.from_utf16(range.start)..self.edit.from_utf16(range.end);
        self.edit.reversed = false;
        cx.notify();
    }
}

struct SearchTextElement {
    input: Entity<SearchField>,
}
struct TextPaint {
    line: ShapedLine,
    selection: Option<PaintQuad>,
    cursor: Option<PaintQuad>,
    scroll_x: Pixels,
}

impl IntoElement for SearchTextElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}

impl Element for SearchTextElement {
    type RequestLayoutState = ();
    type PrepaintState = TextPaint;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> TextPaint {
        let input = self.input.read(cx);
        let focused = input.focus.is_focused(window);
        let display = input.display_text();
        let content = if input.edit.text.is_empty() && !focused {
            input.placeholder
        } else {
            &display
        };
        let style = window.text_style();
        let text_color = if input.edit.text.is_empty() {
            input.theme.colors.text_muted()
        } else {
            input.theme.colors.text_primary()
        };
        let run = TextRun {
            len: content.len(),
            font: style.font(),
            color: text_color.into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let marked = input
            .edit
            .marked
            .as_ref()
            .map(|marked| input.display_offset(marked.start)..input.display_offset(marked.end));
        let runs = if let Some(marked) = &marked {
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.end - marked.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: content.len() - marked.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line =
            window
                .text_system()
                .shape_line(content.to_string().into(), font_size, &runs, None);
        let caret_x = line.x_for_index(input.display_offset(input.edit.caret()));
        let scroll_x = if focused {
            (caret_x - bounds.size.width + px(4.0)).max(px(0.0))
        } else {
            px(0.0)
        };
        let origin_x = bounds.left() - scroll_x;
        let selection = (!input.edit.selection.is_empty()).then(|| {
            fill(
                Bounds::from_corners(
                    point(
                        origin_x
                            + line.x_for_index(input.display_offset(input.edit.selection.start)),
                        bounds.top(),
                    ),
                    point(
                        origin_x + line.x_for_index(input.display_offset(input.edit.selection.end)),
                        bounds.bottom(),
                    ),
                ),
                input.theme.colors.glass_fill_strong(),
            )
        });
        let cursor = (focused && input.edit.selection.is_empty()).then(|| {
            fill(
                Bounds::new(
                    point(origin_x + caret_x, bounds.top()),
                    size(px(1.0), bounds.size.height),
                ),
                input.theme.colors.accent_default(),
            )
        });
        TextPaint {
            line,
            selection,
            cursor,
            scroll_x,
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        paint: &mut TextPaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus = self.input.read(cx).focus.clone();
        window.handle_input(
            &focus,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        if let Some(selection) = paint.selection.take() {
            window.paint_quad(selection);
        }
        paint
            .line
            .paint(
                point(bounds.left() - paint.scroll_x, bounds.top()),
                window.line_height(),
                TextAlign::Left,
                None,
                window,
                cx,
            )
            .unwrap();
        if let Some(cursor) = paint.cursor.take() {
            window.paint_quad(cursor);
        }
        self.input.update(cx, |input, _| {
            input.layout = Some(paint.line.clone());
            input.bounds = Some(bounds);
            input.scroll_x = paint.scroll_x;
        });
    }
}

#[derive(Clone)]
struct VisualLine {
    offset: usize,
    y: Pixels,
    line: Arc<gpui::WrappedLine>,
}

fn wrapped_position(lines: &[VisualLine], index: usize) -> Point<Pixels> {
    let Some(line) = lines.iter().rev().find(|line| line.offset <= index) else {
        return point(px(0.0), px(0.0));
    };
    line.line
        .position_for_index((index - line.offset).min(line.line.len()), px(19.0))
        .unwrap_or_default()
        + point(px(0.0), line.y)
}
fn wrapped_index(lines: &[VisualLine], position: Point<Pixels>) -> usize {
    let Some(line) = lines
        .iter()
        .rev()
        .find(|line| line.y <= position.y)
        .or(lines.first())
    else {
        return 0;
    };
    let index = line
        .line
        .closest_index_for_position(position - point(px(0.0), line.y), px(19.0));
    line.offset + index.unwrap_or_else(|index| index)
}
struct MultilineTextElement {
    input: Entity<SearchField>,
}
struct MultilinePaint {
    lines: Vec<VisualLine>,
    quads: Vec<PaintQuad>,
    scroll: Pixels,
}
impl IntoElement for MultilineTextElement {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for MultilineTextElement {
    type RequestLayoutState = ();
    type PrepaintState = MultilinePaint;
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, ()) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = px(self.input.read(cx).multiline_height.unwrap_or(120.0) - 24.0).into();
        (window.request_layout(style, [], cx), ())
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) -> MultilinePaint {
        let input = self.input.read(cx);
        let focused = input.focus.is_focused(window);
        let content = if input.edit.text.is_empty() && !focused {
            input.placeholder
        } else {
            &input.edit.text
        };
        let style = window.text_style();
        let run = TextRun {
            len: content.len(),
            font: style.font(),
            color: if input.edit.text.is_empty() {
                input.theme.colors.text_muted().into()
            } else {
                input.theme.colors.text_primary().into()
            },
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = if let Some(marked) = &input.edit.marked {
            vec![
                TextRun {
                    len: marked.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked.len(),
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: content.len() - marked.end,
                    ..run
                },
            ]
        } else {
            vec![run]
        };
        let shaped = window
            .text_system()
            .shape_text(
                content.to_owned().into(),
                style.font_size.to_pixels(window.rem_size()),
                &runs,
                Some(bounds.size.width),
                None,
            )
            .unwrap_or_default();
        let mut offset = 0;
        let mut y = px(0.0);
        let mut lines = Vec::new();
        for line in shaped {
            let height = line.size(px(19.0)).height;
            let len = line.len();
            lines.push(VisualLine {
                offset,
                y,
                line: Arc::new(line),
            });
            offset += len + 1;
            y += height;
        }
        let caret = wrapped_position(&lines, input.edit.caret());
        let mut scroll = input.scroll_y;
        if focused && input.last_caret != Some(input.edit.caret()) {
            if caret.y < scroll {
                scroll = caret.y;
            } else if caret.y + px(19.0) > scroll + bounds.size.height {
                scroll = caret.y + px(19.0) - bounds.size.height;
            }
        }
        scroll = scroll
            .max(px(0.0))
            .min((y - bounds.size.height).max(px(0.0)));
        let origin = bounds.origin - point(px(0.0), scroll);
        let mut quads = Vec::new();
        if focused {
            for line in &lines {
                let height = line.line.size(px(19.0)).height;
                let mut row_y = px(0.0);
                while row_y < height {
                    let start = wrapped_index(&lines, point(px(0.0), line.y + row_y));
                    let end = wrapped_index(&lines, point(bounds.size.width, line.y + row_y));
                    let left = start.max(input.edit.selection.start);
                    let right = end.min(input.edit.selection.end);
                    if left < right {
                        let x0 = if left == start {
                            px(0.0)
                        } else {
                            wrapped_position(&lines, left).x
                        };
                        let x1 = if right == end {
                            bounds.size.width
                        } else {
                            wrapped_position(&lines, right).x
                        };
                        quads.push(fill(
                            Bounds::new(
                                origin + point(x0, line.y + row_y),
                                size((x1 - x0).max(px(1.0)), px(19.0)),
                            ),
                            input.theme.colors.glass_fill_strong(),
                        ));
                    }
                    row_y += px(19.0);
                }
            }
            if input.edit.selection.is_empty() {
                quads.push(fill(
                    Bounds::new(origin + caret, size(px(1.0), px(19.0))),
                    input.theme.colors.accent_default(),
                ));
            }
        }
        MultilinePaint {
            lines,
            quads,
            scroll,
        }
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut (),
        paint: &mut MultilinePaint,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.handle_input(
            &self.input.read(cx).focus.clone(),
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );
        for quad in paint.quads.drain(..) {
            window.paint_quad(quad);
        }
        for line in &paint.lines {
            let _ = line.line.paint(
                bounds.origin + point(px(0.0), line.y - paint.scroll),
                px(19.0),
                TextAlign::Left,
                None,
                window,
                cx,
            );
        }
        self.input.update(cx, |input, _| {
            input.wrapped = paint.lines.clone();
            input.bounds = Some(bounds);
            input.scroll_y = paint.scroll;
            input.last_caret = Some(input.edit.caret());
        });
    }
}

impl Render for SearchField {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.theme = Theme::current(cx);
        let theme = Theme::current(cx);
        let focused = self.focus.is_focused(window);
        div()
            .id("title-search")
            .w(px(self.width))
            .when(self.fill_width, |field| field.w_full())
            .h(px(self.multiline_height.unwrap_or(ControlSize::MD)))
            .px(px(SpacingScale::S3))
            .flex()
            .items_center()
            .gap(px(SpacingScale::S2))
            .rounded(theme.radius.control())
            .border_1()
            // Neutral at rest: a field is not a selection, so lavender only
            // arrives with focus. The focused border already is the ring.
            .border_color(if focused {
                theme.colors.accent_default()
            } else {
                theme.colors.glass_border_control()
            })
            .bg(if focused {
                theme.colors.glass_fill_card_hover()
            } else {
                theme.colors.glass_fill_card()
            })
            .when(!focused, |field| {
                field.hover(move |style| style.border_color(theme.colors.glass_border_card_hover()))
            })
            .track_focus(&self.focus)
            .key_context("SearchField")
            .role(Role::TextInput)
            .aria_label(self.placeholder)
            .cursor_text()
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::select_left))
            .on_action(cx.listener(Self::select_right))
            .on_action(cx.listener(Self::select_all))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::copy))
            .on_action(cx.listener(Self::cut))
            .on_action(cx.listener(Self::clear))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_mouse_up_out(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if this.multiline_height.is_none() {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "enter" if this.edit.marked.is_none() && !event.keystroke.modifiers.control => {
                        this.edit.replace(None, "\n");
                        this.changed(cx);
                    }
                    "up" | "down" => {
                        let mut position = wrapped_position(&this.wrapped, this.edit.caret());
                        position.y = (position.y
                            + px(if event.keystroke.key == "up" {
                                -19.0
                            } else {
                                19.0
                            }))
                        .max(px(0.0));
                        let index =
                            wrapped_index(&this.wrapped, position).min(this.edit.text.len());
                        if event.keystroke.modifiers.shift {
                            this.edit.select_to(index);
                        } else {
                            this.edit.move_to(index);
                        }
                        cx.notify();
                    }
                    _ => return,
                }
                cx.stop_propagation();
            }))
            .on_scroll_wheel(cx.listener(|this, event: &gpui::ScrollWheelEvent, _, cx| {
                if this.multiline_height.is_some() {
                    let height = this
                        .bounds
                        .map(|bounds| bounds.size.height)
                        .unwrap_or(px(0.0));
                    let total = this
                        .wrapped
                        .last()
                        .map(|line| line.y + line.line.size(px(19.0)).height)
                        .unwrap_or(height);
                    this.scroll_y = (this.scroll_y - event.delta.pixel_delta(px(19.0)).y)
                        .max(px(0.0))
                        .min((total - height).max(px(0.0)));
                    cx.notify();
                    cx.stop_propagation();
                }
            }))
            .when(!self.fill_width, |field| {
                field.child(icon(
                    IconName::Search,
                    14.0,
                    if focused {
                        theme.colors.text_secondary()
                    } else {
                        theme.colors.text_muted()
                    },
                ))
            })
            .child(
                text_style(div(), TypeScale::BODY_SMALL)
                    .flex_1()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .child(if self.multiline_height.is_some() {
                        MultilineTextElement { input: cx.entity() }.into_any_element()
                    } else {
                        SearchTextElement { input: cx.entity() }.into_any_element()
                    }),
            )
            .when(
                !self.fill_width && (self.show_shortcut || focused),
                |field| {
                    field.child(
                        text_style(div(), TypeScale::META)
                            .text_color(theme.colors.text_muted())
                            .child(if focused {
                                t::search_clear_hint()
                            } else {
                                "Ctrl F"
                            }),
                    )
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{masked_offset, unmasked_offset, MASK};

    #[test]
    fn mask_offsets_round_trip_over_multibyte_characters() {
        let text = "aé€z";
        let masked: String = text.chars().map(|_| MASK).collect();
        for (index, _) in text.char_indices().chain([(text.len(), ' ')]) {
            let display = masked_offset(text, index);
            assert!(masked.is_char_boundary(display));
            assert_eq!(unmasked_offset(text, display), index);
        }
        assert_eq!(unmasked_offset(text, masked.len() + 3), text.len());
    }
}
