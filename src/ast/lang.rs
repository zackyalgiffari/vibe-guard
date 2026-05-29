use tree_sitter::Language as TsLanguage;

/// A source language supported by the AST engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
    Python,
    Go,
}

impl Language {
    /// Infer the language from a file path's extension.
    pub fn from_path(path: &str) -> Option<Language> {
        let ext = path.rsplit('.').next()?.to_ascii_lowercase();
        Some(match ext.as_str() {
            "rs" => Language::Rust,
            "ts" | "mts" | "cts" => Language::TypeScript,
            "tsx" => Language::Tsx,
            "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            _ => return None,
        })
    }

    /// Short code used by the `--lang` filter and reports.
    pub fn code(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "ts",
            Language::Tsx => "tsx",
            Language::JavaScript => "js",
            Language::Python => "py",
            Language::Go => "go",
        }
    }

    /// Parse a `--lang` filter token into the language(s) it selects.
    pub fn matches_filter(self, token: &str) -> bool {
        let t = token.trim().to_ascii_lowercase();
        match t.as_str() {
            "rust" | "rs" => self == Language::Rust,
            "ts" | "typescript" => self == Language::TypeScript || self == Language::Tsx,
            "js" | "javascript" => self == Language::JavaScript,
            "py" | "python" => self == Language::Python,
            "go" | "golang" => self == Language::Go,
            _ => false,
        }
    }

    /// The tree-sitter grammar for this language.
    ///
    /// Plain JavaScript is parsed with the TSX grammar, which is a superset that
    /// accepts JS + JSX; this avoids pulling a separate `tree-sitter-javascript`
    /// crate while still producing correct ASTs for JS sources.
    pub fn ts_language(self) -> TsLanguage {
        match self {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::Tsx | Language::JavaScript => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
            Language::Go => tree_sitter_go::LANGUAGE.into(),
        }
    }

    /// Node kinds that represent a named, body-bearing function/method.
    pub fn func_kinds(self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["function_item"],
            Language::TypeScript | Language::Tsx | Language::JavaScript => &[
                "function_declaration",
                "generator_function_declaration",
                "method_definition",
            ],
            Language::Python => &["function_definition"],
            Language::Go => &["function_declaration", "method_declaration"],
        }
    }
}
