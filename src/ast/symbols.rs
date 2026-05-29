use super::lang::Language;
use tree_sitter::Node;

/// A named function/method extracted from a parse tree, reduced to the data the
/// classifier compares between the before/after versions of a file.
#[derive(Debug, Clone)]
pub struct FuncInfo {
    pub name: String,
    /// Normalized token stream of the signature (everything but the body).
    pub sig_tokens: String,
    /// Normalized token stream of the body block.
    pub body_tokens: String,
    /// 1-based line where the function starts.
    pub line: usize,
}

/// Walk a parse tree and collect every named function/method, including those
/// nested inside `impl`/`class`/module blocks.
pub fn collect_functions(root: Node, src: &str, lang: Language) -> Vec<FuncInfo> {
    let mut out = Vec::new();
    visit(root, src, lang, &mut out);
    out
}

fn visit(node: Node, src: &str, lang: Language, out: &mut Vec<FuncInfo>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if lang.func_kinds().contains(&child.kind()) {
            if let Some(name_node) = child.child_by_field_name("name") {
                let name = node_text(name_node, src).to_string();
                let body = child.child_by_field_name("body");
                let body_tokens = body.map(|b| normalized_tokens(b, src)).unwrap_or_default();
                let sig_tokens = normalized_tokens_excluding(child, src, body.map(|b| b.id()));
                out.push(FuncInfo {
                    name,
                    sig_tokens,
                    body_tokens,
                    line: child.start_position().row + 1,
                });
            }
        }
        // Always recurse so methods inside impl/class blocks are captured.
        visit(child, src, lang, out);
    }
}

fn node_text<'a>(node: Node, src: &'a str) -> &'a str {
    node.utf8_text(src.as_bytes()).unwrap_or("")
}

/// Canonical token stream of a subtree: leaf tokens joined by single spaces,
/// with comments and whitespace dropped. Two subtrees with the same tokens are
/// semantically equivalent for our purposes (formatting/comment changes vanish).
pub fn normalized_tokens(node: Node, src: &str) -> String {
    let mut out = String::new();
    push_leaves(node, src, None, &mut out);
    out
}

fn normalized_tokens_excluding(node: Node, src: &str, exclude_id: Option<usize>) -> String {
    let mut out = String::new();
    push_leaves(node, src, exclude_id, &mut out);
    out
}

fn push_leaves(node: Node, src: &str, exclude_id: Option<usize>, out: &mut String) {
    if Some(node.id()) == exclude_id {
        return;
    }
    if node.child_count() == 0 {
        if node.kind().contains("comment") {
            return;
        }
        let t = node_text(node, src).trim();
        if !t.is_empty() {
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(t);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        push_leaves(child, src, exclude_id, out);
    }
}
