use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    Python,
    JavaScript,
    C,
    Cpp,
    Java,
    CSharp,
    Sql,
    R,
    Pascal,
    Go,
    Assembly,
    Php,
    Kotlin,
    Dart,
    Lua,
    Julia,
    Lisp,
    Scala,
    ObjectiveC,
    Swift,
    Json,
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
            "java" => LanguageId::Java,
            "cs" | "csx" => LanguageId::CSharp,
            "sql" => LanguageId::Sql,
            "r" | "R" => LanguageId::R,
            "pas" | "pp" | "dpr" | "dpk" => LanguageId::Pascal,
            "go" => LanguageId::Go,
            "asm" | "s" | "S" => LanguageId::Assembly,
            "php" | "phtml" | "php5" => LanguageId::Php,
            "kt" | "kts" => LanguageId::Kotlin,
            "dart" => LanguageId::Dart,
            "lua" => LanguageId::Lua,
            "jl" => LanguageId::Julia,
            "lisp" | "cl" | "clj" | "scm" | "el" => LanguageId::Lisp,
            "scala" | "sc" => LanguageId::Scala,
            "m" | "mm" => LanguageId::ObjectiveC,
            "swift" => LanguageId::Swift,
            "json" | "jsonc" | "json5" => LanguageId::Json,
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
            LanguageId::Java => "Java",
            LanguageId::CSharp => "C#",
            LanguageId::Sql => "SQL",
            LanguageId::R => "R",
            LanguageId::Pascal => "Delphi/Object Pascal",
            LanguageId::Go => "Go",
            LanguageId::Assembly => "Assembly",
            LanguageId::Php => "PHP",
            LanguageId::Kotlin => "Kotlin",
            LanguageId::Dart => "Dart",
            LanguageId::Lua => "Lua",
            LanguageId::Julia => "Julia",
            LanguageId::Lisp => "Lisp",
            LanguageId::Scala => "Scala",
            LanguageId::ObjectiveC => "Objective-C",
            LanguageId::Swift => "Swift",
            LanguageId::Json => "JSON",
            LanguageId::PlainText => "Text",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_existing_languages() {
        assert_eq!(LanguageId::from_path(Path::new("a.rs")), LanguageId::Rust);
        assert_eq!(LanguageId::from_path(Path::new("a.py")), LanguageId::Python);
        assert_eq!(
            LanguageId::from_path(Path::new("a.js")),
            LanguageId::JavaScript
        );
        assert_eq!(LanguageId::from_path(Path::new("a.c")), LanguageId::C);
        assert_eq!(LanguageId::from_path(Path::new("a.h")), LanguageId::C);
        assert_eq!(LanguageId::from_path(Path::new("a.cpp")), LanguageId::Cpp);
        assert_eq!(
            LanguageId::from_path(Path::new("Makefile")),
            LanguageId::PlainText
        );
        assert_eq!(
            LanguageId::from_path(Path::new("a.xyz")),
            LanguageId::PlainText
        );
    }

    #[test]
    fn detects_new_languages() {
        assert_eq!(LanguageId::from_path(Path::new("a.java")), LanguageId::Java);
        assert_eq!(LanguageId::from_path(Path::new("a.cs")), LanguageId::CSharp);
        assert_eq!(
            LanguageId::from_path(Path::new("a.csx")),
            LanguageId::CSharp
        );
        assert_eq!(LanguageId::from_path(Path::new("a.sql")), LanguageId::Sql);
        assert_eq!(LanguageId::from_path(Path::new("script.r")), LanguageId::R);
        assert_eq!(LanguageId::from_path(Path::new("plot.R")), LanguageId::R);
        assert_eq!(
            LanguageId::from_path(Path::new("a.pas")),
            LanguageId::Pascal
        );
        assert_eq!(LanguageId::from_path(Path::new("a.pp")), LanguageId::Pascal);
        assert_eq!(
            LanguageId::from_path(Path::new("main.dpr")),
            LanguageId::Pascal
        );
        assert_eq!(LanguageId::from_path(Path::new("main.go")), LanguageId::Go);
        assert_eq!(
            LanguageId::from_path(Path::new("a.asm")),
            LanguageId::Assembly
        );
        assert_eq!(
            LanguageId::from_path(Path::new("a.s")),
            LanguageId::Assembly
        );
        assert_eq!(LanguageId::from_path(Path::new("a.php")), LanguageId::Php);
        assert_eq!(LanguageId::from_path(Path::new("a.kt")), LanguageId::Kotlin);
        assert_eq!(
            LanguageId::from_path(Path::new("a.kts")),
            LanguageId::Kotlin
        );
        assert_eq!(LanguageId::from_path(Path::new("a.dart")), LanguageId::Dart);
        assert_eq!(LanguageId::from_path(Path::new("a.lua")), LanguageId::Lua);
        assert_eq!(LanguageId::from_path(Path::new("a.jl")), LanguageId::Julia);
        assert_eq!(LanguageId::from_path(Path::new("a.lisp")), LanguageId::Lisp);
        assert_eq!(LanguageId::from_path(Path::new("a.cl")), LanguageId::Lisp);
        assert_eq!(LanguageId::from_path(Path::new("a.clj")), LanguageId::Lisp);
        assert_eq!(
            LanguageId::from_path(Path::new("a.scala")),
            LanguageId::Scala
        );
        assert_eq!(
            LanguageId::from_path(Path::new("Foo.m")),
            LanguageId::ObjectiveC
        );
        assert_eq!(
            LanguageId::from_path(Path::new("a.swift")),
            LanguageId::Swift
        );
        assert_eq!(LanguageId::from_path(Path::new("a.json")), LanguageId::Json);
        assert_eq!(
            LanguageId::from_path(Path::new("a.jsonc")),
            LanguageId::Json
        );
    }

    #[test]
    fn label_nonempty_for_all() {
        let langs = [
            LanguageId::Rust,
            LanguageId::Python,
            LanguageId::JavaScript,
            LanguageId::C,
            LanguageId::Cpp,
            LanguageId::Java,
            LanguageId::CSharp,
            LanguageId::Sql,
            LanguageId::R,
            LanguageId::Pascal,
            LanguageId::Go,
            LanguageId::Assembly,
            LanguageId::Php,
            LanguageId::Kotlin,
            LanguageId::Dart,
            LanguageId::Lua,
            LanguageId::Julia,
            LanguageId::Lisp,
            LanguageId::Scala,
            LanguageId::ObjectiveC,
            LanguageId::Swift,
            LanguageId::Json,
            LanguageId::PlainText,
        ];
        for l in langs {
            assert!(!l.label().is_empty());
        }
    }
}
