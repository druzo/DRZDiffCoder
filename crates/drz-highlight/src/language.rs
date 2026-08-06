use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    Python,
    JavaScript,
    C,
    Cpp,
    PlainText,
}

impl LanguageId {
    pub fn from_path(path: &Path) -> LanguageId {
        match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
            "rs" => LanguageId::Rust,
            "py" | "pyi" => LanguageId::Python,
            "js" | "mjs" | "cjs" | "jsx" => LanguageId::JavaScript,
            "c" | "h" => LanguageId::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" => LanguageId::Cpp,
            _ => LanguageId::PlainText,
        }
    }

    /// Short human-readable label for UI display.
    pub fn label(self) -> &'static str {
        match self {
            LanguageId::Rust => "Rust",
            LanguageId::Python => "Python",
            LanguageId::JavaScript => "JavaScript",
            LanguageId::C => "C",
            LanguageId::Cpp => "C++",
            LanguageId::PlainText => "Text",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_by_extension() {
        assert_eq!(LanguageId::from_path(Path::new("a.rs")), LanguageId::Rust);
        assert_eq!(LanguageId::from_path(Path::new("a.py")), LanguageId::Python);
        assert_eq!(
            LanguageId::from_path(Path::new("a.js")),
            LanguageId::JavaScript
        );
        assert_eq!(LanguageId::from_path(Path::new("a.c")), LanguageId::C);
        assert_eq!(LanguageId::from_path(Path::new("a.h")), LanguageId::C);
        assert_eq!(LanguageId::from_path(Path::new("a.cpp")), LanguageId::Cpp);
        assert_eq!(LanguageId::from_path(Path::new("a.hpp")), LanguageId::Cpp);
        assert_eq!(
            LanguageId::from_path(Path::new("Makefile")),
            LanguageId::PlainText
        );
        assert_eq!(
            LanguageId::from_path(Path::new("a.xyz")),
            LanguageId::PlainText
        );
    }
}
