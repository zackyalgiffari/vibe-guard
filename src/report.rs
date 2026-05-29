use crate::engine::{ChangeType, StructuredDiff};
use crate::indexer::CoverageReport;
use crate::intent::IntentReport;
use crate::safety::SensitiveHit;
use owo_colors::OwoColorize;
use serde_json::json;

/// Everything produced by a `check` run, for rendering or JSON output.
pub struct Outcome<'a> {
    pub diff: &'a StructuredDiff,
    pub summary: &'a str,
    pub coverage: Option<&'a CoverageReport>,
    pub intent: Option<&'a IntentReport>,
    pub sensitive: &'a [SensitiveHit],
    pub confidence_threshold: f32,
}

impl<'a> Outcome<'a> {
    /// Print the human-readable report (the `plan.md` terminal layout).
    pub fn print_human(&self) {
        // RTK-Diff lines.
        for f in &self.diff.files {
            let tag = format!("{:<11}", f.change_type.label());
            let tag = match f.change_type {
                ChangeType::Behavioral => tag.yellow().to_string(),
                ChangeType::Refactor => tag.cyan().to_string(),
                ChangeType::Boilerplate => tag.dimmed().to_string(),
            };
            println!("[RTK-Diff]  {:<24} → {} ({})", f.path, tag, f.detail);
        }

        println!();
        println!("[Vibe]      {}", self.summary.bold());

        if let Some(cov) = self.coverage {
            let pct = cov.ratio() * 100.0;
            let line = format!(
                "Context coverage: {pct:.0}% ({}/{} fresh)",
                cov.fresh, cov.total
            );
            if cov.ratio() >= 1.0 {
                println!("[Indexer]   {}", line.green());
            } else {
                println!("[Indexer]   {}", line.yellow());
                for p in cov.stale.iter().chain(cov.uncovered.iter()) {
                    println!("              ↳ uncovered/stale: {p}");
                }
            }
        }

        if let Some(rep) = self.intent {
            println!();
            let conf = format!("Confidence: {:.2}", rep.confidence);
            let mark = if rep.confidence >= self.confidence_threshold {
                "✅"
            } else {
                "⚠️"
            };
            println!(
                "[Intent Guard] {} {mark}  (via {})",
                if rep.confidence >= self.confidence_threshold {
                    conf.green().to_string()
                } else {
                    conf.yellow().to_string()
                },
                rep.provider
            );
            let im = if rep.intent_match {
                "Intent matched ✅".green().to_string()
            } else {
                "Intent NOT clearly matched ❌".red().to_string()
            };
            println!("  ↳ {im}");
            for se in &rep.side_effects {
                println!("  ↳ {} {}", "Side effect:".red(), se);
            }
        }

        if !self.sensitive.is_empty() {
            println!();
            println!(
                "{}",
                "[Safety Guard] ⛔ SENSITIVE FILE DETECTED".red().bold()
            );
            for h in self.sensitive {
                println!("  ↳ {}: {}", h.path.bold(), h.reason);
            }
        }
    }

    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "summary": self.summary,
            "diff": self.diff,
            "coverage": self.coverage,
            "intent": self.intent,
            "sensitive_files": self.sensitive,
        })
    }
}
