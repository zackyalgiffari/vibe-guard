use super::IntentReport;
use crate::engine::StructuredDiff;
use anyhow::Result;

/// A backend that scores whether a structural diff matches stated intent.
pub trait LlmProvider {
    fn name(&self) -> &str;
    fn evaluate(&self, intent: &str, diff: &StructuredDiff) -> Result<IntentReport>;
}

/// Render the structured diff into a compact, token-frugal textual form for the
/// LLM prompt (RTK-inspired: only functional changes, no raw line noise).
pub fn render_diff(diff: &StructuredDiff) -> String {
    let mut s = String::new();
    for f in diff.functional_files() {
        s.push_str(&format!(
            "- {} [{}] {}\n",
            f.path,
            f.change_type.label(),
            f.detail
        ));
        for sym in &f.symbols {
            s.push_str(&format!(
                "    {:?} {}{}\n",
                sym.kind,
                sym.name,
                sym.line.map(|l| format!(" (line {l})")).unwrap_or_default()
            ));
        }
    }
    if s.is_empty() {
        s.push_str("(no functional changes)\n");
    }
    s
}
