use crate::error::TldError;
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn get_file_last_commit_at(
    repo_root: &str,
    file_path: &str,
) -> Result<DateTime<Utc>, TldError> {
    let output = Command::new("git")
        .args(["log", "-1", "--format=%cI", file_path])
        .current_dir(repo_root)
        .output()
        .map_err(|e| TldError::Generic(format!("Failed to execute git: {e}")))?;

    if !output.status.success() {
        return Err(TldError::Generic("Git command failed".to_string()));
    }

    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        return Err(TldError::Generic(
            "No commit history found for file".to_string(),
        ));
    }

    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| TldError::Generic(format!("Failed to parse git date {s}: {e}")))
}

pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn get_remotes(path: &Path) -> std::collections::HashMap<String, String> {
    let mut remotes = std::collections::HashMap::new();
    let output = Command::new("git")
        .args(["remote"])
        .current_dir(path)
        .output()
        .ok();

    if let Some(o) = output.filter(|o| o.status.success()) {
        let stdout = String::from_utf8_lossy(&o.stdout);
        for name in stdout.lines() {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            if let Some(url) = get_remote_url_by_name(path, name) {
                remotes.insert(name.to_string(), url);
            }
        }
    }
    remotes
}

pub fn get_remote_url_by_name(path: &Path, name: &str) -> Option<String> {
    Command::new("git")
        .args(["remote", "get-url", name])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn get_toplevel(path: &Path) -> Option<PathBuf> {
    Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_git_utilities() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Not a repo initially
        assert!(!is_git_repo(repo_path));
        assert!(get_toplevel(repo_path).is_none());
        assert!(get_remotes(repo_path).is_empty());

        // Init repo
        Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .unwrap();

        assert!(is_git_repo(repo_path));
        assert!(get_toplevel(repo_path).is_some());

        // Add remote
        Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/example/repo.git",
            ])
            .current_dir(repo_path)
            .output()
            .unwrap();

        let remotes = get_remotes(repo_path);
        assert_eq!(
            remotes.get("origin").unwrap(),
            "https://github.com/example/repo.git"
        );
        assert_eq!(
            get_remote_url_by_name(repo_path, "origin").unwrap(),
            "https://github.com/example/repo.git"
        );
    }
}
