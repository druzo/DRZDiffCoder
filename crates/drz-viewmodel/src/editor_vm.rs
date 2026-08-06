use crate::types::LineSpan;
use drz_core::{CoreError, Document, NewlinePolicy, TextEdit};
use drz_highlight::{HighlightEdit, HighlightEngine, LanguageId};
use std::path::Path;

/// Half-open text selection in `(line, byte_col)` coordinates.
/// `anchor` is fixed (click position); `cursor` follows pointer / arrow keys.
/// Byte-col, not char-col, to match the rest of the editor's coordinate system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: (usize, usize),
    pub cursor: (usize, usize),
}

impl Selection {
    pub fn new(anchor: (usize, usize), cursor: (usize, usize)) -> Self {
        Self { anchor, cursor }
    }

    /// Return `(start, end)` with `start <= end` bytewise
    /// (`(line, col).0 * u32::MAX as usize + col` ordering). Handles both
    /// same-line and cross-line ordering.
    pub fn ordered(&self) -> ((usize, usize), (usize, usize)) {
        let a = self.anchor;
        let c = self.cursor;
        let key = |p: (usize, usize)| (p.0, p.1);
        if key(a) <= key(c) {
            (a, c)
        } else {
            (c, a)
        }
    }

    /// `true` iff the selection covers at least one byte (cursor != anchor).
    pub fn is_selected(&self) -> bool {
        self.anchor != self.cursor
    }

    /// Collapse to anchor — cursor jumps to anchor. Keeps anchor fixed so a
    /// subsequent Shift+arrow extends from the same anchor.
    pub fn collapse(&mut self) {
        self.cursor = self.anchor;
    }
}

pub struct EditorViewModel {
    doc: Document,
    engine: Option<HighlightEngine>,
    #[allow(dead_code)]
    lang: LanguageId,
    /// Mutation counter: bumped by `edit()` only, so readers (e.g. the diff
    /// view) can tell real edits apart from render-path access.
    edit_seq: u64,
}

impl EditorViewModel {
    pub fn open(path: &Path) -> Result<EditorViewModel, CoreError> {
        let doc = Document::open(path)?;
        let lang = LanguageId::from_path(path);
        let engine = HighlightEngine::new(lang).ok().flatten();
        let mut vm = EditorViewModel {
            doc,
            engine,
            lang,
            edit_seq: 0,
        };
        vm.reparse_full();
        Ok(vm)
    }

    pub fn from_text(text: &str, lang: LanguageId) -> EditorViewModel {
        let engine = HighlightEngine::new(lang).ok().flatten();
        let mut vm = EditorViewModel {
            doc: Document::from_text(text),
            engine,
            lang,
            edit_seq: 0,
        };
        vm.reparse_full();
        vm
    }

    fn reparse_full(&mut self) {
        if let Some(engine) = &mut self.engine {
            let _ = engine.parse_full(self.doc.rope());
        }
    }

    pub fn edit(&mut self, start_byte: usize, old_end_byte: usize, text: &str) {
        self.edit_seq += 1;
        let hl_edit =
            HighlightEdit::from_rope_edit(self.doc.rope(), start_byte, old_end_byte, text);
        self.doc.apply(&TextEdit {
            start_byte,
            old_end_byte,
            inserted: text.to_string(),
        });
        if let Some(engine) = &mut self.engine {
            let _ = engine.apply_edit(&hl_edit, self.doc.rope());
        }
    }

    pub fn insert_at_line_col(&mut self, line: usize, col_byte: usize, text: &str) {
        if line >= self.doc.len_lines() {
            return;
        }
        let (start, end) = self.doc.line_byte_range(line);
        let byte = (start + col_byte).min(end);
        self.edit(byte, byte, text);
    }

    pub fn delete_range_line_col(&mut self, start: (usize, usize), end: (usize, usize)) {
        let byte_of = |(line, col): (usize, usize)| -> usize {
            if line >= self.doc.len_lines() {
                return self.doc.rope().len_bytes();
            }
            let (ls, le) = self.doc.line_byte_range(line);
            (ls + col).min(le)
        };
        let s = byte_of(start);
        let e = byte_of(end).max(s);
        self.edit(s, e, "");
    }

    pub fn styled_line(&self, line_idx: usize) -> (String, Vec<LineSpan>) {
        let text = self.doc.line(line_idx);
        let spans = match &self.engine {
            Some(engine) => engine
                .highlight_line(self.doc.rope(), line_idx)
                .into_iter()
                .map(|s| LineSpan {
                    start: s.start,
                    end: s.end,
                    style: s.style,
                })
                .collect(),
            None => Vec::new(),
        };
        (text, spans)
    }

    pub fn len_lines(&self) -> usize {
        self.doc.len_lines()
    }
    pub fn line(&self, idx: usize) -> String {
        self.doc.line(idx)
    }
    pub fn line_byte_range(&self, idx: usize) -> (usize, usize) {
        self.doc.line_byte_range(idx)
    }
    pub fn path(&self) -> Option<&Path> {
        self.doc.path()
    }
    pub fn is_dirty(&self) -> bool {
        self.doc.is_dirty()
    }
    pub fn edit_seq(&self) -> u64 {
        self.edit_seq
    }
    pub fn encoding_guessed(&self) -> bool {
        self.doc.encoding_guessed()
    }
    pub fn document_text(&self) -> String {
        self.doc.to_string()
    }

    pub fn save(&mut self) -> Result<(), CoreError> {
        self.doc.save()
    }

    /// Whole-line replacement for merge: newline semantics come from
    /// `Document::line_replace_edit` (single shared code path in drz-core);
    /// `Exact` makes the target reproduce the source block byte-for-byte,
    /// including its trailing-newline state at end of document. The mutation
    /// still routes through `edit()`, so exactly one HighlightEdit reaches
    /// the engine.
    pub fn replace_lines(&mut self, start: usize, end: usize, text: &str) {
        let edit = self
            .doc
            .line_replace_edit(start, end, text, NewlinePolicy::Exact);
        self.edit(edit.start_byte, edit.old_end_byte, &edit.inserted);
    }

    /// Read the text covered by a half-open selection. `(line, byte_col)`
    /// endpoints; the second endpoint is treated as exclusive of the byte
    /// itself (matching the existing rope `delete_range_line_col` convention)
    /// — but for selection *display*, callers that want inclusive end-of-range
    /// should pass `end.1 + 1` on the same line, or the start of the next
    /// line. The convention here is: end is the cursor position after the
    /// last selected byte. So `text_in_range((0,1),(0,4))` over "hello\n"
    /// yields "ell" (cols 1,2,3; col 4 excluded).
    pub fn text_in_range(&self, start: (usize, usize), end: (usize, usize)) -> String {
        let sel = Selection::new(start, end);
        let (s, e) = sel.ordered();
        if s == e {
            return String::new();
        }
        let same_line = s.0 == e.0;
        let start_col = s.1;
        let end_col = e.1;
        let mut out = String::new();
        for line in s.0..=e.0 {
            let text = self.doc.line(line);
            let begin = if line == s.0 {
                start_col.min(text.len())
            } else {
                0
            };
            let finish = if line == e.0 {
                let raw = end_col.min(text.len());
                if !same_line && raw == 0 {
                    1.min(text.len())
                } else {
                    raw
                }
            } else {
                text.len()
            };
            if same_line {
                out.push_str(&text[begin..finish]);
                return out;
            }
            if begin < finish {
                out.push_str(&text[begin..finish]);
            }
            if line < e.0 {
                out.push('\n');
            }
        }
        out
    }

    /// Replace the byte range `[start, end)` with `new_text`. Routes through
    /// the single `edit()` entry point so exactly one `tree_sitter::InputEdit`
    /// reaches `drz-highlight` (per AGENTS.md hard rule).
    ///
    /// Returns the caret position after the insert: if `new_text` is empty,
    /// returns `start`; otherwise the caret sits at the byte just after the
    /// last inserted byte (line = `start.0 + count('\n')`, col = byte length
    /// of the trailing line segment).
    pub fn replace_selection_with(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        new_text: &str,
    ) -> (usize, usize) {
        if new_text.is_empty() && start == end {
            return start;
        }
        // Compute byte positions in the rope.
        let byte_of = |(line, col): (usize, usize)| -> usize {
            if line >= self.doc.len_lines() {
                return self.doc.rope().len_bytes();
            }
            let (ls, le) = self.doc.line_byte_range(line);
            (ls + col).min(le)
        };
        let s_byte = byte_of(start);
        let e_byte = byte_of(end).max(s_byte);
        self.edit(s_byte, e_byte, new_text);

        if new_text.is_empty() {
            return start;
        }
        // Compute new caret: count newlines, take col of trailing segment.
        let mut newlines = 0usize;
        let mut last_seg_start = 0usize;
        for (i, b) in new_text.bytes().enumerate() {
            if b == b'\n' {
                newlines += 1;
                last_seg_start = i + 1;
            }
        }
        let trailing = &new_text[last_seg_start..];
        let new_col = start.1 + trailing.len();
        let new_line = start.0 + newlines;
        (new_line, new_col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Style;
    use drz_highlight::LanguageId;

    #[test]
    fn from_text_plain_no_highlight() {
        let vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        let (text, spans) = vm.styled_line(0);
        assert_eq!(text, "hello");
        assert!(spans.is_empty());
    }

    #[test]
    fn rust_edit_keeps_highlight_in_sync() {
        let mut vm = EditorViewModel::from_text("fn main() {}\n", LanguageId::Rust);
        // insert comment after "{}" : bytes 12..12
        vm.edit(12, 12, " // x");
        assert_eq!(vm.line(0), "fn main() {} // x");
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }

    #[test]
    fn insert_at_line_col_works() {
        let mut vm = EditorViewModel::from_text("ab\ncd\n", LanguageId::PlainText);
        vm.insert_at_line_col(1, 1, "Z");
        assert_eq!(vm.line(1), "cZd");
    }

    #[test]
    fn replace_lines_updates_highlight_and_text() {
        let mut vm = EditorViewModel::from_text("let a = 1;\nlet b = 2;\n", LanguageId::Rust);
        vm.replace_lines(0, 1, "// gone");
        assert_eq!(vm.line(0), "// gone");
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }

    #[test]
    fn selection_ordered_returns_min_max_bytewise() {
        let s = Selection::new((2, 5), (1, 3));
        assert_eq!(s.ordered(), ((1, 3), (2, 5)));
    }

    #[test]
    fn selection_ordered_already_in_order() {
        let s = Selection::new((0, 1), (0, 4));
        assert_eq!(s.ordered(), ((0, 1), (0, 4)));
    }

    #[test]
    fn selection_is_selected_false_when_collapsed() {
        let s = Selection::new((2, 5), (2, 5));
        assert!(!s.is_selected());
    }

    #[test]
    fn selection_is_selected_true_when_anchor_differs() {
        let s = Selection::new((0, 0), (3, 0));
        assert!(s.is_selected());
        let s = Selection::new((0, 0), (0, 7));
        assert!(s.is_selected());
    }

    #[test]
    fn selection_collapse_moves_cursor_to_anchor() {
        let mut s = Selection::new((0, 3), (5, 0));
        s.collapse();
        assert_eq!(s.cursor, s.anchor);
        assert_eq!(s.cursor, (0, 3));
    }

    #[test]
    fn selection_new_accepts_reversed_endpoints() {
        let s = Selection::new((4, 2), (1, 5));
        assert_eq!(s.anchor, (4, 2));
        assert_eq!(s.cursor, (1, 5));
    }

    #[test]
    fn text_in_range_single_line_slice() {
        let vm = EditorViewModel::from_text("hello\nworld\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 1), (0, 4)), "ell");
    }

    #[test]
    fn text_in_range_multi_line_inclusive_end() {
        // "ab\ncd\nef" → range ((0,1),(2,1)) yields "b\ncd\ne"
        let vm = EditorViewModel::from_text("ab\ncd\nef\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 1), (2, 1)), "b\ncd\ne");
    }

    #[test]
    fn text_in_range_empty_when_start_eq_end() {
        let vm = EditorViewModel::from_text("abc\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 2), (0, 2)), "");
    }

    #[test]
    fn text_in_range_reversed_endpoints_swaps() {
        let vm = EditorViewModel::from_text("abc\ndef\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((1, 0), (0, 2)), "c\nd");
    }

    #[test]
    fn text_in_range_utf8_bytewise() {
        // "aé💣b\n" → a=1B, é=2B, 💣=4B, b=1B. bytes 1..3 == "é".
        let vm = EditorViewModel::from_text("aé💣b\n", LanguageId::PlainText);
        assert_eq!(vm.text_in_range((0, 1), (0, 3)), "é");
        assert_eq!(vm.text_in_range((0, 3), (0, 7)), "💣");
    }

    #[test]
    fn replace_selection_with_single_char_no_selection() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        let before = vm.edit_seq();
        let (nl, nc) = vm.replace_selection_with((0, 5), (0, 5), "!");
        assert_eq!(vm.line(0), "hello!");
        assert_eq!((nl, nc), (0, 6));
        assert_eq!(vm.edit_seq(), before + 1);
    }

    #[test]
    fn replace_selection_with_replaces_range_and_returns_end() {
        let mut vm = EditorViewModel::from_text("hello world\n", LanguageId::PlainText);
        let (nl, nc) = vm.replace_selection_with((0, 6), (0, 11), "Rust");
        assert_eq!(vm.line(0), "hello Rust");
        assert_eq!((nl, nc), (0, 10));
    }

    #[test]
    fn replace_selection_with_multiline_text_advances_line_and_col() {
        let mut vm = EditorViewModel::from_text("ab\ncd\n", LanguageId::PlainText);
        let (nl, nc) = vm.replace_selection_with((0, 0), (0, 0), "x\ny\nz");
        // After insert: "x\ny\nzab\ncd\n". Caret at end of "z" → line 2, col 1.
        assert_eq!(vm.line(0), "x");
        assert_eq!(vm.line(1), "y");
        assert_eq!(vm.line(2), "zab");
        assert_eq!((nl, nc), (2, 1));
    }

    #[test]
    fn replace_selection_with_empty_text_deletes_range() {
        let mut vm = EditorViewModel::from_text("hello world\n", LanguageId::PlainText);
        let (nl, nc) = vm.replace_selection_with((0, 5), (0, 11), "");
        assert_eq!(vm.line(0), "hello");
        assert_eq!((nl, nc), (0, 5));
    }

    #[test]
    fn replace_selection_with_emits_exactly_one_edit() {
        let mut vm = EditorViewModel::from_text("aaa\nbbb\n", LanguageId::PlainText);
        let before = vm.edit_seq();
        vm.replace_selection_with((0, 1), (0, 2), "Z");
        assert_eq!(
            vm.edit_seq(),
            before + 1,
            "exactly one edit() call per replace"
        );
    }

    #[test]
    fn replace_selection_with_keeps_highlight_in_sync() {
        // Same invariant as rust_edit_keeps_highlight_in_sync but routed
        // through the new method: HighlightEdit must reach the engine once.
        let mut vm = EditorViewModel::from_text("fn main() {}\n", LanguageId::Rust);
        vm.replace_selection_with((0, 12), (0, 12), " // x");
        assert_eq!(vm.line(0), "fn main() {} // x");
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }
}
