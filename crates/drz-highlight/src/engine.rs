use crate::error::HighlightError;
use crate::language::LanguageId;
use crate::style::{Style, StyledSpan};
use ropey::Rope;
use tree_sitter::{
    InputEdit, Language, Parser, Point, Query, QueryCursor, StreamingIterator, Tree,
};

#[derive(Debug, Clone, Copy)]
pub struct HighlightEdit {
    pub start_byte: usize,
    pub old_end_byte: usize,
    pub new_end_byte: usize,
    pub start_point: (usize, usize),
    pub old_end_point: (usize, usize),
    pub new_end_point: (usize, usize),
}

impl HighlightEdit {
    pub fn from_rope_edit(
        rope_before: &Rope,
        start_byte: usize,
        old_end_byte: usize,
        inserted: &str,
    ) -> HighlightEdit {
        let point_of = |byte: usize| -> (usize, usize) {
            let row = rope_before.byte_to_line(byte.min(rope_before.len_bytes()));
            let row_start = rope_before.line_to_byte(row);
            (row, byte - row_start)
        };
        let start_point = point_of(start_byte);
        let old_end_point = point_of(old_end_byte);
        // new_end_point: walk inserted text from start_point
        let mut row = start_point.0;
        let mut col = start_point.1;
        for ch in inserted.chars() {
            if ch == '\n' {
                row += 1;
                col = 0;
            } else {
                col += ch.len_utf8();
            }
        }
        HighlightEdit {
            start_byte,
            old_end_byte,
            new_end_byte: start_byte + inserted.len(),
            start_point,
            old_end_point,
            new_end_point: (row, col),
        }
    }
}

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
        LanguageId::Java => (
            Language::new(tree_sitter_java::LANGUAGE),
            include_str!("../queries/java.scm"),
        ),
        LanguageId::CSharp => (
            Language::new(tree_sitter_c_sharp::LANGUAGE),
            include_str!("../queries/csharp.scm"),
        ),
        LanguageId::Sql => (
            tree_sitter_sequel::LANGUAGE.into(),
            include_str!("../queries/sql.scm"),
        ),
        LanguageId::R => (
            Language::new(tree_sitter_r::LANGUAGE),
            include_str!("../queries/r.scm"),
        ),
        LanguageId::Pascal => (
            Language::new(tree_sitter_pascal::LANGUAGE),
            include_str!("../queries/pascal.scm"),
        ),
        LanguageId::Go => (
            Language::new(tree_sitter_go::LANGUAGE),
            include_str!("../queries/go.scm"),
        ),
        LanguageId::Assembly => (
            Language::new(tree_sitter_asm::LANGUAGE),
            include_str!("../queries/asm.scm"),
        ),
        LanguageId::Php => (
            tree_sitter_php::LANGUAGE_PHP.into(),
            include_str!("../queries/php.scm"),
        ),
        LanguageId::Kotlin => (
            tree_sitter_kotlin_ng::LANGUAGE.into(),
            include_str!("../queries/kotlin.scm"),
        ),
        LanguageId::Dart => (
            Language::new(tree_sitter_dart::LANGUAGE),
            include_str!("../queries/dart.scm"),
        ),
        LanguageId::Lua => (
            Language::new(tree_sitter_lua::LANGUAGE),
            include_str!("../queries/lua.scm"),
        ),
        LanguageId::Julia => (
            Language::new(tree_sitter_julia::LANGUAGE),
            include_str!("../queries/julia.scm"),
        ),
        LanguageId::Lisp => (
            tree_sitter_commonlisp::LANGUAGE_COMMONLISP.into(),
            include_str!("../queries/lisp.scm"),
        ),
        LanguageId::Scala => (
            Language::new(tree_sitter_scala::LANGUAGE),
            include_str!("../queries/scala.scm"),
        ),
        LanguageId::ObjectiveC => (
            Language::new(tree_sitter_objc::LANGUAGE),
            include_str!("../queries/objc.scm"),
        ),
        LanguageId::Swift => (
            Language::new(tree_sitter_swift::LANGUAGE),
            include_str!("../queries/swift.scm"),
        ),
        LanguageId::Json => (
            Language::new(tree_sitter_json::LANGUAGE),
            include_str!("../queries/json.scm"),
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
        // Fall back to an empty query when the curated highlights query has
        // bad node references (some grammars drift between versions). The
        // language is still detected and rendered; only coloring degrades.
        let query = match Query::new(&language, query_src) {
            Ok(q) => q,
            Err(_) => {
                Query::new(&language, "").map_err(|e| HighlightError::QueryFailed(e.to_string()))?
            }
        };
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

    pub fn apply_edit(&mut self, edit: &HighlightEdit, rope: &Rope) -> Result<(), HighlightError> {
        if let Some(tree) = &mut self.tree {
            tree.edit(&InputEdit {
                start_byte: edit.start_byte,
                old_end_byte: edit.old_end_byte,
                new_end_byte: edit.new_end_byte,
                start_position: Point::new(edit.start_point.0, edit.start_point.1),
                old_end_position: Point::new(edit.old_end_point.0, edit.old_end_point.1),
                new_end_position: Point::new(edit.new_end_point.0, edit.new_end_point.1),
            });
        }
        let old = self.tree.take();
        // NOTE(perf): `rope.to_string()` per keystroke is a v1 simplification;
        // reparse itself stays incremental (old tree passed). Future task may
        // switch to `parse_with` rope-chunk callback if profiling demands.
        let text = rope.to_string();
        let tree = self
            .parser
            .parse(text.as_bytes(), old.as_ref())
            .ok_or(HighlightError::ParseFailed)?;
        self.tree = Some(tree);
        Ok(())
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
                // Captures may fall outside the queried byte range when their
                // match's root intersects it — skip those.
                if node.end_byte() <= line_start || node.start_byte() >= line_end {
                    continue;
                }
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
        assert!(HighlightEngine::new(LanguageId::PlainText)
            .unwrap()
            .is_none());
    }

    #[test]
    fn rust_keyword_and_string_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
        let rope = Rope::from_str("fn main() { let s = \"hi\"; }\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        // "fn" at bytes 0..2 → Keyword
        assert!(spans
            .iter()
            .any(|s| s.start == 0 && s.end == 2 && s.style == Style::Keyword));
        // "hi" string literal → StringLit covering bytes 20..24 (includes quotes)
        assert!(spans
            .iter()
            .any(|s| s.style == Style::StringLit && s.start <= 21 && s.end >= 24));
    }

    #[test]
    fn incremental_reparse_matches_full_reparse() {
        let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
        let before = Rope::from_str("fn main() {\n    let x = 1;\n}\n");
        eng.parse_full(&before).unwrap();

        // insert a comment line at byte position 14 (start of line 1)
        let inserted = "    // hi\n";
        let edit = HighlightEdit::from_rope_edit(&before, 14, 14, inserted);
        let mut after = before.clone();
        let char_pos = after.byte_to_char(14);
        after.insert(char_pos, inserted);

        eng.apply_edit(&edit, &after).unwrap();

        // full-reparse oracle
        let mut oracle = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
        oracle.parse_full(&after).unwrap();

        for line in 0..after.len_lines() {
            assert_eq!(
                eng.highlight_line(&after, line),
                oracle.highlight_line(&after, line),
                "line {line} mismatch"
            );
        }
    }

    #[test]
    fn highlight_after_edit_has_comment() {
        let mut eng = HighlightEngine::new(LanguageId::Rust).unwrap().unwrap();
        let before = Rope::from_str("fn main() {\n    let x = 1;\n}\n");
        eng.parse_full(&before).unwrap();
        let inserted = "    // hi\n";
        let edit = HighlightEdit::from_rope_edit(&before, 14, 14, inserted);
        let mut after = before.clone();
        let char_pos = after.byte_to_char(14);
        after.insert(char_pos, inserted);
        eng.apply_edit(&edit, &after).unwrap();
        let spans = eng.highlight_line(&after, 1);
        assert!(spans.iter().any(|s| s.style == Style::Comment));
    }

    #[test]
    fn python_comment_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Python).unwrap().unwrap();
        let rope = Rope::from_str("# hello\nx = 1\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        assert!(spans
            .iter()
            .any(|s| s.style == Style::Comment && s.start == 0));
    }

    #[test]
    fn java_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Java).unwrap().unwrap();
        let rope = Rope::from_str("class Foo { int x; }\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn java_keywords_types_comments_strings() {
        let mut eng = HighlightEngine::new(LanguageId::Java).unwrap().unwrap();
        let src = "/* doc */\npublic class Greeter {\n    public String hello() { return \"hi\"; }\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert_eq!(
            line_styles(0, "/* doc */"),
            vec![Style::Comment],
            "block comment should be styled"
        );
        assert!(
            line_styles(1, "public").contains(&Style::Keyword),
            "keyword 'public' (line 1) not styled"
        );
        assert!(
            line_styles(1, "class").contains(&Style::Keyword),
            "keyword 'class' (line 1) not styled"
        );
        assert!(
            line_styles(2, "public").contains(&Style::Keyword),
            "keyword 'public' (line 2) not styled"
        );
        assert!(
            line_styles(2, "String").contains(&Style::Type),
            "type identifier 'String' (line 2) not styled"
        );
        assert!(
            line_styles(2, "hello").contains(&Style::Function),
            "method 'hello' (line 2) not styled"
        );
        assert!(
            line_styles(2, "return").contains(&Style::Keyword),
            "keyword 'return' (line 2) not styled"
        );
        assert!(
            line_styles(2, "\"hi\"").contains(&Style::StringLit),
            "string literal '\"hi\"' (line 2) not styled"
        );
    }

    #[test]
    fn dart_keywords_types_functions_strings() {
        let mut eng = HighlightEngine::new(LanguageId::Dart).unwrap().unwrap();
        let src = "// regular comment\nclass Greeter {\n  String hello() => 'hi';\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "// regular comment")
                .contains(&Style::Comment),
            "comment should be styled"
        );
        assert!(
            line_styles(1, "class").contains(&Style::Keyword),
            "keyword 'class' not styled"
        );
        assert!(
            line_styles(1, "Greeter").contains(&Style::Type),
            "class name 'Greeter' not styled as type"
        );
        assert!(
            line_styles(2, "String").contains(&Style::Type),
            "type identifier 'String' not styled"
        );
        assert!(
            line_styles(2, "hello").contains(&Style::Function),
            "method 'hello' not styled as function"
        );
        assert!(
            line_styles(2, "'hi'").contains(&Style::StringLit),
            "string literal \"'hi'\" not styled"
        );
    }

    #[test]
    fn julia_keywords_types_functions_strings() {
        let mut eng = HighlightEngine::new(LanguageId::Julia).unwrap().unwrap();
        let src = "# comment\nfunction greet(name::String)::Int\n    return length(name) + 1\nend\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "# comment").contains(&Style::Comment),
            "julia line comment not styled"
        );
        assert!(
            line_styles(1, "function").contains(&Style::Keyword),
            "keyword 'function' not styled"
        );
        assert!(
            line_styles(1, "String").contains(&Style::Type),
            "type annotation 'String' not styled"
        );
        assert!(
            line_styles(2, "return").contains(&Style::Keyword),
            "keyword 'return' not styled"
        );
        assert!(
            line_styles(3, "end").contains(&Style::Keyword),
            "keyword 'end' (closing function) not styled"
        );
    }

    #[test]
    fn kotlin_keywords_types_functions_strings() {
        let mut eng = HighlightEngine::new(LanguageId::Kotlin).unwrap().unwrap();
        let src = "// comment\nclass Greeter(val name: String) {\n    fun hello(): String { return \"hi\" }\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let mut all_spans: Vec<(usize, usize, Style)> = Vec::new();
        for i in 0..rope.len_lines() {
            let line_start = rope.line_to_byte(i);
            for s in eng.highlight_line(&rope, i) {
                all_spans.push((line_start + s.start, line_start + s.end, s.style));
            }
        }

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "// comment").contains(&Style::Comment),
            "kotlin line comment not styled"
        );
        assert!(
            line_styles(1, "class").contains(&Style::Keyword),
            "keyword 'class' not styled"
        );
        assert!(
            line_styles(1, "Greeter").contains(&Style::Type),
            "class name 'Greeter' not styled as type"
        );
        assert!(
            line_styles(1, "String").contains(&Style::Type),
            "type 'String' not styled"
        );
        assert!(
            line_styles(2, "fun").contains(&Style::Keyword),
            "keyword 'fun' not styled"
        );
        assert!(
            line_styles(2, "hello").contains(&Style::Function),
            "method 'hello' not styled as function"
        );
        assert!(
            line_styles(2, "return").contains(&Style::Keyword),
            "keyword 'return' not styled"
        );
        assert!(
            line_styles(2, "\"hi\"").contains(&Style::StringLit),
            "string literal '\"hi\"' not styled"
        );
    }

    #[test]
    fn pascal_keywords_types_functions_strings() {
        let mut eng = HighlightEngine::new(LanguageId::Pascal).unwrap().unwrap();
        let src = "{ comment }\nprogram Greeter;\nvar x: Integer;\nbegin\n  x := 42;\nend.\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "{ comment }").contains(&Style::Comment),
            "pascal comment not styled"
        );
        assert!(
            line_styles(1, "program").contains(&Style::Keyword),
            "keyword 'program' not styled"
        );
        assert!(
            line_styles(2, "var").contains(&Style::Keyword),
            "keyword 'var' not styled"
        );
        assert!(
            line_styles(2, "Integer").contains(&Style::Type),
            "type 'Integer' not styled"
        );
        assert!(
            line_styles(3, "begin").contains(&Style::Keyword),
            "keyword 'begin' not styled"
        );
        assert!(
            line_styles(4, "42").contains(&Style::Number),
            "number '42' not styled"
        );
    }

    #[test]
    fn go_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Go).unwrap().unwrap();
        let rope = Rope::from_str("package main\nfunc hi() {}\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn swift_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Swift).unwrap().unwrap();
        let rope = Rope::from_str("func hi() { let x = 1 }\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn json_string_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Json).unwrap().unwrap();
        let rope = Rope::from_str("{\"a\": 1}\n");
        eng.parse_full(&rope).unwrap();
        let spans = eng.highlight_line(&rope, 0);
        assert!(spans.iter().any(|s| s.style == Style::StringLit));
        assert!(spans.iter().any(|s| s.style == Style::Number));
    }

    #[test]
    fn json_keys_distinct_from_values() {
        let mut eng = HighlightEngine::new(LanguageId::Json).unwrap().unwrap();
        let src = "{\n  \"name\": \"alice\",\n  \"count\": 42,\n  \"admin\": true,\n  \"tags\": null\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(1, "\"name\"").contains(&Style::Keyword),
            "object key 'name' should be styled as keyword"
        );
        assert!(
            line_styles(1, "\"alice\"").contains(&Style::StringLit),
            "string value 'alice' should be styled as string"
        );
        assert!(
            line_styles(2, "42").contains(&Style::Number),
            "number value 42 should be styled as number"
        );
        assert!(
            line_styles(3, "true").contains(&Style::Constant),
            "boolean 'true' should be styled as constant"
        );
        assert!(
            line_styles(4, "null").contains(&Style::Constant),
            "literal 'null' should be styled as constant"
        );
    }

    #[test]
    fn lua_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Lua).unwrap().unwrap();
        let rope = Rope::from_str("local function hi() return 1 end\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn lua_strings_numbers_keywords_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Lua).unwrap().unwrap();
        let src = "-- comment line\nlocal x = 42\nlocal s = \"hello\"\nlocal b = true\nlocal n = nil\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "-- comment line").contains(&Style::Comment),
            "lua line comment not styled"
        );
        assert!(
            line_styles(1, "local").contains(&Style::Keyword),
            "keyword 'local' not styled"
        );
        assert!(
            line_styles(1, "42").contains(&Style::Number),
            "number '42' not styled"
        );
        assert!(
            line_styles(2, "\"hello\"").contains(&Style::StringLit),
            "string literal not styled"
        );
        assert!(
            line_styles(3, "true").contains(&Style::Constant),
            "boolean 'true' not styled as constant"
        );
        assert!(
            line_styles(4, "nil").contains(&Style::Constant),
            "literal 'nil' not styled as constant"
        );
    }

    #[test]
    fn lua_functions_and_control_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Lua).unwrap().unwrap();
        let src = "function greet(name)\n  if name then\n    return name\n  end\nend\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "function").contains(&Style::Keyword)
                || line_styles(0, "greet").contains(&Style::Function),
            "function declaration should be styled"
        );
        assert!(
            line_styles(1, "if").contains(&Style::Keyword),
            "keyword 'if' not styled"
        );
        assert!(
            line_styles(1, "then").contains(&Style::Keyword),
            "keyword 'then' not styled"
        );
        assert!(
            line_styles(2, "return").contains(&Style::Keyword),
            "keyword 'return' not styled"
        );
        assert!(
            line_styles(3, "end").contains(&Style::Keyword),
            "keyword 'end' not styled"
        );
    }

    #[test]
    fn objc_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::ObjectiveC).unwrap().unwrap();
        let rope = Rope::from_str("@interface Foo : NSObject\n@end\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn objc_strings_numbers_keywords_styled() {
        let mut eng = HighlightEngine::new(LanguageId::ObjectiveC).unwrap().unwrap();
        let src = "// comment\n#define MAX 100\nNSString *s = @\"hello\";\nint x = 42;\nif (x > 0) return YES;\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "// comment").contains(&Style::Comment),
            "objc line comment not styled"
        );
        assert!(
            line_styles(1, "#define").contains(&Style::Keyword),
            "preprocessor 'define' not styled"
        );
        assert!(
            line_styles(2, "@\"hello\"").contains(&Style::StringLit),
            "objc string literal not styled"
        );
        assert!(
            line_styles(3, "42").contains(&Style::Number),
            "number '42' not styled"
        );
        assert!(
            line_styles(4, "if").contains(&Style::Keyword),
            "keyword 'if' not styled"
        );
        assert!(
            line_styles(4, "return").contains(&Style::Keyword),
            "keyword 'return' not styled"
        );
    }

    #[test]
    fn objc_at_directives_styled() {
        let mut eng = HighlightEngine::new(LanguageId::ObjectiveC).unwrap().unwrap();
        let src = "@interface Foo : NSObject\n@property NSString *name;\n@end\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "@interface").contains(&Style::Keyword),
            "@interface not styled"
        );
        assert!(
            line_styles(1, "@property").contains(&Style::Keyword),
            "@property not styled"
        );
        assert!(
            line_styles(2, "@end").contains(&Style::Keyword),
            "@end not styled"
        );
    }

    #[test]
    fn php_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Php).unwrap().unwrap();
        let rope = Rope::from_str("<?php function hi() { return 1; }\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

#[test]
    fn php_strings_numbers_keywords_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Php).unwrap().unwrap();
        let src = r#"<?php
// comment
$s = "hello";
$x = 42;
$b = true;
$n = null;
"#;
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(1, "// comment").contains(&Style::Comment),
            "php line comment not styled"
        );
        assert!(
            line_styles(2, "\"hello\"").contains(&Style::StringLit),
            "php string literal not styled"
        );
        assert!(
            line_styles(3, "42").contains(&Style::Number),
            "number '42' not styled"
        );
        assert!(
            line_styles(4, "true").contains(&Style::Constant),
            "boolean 'true' not styled as constant"
        );
        assert!(
            line_styles(5, "null").contains(&Style::Constant),
            "literal 'null' not styled as constant"
        );
    }

    #[test]
    fn php_functions_classes_control_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Php).unwrap().unwrap();
        let src = "<?php\nclass Foo {\n  public function bar() {\n    if (true) return 1;\n  }\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(1, "class").contains(&Style::Keyword),
            "keyword 'class' not styled"
        );
        assert!(
            line_styles(2, "function").contains(&Style::Keyword),
            "keyword 'function' not styled"
        );
        assert!(
            line_styles(2, "bar").contains(&Style::Function)
                || line_styles(2, "bar").contains(&Style::Keyword),
            "function 'bar' should be styled"
        );
        assert!(
            line_styles(3, "if").contains(&Style::Keyword),
            "keyword 'if' not styled"
        );
        assert!(
            line_styles(3, "return").contains(&Style::Keyword),
            "keyword 'return' not styled"
        );
    }

    #[test]
    fn r_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::R).unwrap().unwrap();
        let rope = Rope::from_str("x <- 1\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn r_strings_numbers_keywords_styled() {
        let mut eng = HighlightEngine::new(LanguageId::R).unwrap().unwrap();
        let src = "# comment\nx <- 42\ns <- \"hello\"\nb <- TRUE\nn <- NULL\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "# comment").contains(&Style::Comment),
            "r line comment not styled"
        );
        assert!(
            line_styles(1, "42").contains(&Style::Number),
            "number '42' not styled"
        );
        assert!(
            line_styles(2, "\"hello\"").contains(&Style::StringLit),
            "r string literal not styled"
        );
        assert!(
            line_styles(3, "TRUE").contains(&Style::Constant),
            "boolean 'TRUE' not styled as constant"
        );
        assert!(
            line_styles(4, "NULL").contains(&Style::Constant),
            "literal 'NULL' not styled as constant"
        );
    }

    #[test]
    fn r_functions_and_control_styled() {
        let mut eng = HighlightEngine::new(LanguageId::R).unwrap().unwrap();
        let src = "myfunc <- function(x) {\n  if (x > 0) {\n    y <- x * 2\n  }\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "function").contains(&Style::Keyword),
            "keyword 'function' not styled"
        );
        assert!(
            line_styles(1, "if").contains(&Style::Keyword),
            "keyword 'if' not styled"
        );
    }

    #[test]
    fn scala_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Scala).unwrap().unwrap();
        let rope = Rope::from_str("object Main extends App { def main(args: Array[String]) = {} }\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn scala_strings_numbers_keywords_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Scala).unwrap().unwrap();
        let src = "// comment\nval x = 42\nval s = \"hello\"\nval b = true\nval n = null\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "// comment").contains(&Style::Comment),
            "scala line comment not styled"
        );
        assert!(
            line_styles(1, "val").contains(&Style::Keyword),
            "keyword 'val' not styled"
        );
        assert!(
            line_styles(1, "42").contains(&Style::Number),
            "number '42' not styled"
        );
        assert!(
            line_styles(2, "\"hello\"").contains(&Style::StringLit),
            "scala string literal not styled"
        );
        assert!(
            line_styles(3, "true").contains(&Style::Constant),
            "boolean 'true' not styled as constant"
        );
    }

    #[test]
    fn scala_functions_classes_control_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Scala).unwrap().unwrap();
        let src = "class Foo {\n  def bar(): Int = {\n    if (true) return 1\n    else return 0\n  }\n}\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "class").contains(&Style::Keyword),
            "keyword 'class' not styled"
        );
        assert!(
            line_styles(1, "def").contains(&Style::Keyword),
            "keyword 'def' not styled"
        );
        assert!(
            line_styles(2, "if").contains(&Style::Keyword),
            "keyword 'if' not styled"
        );
        assert!(
            line_styles(2, "return").contains(&Style::Keyword),
            "keyword 'return' not styled"
        );
    }

    #[test]
    fn sql_engine_parses() {
        let mut eng = HighlightEngine::new(LanguageId::Sql).unwrap().unwrap();
        let rope = Rope::from_str("SELECT * FROM users;\n");
        eng.parse_full(&rope).unwrap();
        let _spans = eng.highlight_line(&rope, 0);
    }

    #[test]
    fn sql_strings_numbers_keywords_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Sql).unwrap().unwrap();
        let src = "-- comment\nSELECT id FROM users WHERE age > 18;\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "-- comment").contains(&Style::Comment),
            "sql line comment not styled"
        );
        assert!(
            line_styles(1, "SELECT").contains(&Style::Keyword),
            "keyword 'SELECT' not styled"
        );
        assert!(
            line_styles(1, "FROM").contains(&Style::Keyword),
            "keyword 'FROM' not styled"
        );
        assert!(
            line_styles(1, "WHERE").contains(&Style::Keyword),
            "keyword 'WHERE' not styled"
        );
    }

    #[test]
    fn sql_types_and_functions_styled() {
        let mut eng = HighlightEngine::new(LanguageId::Sql).unwrap().unwrap();
        let src = "CREATE TABLE users (id INT, name VARCHAR(100));\nSELECT COUNT(*) FROM users;\n";
        let rope = Rope::from_str(src);
        eng.parse_full(&rope).unwrap();

        let line_styles = |line_idx: usize, needle: &str| -> Vec<Style> {
            let line_start = rope.line_to_byte(line_idx);
            let spans = eng.highlight_line(&rope, line_idx);
            let Some(local) = src[line_start..].find(needle) else {
                return Vec::new();
            };
            let end = local + needle.len();
            spans
                .into_iter()
                .filter(|s| s.start <= local && s.end >= end)
                .map(|s| s.style)
                .collect()
        };

        assert!(
            line_styles(0, "CREATE").contains(&Style::Keyword),
            "keyword 'CREATE' not styled"
        );
        assert!(
            line_styles(0, "TABLE").contains(&Style::Keyword),
            "keyword 'TABLE' not styled"
        );
        assert!(
            line_styles(0, "INT").contains(&Style::Type),
            "type 'INT' not styled"
        );
        assert!(
            line_styles(1, "COUNT").contains(&Style::Keyword)
                || line_styles(1, "FROM").contains(&Style::Keyword),
            "sql SELECT clause should be styled"
        );
    }
}
