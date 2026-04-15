use std::process::Command;
use chrono::{DateTime, Utc};
use crate::error::TldError;

pub fn get_file_last_commit_at(repo_root: &str, file_path: &str) -> Result<DateTime<Utc>, TldError> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cI", file_path])
        .current_dir(repo_root)
        .output()
        .map_err(|e| TldError::Generic(format!("Failed to execute git: {}", e)))?;

    if !output.status.success() {
        return Err(TldError::Generic("Git command failed".to_string()));
    }

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        return Err(TldError::Generic("No commit history found for file".to_string()));
    }

    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| TldError::Generic(format!("Failed to parse git date {}: {}", s, e)))
}
