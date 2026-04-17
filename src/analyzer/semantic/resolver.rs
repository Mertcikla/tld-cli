#![allow(dead_code)]
//! Semantic resolver — converts syntax facts into `SemanticBundle`.
//!
//! Builds stable SymbolIds, resolves call edges using LSP `target_path` when available,
//! and falls back to bare-name matching. Unresolved refs are kept explicitly.

use super::types::{
    ControlMetrics, EdgeKind, EdgeOrigin, EdgeTarget, SemanticBundle, SemanticEdge, SemanticSymbol,
    SymbolId, SymbolSpans, UnresolvedRef, Visibility,
};
use crate::analyzer::syntax::{
    self,
    types::{ControlKind, RefKind, SyntaxBundle, SyntaxDecl},
};
use crate::analyzer::types::AnalysisResult;
use crate::workspace::slugify;
use std::collections::HashMap;
use std::path::Path;

/// Build a `SemanticBundle` from an `AnalysisResult`.
///
/// `scan_parent` is the parent of the scanned root directory (used to compute
/// relative file paths that form part of stable symbol IDs).
pub fn resolve(result: &AnalysisResult, repo_name: &str, scan_parent: &str) -> SemanticBundle {
    let syntax = syntax::from_analysis_result(result, repo_name);
    resolve_syntax(&syntax, scan_parent)
}

/// Build a `SemanticBundle` from the syntax IR.
pub fn resolve_syntax(bundle: &SyntaxBundle, scan_parent: &str) -> SemanticBundle {
    // ── 1. Build an index from (file_rel, name) → SymbolId ──────────────────
    let mut id_index: HashMap<(String, String), SymbolId> = HashMap::new();
    let mut local_id_to_symbol_id: HashMap<(String, String), SymbolId> = HashMap::new();
    let mut control_by_local_id: HashMap<(String, String), ControlMetrics> = HashMap::new();

    for file in &bundle.files {
        let file_rel = rel_from_base(&file.path, scan_parent);
        for block in &file.blocks {
            let Some(owner_local_id) = block.owner_local_id.as_ref() else {
                continue;
            };
            let metrics = control_by_local_id
                .entry((file_rel.clone(), owner_local_id.clone()))
                .or_default();
            match block.kind {
                ControlKind::Branch => metrics.branches += 1,
                ControlKind::Loop => metrics.loops += 1,
                ControlKind::TryCatch => metrics.tries += 1,
                ControlKind::EarlyReturn => metrics.early_returns += 1,
            }
        }
    }

    let symbols: Vec<SemanticSymbol> = bundle
        .files
        .iter()
        .flat_map(|file| {
            let file_rel = rel_from_base(&file.path, scan_parent);
            file.decls
                .iter()
                .map(move |decl| (file, file_rel.clone(), decl))
        })
        .map(|(file, file_rel, decl)| {
            let sym_id = make_symbol_id_from_decl(&file.repo_name, &file_rel, decl, &file.decls);
            id_index.insert((file_rel.clone(), decl.name.clone()), sym_id.clone());
            local_id_to_symbol_id.insert((file_rel.clone(), decl.local_id.clone()), sym_id.clone());

            SemanticSymbol {
                symbol_id: sym_id,
                repo_name: file.repo_name.clone(),
                file_path: file_rel.clone(),
                name: decl.name.clone(),
                kind: decl.kind.clone(),
                owner: None,
                visibility: Visibility::Unknown,
                external: false,
                description: decl.description.clone(),
                spans: SymbolSpans {
                    body_start: decl.span.start,
                    body_end: decl.span.end.max(decl.span.start),
                    sig_line: decl.signature_span.start,
                },
                control: control_by_local_id
                    .get(&(file_rel.clone(), decl.local_id.clone()))
                    .cloned()
                    .unwrap_or_default(),
                annotations: decl.annotations.clone(),
            }
        })
        .collect();

    // ── 2. Fill owner references ─────────────────────────────────────────────
    let symbols: Vec<SemanticSymbol> = symbols
        .into_iter()
        .map(|mut sem| {
            if let Some(file) = bundle
                .files
                .iter()
                .find(|file| rel_from_base(&file.path, scan_parent) == sem.file_path)
                && let Some(decl) = file.decls.iter().find(|decl| {
                    make_symbol_id_from_decl(&file.repo_name, &sem.file_path, decl, &file.decls)
                        == sem.symbol_id
                })
                && let Some(parent_local_id) = &decl.parent_local_id
            {
                sem.owner = local_id_to_symbol_id
                    .get(&(sem.file_path.clone(), parent_local_id.clone()))
                    .cloned();
            }
            sem
        })
        .collect();

    // ── 3. Build a name-based lookup for fallback resolution ─────────────────
    // name → list of SymbolIds (multiple files can define the same name)
    let mut name_to_ids: HashMap<String, Vec<SymbolId>> = HashMap::new();
    for sym in &symbols {
        name_to_ids
            .entry(sym.name.clone())
            .or_default()
            .push(sym.symbol_id.clone());
    }

    // Slug → SymbolId for the fallback that mirrors workspace_builder's slug matching.
    let mut slug_to_id: HashMap<String, SymbolId> = HashMap::new();
    for sym in &symbols {
        let slug = slugify(&sym.name);
        slug_to_id
            .entry(slug)
            .or_insert_with(|| sym.symbol_id.clone());
    }

    // ── 4. Build source-symbol lookup by (file_rel, line) ────────────────────
    // Used to find which symbol owns a given call site.
    let sym_by_file_rel: HashMap<String, Vec<&SemanticSymbol>> = {
        let mut m: HashMap<String, Vec<&SemanticSymbol>> = HashMap::new();
        for sym in &symbols {
            m.entry(sym.file_path.clone()).or_default().push(sym);
        }
        m
    };

    // ── 5. Convert refs into edges ───────────────────────────────────────────
    let mut edges: Vec<SemanticEdge> = Vec::new();
    let mut unresolved: Vec<UnresolvedRef> = Vec::new();

    for (idx, (file, r)) in bundle
        .files
        .iter()
        .flat_map(|file| file.refs.iter().map(move |r| (file, r)))
        .enumerate()
    {
        let src_rel = rel_from_base(&file.path, scan_parent);
        let source_id = r
            .owner_local_id
            .as_ref()
            .and_then(|owner_local_id| {
                local_id_to_symbol_id
                    .get(&(src_rel.clone(), owner_local_id.clone()))
                    .cloned()
            })
            .or_else(|| find_containing_symbol_id(&sym_by_file_rel, &src_rel, r.span.start_line));
        let Some(source_id) = source_id else { continue };

        let edge_kind = match r.kind {
            RefKind::Import => EdgeKind::Imports,
            RefKind::Construct => EdgeKind::Constructs,
            RefKind::Read => EdgeKind::Reads,
            RefKind::Write => EdgeKind::Writes,
            RefKind::Return => EdgeKind::Returns,
            RefKind::Throw => EdgeKind::Throws,
            RefKind::Call => EdgeKind::Calls,
        };

        // Try to resolve target.
        if !r.resolved_target_path.is_empty() {
            // LSP or import gave us a target file.
            let tgt_rel = rel_from_base(&r.resolved_target_path, scan_parent);
            if let Some(target_id) = id_index.get(&(tgt_rel.clone(), r.text.clone())) {
                let cross = target_rel_differs_file(&source_id, target_id);
                edges.push(SemanticEdge {
                    source: source_id,
                    target: EdgeTarget::Resolved(target_id.clone()),
                    kind: edge_kind,
                    origin: if matches!(r.kind, RefKind::Import) {
                        EdgeOrigin::Import
                    } else {
                        EdgeOrigin::Lsp
                    },
                    order_index: idx,
                    cross_boundary: cross,
                });
                continue;
            }

            // Target file is known but we didn't find the exact symbol — still useful
            // as a cross-file edge if we can find any symbol in that file.
            if let Some(sym_list) = sym_by_file_rel
                .iter()
                .find(|(k, _)| k.as_str() == tgt_rel)
                .map(|(_, v)| v)
                && let Some(target_sym) = sym_list.first()
            {
                edges.push(SemanticEdge {
                    source: source_id.clone(),
                    target: EdgeTarget::Resolved(target_sym.symbol_id.clone()),
                    kind: edge_kind,
                    origin: if matches!(r.kind, RefKind::Import) {
                        EdgeOrigin::Import
                    } else {
                        EdgeOrigin::Lsp
                    },
                    order_index: idx,
                    cross_boundary: true,
                });
                continue;
            }
        }

        // Fallback: bare name matching.
        if let Some(target_ids) = name_to_ids.get(&r.text)
            && let Some(target_id) = target_ids.first()
            && target_id != &source_id
        {
            let cross = target_rel_differs_file(&source_id, target_id);
            edges.push(SemanticEdge {
                source: source_id,
                target: EdgeTarget::Resolved(target_id.clone()),
                kind: edge_kind,
                origin: EdgeOrigin::BareName,
                order_index: idx,
                cross_boundary: cross,
            });
            continue;
        }

        // Slug fallback.
        let slug = slugify(&r.text);
        if let Some(target_id) = slug_to_id.get(&slug)
            && target_id != &source_id
        {
            let cross = target_rel_differs_file(&source_id, target_id);
            edges.push(SemanticEdge {
                source: source_id,
                target: EdgeTarget::Resolved(target_id.clone()),
                kind: edge_kind,
                origin: EdgeOrigin::BareName,
                order_index: idx,
                cross_boundary: cross,
            });
            continue;
        }

        // Nothing worked — record as unresolved.
        unresolved.push(UnresolvedRef {
            source: source_id,
            text: r.text.clone(),
            kind: edge_kind,
        });
    }

    SemanticBundle {
        symbols,
        edges,
        unresolved_refs: unresolved,
    }
}

fn make_symbol_id_from_decl(
    repo_name: &str,
    file_rel: &str,
    decl: &SyntaxDecl,
    decls: &[SyntaxDecl],
) -> SymbolId {
    if let Some(parent_local_id) = &decl.parent_local_id
        && let Some(parent_decl) = decls
            .iter()
            .find(|candidate| &candidate.local_id == parent_local_id)
    {
        return format!("{repo_name}:{file_rel}:{}::{}", parent_decl.name, decl.name);
    }
    format!("{repo_name}:{file_rel}:{}", decl.name)
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn rel_from_base(abs: &str, base: &str) -> String {
    if base.is_empty() {
        return abs.to_string();
    }
    Path::new(abs).strip_prefix(Path::new(base)).map_or_else(
        |_| abs.to_string(),
        |p| p.to_str().unwrap_or(abs).to_string(),
    )
}

/// Find the symbol ID of the declaration that contains the given line in the given file.
fn find_containing_symbol_id(
    sym_by_file_rel: &HashMap<String, Vec<&SemanticSymbol>>,
    file_rel: &str,
    line: u32,
) -> Option<SymbolId> {
    let syms = sym_by_file_rel.get(file_rel)?;
    syms.iter()
        .filter(|s| {
            s.spans.body_start <= line && (s.spans.body_end == 0 || line <= s.spans.body_end)
        })
        .min_by_key(|s| {
            if s.spans.body_end >= s.spans.body_start {
                s.spans.body_end - s.spans.body_start
            } else {
                u32::MAX
            }
        })
        .map(|s| s.symbol_id.clone())
}

/// True when the two symbol IDs are in different source files.
fn target_rel_differs_file(source_id: &str, target_id: &str) -> bool {
    // SymbolId format: `{repo}:{file_rel}:{...}`
    let src_file = id_file_part(source_id);
    let tgt_file = id_file_part(target_id);
    src_file != tgt_file
}

fn id_file_part(id: &str) -> &str {
    // Split on the first colon (repo name), then take up to the next
    // segment that is the file rel — everything between the first and
    // last colon separators.
    let mut parts = id.splitn(3, ':');
    parts.next(); // repo
    parts.next().unwrap_or("") // file_rel
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::syntax::types::{
        ControlKind, ControlRegion, DeclKind, LineColSpan, LineSpan, RefRole, SyntaxBundle,
        SyntaxDecl, SyntaxFile, SyntaxRef,
    };

    #[test]
    fn resolve_syntax_preserves_owner_and_call_resolution() {
        let bundle = SyntaxBundle {
            files: vec![SyntaxFile {
                path: "/tmp/repo/src/order.ts".to_string(),
                repo_name: "repo".to_string(),
                language: "typescript".to_string(),
                decls: vec![
                    SyntaxDecl {
                        local_id: "class:OrderService".to_string(),
                        name: "OrderService".to_string(),
                        kind: DeclKind::Class,
                        parent_local_id: None,
                        span: LineSpan { start: 1, end: 20 },
                        signature_span: LineSpan { start: 1, end: 1 },
                        description: String::new(),
                        annotations: Vec::new(),
                    },
                    SyntaxDecl {
                        local_id: "method:placeOrder".to_string(),
                        name: "placeOrder".to_string(),
                        kind: DeclKind::Method,
                        parent_local_id: Some("class:OrderService".to_string()),
                        span: LineSpan { start: 3, end: 10 },
                        signature_span: LineSpan { start: 3, end: 3 },
                        description: String::new(),
                        annotations: Vec::new(),
                    },
                    SyntaxDecl {
                        local_id: "fn:charge".to_string(),
                        name: "charge".to_string(),
                        kind: DeclKind::Function,
                        parent_local_id: None,
                        span: LineSpan { start: 12, end: 14 },
                        signature_span: LineSpan { start: 12, end: 12 },
                        description: String::new(),
                        annotations: Vec::new(),
                    },
                ],
                refs: vec![SyntaxRef {
                    owner_local_id: Some("method:placeOrder".to_string()),
                    kind: RefKind::Call,
                    text: "charge".to_string(),
                    receiver: String::new(),
                    span: LineColSpan {
                        start_line: 5,
                        start_col: 8,
                        end_line: 5,
                        end_col: 14,
                    },
                    order_index: 0,
                    role: RefRole::Unknown,
                    resolved_target_path: String::new(),
                }],
                blocks: vec![
                    ControlRegion {
                        kind: ControlKind::Loop,
                        span: LineSpan { start: 4, end: 8 },
                        owner_local_id: Some("method:placeOrder".to_string()),
                    },
                    ControlRegion {
                        kind: ControlKind::Branch,
                        span: LineSpan { start: 5, end: 6 },
                        owner_local_id: Some("method:placeOrder".to_string()),
                    },
                ],
            }],
        };

        let semantic = resolve_syntax(&bundle, "/tmp/repo");
        let place_order = semantic
            .symbols
            .iter()
            .find(|symbol| symbol.name == "placeOrder")
            .expect("placeOrder symbol should exist");
        let charge = semantic
            .symbols
            .iter()
            .find(|symbol| symbol.name == "charge")
            .expect("charge symbol should exist");

        assert_eq!(
            place_order.owner.as_deref(),
            Some("repo:src/order.ts:OrderService")
        );
        assert_eq!(place_order.control.loops, 1);
        assert_eq!(place_order.control.branches, 1);
        assert!(semantic.unresolved_refs.is_empty());
        assert!(semantic.edges.iter().any(|edge| {
            edge.source == place_order.symbol_id
                && edge.target.resolved_id() == Some(charge.symbol_id.as_str())
                && edge.kind == EdgeKind::Calls
        }));
    }
}
