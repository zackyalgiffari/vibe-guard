//! Local stand-in for the "Indexing LLM Read" tool.
//!
//! `index sync` records each tracked file's content hash at `HEAD` in
//! `~/.vibe-guard/index.json`. At check time we compare the changed files'
//! current `HEAD` hashes against that manifest to estimate how confident we can
//! be that an assistant saw the latest versions. Stale/untracked files lower the
//! coverage and, in turn, the intent-match confidence.

use crate::config::Config;
use crate::git;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Index {
    /// path -> sha256 of HEAD content at the time of `index sync`.
    pub hashes: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoverageReport {
    pub total: usize,
    pub fresh: usize,
    pub stale: Vec<String>,
    pub uncovered: Vec<String>,
}

impl CoverageReport {
    pub fn ratio(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            self.fresh as f32 / self.total as f32
        }
    }
}

fn index_path() -> PathBuf {
    Config::dir().join("index.json")
}

pub fn load() -> Result<Index> {
    let path = index_path();
    if !path.exists() {
        return Ok(Index::default());
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

/// Rebuild the index from every tracked file's current HEAD hash.
pub fn sync() -> Result<usize> {
    let out = std::process::Command::new("git")
        .args(["ls-files"])
        .output()
        .context("running git ls-files")?;
    let mut index = Index::default();
    for path in String::from_utf8_lossy(&out.stdout).lines() {
        let path = path.trim();
        if path.is_empty() {
            continue;
        }
        if let Some(hash) = git::head_hash(path) {
            index.hashes.insert(path.to_string(), hash);
        }
    }
    let dir = Config::dir();
    std::fs::create_dir_all(&dir)?;
    let text = serde_json::to_string_pretty(&index)?;
    std::fs::write(index_path(), text)?;
    Ok(index.hashes.len())
}

/// Compare the changed files against the saved index.
pub fn coverage(paths: &[String]) -> Result<CoverageReport> {
    let index = load()?;
    let mut fresh = 0;
    let mut stale = Vec::new();
    let mut uncovered = Vec::new();

    for path in paths {
        match index.hashes.get(path) {
            None => uncovered.push(path.clone()),
            Some(saved) => match git::head_hash(path) {
                Some(current) if &current == saved => fresh += 1,
                _ => stale.push(path.clone()),
            },
        }
    }

    Ok(CoverageReport {
        total: paths.len(),
        fresh,
        stale,
        uncovered,
    })
}
