pub mod ignore;
pub mod lsp;
pub mod parsers;
pub mod projection;
pub mod scope;
pub mod semantic;
pub mod syntax;
pub mod types;

use crate::analyzer::syntax::types::SyntaxBundle;
use crate::error::TldError;
pub use ignore::Rules;
use std::fs;
use std::path::Path;
use ts_pack_core::{detect_language_from_path, get_language};
pub use types::*;

/// Callback invoked for each file or directory visited during analysis.
/// Arguments: (path, is_dir).
pub type OnEntry<'a> = Option<&'a dyn Fn(&str, bool)>;

pub trait Service {
    fn extract_path(
        &self,
        path: &str,
        rules: &Rules,
        on_entry: OnEntry<'_>,
    ) -> Result<AnalysisResult, TldError>;
}

pub struct TreeSitterService {}

impl TreeSitterService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn extract_file(path: &str) -> Result<AnalysisResult, TldError> {
        let lang_name = if let Some(l) = detect_language_from_path(path) {
            l
        } else {
            let ext = Path::new(path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("<none>");
            match ext {
                "py" => "python",
                "java" => "java",
                "cpp" | "cc" | "hpp" | "h" | "cxx" => "cpp",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "go" => "go",
                "rs" => "rust",
                _ => return Err(TldError::UnsupportedLanguage(ext.to_string())),
            }
        };

        // Load the parser — auto-downloads when the `download` feature is enabled.
        // If the download fails we surface a helpful error instead of a generic one.
        let language = get_language(lang_name).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("download") || msg.contains("network") || msg.contains("http") {
                TldError::ParserDownloadRequired {
                    lang: lang_name.to_string(),
                    reason: msg,
                }
            } else {
                TldError::UnsupportedLanguage(lang_name.to_string())
            }
        })?;

        // Check that we have a parser implementation for this language.
        if !is_parser_implemented(lang_name) {
            return Err(TldError::ParserNotImplemented(lang_name.to_string()));
        }

        let source = fs::read_to_string(path)?;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| TldError::Generic(e.to_string()))?;

        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| TldError::Generic(format!("Failed to parse {path}")))?;

        let mut result = AnalysisResult::default();
        let technology = lang_name.to_string();

        match lang_name {
            "go" => {
                parsers::go::parse(
                    &tree.root_node(),
                    source.as_bytes(),
                    path,
                    &language,
                    &mut result,
                );
            }
            "rust" => {
                parsers::rust::parse(
                    &tree.root_node(),
                    source.as_bytes(),
                    path,
                    &language,
                    &mut result,
                );
            }
            "cpp" => {
                parsers::cpp::parse(
                    &tree.root_node(),
                    source.as_bytes(),
                    path,
                    &language,
                    &mut result,
                );
            }
            "java" => {
                parsers::java::parse(
                    &tree.root_node(),
                    source.as_bytes(),
                    path,
                    &language,
                    &mut result,
                );
            }
            "python" => {
                parsers::python::parse(
                    &tree.root_node(),
                    source.as_bytes(),
                    path,
                    &language,
                    &mut result,
                );
            }
            "typescript" | "javascript" | "tsx" => {
                parsers::typescript::parse(
                    &tree.root_node(),
                    source.as_bytes(),
                    path,
                    &language,
                    &mut result,
                );
            }
            _ => {
                // Guarded above by is_parser_implemented; this branch should not be reached.
                return Err(TldError::ParserNotImplemented(lang_name.to_string()));
            }
        }

        for sym in &mut result.symbols {
            sym.technology.clone_from(&technology);
        }

        Ok(result)
    }

    pub fn extract_file_syntax(path: &str, repo_name: &str) -> Result<SyntaxBundle, TldError> {
        let result = Self::extract_file(path)?;
        Ok(syntax::from_analysis_result(&result, repo_name))
    }

    #[expect(
        dead_code,
        reason = "Used by tests while syntax-first path migration is in progress"
    )]
    pub fn extract_path_syntax(
        &self,
        path: &str,
        rules: &Rules,
        repo_name: &str,
        on_entry: OnEntry<'_>,
    ) -> Result<SyntaxBundle, TldError> {
        let result = self.extract_path(path, rules, on_entry)?;
        Ok(syntax::from_analysis_result(&result, repo_name))
    }
}

/// Returns true when tld has an AST-walk implementation for the language.
///
/// A language can be recognised by tree-sitter-language-pack (and downloaded)
/// without tld having a parser written for it yet.  Separating the two checks
/// lets us give a clear "not yet implemented" message rather than a generic
/// tree-sitter error.
fn is_parser_implemented(lang_name: &str) -> bool {
    matches!(
        lang_name,
        "go" | "rust" | "cpp" | "java" | "python" | "typescript" | "javascript" | "tsx"
    )
}

impl Service for TreeSitterService {
    fn extract_path(
        &self,
        path: &str,
        rules: &Rules,
        on_entry: OnEntry<'_>,
    ) -> Result<AnalysisResult, TldError> {
        let metadata = fs::metadata(path)?;
        if metadata.is_dir() {
            Self::extract_dir(path, rules, on_entry)
        } else {
            if rules.should_ignore_path(path) {
                return Ok(AnalysisResult::default());
            }
            if let Some(cb) = on_entry {
                cb(path, false);
            }
            let mut result = Self::extract_file(path)?;
            result.files_scanned.push(path.to_string());
            Ok(result)
        }
    }
}

impl TreeSitterService {
    fn extract_dir(
        root: &str,
        rules: &Rules,
        on_entry: OnEntry<'_>,
    ) -> Result<AnalysisResult, TldError> {
        let mut merged = AnalysisResult::default();
        Self::walk_dir(Path::new(root), root, rules, on_entry, &mut merged)?;
        Ok(merged)
    }

    fn walk_dir(
        dir: &Path,
        root: &str,
        rules: &Rules,
        on_entry: OnEntry<'_>,
        merged: &mut AnalysisResult,
    ) -> Result<(), TldError> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_str()
                .unwrap_or("");

            if entry.file_type()?.is_dir() {
                if rules.should_ignore_path(rel_path) {
                    continue;
                }
                if let Some(cb) = on_entry {
                    cb(path.to_str().unwrap_or(""), true);
                }
                Self::walk_dir(&path, root, rules, on_entry, merged)?;
            } else {
                if rules.should_ignore_path(rel_path) {
                    continue;
                }
                if let Some(cb) = on_entry {
                    cb(path.to_str().unwrap_or(""), false);
                }
                let abs_path = path.to_str().unwrap_or("").to_string();
                // Always record the file as scanned, regardless of parse outcome.
                merged.files_scanned.push(abs_path.clone());
                match Self::extract_file(&abs_path) {
                    Ok(result) => merged.merge(result),
                    Err(TldError::UnsupportedLanguage(_)) => {
                        // Silently skip files in languages tld doesn't support.
                    }
                    Err(TldError::ParserNotImplemented(lang)) => {
                        // Language is recognised but tld doesn't have a parser
                        // for it yet — skip silently during directory walks.
                        let _ = lang;
                    }
                    Err(TldError::ParserDownloadRequired { lang, reason }) => {
                        // Surface download errors so the CLI can prompt the user.
                        return Err(TldError::ParserDownloadRequired { lang, reason });
                    }
                    Err(e) => {
                        // Skip files that fail to parse for other reasons.
                        let _ = e;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Rules, TreeSitterService};

    #[test]
    fn extract_path_syntax_returns_files_and_decls() {
        let service = TreeSitterService::new();
        let rules = Rules::new(Vec::new());

        let syntax = service
            .extract_path_syntax("tests/test-codebase/typescript", &rules, "typescript", None)
            .expect("syntax extraction should succeed");

        assert!(
            !syntax.files.is_empty(),
            "syntax bundle should contain files"
        );
        assert!(
            syntax.files.iter().any(|file| !file.decls.is_empty()),
            "syntax bundle should contain declarations"
        );
        assert!(
            syntax.files.iter().any(|file| !file.refs.is_empty()),
            "syntax bundle should contain refs"
        );
    }
}
