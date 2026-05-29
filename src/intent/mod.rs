pub mod heuristic;
pub mod ollama;
pub mod provider;

use crate::config::Config;
use crate::engine::StructuredDiff;
use crate::indexer::CoverageReport;
use provider::LlmProvider;
use serde::Serialize;

/// The Intent Guard's verdict: does the structural change match stated intent?
#[derive(Debug, Clone, Serialize)]
pub struct IntentReport {
    pub confidence: f32,
    pub intent_match: bool,
    pub side_effects: Vec<String>,
    /// Name of the provider that produced this report.
    pub provider: String,
}

/// Run the Intent Guard. Tries the Ollama provider first (unless intent is
/// empty); on any failure, falls back to the always-available heuristic
/// provider. The coverage ratio dampens the final confidence.
pub fn evaluate(
    intent: &str,
    diff: &StructuredDiff,
    coverage: &CoverageReport,
    cfg: &Config,
    model_override: Option<&str>,
) -> IntentReport {
    let model = model_override.unwrap_or(&cfg.model);

    let mut report = {
        let ollama = ollama::OllamaProvider::new(&cfg.ollama_url, model);
        match ollama.evaluate(intent, diff) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("note: Ollama unavailable ({e}); using heuristic Intent Guard");
                heuristic::HeuristicProvider::new(cfg)
                    .evaluate(intent, diff)
                    .expect("heuristic provider is infallible")
            }
        }
    };

    // Stale/uncovered context erodes how much we trust the match.
    let ratio = coverage.ratio();
    if ratio < 1.0 {
        report.confidence *= 0.5 + 0.5 * ratio;
        report.side_effects.push(format!(
            "Context coverage {:.0}% — assistant may not have seen latest versions",
            ratio * 100.0
        ));
    }
    report.confidence = report.confidence.clamp(0.0, 1.0);
    report
}
