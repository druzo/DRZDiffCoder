use crate::types::LineSpan;
use drz_core::{CoreError, Document, NewlinePolicy, TextEdit};
use drz_highlight::{HighlightEdit, HighlightEngine, LanguageId};
use std::path::Path;

/// One unit of undo/redo history: the document text BEFORE an edit was
/// applied, plus the caret position that should be restored when this entry
/// is undone. The editor (View) supplies the caret, since caret lives there;
/// the VM only owns the document state. Stored as a full string snapshot
/// (not a diff) so undo/redo are O(1) per step at the cost of memory.
/// MVP granularity: each `edit()` call is one undo step (no typing-burst
/// coalescing).
#[derive(Debug, Clone)]
struct UndoEntry {
    text: String,
    caret: Option<(usize, usize)>,
}

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
    undo_stack: Vec<UndoEntry>,
    redo_stack: Vec<UndoEntry>,
    /// Set while an undo or redo playback is in progress so the synthetic
    /// internal edits don't push onto their own stacks.
    in_history_playback: bool,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            in_history_playback: false,
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
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            in_history_playback: false,
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
        self.edit_with_caret(start_byte, old_end_byte, text, None);
    }

    /// Variant of [`edit`] that records `caret` for undo restoration. The
    /// caret should be the position the editor held BEFORE the edit (the
    /// position the user will return to on undo). `caret` may be `None` when
    /// the caller doesn't have a caret (merge chunks, replace_lines), in
    /// which case undo restores to the byte after the replaced range.
    pub fn edit_with_caret(
        &mut self,
        start_byte: usize,
        old_end_byte: usize,
        text: &str,
        caret: Option<(usize, usize)>,
    ) {
        // Snapshot the pre-edit rope + caret so undo can restore this state.
        // History playback (undo/redo) calls edit_with_caret with
        // in_history_playback=true so we don't push the synthetic edit onto
        // its own stack.
        if !self.in_history_playback {
            self.undo_stack.push(UndoEntry {
                text: self.doc.to_string(),
                caret,
            });
            // Any new user edit invalidates the redo stack.
            self.redo_stack.clear();
        }
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

    /// Restore the most recent edit, if any. Returns the caret position the
    /// editor should re-display, or `None` if the undo stack was empty.
    /// `current_caret` is the caret the editor currently shows — it gets
    /// pushed onto the redo stack so a subsequent redo can restore the
    /// post-undo caret. Pass `None` if the editor has no caret.
    /// Pushes the current state onto the redo stack.
    pub fn undo(&mut self, current_caret: Option<(usize, usize)>) -> Option<(usize, usize)> {
        let entry = self.undo_stack.pop()?;
        let current_text = self.doc.to_string();
        // The redo entry stores the state BEFORE the undo (i.e. the state
        // we're restoring back to). We attach the editor's current caret
        // so a subsequent redo can place the caret at the post-edit
        // position the user last saw.
        self.redo_stack.push(UndoEntry {
            text: current_text,
            caret: current_caret,
        });
        self.replace_document_text(&entry.text);
        entry.caret
    }

    /// Replay the most recently undone edit, if any. Returns the caret
    /// position the editor should re-display, or `None` if the redo stack
    /// was empty. `current_caret` is the caret the editor currently shows
    /// (post-undo); it gets pushed onto the undo stack so a subsequent
    /// undo restores the caret to where the user was before the redo.
    pub fn redo(&mut self, current_caret: Option<(usize, usize)>) -> Option<(usize, usize)> {
        let entry = self.redo_stack.pop()?;
        let current_text = self.doc.to_string();
        self.undo_stack.push(UndoEntry {
            text: current_text,
            caret: current_caret,
        });
        self.replace_document_text(&entry.text);
        entry.caret
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Drop both stacks. Call after `Document::open` or other operations
    /// that semantically replace the document (e.g. swap_sides). Undo/redo
    /// across a swap would otherwise restore stale rope content.
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Replace the entire document text in one shot. Used by undo/redo to
    /// restore a snapshot. Goes through `edit_with_caret` with the playback
    /// flag so the synthetic edit doesn't push onto its own stack, and so
    /// the highlight engine receives exactly one InputEdit per undo/redo
    /// step (preserving the AGENTS.md invariant).
    fn replace_document_text(&mut self, new_text: &str) {
        let old_len = self.doc.rope().len_bytes();
        self.in_history_playback = true;
        self.edit_with_caret(0, old_len, new_text, None);
        self.in_history_playback = false;
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
    /// last inserted byte (line = `start.0 + count('\n')`, col = `start.1` +
    /// byte length of the trailing line segment).
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

    #[test]
    fn handle_keys_backspace_with_selection_uses_ordered_range() {
        // Regression: handle_keys' Backspace-with-selection branch must take
        // the FULL ordered range from Selection::ordered(), not (s, s).
        // If it passes (s, s) the range is empty and the selected text is
        // not deleted.
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        // Simulate a selection covering bytes 1..4 ("ell") of "hello".
        let sel = Selection::new((0, 1), (0, 4));
        let (s, e) = sel.ordered();
        let (nl, nc) = vm.replace_selection_with(s, e, "");
        assert_eq!(vm.line(0), "ho");
        assert_eq!((nl, nc), (0, 1));
    }

    #[test]
    fn handle_keys_paste_with_selection_uses_ordered_range() {
        // Regression: handle_keys' Paste-with-selection branch must pass
        // the FULL ordered range to replace_selection_with, not (s, s).
        // Passing (s, s) would insert clipboard text without removing the
        // selection, yielding a duplicated region.
        let mut vm = EditorViewModel::from_text("hello world\n", LanguageId::PlainText);
        // Selection over "world" (bytes 6..11); clipboard contains "Rust".
        let sel = Selection::new((0, 6), (0, 11));
        let (s, e) = sel.ordered();
        let (nl, nc) = vm.replace_selection_with(s, e, "Rust");
        assert_eq!(vm.line(0), "hello Rust");
        assert_eq!((nl, nc), (0, 10));
    }

    #[test]
    fn replace_selection_with_triple_click_full_line_keeps_trailing_newline() {
        // Regression for I3 (triple-click off-by-one): the editor's
        // triple-click handler builds the selection as
        // [(line, 0), (line, line_byte_len)) where line_byte_len comes
        // from `Document::line_byte_range` (which EXCLUDES the trailing
        // newline). The endpoint must not reach into the trailing `\n`,
        // so deleting the range leaves the prior newline intact.
        let mut vm = EditorViewModel::from_text("alpha\nbeta\n", LanguageId::PlainText);
        // Lines: 0="alpha", 1="beta", 2="" (empty trailing from ropey).
        let target = 1; // the "beta" line — has a trailing newline to protect
        let (ls, le) = vm.line_byte_range(target);
        let content_len = le - ls;
        assert_eq!(content_len, 4);
        assert_eq!(ls, 6);
        let before = vm.edit_seq();
        let (nl, nc) = vm.replace_selection_with((target, 0), (target, content_len), "");
        assert_eq!(vm.line(0), "alpha");
        // "beta" replaced with empty; the \n before it stays.
        assert_eq!(vm.line(target), "");
        assert_eq!(vm.document_text(), "alpha\n\n");
        assert_eq!((nl, nc), (target, 0));
        assert_eq!(vm.edit_seq(), before + 1);
    }

    #[test]
    fn replace_selection_with_triple_click_last_line_no_trailing_newline() {
        // Same invariant as above, but for a doc without a trailing newline.
        let mut vm = EditorViewModel::from_text("alpha\nbeta", LanguageId::PlainText);
        let last = 1; // line 0 = "alpha", line 1 = "beta" (no \n after).
        let (ls, le) = vm.line_byte_range(last);
        let content_len = le - ls;
        let (nl, nc) = vm.replace_selection_with((last, 0), (last, content_len), "");
        assert_eq!(vm.document_text(), "alpha\n");
        assert_eq!((nl, nc), (last, 0));
    }

    // -------------------------------------------------------------------
    // Undo / redo
    // -------------------------------------------------------------------

    #[test]
    fn undo_on_empty_stack_is_noop() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        assert!(!vm.can_undo());
        assert!(vm.undo(None).is_none());
        assert_eq!(vm.document_text(), "hello\n");
        assert_eq!(vm.edit_seq(), 0);
    }

    #[test]
    fn undo_restores_previous_text() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        vm.edit_with_caret(5, 5, " world", Some((0, 5)));
        assert_eq!(vm.document_text(), "hello world\n");
        assert!(vm.can_undo());
        let caret = vm.undo(None);
        assert_eq!(caret, Some((0, 5)));
        assert_eq!(vm.document_text(), "hello\n");
        assert!(!vm.can_undo());
    }

    #[test]
    fn redo_replays_undone_edit() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        vm.edit_with_caret(5, 5, " world", Some((0, 5)));
        // post-edit caret lives at the end of the inserted text. The editor
        // passes this into undo so redo can restore it.
        let _ = vm.undo(Some((0, 11)));
        assert!(vm.can_redo());
        let caret = vm.redo(None);
        assert_eq!(vm.document_text(), "hello world\n");
        // Redo returns the pre-redo caret that the editor passed to undo.
        assert_eq!(caret, Some((0, 11)));
        assert!(!vm.can_redo());
    }

    #[test]
    fn redo_restores_post_edit_caret() {
        // Type 'x' at col 5 of "hello\n" → doc becomes "hellox\n", caret
        // moves to col 6. Undo restores caret to (0,5). Redo must put caret
        // back at (0,6) (the post-edit position the user saw).
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        vm.edit_with_caret(5, 5, "x", Some((0, 5)));
        // After edit, editor cursor is at (0, 6).
        let caret_after_undo = vm.undo(Some((0, 6)));
        assert_eq!(caret_after_undo, Some((0, 5)));
        let caret_after_redo = vm.redo(Some((0, 5)));
        assert_eq!(caret_after_redo, Some((0, 6)));
    }

    #[test]
    fn new_edit_clears_redo_stack() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        vm.edit(5, 5, " world");
        let _ = vm.undo(None);
        assert!(vm.can_redo());
        // A fresh user edit must invalidate redo history.
        vm.edit(0, 0, ">> ");
        assert!(!vm.can_redo());
    }

    #[test]
    fn undo_redo_undo_round_trip() {
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        // After open, doc is "hello\n" (6 bytes). Insert "world\n" at byte 6
        // → "hello\nworld\n" (12 bytes). Then insert "!" at byte 11 (between
        // 'd' and the trailing \n) → "hello\nworld!\n" (13 bytes).
        vm.edit_with_caret(6, 6, "world\n", Some((1, 5)));
        assert_eq!(vm.document_text(), "hello\nworld\n");
        vm.edit_with_caret(11, 11, "!", Some((1, 6)));
        assert_eq!(vm.document_text(), "hello\nworld!\n");
        let _ = vm.undo(None);
        assert_eq!(vm.document_text(), "hello\nworld\n");
        let _ = vm.undo(None);
        assert_eq!(vm.document_text(), "hello\n");
        let _ = vm.redo(None);
        assert_eq!(vm.document_text(), "hello\nworld\n");
        let _ = vm.redo(None);
        assert_eq!(vm.document_text(), "hello\nworld!\n");
    }

    #[test]
    fn undo_restores_caret_to_pre_edit_position() {
        // Verifies caret round-trip: the caret passed to edit_with_caret is
        // what undo() returns, so the View layer can restore cursor display.
        let mut vm = EditorViewModel::from_text("hello\n", LanguageId::PlainText);
        vm.edit_with_caret(5, 5, " world", Some((0, 11)));
        let caret = vm.undo(None).expect("undo returns caret");
        assert_eq!(caret, (0, 11));
    }

    #[test]
    fn undo_restores_highlight_in_sync() {
        // Same invariant as rust_edit_keeps_highlight_in_sync, but routing
        // through undo. The highlight engine must still produce the same
        // spans after a round trip.
        let mut vm = EditorViewModel::from_text("fn main() {}\n", LanguageId::Rust);
        vm.edit_with_caret(12, 12, " // x", Some((0, 12)));
        // Pre-undo state has the comment.
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
        let _ = vm.undo(None);
        // Post-undo state has no comment, no Comment span.
        assert_eq!(vm.line(0), "fn main() {}");
        let (_, spans) = vm.styled_line(0);
        assert!(!spans.iter().any(|s| s.style == Style::Comment));
        let _ = vm.redo(None);
        let (_, spans) = vm.styled_line(0);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }

    #[test]
    fn undo_emits_exactly_one_edit_seq_per_step() {
        // Each edit() call bumps edit_seq by 1; each undo/redo is also one
        // edit() under the hood. This test pins the contract: undo/redo are
        // observable to the diff view as ordinary edits, so the background
        // diff picks them up correctly.
        let mut vm = EditorViewModel::from_text("a\n", LanguageId::PlainText);
        let baseline = vm.edit_seq();
        vm.edit(1, 1, "b");
        assert_eq!(vm.edit_seq(), baseline + 1);
        let _ = vm.undo(None);
        assert_eq!(vm.edit_seq(), baseline + 2);
        let _ = vm.redo(None);
        assert_eq!(vm.edit_seq(), baseline + 3);
    }

    #[test]
    fn clear_history_drops_both_stacks() {
        let mut vm = EditorViewModel::from_text("a\n", LanguageId::PlainText);
        vm.edit(1, 1, "b");
        let _ = vm.undo(None);
        assert!(!vm.can_undo());
        assert!(vm.can_redo());
        vm.edit(0, 0, ">> ");
        assert!(vm.can_undo());
        vm.clear_history();
        assert!(!vm.can_undo());
        assert!(!vm.can_redo());
    }

    #[test]
    fn edit_without_caret_snapshots_null_caret() {
        // Plain edit() (no caret) must still produce an undoable state.
        // The undo caret will be None — caller responsibility.
        let mut vm = EditorViewModel::from_text("a\n", LanguageId::PlainText);
        vm.edit(1, 1, "b");
        let caret = vm.undo(None);
        assert_eq!(caret, None);
        assert_eq!(vm.document_text(), "a\n");
    }
}
