#![allow(dead_code)]
//! Syntax extraction layer.
//!
//! Provides `SyntaxBundle` (the richer intermediate representation) and a
//! compatibility bridge that converts the existing `AnalysisResult` produced
//! by the current tree-sitter parsers into a `SyntaxBundle`. This bridge
//! lets downstream semantic stages work with the new types while the parsers
//! continue to be migrated one at a time.

pub mod types;
pub use types::*;

use crate::analyzer::types::{AnalysisResult, Ref as AnalyzerRef, Symbol};

/// Convert a legacy `AnalysisResult` into a `SyntaxBundle`.
///
/// This is the compatibility bridge used while per-language parsers are
/// migrated to emit `SyntaxBundle` data directly. No information is invented;
/// only type-level translations are performed.
pub fn from_analysis_result(result: &AnalysisResult, repo_name: &str) -> SyntaxBundle {
    // Collect all unique file paths mentioned in symbols and refs.
    let mut all_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for sym in &result.symbols {
        if !sym.file_path.is_empty() {
            all_files.insert(sym.file_path.clone());
        }
    }
    for r in &result.refs {
        if !r.file_path.is_empty() {
            all_files.insert(r.file_path.clone());
        }
    }

    let files = all_files
        .into_iter()
        .map(|path| build_syntax_file(&path, repo_name, result))
        .collect();

    SyntaxBundle { files }
}

fn build_syntax_file(path: &str, repo_name: &str, result: &AnalysisResult) -> SyntaxFile {
    let language = detect_language_from_ext(path);

    let file_syms: Vec<&Symbol> = result
        .symbols
        .iter()
        .filter(|s| s.file_path == path)
        .collect();

    let file_refs: Vec<&AnalyzerRef> = result.refs.iter().filter(|r| r.file_path == path).collect();

    let decls = convert_symbols(&file_syms);
    let refs = convert_refs(&file_refs, &decls);

    SyntaxFile {
        path: path.to_string(),
        repo_name: repo_name.to_string(),
        language,
        decls,
        refs,
        blocks: vec![],
    }
}

fn detect_language_from_ext(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "go" => "go",
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "java" => "java",
        "cpp" | "cc" | "cxx" | "h" | "hpp" => "cpp",
        _ => "unknown",
    }
    .to_string()
}

fn convert_symbols(symbols: &[&Symbol]) -> Vec<SyntaxDecl> {
    symbols
        .iter()
        .enumerate()
        .map(|(i, sym)| {
            let kind = DeclKind::from_str(&sym.kind);
            // Represent the owning class/struct as a local_id reference.
            let parent_local_id = if sym.parent.is_empty() {
                None
            } else {
                // Find the parent decl among preceding symbols.
                Some(parent_local_id_for(sym.parent.as_str(), symbols))
            };
            SyntaxDecl {
                local_id: format!("{}:{}", i, sym.name),
                name: sym.name.clone(),
                kind,
                parent_local_id,
                span: LineSpan {
                    start: sym.line.cast_unsigned(),
                    end: sym.end_line.max(sym.line).cast_unsigned(),
                },
                signature_span: LineSpan {
                    start: sym.line.cast_unsigned(),
                    end: sym.line.cast_unsigned(),
                },
                description: sym.description.clone(),
            }
        })
        .collect()
}

/// Synthesise a local_id for a parent symbol given its name.
fn parent_local_id_for(parent_name: &str, all: &[&Symbol]) -> String {
    all.iter()
        .enumerate()
        .find(|(_, s)| s.name == parent_name)
        .map_or_else(
            || format!("parent:{parent_name}"),
            |(i, s)| format!("{}:{}", i, s.name),
        )
}

fn convert_refs(refs: &[&AnalyzerRef], decls: &[SyntaxDecl]) -> Vec<SyntaxRef> {
    refs.iter()
        .enumerate()
        .map(|(i, r)| {
            let kind = RefKind::from_str(&r.kind);
            // Find the narrowest owning decl by line.
            let owner_local_id = decls
                .iter()
                .filter(|d| d.contains_line(r.line.cast_unsigned()))
                .min_by_key(|d| d.body_lines())
                .map(|d| d.local_id.clone());

            SyntaxRef {
                owner_local_id,
                kind,
                text: r.name.clone(),
                span: LineColSpan {
                    start_line: r.line.cast_unsigned(),
                    start_col: r.column.cast_unsigned(),
                    end_line: r.line.cast_unsigned(),
                    end_col: r.column.cast_unsigned(),
                },
                order_index: i,
                role: RefRole::Unknown,
                resolved_target_path: r.target_path.clone(),
            }
        })
        .collect()
}
