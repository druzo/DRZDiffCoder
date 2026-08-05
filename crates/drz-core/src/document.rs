use ropey::Rope;
use crate::edit::TextEdit;

pub struct Document {
    rope: Rope,
}

impl Document {
    pub fn from_text(text: &str) -> Document {
        Document { rope: Rope::from_str(text) }
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
    }

    pub fn to_string(&self) -> String {
        self.rope.to_string()
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
        if s.ends_with('\n') {
            len -= 1;
            if len > 0 && s[..s.len()-1].ends_with('\r') {
                len -= 1;
            }
        }
        (start, start + len)
    }

    pub fn replace_lines(&mut self, start: usize, end: usize, text: &str) {
        let start_byte = self.rope.line_to_byte(start);
        let end_byte = if end >= self.rope.len_lines() {
            self.rope.len_bytes()
        } else {
            self.rope.line_to_byte(end)
        };
        let mut inserted = text.to_string();
        // Replacing whole lines: keep the line break so following lines stay intact.
        if end_byte < self.rope.len_bytes() && !inserted.ends_with('\n') {
            inserted.push('\n');
        }
        self.apply(&TextEdit { start_byte, old_end_byte: end_byte, inserted });
    }

    pub fn rope(&self) -> &Rope {
        &self.rope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_insert_updates_text() {
        let mut doc = Document::from_text("hello\nworld\n");
        doc.apply(&TextEdit { start_byte: 0, old_end_byte: 0, inserted: ">> ".into() });
        assert_eq!(doc.to_string(), ">> hello\nworld\n");
    }

    #[test]
    fn apply_delete_updates_text() {
        let mut doc = Document::from_text("hello\nworld\n");
        doc.apply(&TextEdit { start_byte: 0, old_end_byte: 6, inserted: String::new() });
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
}
