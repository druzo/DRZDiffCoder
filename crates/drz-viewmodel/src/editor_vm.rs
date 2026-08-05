use crate::types::LineSpan;
use drz_core::{CoreError, Document, NewlinePolicy, TextEdit};
use drz_highlight::{HighlightEdit, HighlightEngine, LanguageId};
use std::path::Path;

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
}
