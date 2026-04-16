#![allow(dead_code)]
//! Semantic resolver — converts `AnalysisResult` into `SemanticBundle`.
//!
//! Builds stable SymbolIds, resolves call edges using LSP `target_path` when available,
//! and falls back to bare-name matching. Unresolved refs are kept explicitly.

use super::types::{
    EdgeKind, EdgeOrigin, EdgeTarget, SemanticBundle, SemanticEdge, SemanticSymbol, SymbolId,
    SymbolSpans, UnresolvedRef, Visibility,
};
use crate::analyzer::syntax::types::DeclKind;
use crate::analyzer::types::{AnalysisResult, Symbol};
use crate::workspace::slugify;
use std::collections::HashMap;
use std::path::Path;

/// Build a `SemanticBundle` from an `AnalysisResult`.
///
/// `scan_parent` is the parent of the scanned root directory (used to compute
/// relative file paths that form part of stable symbol IDs).
pub fn resolve(result: &AnalysisResult, repo_name: &str, scan_parent: &str) -> SemanticBundle {
    // ── 1. Build an index from (file_rel, name) → SymbolId ──────────────────
    let mut id_index: HashMap<(String, String), SymbolId> = HashMap::new();

    let symbols: Vec<SemanticSymbol> = result
        .symbols
        .iter()
        .map(|sym| {
            let file_rel = rel_from_base(&sym.file_path, scan_parent);
            let sym_id = make_symbol_id(repo_name, &file_rel, sym);
            id_index.insert((file_rel.clone(), sym.name.clone()), sym_id.clone());

            SemanticSymbol {
                symbol_id: sym_id,
                repo_name: repo_name.to_string(),
                file_path: file_rel,
                name: sym.name.clone(),
                kind: DeclKind::from_str(&sym.kind),
                owner: None, // Filled in a second pass below.
                visibility: Visibility::Unknown,
                external: false,
                description: sym.description.clone(),
                spans: SymbolSpans {
                    body_start: sym.line.cast_unsigned(),
                    body_end: sym.end_line.max(sym.line).cast_unsigned(),
                    sig_line: sym.line.cast_unsigned(),
                },
            }
        })
        .collect();

    // ── 2. Fill owner references ─────────────────────────────────────────────
    let symbols: Vec<SemanticSymbol> = symbols
        .into_iter()
        .zip(result.symbols.iter())
        .map(|(mut sem, raw)| {
            if !raw.parent.is_empty() {
                let owner_id = id_index.get(&(sem.file_path.clone(), raw.parent.clone()));
                sem.owner = owner_id.cloned();
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

    for (idx, r) in result.refs.iter().enumerate() {
        let src_rel = rel_from_base(&r.file_path, scan_parent);
        let source_id =
            find_containing_symbol_id(&sym_by_file_rel, &src_rel, r.line.cast_unsigned());
        let Some(source_id) = source_id else { continue };

        let edge_kind = match r.kind.as_str() {
            "import" => EdgeKind::Imports,
            _ => EdgeKind::Calls,
        };

        // Try to resolve target.
        if !r.target_path.is_empty() {
            // LSP or import gave us a target file.
            let tgt_rel = rel_from_base(&r.target_path, scan_parent);
            if let Some(target_id) = id_index.get(&(tgt_rel.clone(), r.name.clone())) {
                let cross = target_rel_differs_file(&source_id, target_id);
                edges.push(SemanticEdge {
                    source: source_id,
                    target: EdgeTarget::Resolved(target_id.clone()),
                    kind: edge_kind,
                    origin: if r.kind == "import" {
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
                    origin: EdgeOrigin::Import,
                    order_index: idx,
                    cross_boundary: true,
                });
                continue;
            }
        }

        // Fallback: bare name matching.
        if let Some(target_ids) = name_to_ids.get(&r.name)
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
        let slug = slugify(&r.name);
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
            text: r.name.clone(),
            kind: edge_kind,
        });
    }

    SemanticBundle {
        symbols,
        edges,
        unresolved_refs: unresolved,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn make_symbol_id(repo_name: &str, file_rel: &str, sym: &Symbol) -> SymbolId {
    if sym.parent.is_empty() {
        format!("{repo_name}:{file_rel}:{}", sym.name)
    } else {
        format!("{repo_name}:{file_rel}:{}::{}", sym.parent, sym.name)
    }
}

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
