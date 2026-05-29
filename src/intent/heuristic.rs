use super::provider::LlmProvider;
use super::IntentReport;
use crate::config::Config;
use crate::engine::{StructuredDiff, SymbolKind};
use anyhow::Result;
use std::collections::HashSet;

/// A model-free Intent Guard. It scores intent/diff overlap by keyword and
/// flags deletions or edits of security-sensitive symbols as side effects.
/// Always available, so the tool works with no LLM installed.
pub struct HeuristicProvider {
    secret_words: Vec<String>,
}

impl HeuristicProvider {
    pub fn new(cfg: &Config) -> Self {
        // Security-relevant symbol words whose removal is worth warning about.
        let mut words: Vec<String> = vec![
            "auth",
            "login",
            "password",
            "passwd",
            "validate",
            "validation",
            "verify",
            "token",
            "secret",
            "permission",
            "authorize",
            "encrypt",
            "decrypt",
            "sanitize",
            "csrf",
            "session",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        words.extend(cfg.secret_identifiers.iter().cloned());
        HeuristicProvider {
            secret_words: words,
        }
    }

    fn is_security_symbol(&self, name: &str) -> bool {
        let lower = name.to_ascii_lowercase();
        self.secret_words.iter().any(|w| lower.contains(w.as_str()))
    }
}

impl LlmProvider for HeuristicProvider {
    fn name(&self) -> &str {
        "heuristic"
    }

    fn evaluate(&self, intent: &str, diff: &StructuredDiff) -> Result<IntentReport> {
        let intent_words = tokenize(intent);
        let mut symbol_words: HashSet<String> = HashSet::new();
        let mut changed_symbols = 0usize;
        let mut matched_symbols = 0usize;
        let mut side_effects = Vec::new();

        for f in diff.functional_files() {
            for w in tokenize(&f.path) {
                symbol_words.insert(w);
            }
            for sym in &f.symbols {
                changed_symbols += 1;
                let words = tokenize(&sym.name);
                let overlaps = words.iter().any(|w| intent_words.contains(w));
                if overlaps {
                    matched_symbols += 1;
                }
                for w in words {
                    symbol_words.insert(w);
                }

                // A removed/modified security symbol not named in the intent is
                // a classic unintended side effect.
                if matches!(sym.kind, SymbolKind::Removed | SymbolKind::Modified)
                    && self.is_security_symbol(&sym.name)
                    && !intent_mentions_security(&intent_words, &sym.name)
                {
                    let verb = if sym.kind == SymbolKind::Removed {
                        "removed"
                    } else {
                        "modified"
                    };
                    let loc = sym
                        .line
                        .map(|l| format!(" ({}:{l})", f.path))
                        .unwrap_or_else(|| format!(" ({})", f.path));
                    side_effects.push(format!(
                        "Security-relevant logic '{}' was {verb}{loc} — not in stated intent",
                        sym.name
                    ));
                }
            }
        }

        // Confidence: lexical overlap between intent and changed paths/symbols,
        // penalized by unexplained side effects.
        let overlap = if intent_words.is_empty() {
            0.5
        } else {
            let common = intent_words.intersection(&symbol_words).count();
            (common as f32 / intent_words.len() as f32).min(1.0)
        };
        let symbol_match_ratio = if changed_symbols == 0 {
            1.0
        } else {
            matched_symbols as f32 / changed_symbols as f32
        };

        let mut confidence = 0.4 + 0.35 * overlap + 0.25 * symbol_match_ratio;
        confidence -= 0.2 * side_effects.len() as f32;
        let confidence = confidence.clamp(0.05, 0.98);

        let intent_match = side_effects.is_empty() && confidence >= 0.6;

        Ok(IntentReport {
            confidence,
            intent_match,
            side_effects,
            provider: "heuristic".to_string(),
        })
    }
}

fn tokenize(s: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let mut cur = String::new();
    let push = |cur: &mut String, out: &mut HashSet<String>| {
        if cur.len() >= 3 {
            out.insert(cur.to_ascii_lowercase());
        }
        cur.clear();
    };
    for ch in s.chars() {
        if ch.is_alphanumeric() {
            // Split camelCase boundaries into separate tokens.
            if ch.is_uppercase() && !cur.is_empty() {
                let mut c = cur.clone();
                push(&mut c, &mut out);
                cur = String::new();
            }
            cur.push(ch);
        } else {
            push(&mut cur, &mut out);
        }
    }
    push(&mut cur, &mut out);
    out
}

fn intent_mentions_security(intent_words: &HashSet<String>, symbol: &str) -> bool {
    tokenize(symbol).iter().any(|w| intent_words.contains(w))
}
