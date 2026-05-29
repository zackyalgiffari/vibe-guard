use super::provider::{render_diff, LlmProvider};
use super::IntentReport;
use crate::engine::StructuredDiff;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::time::Duration;

/// Talks to a local Ollama server over HTTP. Fully local; nothing leaves the box.
pub struct OllamaProvider {
    url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(base_url: &str, model: &str) -> Self {
        OllamaProvider {
            url: format!("{}/api/generate", base_url.trim_end_matches('/')),
            model: model.to_string(),
        }
    }
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

/// The JSON we ask the model to emit inside its `response` field.
#[derive(Deserialize, Default)]
struct Verdict {
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    intent_match: Option<bool>,
    #[serde(default)]
    side_effects: Option<Vec<String>>,
}

impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn evaluate(&self, intent: &str, diff: &StructuredDiff) -> Result<IntentReport> {
        let prompt = build_prompt(intent, diff);

        let config = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(90)))
            .build();
        let agent: ureq::Agent = config.into();

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
            "options": { "temperature": 0.0 }
        });

        let mut resp = agent
            .post(&self.url)
            .send_json(body)
            .with_context(|| format!("POST {}", self.url))?;
        let outer: OllamaResponse = resp
            .body_mut()
            .read_json()
            .context("decoding Ollama response")?;

        let verdict: Verdict = serde_json::from_str(outer.response.trim())
            .map_err(|e| anyhow!("model did not return valid JSON: {e}"))?;

        Ok(IntentReport {
            confidence: verdict.confidence.unwrap_or(0.5).clamp(0.0, 1.0),
            intent_match: verdict.intent_match.unwrap_or(false),
            side_effects: verdict.side_effects.unwrap_or_default(),
            provider: format!("ollama:{}", self.model),
        })
    }
}

fn build_prompt(intent: &str, diff: &StructuredDiff) -> String {
    format!(
        "You are a code-review assistant. A developer stated this intent:\n\
         \"{intent}\"\n\n\
         The following STRUCTURAL changes were made (AST-level, not raw lines):\n\
         {changes}\n\
         Decide whether the changes match the stated intent, and list any side \
         effects that were NOT part of the intent (e.g. deleted validation, \
         changed security logic).\n\
         Respond with ONLY a JSON object of the form:\n\
         {{\"confidence\": <0..1>, \"intent_match\": <true|false>, \
         \"side_effects\": [\"...\"]}}",
        intent = if intent.is_empty() {
            "(none provided)"
        } else {
            intent
        },
        changes = render_diff(diff),
    )
}
