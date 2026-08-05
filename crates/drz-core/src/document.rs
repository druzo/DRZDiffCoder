use crate::edit::TextEdit;
use ropey::Rope;
use std::path::{Path, PathBuf};

/// Trailing-newline policy for whole-line replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewlinePolicy {
    /// Keep document structure: a non-empty replacement not ending in '\n'
    /// gains one when followed by more text or when the removed range ended
    /// with a line break. An empty replacement stays empty (pure deletion).
    Preserve,
    /// Use the replacement verbatim: a non-empty replacement not ending in
    /// '\n' gains one only when followed by more text. At end of document
    /// the caller's trailing-newline state is reproduced exactly (merge uses
    /// this to mirror the source block byte-for-byte).
    Exact,
}

pub struct Document {
    rope: Rope,
    path: Option<PathBuf>,
    dirty: bool,
    encoding_guessed: bool,
}

impl Document {
    pub fn from_text(text: &str) -> Document {
        Document {
            rope: Rope::from_str(text),
            path: None,
            dirty: false,
            encoding_guessed: false,
        }
    }

    pub fn from_file(text: String, path: PathBuf, encoding_guessed: bool) -> Document {
        Document {
            rope: Rope::from_str(&text),
            path: Some(path),
            dirty: false,
            encoding_guessed,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn encoding_guessed(&self) -> bool {
        self.encoding_guessed
    }
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    pub fn apply(&mut self, edit: &TextEdit) {
        let start = edit.start_byte.min(self.rope.len_bytes());
        let old_end = edit.old_end_byte.min(self.rope.len_bytes()).max(start);
        if old_end > start {
            let s = self.rope.byte_to_char(start);
            let e = self.rope.byte_to_char(old_end);
            self.rope.remove(s..e);
        }
        if !edit.inserted.is_empty() {
            let s = self.rope.byte_to_char(start);
            self.rope.insert(s, &edit.inserted);
        }
        self.dirty = true;
    }

    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line(&self, idx: usize) -> String {
        let line = self.rope.line(idx);
        let mut s = line.to_string();
        while s.ends_with('\n') || s.ends_with('\r') {
            s.pop();
        }
        s
    }

    pub fn line_byte_range(&self, idx: usize) -> (usize, usize) {
        let start = self.rope.line_to_byte(idx);
        let line = self.rope.line(idx);
        let mut len = line.len_bytes();
        // strip trailing \n / \r\n
        let s = line.as_str().unwrap_or("");
        if let Some(stripped) = s.strip_suffix('\n') {
            len -= 1;
            if len > 0 && stripped.ends_with('\r') {
                len -= 1;
            }
        }
        (start, start + len)
    }

    /// Compute the `TextEdit` for replacing whole lines `start..end` with
    /// `text`, owning the newline semantics shared by all callers: the byte
    /// range covers the lines including their terminators, and `policy`
    /// decides whether a trailing '\n' is appended to a non-empty
    /// replacement. An empty replacement is always inserted verbatim, so a
    /// pure deletion never leaves a blank line behind.
    pub fn line_replace_edit(
        &self,
        start: usize,
        end: usize,
        text: &str,
        policy: NewlinePolicy,
    ) -> TextEdit {
        let len = self.rope.len_lines();
        let start_byte = self.rope.line_to_byte(start.min(len));
        let end_byte = if end >= len {
            self.rope.len_bytes()
        } else {
            self.rope.line_to_byte(end)
        };
        let mut inserted = text.to_string();
        if !inserted.is_empty() && !inserted.ends_with('\n') {
            let append = match policy {
                NewlinePolicy::Exact => end_byte < self.rope.len_bytes(),
                NewlinePolicy::Preserve => {
                    end_byte < self.rope.len_bytes()
                        || (self.rope.len_bytes() > 0
                            && self.rope.byte(self.rope.len_bytes() - 1) == b'\n')
                }
            };
            if append {
                inserted.push('\n');
            }
        }
        TextEdit {
            start_byte,
            old_end_byte: end_byte,
            inserted,
        }
    }

    pub fn replace_lines(&mut self, start: usize, end: usize, text: &str) {
        let edit = self.line_replace_edit(start, end, text, NewlinePolicy::Preserve);
        self.apply(&edit);
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

impl std::fmt::Display for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for chunk in self.rope.chunks() {
            f.write_str(chunk)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_insert_updates_text() {
        let mut doc = Document::from_text("hello\nworld\n");
        doc.apply(&TextEdit {
            start_byte: 0,
            old_end_byte: 0,
            inserted: ">> ".into(),
        });
        assert_eq!(doc.to_string(), ">> hello\nworld\n");
    }

    #[test]
    fn apply_delete_updates_text() {
        let mut doc = Document::from_text("hello\nworld\n");
        doc.apply(&TextEdit {
            start_byte: 0,
            old_end_byte: 6,
            inserted: String::new(),
        });
        assert_eq!(doc.to_string(), "world\n");
    }

    #[test]
    fn line_accessors() {
        let doc = Document::from_text("ab\ncd\n");
        assert_eq!(doc.len_lines(), 3); // trailing newline → empty last line
        assert_eq!(doc.line(1), "cd");
        assert_eq!(doc.line_byte_range(1), (3, 5));
    }

    #[test]
    fn replace_lines_swaps_content() {
        let mut doc = Document::from_text("a\nb\nc\n");
        doc.replace_lines(1, 2, "X\nY");
        assert_eq!(doc.to_string(), "a\nX\nY\nc\n");
    }

    #[test]
    fn replace_lines_empty_deletes_without_blank_line() {
        let mut doc = Document::from_text("a\nb\nc\n");
        doc.replace_lines(1, 2, "");
        assert_eq!(doc.to_string(), "a\nc\n");
    }

    #[test]
    fn replace_lines_empty_at_eof_deletes_without_blank_line() {
        let mut doc = Document::from_text("a\nb\n");
        doc.replace_lines(1, 2, "");
        assert_eq!(doc.to_string(), "a\n");
    }

    #[test]
    fn line_replace_edit_exact_is_verbatim_at_eof() {
        let doc = Document::from_text("a\nb\n");
        let e = doc.line_replace_edit(1, 2, "b", NewlinePolicy::Exact);
        assert_eq!(e.inserted, "b");
        let e = doc.line_replace_edit(1, 2, "b", NewlinePolicy::Preserve);
        assert_eq!(e.inserted, "b\n");
    }

    #[test]
    fn line_replace_edit_exact_separates_from_following_lines() {
        let doc = Document::from_text("a\nb\nc\n");
        let e = doc.line_replace_edit(1, 2, "X", NewlinePolicy::Exact);
        assert_eq!(e.inserted, "X\n");
    }
}
