//! Single-line search text model; byte ranges at the GPUI boundary, UTF-16 at the OS boundary.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

/// Editable search value with a selection and optional IME composition.
#[derive(Default)]
pub struct SearchEdit {
    /// Retains line breaks for document editors; searches remain single-line.
    pub multiline: bool,
    /// Current text.
    pub text: String,
    /// Selected UTF-8 byte range.
    pub selection: Range<usize>,
    /// True when the caret is at the start of the selected range.
    pub reversed: bool,
    /// Active IME composition in UTF-8 bytes.
    pub marked: Option<Range<usize>>,
}

impl SearchEdit {
    /// Replaces selected text or a specified UTF-8 range, keeping a single line.
    pub fn replace(&mut self, range: Option<Range<usize>>, text: &str) {
        let range = range
            .or_else(|| self.marked.clone())
            .unwrap_or(self.selection.clone());
        let text = self.normalize(text);
        self.text.replace_range(range.clone(), &text);
        let offset = range.start + text.len();
        self.selection = offset..offset;
        self.reversed = false;
        self.marked = None;
    }

    /// Converts a UTF-16 code-unit offset from GPUI/Windows to a UTF-8 byte offset.
    pub fn from_utf16(&self, offset: usize) -> usize {
        utf16_to_byte(&self.text, offset)
    }

    /// Converts a UTF-8 byte offset to UTF-16 code units.
    pub fn to_utf16(&self, offset: usize) -> usize {
        self.text[..offset].encode_utf16().count()
    }

    /// Deletes one user-perceived character to the left of the selection.
    pub fn backspace(&mut self) {
        if self.selection.is_empty() {
            let caret = self.caret();
            self.selection = self.previous_boundary(caret)..caret;
        }
        self.replace(None, "");
    }

    /// Deletes one user-perceived character to the right of the selection.
    pub fn delete(&mut self) {
        if self.selection.is_empty() {
            let caret = self.caret();
            self.selection = caret..self.next_boundary(caret);
        }
        self.replace(None, "");
    }

    /// Cursor end of the selection, accounting for a backwards selection.
    pub fn caret(&self) -> usize {
        if self.reversed {
            self.selection.start
        } else {
            self.selection.end
        }
    }

    /// Collapses selection to the specified UTF-8 byte boundary.
    pub fn move_to(&mut self, offset: usize) {
        self.selection = offset..offset;
        self.reversed = false;
    }

    /// Extends the selection to the specified UTF-8 byte boundary.
    pub fn select_to(&mut self, offset: usize) {
        let anchor = if self.reversed {
            self.selection.end
        } else {
            self.selection.start
        };
        self.selection = anchor.min(offset)..anchor.max(offset);
        self.reversed = offset < anchor;
    }

    /// Closest grapheme boundary strictly before the offset.
    pub fn previous_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .rev()
            .find_map(|(index, _)| (index < offset).then_some(index))
            .unwrap_or(0)
    }

    /// Closest grapheme boundary strictly after the offset.
    pub fn next_boundary(&self, offset: usize) -> usize {
        self.text
            .grapheme_indices(true)
            .find_map(|(index, _)| (index > offset).then_some(index))
            .unwrap_or(self.text.len())
    }

    /// Applies an IME composition; its selection is relative to the new text.
    pub fn replace_and_mark(
        &mut self,
        range: Option<Range<usize>>,
        text: &str,
        selected: Option<Range<usize>>,
    ) {
        let range = range
            .or_else(|| self.marked.clone())
            .unwrap_or(self.selection.clone());
        let text = self.normalize(text);
        self.text.replace_range(range.clone(), &text);
        self.marked = (!text.is_empty()).then(|| range.start..range.start + text.len());
        self.selection = selected
            .map(|selection| {
                range.start + utf16_to_byte(&text, selection.start)
                    ..range.start + utf16_to_byte(&text, selection.end)
            })
            .unwrap_or_else(|| range.start + text.len()..range.start + text.len());
        self.reversed = false;
    }

    fn normalize(&self, text: &str) -> String {
        if self.multiline {
            text.replace("\r\n", "\n").replace('\r', "\n")
        } else {
            text.replace("\r\n", " ").replace(['\r', '\n'], " ")
        }
    }
}

fn utf16_to_byte(text: &str, offset: usize) -> usize {
    let mut count = 0;
    for (index, ch) in text.char_indices() {
        if count >= offset || count + ch.len_utf16() > offset {
            return index;
        }
        count += ch.len_utf16();
    }
    text.len()
}

#[cfg(test)]
mod tests {
    use super::SearchEdit;

    #[test]
    fn replaces_selection_without_appending_at_end() {
        let mut edit = SearchEdit {
            text: "projeto".into(),
            selection: 0..3,
            ..Default::default()
        };
        edit.replace(None, "meu");
        assert_eq!(edit.text, "meujeto");
        assert_eq!(edit.selection, 3..3);
    }

    #[test]
    fn utf16_offsets_survive_emoji_before_caret() {
        let edit = SearchEdit {
            text: "a🪷b".into(),
            ..Default::default()
        };
        assert_eq!(edit.from_utf16(3), 5);
        assert_eq!(edit.to_utf16(5), 3);
    }

    #[test]
    fn backspace_removes_a_whole_combining_grapheme() {
        let mut edit = SearchEdit {
            text: "e\u{301}x".into(),
            selection: 3..3,
            ..Default::default()
        };
        edit.backspace();
        assert_eq!(edit.text, "x");
        assert_eq!(edit.selection, 0..0);
    }

    #[test]
    fn paste_cannot_turn_search_into_multiline_text() {
        let mut edit = SearchEdit::default();
        edit.replace(None, "foo\r\nbar");
        assert_eq!(edit.text, "foo bar");
    }

    #[test]
    fn document_editing_preserves_lines_and_composition_offsets() {
        let mut edit = SearchEdit {
            multiline: true,
            ..Default::default()
        };
        edit.replace(None, "um\r\n🪷\ndois");
        assert_eq!(edit.text, "um\n🪷\ndois");
        edit.move_to(3);
        edit.replace_and_mark(None, "é\n", Some(2..2));
        assert_eq!(edit.text, "um\né\n🪷\ndois");
        assert_eq!(edit.caret(), 6);
        edit.replace(None, "x\n");
        assert_eq!(edit.text, "um\nx\n🪷\ndois");
    }

    #[test]
    fn composition_replaces_the_marked_range_and_respects_utf16_selection() {
        let mut edit = SearchEdit::default();
        edit.replace_and_mark(None, "🪷a", Some(2..2));
        assert_eq!(edit.marked, Some(0..5));
        assert_eq!(edit.selection, 4..4);
        edit.replace(None, "z");
        assert_eq!(edit.text, "z");
        assert_eq!(edit.marked, None);
    }
}
