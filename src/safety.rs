//! Local stand-in for the "Env Encryption layer": flags changes that touch
//! secrets, environment variables, or sensitive files.

use crate::config::Config;
use crate::engine::StructuredDiff;
use crate::git;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SensitiveHit {
    pub path: String,
    pub reason: String,
}

/// Patterns that indicate code reading/writing environment-backed secrets,
/// across the supported languages.
const ENV_ACCESS: &[&str] = &[
    "process.env.",    // JS/TS
    "import.meta.env", // Vite/TS
    "os.environ",      // Python
    "os.getenv",       // Python
    "std::env::var",   // Rust
    "env::var",        // Rust
    "os.Getenv",       // Go
];

/// Inspect each changed file's current content + name for sensitive markers.
pub fn scan(diff: &StructuredDiff, cfg: &Config) -> Vec<SensitiveHit> {
    let mut hits = Vec::new();
    for f in &diff.files {
        let lower_path = f.path.to_ascii_lowercase();
        if let Some(pat) = cfg
            .sensitive_patterns
            .iter()
            .find(|p| lower_path.contains(&p.to_ascii_lowercase()))
        {
            hits.push(SensitiveHit {
                path: f.path.clone(),
                reason: format!("sensitive filename (matched '{pat}')"),
            });
            continue;
        }

        let content = match git::working_tree(&f.path) {
            Some(c) => c,
            None => continue, // deleted file: nothing to read
        };
        let lower = content.to_ascii_lowercase();

        if let Some(marker) = ENV_ACCESS.iter().find(|m| content.contains(**m)) {
            // Try to surface the specific env var referenced.
            let var = extract_env_var(&content, marker).unwrap_or_default();
            let suffix = if var.is_empty() {
                String::new()
            } else {
                format!(" ({var})")
            };
            hits.push(SensitiveHit {
                path: f.path.clone(),
                reason: format!("reads environment secret via '{marker}'{suffix}"),
            });
            continue;
        }

        if let Some(id) = cfg
            .secret_identifiers
            .iter()
            .find(|id| lower.contains(id.as_str()))
        {
            hits.push(SensitiveHit {
                path: f.path.clone(),
                reason: format!("references secret-like identifier '{id}'"),
            });
        }
    }
    hits
}

/// Best-effort extraction of the identifier following an env-access marker,
/// e.g. `process.env.DB_SECRET_KEY` -> `process.env.DB_SECRET_KEY`.
fn extract_env_var(content: &str, marker: &str) -> Option<String> {
    let idx = content.find(marker)?;
    let rest = &content[idx..];
    let token: String = rest
        .chars()
        .take_while(|c| {
            c.is_alphanumeric() || matches!(c, '.' | '_' | ':' | '(' | ')' | '"' | '\'')
        })
        .collect();
    let token = token.trim_end_matches(['(', ')', '"', '\'']).to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}
