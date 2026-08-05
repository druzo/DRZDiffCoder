use crate::error::HighlightError;
use crate::language::LanguageId;
use crate::style::{Style, StyledSpan};
use ropey::Rope;
use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

pub struct HighlightEngine {
    parser: Parser,
    tree: Option<Tree>,
    query: Query,
    // Retained for Task 8 incremental edits (re-parse with edited language state).
    #[allow(dead_code)]
    language: Language,
    capture_styles: Vec<Style>,
}

fn language_and_query(lang: LanguageId) -> Option<(Language, &'static str)> {
    let pair = match lang {
        LanguageId::Rust => (
            Language::new(tree_sitter_rust::LANGUAGE),
            include_str!("../queries/rust.scm"),
        ),
        LanguageId::Python => (
            Language::new(tree_sitter_python::LANGUAGE),
            include_str!("../queries/python.scm"),
        ),
        LanguageId::JavaScript => (
            Language::new(tree_sitter_javascript::LANGUAGE),
            include_str!("../queries/javascript.scm"),
        ),
        LanguageId::C => (
            Language::new(tree_sitter_c::LANGUAGE),
            include_str!("../queries/c.scm"),
        ),
        LanguageId::Cpp => (
            Language::new(tree_sitter_cpp::LANGUAGE),
            include_str!("../queries/cpp.scm"),
        ),
        LanguageId::PlainText => return None,
    };
    Some(pair)
}

fn style_for_capture(name: &str) -> Style {
    match name {
        "keyword" => Style::Keyword,
        "string" => Style::StringLit,
        "comment" => Style::Comment,
        "function" => Style::Function,
        "type" => Style::Type,
        "number" => Style::Number,
        "constant" => Style::Constant,
        _ => Style::Default,
    }
}

impl HighlightEngine {
    pub fn new(lang: LanguageId) -> Result<Option<HighlightEngine>, HighlightError> {
        let Some((language, query_src)) = language_and_query(lang) else {
            return Ok(None);
        };
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| HighlightError::UnsupportedLanguage)?;
        let query = Query::new(&language, query_src)
            .map_err(|e| HighlightError::QueryFailed(e.to_string()))?;
        let capture_styles = query
            .capture_names()
            .iter()
            .map(|n| style_for_capture(n))
            .collect();
        Ok(Some(HighlightEngine {
            parser,
            tree: None,
            query,
            language,
            capture_styles,
        }))
    }

    fn parse_with_rope(&mut self, rope: &Rope, old: Option<&Tree>) -> Result<(), HighlightError> {
        let text = rope.to_string();
        let tree = self
            .parser
            .parse(text.as_bytes(), old)
            .ok_or(HighlightError::ParseFailed)?;
        self.tree = Some(tree);
        Ok(())
    }

    pub fn parse_full(&mut self, rope: &Rope) -> Result<(), HighlightError> {
        self.parse_with_rope(rope, None)
    }

    pub fn highlight_line(&self, rope: &Rope, line_idx: usize) -> Vec<StyledSpan> {
        let Some(tree) = &self.tree else {
            return Vec::new();
        };
        if line_idx >= rope.len_lines() {
            return Vec::new();
        }
        let line_start = rope.line_to_byte(line_idx);
        let line_end = if line_idx + 1 < rope.len_lines() {
            rope.line_to_byte(line_idx + 1)
        } else {
            rope.len_bytes()
        };
        let mut cursor = QueryCursor::new();
        cursor.set_byte_range(line_start..line_end);
        let text = rope.to_string();
        let mut spans = Vec::new();
        let mut matches = cursor.matches(&self.query, tree.root_node(), text.as_bytes());
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let node = cap.node;
                let style = self.capture_styles[cap.index as usize];
                let start = node.start_byte().max(line_start) - line_start;
                let end = node.end_byte().min(line_end) - line_start;
                if start < end {
                    spans.push(StyledSpan { start, end, style });
                }
            }
        }
        spans.sort_by_key(|s| (s.start, s.end));
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::LanguageId;
    use ropey::Rope;

    #[test]
    fn plaintext_returns_none() {
        assert!(HighlightEngine::new(LanguageId::PlainText).unwrap().is_none());
    }

    #[test]
    fn rust_keyword_and_string_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
        let rope = Rope::from_str("fn main() { let s = \"hi\"; }\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        // "fn" at bytes 0..2 → Keyword
        assert!(spans.iter().any(|s| s.start == 0 && s.end == 2 && s.style == Style::Keyword));
        // "hi" string literal → StringLit covering bytes 20..24 (includes quotes)
        assert!(spans.iter().any(|s| s.style == Style::StringLit && s.start <= 21 && s.end >= 24));
    }

    #[test]
    fn python_comment_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Python).unwrap().unwrap();
        let rope = Rope::from_str("# hello\nx = 1\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        assert!(spans.iter().any(|s| s.style == Style::Comment && s.start == 0));
    }
}
