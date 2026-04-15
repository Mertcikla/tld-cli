pub mod ignore;
pub mod parsers;
pub mod types;

use crate::error::TldError;
pub use ignore::Rules;
use std::fs;
use std::path::Path;
use tree_sitter_language_pack::get_language;
pub use types::*;

pub trait Service {
    fn extract_path(
        &self,
        path: &str,
        rules: &Rules,
        on_entry: Option<&dyn Fn(&str, bool)>,
    ) -> Result<AnalysisResult, TldError>;
}

pub struct TreeSitterService {}

impl TreeSitterService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn extract_file(&self, path: &str) -> Result<AnalysisResult, TldError> {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // Map extension to tree-sitter language identifier string
        let lang_name = match extension {
            "go" => "go",
            "py" => "python",
            "rs" => "rust",
            "java" => "java",
            "ts" | "tsx" | "mts" | "cts" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
            "c" | "h" => "c",
            _ => {
                return Err(TldError::Generic(format!(
                    "Unsupported language for extension: {}",
                    extension
                )));
            }
        };

        // On-demand download and load via the pack's global registry
        let language = get_language(lang_name).map_err(|e| {
            TldError::Generic(format!("Failed to load language {}: {}", lang_name, e))
        })?;

        let source = fs::read_to_string(path)?;
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&language)
            .map_err(|e| TldError::Generic(e.to_string()))?;

        let tree = parser
            .parse(&source, None)
            .ok_or_else(|| TldError::Generic(format!("Failed to parse {}", path)))?;

        let mut result = AnalysisResult::default();
        let technology = lang_name.to_string();

        // Dispatch to specific parser implementation
        match lang_name {
            "go" => {
                parsers::go::parse(&tree.root_node(), source.as_bytes(), path, &mut result);
            }
            // Add other languages here later
            _ => {
                return Err(TldError::Generic(format!(
                    "Parser logic not yet implemented for {}",
                    technology
                )));
            }
        }

        for sym in &mut result.symbols {
            sym.technology = technology.clone();
        }

        Ok(result)
    }
}

impl Service for TreeSitterService {
    fn extract_path(
        &self,
        path: &str,
        rules: &Rules,
        on_entry: Option<&dyn Fn(&str, bool)>,
    ) -> Result<AnalysisResult, TldError> {
        let metadata = fs::metadata(path)?;
        if metadata.is_dir() {
            self.extract_dir(path, rules, on_entry)
        } else {
            if rules.should_ignore_path(path) {
                return Ok(AnalysisResult::default());
            }
            if let Some(cb) = on_entry {
                cb(path, false);
            }
            self.extract_file(path)
        }
    }
}

impl TreeSitterService {
    fn extract_dir(
        &self,
        root: &str,
        rules: &Rules,
        on_entry: Option<&dyn Fn(&str, bool)>,
    ) -> Result<AnalysisResult, TldError> {
        let mut merged = AnalysisResult::default();
        self.walk_dir(Path::new(root), root, rules, on_entry, &mut merged)?;
        Ok(merged)
    }

    fn walk_dir(
        &self,
        dir: &Path,
        root: &str,
        rules: &Rules,
        on_entry: Option<&dyn Fn(&str, bool)>,
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
                self.walk_dir(&path, root, rules, on_entry, merged)?;
            } else {
                if rules.should_ignore_path(rel_path) {
                    continue;
                }
                if let Some(cb) = on_entry {
                    cb(path.to_str().unwrap_or(""), false);
                }
                if let Ok(result) = self.extract_file(path.to_str().unwrap_or("")) {
                    merged.merge(result);
                }
            }
        }
        Ok(())
    }
}
