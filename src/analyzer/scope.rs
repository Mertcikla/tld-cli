//! Scan planner — builds the deterministic set of files and repositories to analyze.
#![allow(dead_code)]
//!
//! Responsibilities:
//! - Resolve workspace-root scans via `.tld.yaml` repository entries.
//! - Implement `--changed-since` by querying `git diff`.
//! - Implement `--deep` by expanding scope to the git repo root.
//! - Apply exclude patterns consistently.
//! - Produce a deterministic file list.

use crate::error::TldError;
use std::path::Path;

/// The full scope of what will be analyzed.
pub struct AnalyzeScope {
    /// The effective root directory of analysis (may be git root when --deep is used).
    pub root_dir: String,
    pub repositories: Vec<RepositoryScope>,
    /// Flat list of concrete files to analyze (populated only when --changed-since restricts
    /// the set; otherwise, full directory walk is used instead).
    pub files: Vec<FileScope>,
    pub deep: bool,
    pub changed_since: Option<String>,
}

pub struct RepositoryScope {
    pub name: String,
    pub root_dir: String,
    /// Absolute paths of the files to analyze within this repo.
    pub files: Vec<String>,
}

pub struct FileScope {
    pub abs_path: String,
    pub rel_path: String,
    pub repo_name: String,
    pub language: String,
}

/// Build an `AnalyzeScope` from CLI arguments.
///
/// When `changed_since` is `Some`, the scope is restricted to files that `git diff` reports
/// as modified since the given ref, and the caller should analyze only those files instead of
/// doing a full directory walk.
///
/// When `deep` is `true`, the effective scan root is expanded to the git repository root so
/// that cross-file call resolution can follow edges beyond the user-specified path.
pub fn plan(
    path: &str,
    repo_name: &str,
    deep: bool,
    changed_since: Option<&str>,
    exclude: &[String],
) -> Result<AnalyzeScope, TldError> {
    let abs_path = Path::new(path)
        .canonicalize()
        .map_err(|e| TldError::Generic(format!("Cannot resolve path '{path}': {e}")))?;
    let abs_str = abs_path.to_str().unwrap_or(path).to_string();

    // --deep: expand to git repo root so cross-file edges can be followed.
    let effective_root = if deep {
        detect_git_root(&abs_str).unwrap_or_else(|| abs_str.clone())
    } else {
        abs_str.clone()
    };

    let mut files: Vec<FileScope> = Vec::new();

    if let Some(since) = changed_since {
        let changed = git_changed_files(&effective_root, since)?;
        for rel_path in changed {
            let abs_file = format!("{effective_root}/{rel_path}");
            if !Path::new(&abs_file).is_file() {
                continue;
            }
            if exclude.iter().any(|ex| abs_file.contains(ex.as_str())) {
                continue;
            }
            if let Some(lang) = detect_language_from_path(&rel_path) {
                files.push(FileScope {
                    abs_path: abs_file,
                    rel_path,
                    repo_name: repo_name.to_string(),
                    language: lang,
                });
            }
        }
    }

    let repo_files = files.iter().map(|f| f.abs_path.clone()).collect();

    Ok(AnalyzeScope {
        root_dir: effective_root.clone(),
        repositories: vec![RepositoryScope {
            name: repo_name.to_string(),
            root_dir: effective_root,
            files: repo_files,
        }],
        files,
        deep,
        changed_since: changed_since.map(ToString::to_string),
    })
}

/// Walk up from `path` to find the git repository root.
pub fn detect_git_root(path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()?;
    if output.status.success() {
        let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !root.is_empty() {
            return Some(root);
        }
    }
    None
}

/// Return relative paths of files changed since `git_ref` inside `repo_root`.
fn git_changed_files(repo_root: &str, git_ref: &str) -> Result<Vec<String>, TldError> {
    // Include both staged and unstaged changes relative to the given ref.
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", git_ref])
        .current_dir(repo_root)
        .output()
        .map_err(|e| TldError::Generic(format!("git diff failed: {e}")))?;

    if !output.status.success() {
        return Err(TldError::Generic(format!(
            "`git diff --name-only {git_ref}` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ToString::to_string)
        .filter(|l| !l.is_empty())
        .collect())
}

pub fn detect_language_from_path(path: &str) -> Option<String> {
    let ext = Path::new(path).extension()?.to_str()?;
    Some(
        match ext {
            "go" => "go",
            "rs" => "rust",
            "py" => "python",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "java" => "java",
            "cpp" | "cc" | "cxx" | "h" | "hpp" => "cpp",
            _ => return None,
        }
        .to_string(),
    )
}
