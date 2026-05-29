pub mod lang;
pub mod symbols;

pub use lang::Language;
pub use symbols::FuncInfo;

use anyhow::{anyhow, Result};
use tree_sitter::{Parser, Tree};

/// Parse a source string with the grammar for `lang`.
pub fn parse(src: &str, lang: Language) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&lang.ts_language())
        .map_err(|e| anyhow!("failed to load {} grammar: {e}", lang.code()))?;
    parser
        .parse(src, None)
        .ok_or_else(|| anyhow!("failed to parse {} source", lang.code()))
}

/// Extract the named functions/methods from a source string. Empty source
/// (e.g. an added or deleted file) yields an empty list.
pub fn functions(src: &str, lang: Language) -> Result<Vec<FuncInfo>> {
    if src.trim().is_empty() {
        return Ok(Vec::new());
    }
    let tree = parse(src, lang)?;
    Ok(symbols::collect_functions(tree.root_node(), src, lang))
}

/// Whole-file canonical token stream (comments/whitespace removed).
pub fn file_tokens(src: &str, lang: Language) -> Result<String> {
    if src.trim().is_empty() {
        return Ok(String::new());
    }
    let tree = parse(src, lang)?;
    Ok(symbols::normalized_tokens(tree.root_node(), src))
}
