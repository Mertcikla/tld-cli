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

use crate::analyzer::queries;
use crate::analyzer::types::{AnalysisResult, Ref as AnalyzerRef, Symbol};
use std::path::Path;
use std::sync::OnceLock;
use tree_sitter::{Query, QueryCursor, StreamingIterator};
use ts_pack_core::get_language;

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
    let blocks = extract_control_regions(path, &language, &decls);

    SyntaxFile {
        path: path.to_string(),
        repo_name: repo_name.to_string(),
        language,
        decls,
        refs,
        blocks,
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

struct TsControlQuery {
    query: Query,
    branch_idx: u32,
    loop_idx: u32,
    try_idx: u32,
    return_idx: u32,
}

static TS_CONTROL_QUERY: OnceLock<Option<TsControlQuery>> = OnceLock::new();
static JS_CONTROL_QUERY: OnceLock<Option<TsControlQuery>> = OnceLock::new();

struct PyControlQuery {
    query: Query,
    branch_idx: u32,
    loop_idx: u32,
    try_idx: u32,
    return_idx: u32,
}

static PY_CONTROL_QUERY: OnceLock<Option<PyControlQuery>> = OnceLock::new();

struct GoControlQuery {
    query: Query,
    branch_idx: u32,
    loop_idx: u32,
    return_idx: u32,
}

static GO_CONTROL_QUERY: OnceLock<Option<GoControlQuery>> = OnceLock::new();

struct JavaControlQuery {
    query: Query,
    branch_idx: u32,
    loop_idx: u32,
    try_idx: u32,
    return_idx: u32,
}

static JAVA_CONTROL_QUERY: OnceLock<Option<JavaControlQuery>> = OnceLock::new();

fn extract_control_regions(path: &str, language: &str, decls: &[SyntaxDecl]) -> Vec<ControlRegion> {
    match language {
        "typescript" | "javascript" => extract_ts_control_regions(path, decls),
        "python" => extract_py_control_regions(path, decls),
        "go" => extract_go_control_regions(path, decls),
        "java" => extract_java_control_regions(path, decls),
        _ => Vec::new(),
    }
}

fn extract_ts_control_regions(path: &str, decls: &[SyntaxDecl]) -> Vec<ControlRegion> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let lang_key = if has_extension(path, "js") || has_extension(path, "jsx") {
        "javascript"
    } else {
        "typescript"
    };
    let Ok(language) = get_language(lang_key) else {
        return Vec::new();
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&source, None) else {
        return Vec::new();
    };

    let control_store = if lang_key == "javascript" {
        &JS_CONTROL_QUERY
    } else {
        &TS_CONTROL_QUERY
    };

    let control_src = if lang_key == "javascript" {
        queries::javascript_control()
    } else {
        queries::typescript_control()
    };

    let Some(control) = control_store
        .get_or_init(|| {
            let Ok(query) = Query::new(&language, control_src) else {
                return None;
            };
            Some(TsControlQuery {
                branch_idx: query.capture_index_for_name("branch").unwrap_or(u32::MAX),
                loop_idx: query.capture_index_for_name("loop").unwrap_or(u32::MAX),
                try_idx: query.capture_index_for_name("try").unwrap_or(u32::MAX),
                return_idx: query.capture_index_for_name("return").unwrap_or(u32::MAX),
                query,
            })
        })
        .as_ref()
    else {
        return Vec::new();
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&control.query, tree.root_node(), source.as_bytes());
    let mut blocks = Vec::new();

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let Some(owner_local_id) = owner_local_id_for_capture(decls, &capture.node) else {
                continue;
            };

            let kind = match capture.index {
                idx if idx == control.branch_idx => ControlKind::Branch,
                idx if idx == control.loop_idx => ControlKind::Loop,
                idx if idx == control.try_idx => ControlKind::TryCatch,
                idx if idx == control.return_idx => ControlKind::EarlyReturn,
                _ => continue,
            };

            let Some(span) = span_from_node(&capture.node) else {
                continue;
            };

            blocks.push(ControlRegion {
                kind,
                span,
                owner_local_id: Some(owner_local_id),
            });
        }
    }

    blocks
}

fn extract_py_control_regions(path: &str, decls: &[SyntaxDecl]) -> Vec<ControlRegion> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let Ok(language) = get_language("python") else {
        return Vec::new();
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&source, None) else {
        return Vec::new();
    };

    let Some(control) = PY_CONTROL_QUERY
        .get_or_init(|| {
            let query_src = queries::python_control();
            let Ok(query) = Query::new(&language, query_src) else {
                return None;
            };
            Some(PyControlQuery {
                branch_idx: query.capture_index_for_name("branch").unwrap_or(u32::MAX),
                loop_idx: query.capture_index_for_name("loop").unwrap_or(u32::MAX),
                try_idx: query.capture_index_for_name("try").unwrap_or(u32::MAX),
                return_idx: query.capture_index_for_name("return").unwrap_or(u32::MAX),
                query,
            })
        })
        .as_ref()
    else {
        return Vec::new();
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&control.query, tree.root_node(), source.as_bytes());
    let mut blocks = Vec::new();

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let Some(owner_local_id) = owner_local_id_for_capture(decls, &capture.node) else {
                continue;
            };

            let kind = match capture.index {
                idx if idx == control.branch_idx => ControlKind::Branch,
                idx if idx == control.loop_idx => ControlKind::Loop,
                idx if idx == control.try_idx => ControlKind::TryCatch,
                idx if idx == control.return_idx => ControlKind::EarlyReturn,
                _ => continue,
            };

            let Some(span) = span_from_node(&capture.node) else {
                continue;
            };

            blocks.push(ControlRegion {
                kind,
                span,
                owner_local_id: Some(owner_local_id),
            });
        }
    }

    blocks
}

fn extract_go_control_regions(path: &str, decls: &[SyntaxDecl]) -> Vec<ControlRegion> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let Ok(language) = get_language("go") else {
        return Vec::new();
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&source, None) else {
        return Vec::new();
    };

    let Some(control) = GO_CONTROL_QUERY
        .get_or_init(|| {
            let query_src = queries::go_control();
            let Ok(query) = Query::new(&language, query_src) else {
                return None;
            };
            Some(GoControlQuery {
                branch_idx: query.capture_index_for_name("branch").unwrap_or(u32::MAX),
                loop_idx: query.capture_index_for_name("loop").unwrap_or(u32::MAX),
                return_idx: query.capture_index_for_name("return").unwrap_or(u32::MAX),
                query,
            })
        })
        .as_ref()
    else {
        return Vec::new();
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&control.query, tree.root_node(), source.as_bytes());
    let mut blocks = Vec::new();

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let Some(owner_local_id) = owner_local_id_for_capture(decls, &capture.node) else {
                continue;
            };

            let kind = match capture.index {
                idx if idx == control.branch_idx => ControlKind::Branch,
                idx if idx == control.loop_idx => ControlKind::Loop,
                idx if idx == control.return_idx => ControlKind::EarlyReturn,
                _ => continue,
            };

            let Some(span) = span_from_node(&capture.node) else {
                continue;
            };

            blocks.push(ControlRegion {
                kind,
                span,
                owner_local_id: Some(owner_local_id),
            });
        }
    }

    blocks
}

fn extract_java_control_regions(path: &str, decls: &[SyntaxDecl]) -> Vec<ControlRegion> {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let Ok(language) = get_language("java") else {
        return Vec::new();
    };

    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(&source, None) else {
        return Vec::new();
    };

    let Some(control) = JAVA_CONTROL_QUERY
        .get_or_init(|| {
            let query_src = queries::java_control();
            let Ok(query) = Query::new(&language, query_src) else {
                return None;
            };
            Some(JavaControlQuery {
                branch_idx: query.capture_index_for_name("branch").unwrap_or(u32::MAX),
                loop_idx: query.capture_index_for_name("loop").unwrap_or(u32::MAX),
                try_idx: query.capture_index_for_name("try").unwrap_or(u32::MAX),
                return_idx: query.capture_index_for_name("return").unwrap_or(u32::MAX),
                query,
            })
        })
        .as_ref()
    else {
        return Vec::new();
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&control.query, tree.root_node(), source.as_bytes());
    let mut blocks = Vec::new();

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let Some(owner_local_id) = owner_local_id_for_capture(decls, &capture.node) else {
                continue;
            };

            let kind = match capture.index {
                idx if idx == control.branch_idx => ControlKind::Branch,
                idx if idx == control.loop_idx => ControlKind::Loop,
                idx if idx == control.try_idx => ControlKind::TryCatch,
                idx if idx == control.return_idx => ControlKind::EarlyReturn,
                _ => continue,
            };

            let Some(span) = span_from_node(&capture.node) else {
                continue;
            };

            blocks.push(ControlRegion {
                kind,
                span,
                owner_local_id: Some(owner_local_id),
            });
        }
    }

    blocks
}

fn owner_local_id_for_capture(decls: &[SyntaxDecl], node: &tree_sitter::Node) -> Option<String> {
    let line = line_from_row(node.start_position().row)?;

    decls
        .iter()
        .filter(|decl| decl.contains_line(line))
        .min_by_key(|decl| decl.body_lines())
        .map(|decl| decl.local_id.clone())
}

fn has_extension(path: &str, ext: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(ext))
}

fn line_from_row(row: usize) -> Option<u32> {
    let row = u32::try_from(row).ok()?;
    row.checked_add(1)
}

fn span_from_node(node: &tree_sitter::Node) -> Option<LineSpan> {
    Some(LineSpan {
        start: line_from_row(node.start_position().row)?,
        end: line_from_row(node.end_position().row)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::TreeSitterService;

    #[test]
    fn typescript_bridge_extracts_control_regions() {
        let result =
            TreeSitterService::extract_file("tests/test-codebase/typescript/src/services/order.ts")
                .expect("typescript fixture should parse");

        let syntax = from_analysis_result(&result, "typescript");
        let file = syntax
            .files
            .iter()
            .find(|file| file.path.ends_with("src/services/order.ts"))
            .expect("order.ts syntax file should exist");
        let place_order_local_id = file
            .decls
            .iter()
            .find(|decl| decl.name == "placeOrder")
            .map(|decl| decl.local_id.as_str())
            .expect("placeOrder decl should exist");

        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Loop),
            "typescript bridge should extract loops"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Branch),
            "typescript bridge should extract branches"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::TryCatch),
            "typescript bridge should extract try/catch regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::EarlyReturn),
            "typescript bridge should extract return regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.owner_local_id.as_deref() == Some(place_order_local_id)),
            "control regions should be attached to the owning declaration"
        );
    }

    #[test]
    fn python_bridge_extracts_control_regions() {
        let result = TreeSitterService::extract_file("tests/test-codebase/python/app/services.py")
            .expect("python fixture should parse");

        let syntax = from_analysis_result(&result, "python");
        let file = syntax
            .files
            .iter()
            .find(|file| file.path.ends_with("app/services.py"))
            .expect("services.py syntax file should exist");
        let place_order_local_id = file
            .decls
            .iter()
            .find(|decl| decl.name == "place_order")
            .map(|decl| decl.local_id.as_str())
            .expect("place_order decl should exist");

        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Loop),
            "python bridge should extract loops"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Branch),
            "python bridge should extract branches"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::TryCatch),
            "python bridge should extract try/catch regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::EarlyReturn),
            "python bridge should extract return regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.owner_local_id.as_deref() == Some(place_order_local_id)),
            "python control regions should be attached to the owning declaration"
        );
    }

    #[test]
    fn go_bridge_extracts_control_regions() {
        let result = TreeSitterService::extract_file(
            "tests/test-codebase/go/internal/service/order_service.go",
        )
        .expect("go fixture should parse");

        let syntax = from_analysis_result(&result, "go");
        let file = syntax
            .files
            .iter()
            .find(|file| file.path.ends_with("internal/service/order_service.go"))
            .expect("order_service.go syntax file should exist");
        let place_order_local_id = file
            .decls
            .iter()
            .find(|decl| decl.name == "PlaceOrder")
            .map(|decl| decl.local_id.as_str())
            .expect("PlaceOrder decl should exist");

        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Loop),
            "go bridge should extract loops"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Branch),
            "go bridge should extract branches"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::EarlyReturn),
            "go bridge should extract return regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.owner_local_id.as_deref() == Some(place_order_local_id)),
            "go control regions should be attached to the owning declaration"
        );
    }

    #[test]
    fn java_bridge_extracts_control_regions() {
        let result = TreeSitterService::extract_file(
            "tests/test-codebase/java/src/main/java/com/example/ecommerce/service/OrderService.java",
        )
        .expect("java fixture should parse");

        let syntax = from_analysis_result(&result, "java");
        let file = syntax
            .files
            .iter()
            .find(|file| {
                file.path
                    .ends_with("src/main/java/com/example/ecommerce/service/OrderService.java")
            })
            .expect("OrderService.java syntax file should exist");
        let place_order_local_id = file
            .decls
            .iter()
            .find(|decl| decl.name == "placeOrder")
            .map(|decl| decl.local_id.as_str())
            .expect("placeOrder decl should exist");

        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Loop),
            "java bridge should extract loops"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::Branch),
            "java bridge should extract branches"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::TryCatch),
            "java bridge should extract try/catch regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.kind == ControlKind::EarlyReturn),
            "java bridge should extract return regions"
        );
        assert!(
            file.blocks
                .iter()
                .any(|block| block.owner_local_id.as_deref() == Some(place_order_local_id)),
            "java control regions should be attached to the owning declaration"
        );
    }
}
