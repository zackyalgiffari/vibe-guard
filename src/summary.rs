use crate::engine::{ChangeType, StructuredDiff};

/// Roll the structured diff into a single human-readable "vibe check" line,
/// e.g. `"Logic Update in 2 functions, 1 Refactor, 3 boilerplate skipped"`.
pub fn vibe_line(diff: &StructuredDiff) -> String {
    let mut behavioral_fns = 0usize;
    let mut behavioral_files = 0usize;
    let mut refactors = 0usize;
    let mut boilerplate = 0usize;

    for f in &diff.files {
        match f.change_type {
            ChangeType::Behavioral => {
                behavioral_files += 1;
                behavioral_fns += f
                    .symbols
                    .iter()
                    .filter(|s| {
                        matches!(
                            s.kind,
                            crate::engine::SymbolKind::Modified
                                | crate::engine::SymbolKind::Added
                                | crate::engine::SymbolKind::Removed
                        )
                    })
                    .count();
            }
            ChangeType::Refactor => refactors += 1,
            ChangeType::Boilerplate => boilerplate += 1,
        }
    }

    if diff.files.is_empty() {
        return "No changes detected".to_string();
    }

    let mut parts: Vec<String> = Vec::new();
    if behavioral_fns > 0 {
        parts.push(format!(
            "Logic Update in {behavioral_fns} function{}",
            plural(behavioral_fns)
        ));
    } else if behavioral_files > 0 {
        parts.push(format!(
            "Logic Update in {behavioral_files} file{}",
            plural(behavioral_files)
        ));
    }
    if refactors > 0 {
        parts.push(format!("{refactors} Refactor{}", plural(refactors)));
    }
    if boilerplate > 0 {
        parts.push(format!("{boilerplate} boilerplate skipped"));
    }

    if parts.is_empty() {
        "No functional changes".to_string()
    } else {
        parts.join(", ")
    }
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}
