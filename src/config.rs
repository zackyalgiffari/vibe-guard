use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Local LLM model name passed to the provider (e.g. an Ollama tag).
    pub model: String,
    /// Base URL of the Ollama HTTP API.
    pub ollama_url: String,
    /// Reports below this confidence are treated as low-confidence warnings.
    pub confidence_threshold: f32,
    /// Auto-approve boilerplate-only changes without prompting.
    pub auto_approve_boilerplate: bool,
    /// Glob-ish filename markers that flag a file as sensitive.
    pub sensitive_patterns: Vec<String>,
    /// Substrings (case-insensitive) in identifiers that indicate secret access.
    pub secret_identifiers: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            model: "mistral:7b-instruct".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            confidence_threshold: 0.75,
            auto_approve_boilerplate: true,
            sensitive_patterns: vec![
                ".env".into(),
                ".pem".into(),
                ".key".into(),
                "secrets".into(),
                "credentials".into(),
                "id_rsa".into(),
            ],
            secret_identifiers: vec![
                "secret".into(),
                "password".into(),
                "passwd".into(),
                "token".into(),
                "api_key".into(),
                "apikey".into(),
                "credential".into(),
                "private_key".into(),
                "access_key".into(),
            ],
        }
    }
}

impl Config {
    pub fn dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".vibe-guard")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("config.toml")
    }

    /// Load config from disk, or return defaults if it does not exist yet.
    pub fn load() -> Result<Config> {
        let path = Self::path();
        if !path.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        Ok(cfg)
    }

    /// Write the current config to disk, creating the directory if needed.
    pub fn save(&self) -> Result<PathBuf> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = Self::path();
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
        Ok(path)
    }
}
