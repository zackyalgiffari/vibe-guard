use anyhow::{bail, Result};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub status: FileStatus,
}

fn git(args: &[&str]) -> Result<std::process::Output> {
    let out = Command::new("git").args(args).output();
    match out {
        Ok(o) => Ok(o),
        Err(e) => bail!("failed to run git: {e}"),
    }
}

/// True if the current directory is inside a git work tree.
pub fn in_repo() -> bool {
    git(&["rev-parse", "--is-inside-work-tree"])
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// List files changed vs `HEAD`, including untracked files (treated as added).
pub fn changed_files() -> Result<Vec<ChangedFile>> {
    let mut files = Vec::new();

    let out = git(&["diff", "--name-status", "--no-renames", "HEAD"])?;
    if !out.status.success() {
        // Likely an empty repo with no HEAD; fall through to untracked only.
        let msg = String::from_utf8_lossy(&out.stderr);
        if !msg.contains("unknown revision") && !msg.contains("ambiguous argument") {
            bail!("git diff failed: {}", msg.trim());
        }
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        let mut parts = line.splitn(2, '\t');
        let code = parts.next().unwrap_or("");
        let path = match parts.next() {
            Some(p) => p.trim().to_string(),
            None => continue,
        };
        let status = match code.chars().next() {
            Some('A') => FileStatus::Added,
            Some('D') => FileStatus::Deleted,
            Some('R') => FileStatus::Renamed,
            _ => FileStatus::Modified,
        };
        files.push(ChangedFile { path, status });
    }

    // Untracked files are invisible to `git diff`; surface them as additions.
    let untracked = git(&["ls-files", "--others", "--exclude-standard"])?;
    if untracked.status.success() {
        for line in String::from_utf8_lossy(&untracked.stdout).lines() {
            let path = line.trim().to_string();
            if !path.is_empty() {
                files.push(ChangedFile {
                    path,
                    status: FileStatus::Added,
                });
            }
        }
    }

    Ok(files)
}

/// Content of `path` at `HEAD`, or `None` if it does not exist there.
pub fn show_head(path: &str) -> Option<String> {
    let spec = format!("HEAD:{path}");
    let out = git(&["show", &spec]).ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        None
    }
}

/// Current working-tree content of `path`, or `None` if it was deleted.
pub fn working_tree(path: &str) -> Option<String> {
    std::fs::read_to_string(Path::new(path)).ok()
}

/// sha256 (hex) of a file's content at `HEAD`.
pub fn head_hash(path: &str) -> Option<String> {
    use sha2::{Digest, Sha256};
    let content = show_head(path)?;
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    Some(format!("{:x}", hasher.finalize()))
}
