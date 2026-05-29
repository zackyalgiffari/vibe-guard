use super::{ChangeType, FileDiff, StructuredDiff, SymbolChange, SymbolKind};
use crate::ast::{self, Language};
use crate::git::{self, ChangedFile, FileStatus};
use anyhow::Result;
use std::collections::HashMap;

/// Build the structured diff for every changed file, honoring an optional
/// `--lang` filter (list of language tokens; empty = all languages).
pub fn build(files: &[ChangedFile], lang_filter: &[String]) -> Result<StructuredDiff> {
    let mut out = Vec::new();
    for cf in files {
        let language = Language::from_path(&cf.path);
        if !lang_filter.is_empty() {
            match language {
                Some(l) if lang_filter.iter().any(|t| l.matches_filter(t)) => {}
                _ => continue,
            }
        }

        let before = if cf.status == FileStatus::Added {
            String::new()
        } else {
            git::show_head(&cf.path).unwrap_or_default()
        };
        let after = if cf.status == FileStatus::Deleted {
            String::new()
        } else {
            git::working_tree(&cf.path).unwrap_or_default()
        };

        out.push(classify_file(cf, language, &before, &after)?);
    }
    Ok(StructuredDiff { files: out })
}

/// Classify a single file directly from its before/after sources, without
/// touching git. The language is inferred from `path`. Useful for tests and for
/// callers that already hold both versions.
pub fn classify_sources(
    path: &str,
    status: FileStatus,
    before: &str,
    after: &str,
) -> Result<FileDiff> {
    let cf = ChangedFile {
        path: path.to_string(),
        status,
    };
    let language = Language::from_path(path);
    classify_file(&cf, language, before, after)
}

fn classify_file(
    cf: &ChangedFile,
    language: Option<Language>,
    before: &str,
    after: &str,
) -> Result<FileDiff> {
    // Files of unknown language fall back to a whitespace-insensitive text diff.
    let Some(lang) = language else {
        let ct = if strip_ws(before) == strip_ws(after) {
            ChangeType::Boilerplate
        } else {
            ChangeType::Behavioral
        };
        return Ok(FileDiff {
            path: cf.path.clone(),
            language,
            status: cf.status,
            change_type: ct,
            symbols: Vec::new(),
            detail: "unparsed (text diff)".to_string(),
        });
    };

    let before_fns = ast::functions(before, lang).unwrap_or_default();
    let after_fns = ast::functions(after, lang).unwrap_or_default();

    // Match functions by name. Names present on only one side are added/removed;
    // names on both sides are compared by signature + body tokens.
    let mut before_by: HashMap<&str, &ast::FuncInfo> =
        before_fns.iter().map(|f| (f.name.as_str(), f)).collect();

    let mut symbols: Vec<SymbolChange> = Vec::new();
    let mut behavioral = false;

    for af in &after_fns {
        match before_by.remove(af.name.as_str()) {
            Some(bf) => {
                if bf.body_tokens != af.body_tokens || bf.sig_tokens != af.sig_tokens {
                    behavioral = true;
                    symbols.push(SymbolChange {
                        name: af.name.clone(),
                        kind: SymbolKind::Modified,
                        line: Some(af.line),
                    });
                }
            }
            None => {
                behavioral = true;
                symbols.push(SymbolChange {
                    name: af.name.clone(),
                    kind: SymbolKind::Added,
                    line: Some(af.line),
                });
            }
        }
    }
    // Whatever remains in `before_by` was removed.
    let mut removed: Vec<&ast::FuncInfo> = before_by.values().copied().collect();
    removed.sort_by_key(|f| f.line);
    for bf in removed {
        behavioral = true;
        symbols.push(SymbolChange {
            name: bf.name.clone(),
            kind: SymbolKind::Removed,
            line: Some(bf.line),
        });
    }

    // Detect pure renames/moves: identical body multiset, but names changed.
    let before_bodies = body_multiset(&before_fns);
    let after_bodies = body_multiset(&after_fns);
    let bodies_identical = before_bodies == after_bodies;

    // Whole-file token comparison detects formatting/import/comment-only edits.
    let before_tokens = ast::file_tokens(before, lang).unwrap_or_default();
    let after_tokens = ast::file_tokens(after, lang).unwrap_or_default();
    let tokens_identical = before_tokens == after_tokens;

    let change_type = if tokens_identical {
        ChangeType::Boilerplate
    } else if behavioral && !bodies_identical {
        ChangeType::Behavioral
    } else {
        // Function bodies are unchanged as a set, but names/order/top-level
        // structure shifted: a refactor.
        ChangeType::Refactor
    };

    // Mark cross-name renames when bodies match but the name set changed.
    if change_type == ChangeType::Refactor {
        for s in symbols.iter_mut() {
            if matches!(s.kind, SymbolKind::Added | SymbolKind::Removed) {
                s.kind = SymbolKind::Renamed;
            }
        }
    }

    let detail = describe(&cf.status, change_type, &symbols, before, after);

    Ok(FileDiff {
        path: cf.path.clone(),
        language,
        status: cf.status,
        change_type,
        symbols,
        detail,
    })
}

fn body_multiset(fns: &[ast::FuncInfo]) -> Vec<String> {
    let mut v: Vec<String> = fns.iter().map(|f| f.body_tokens.clone()).collect();
    v.sort();
    v
}

fn describe(
    status: &FileStatus,
    ct: ChangeType,
    symbols: &[SymbolChange],
    before: &str,
    after: &str,
) -> String {
    match status {
        FileStatus::Added => return "new file".to_string(),
        FileStatus::Deleted => return "file deleted".to_string(),
        _ => {}
    }
    match ct {
        ChangeType::Boilerplate => "imports/formatting only".to_string(),
        ChangeType::Refactor => {
            let n = symbols.len();
            if n == 0 {
                "structure reordered".to_string()
            } else {
                format!("{n} symbol(s) renamed/moved")
            }
        }
        ChangeType::Behavioral => {
            let n = symbols
                .iter()
                .filter(|s| s.kind == SymbolKind::Modified)
                .count();
            if n > 0 {
                format!("{n} function(s) modified")
            } else if !symbols.is_empty() {
                format!("{} symbol(s) added/removed", symbols.len())
            } else {
                let delta = after.lines().count() as i64 - before.lines().count() as i64;
                format!("logic changed ({delta:+} lines)")
            }
        }
    }
}

fn strip_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Classify a pre-generated unified diff (`--diff <file>`). Without the full
/// before/after sources we can't reconstruct ASTs, so this is a line-based
/// fallback: a file is BOILERPLATE if every changed line is blank, a comment, or
/// an import; otherwise BEHAVIORAL. No per-symbol detail is produced.
pub fn build_from_patch(patch: &str, lang_filter: &[String]) -> Result<StructuredDiff> {
    let mut files: Vec<FileDiff> = Vec::new();
    let mut cur_path: Option<String> = None;
    let mut status = FileStatus::Modified;
    let mut trivial_only = true;
    let mut changed = 0usize;

    let flush = |files: &mut Vec<FileDiff>,
                 path: &Option<String>,
                 status: FileStatus,
                 trivial_only: bool,
                 changed: usize| {
        let Some(path) = path else { return };
        let language = Language::from_path(path);
        if !lang_filter.is_empty() {
            match language {
                Some(l) if lang_filter.iter().any(|t| l.matches_filter(t)) => {}
                _ => return,
            }
        }
        if changed == 0 {
            return;
        }
        let change_type = if trivial_only {
            ChangeType::Boilerplate
        } else {
            ChangeType::Behavioral
        };
        files.push(FileDiff {
            path: path.clone(),
            language,
            status,
            change_type,
            symbols: Vec::new(),
            detail: format!("{changed} line(s) changed (patch mode)"),
        });
    };

    for line in patch.lines() {
        if line.starts_with("+++ ") {
            let p = line.trim_start_matches("+++ ").trim();
            if p == "/dev/null" {
                status = FileStatus::Deleted;
            } else {
                cur_path = Some(strip_ab_prefix(p));
            }
        } else if line.starts_with("--- ") {
            let p = line.trim_start_matches("--- ").trim();
            if p == "/dev/null" {
                status = FileStatus::Added;
            }
        } else if line.starts_with("diff --git") {
            flush(&mut files, &cur_path, status, trivial_only, changed);
            cur_path = None;
            status = FileStatus::Modified;
            trivial_only = true;
            changed = 0;
        } else if (line.starts_with('+') || line.starts_with('-'))
            && !line.starts_with("+++")
            && !line.starts_with("---")
        {
            changed += 1;
            if !is_trivial_line(&line[1..]) {
                trivial_only = false;
            }
        }
    }
    flush(&mut files, &cur_path, status, trivial_only, changed);

    Ok(StructuredDiff { files })
}

fn strip_ab_prefix(p: &str) -> String {
    p.strip_prefix("a/")
        .or_else(|| p.strip_prefix("b/"))
        .unwrap_or(p)
        .to_string()
}

fn is_trivial_line(content: &str) -> bool {
    let t = content.trim();
    t.is_empty()
        || t.starts_with("//")
        || t.starts_with('#')
        || t.starts_with("/*")
        || t.starts_with('*')
        || t.starts_with("import ")
        || t.starts_with("from ")
        || t.starts_with("use ")
        || t.starts_with("require(")
}
