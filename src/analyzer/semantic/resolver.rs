#![allow(dead_code)]
//! Semantic resolver — converts syntax facts into `SemanticBundle`.
//!
//! Builds stable SymbolIds, resolves call edges using LSP `target_path` when available,
//! and falls back to bare-name matching. Unresolved refs are kept explicitly.

use super::endpoints::detect_route_call;
use super::types::{
    ControlMetrics, EdgeKind, EdgeOrigin, EdgeTarget, SemanticBundle, SemanticEdge, SemanticSymbol,
    SymbolId, SymbolSpans, UnresolvedRef, Visibility,
};
use crate::analyzer::syntax::{
    self,
    types::{ControlKind, RefKind, SyntaxBundle, SyntaxDecl},
};
use crate::analyzer::types::AnalysisResult;
use crate::analyzer::types::Annotation;
use crate::workspace::slugify;
use std::collections::{HashMap, HashSet};
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

    let mut symbols: Vec<SemanticSymbol> = bundle
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
    symbols = symbols
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

    let mut symbol_index: HashMap<SymbolId, usize> = HashMap::new();
    for (idx, sym) in symbols.iter().enumerate() {
        symbol_index.insert(sym.symbol_id.clone(), idx);
    }

    // ── 3. Build a name-based lookup for fallback resolution ─────────────────
    // name → list of SymbolIds (multiple files can define the same name)
    let mut name_to_ids: HashMap<String, Vec<SymbolId>> = HashMap::new();
    for sym in &symbols {
        name_to_ids
            .entry(sym.name.clone())
            .or_default()
            .push(sym.symbol_id.clone());
    }

    // Slug → SymbolIds for the fallback that mirrors workspace_builder's slug matching.
    let mut slug_to_ids: HashMap<String, Vec<SymbolId>> = HashMap::new();
    for sym in &symbols {
        let slug = slugify(&sym.name);
        slug_to_ids
            .entry(slug)
            .or_default()
            .push(sym.symbol_id.clone());
    }

    let source_lines_by_rel: HashMap<String, Vec<String>> = bundle
        .files
        .iter()
        .map(|file| {
            let rel = rel_from_base(&file.path, scan_parent);
            let lines = std::fs::read_to_string(&file.path)
                .map(|text| text.lines().map(ToString::to_string).collect())
                .unwrap_or_default();
            (rel, lines)
        })
        .collect();

    // ── 5. Route-call endpoint stamping ──────────────────────────────────────
    for (file, r) in bundle
        .files
        .iter()
        .flat_map(|file| file.refs.iter().map(move |r| (file, r)))
    {
        if !matches!(r.kind, RefKind::Call) {
            continue;
        }
        let Some(method) = detect_route_call(&r.receiver, &r.text) else {
            continue;
        };

        let src_rel = rel_from_base(&file.path, scan_parent);
        let Some(lines) = source_lines_by_rel.get(&src_rel) else {
            continue;
        };
        let Some(line_text) = lines.get(r.span.start_line.saturating_sub(1) as usize) else {
            continue;
        };
        let Some((path_arg, handler_name)) = route_call_details(line_text, &r.text) else {
            continue;
        };
        let Some(target_id) =
            resolve_symbol_name(&handler_name, &src_rel, None, &name_to_ids, &slug_to_ids)
        else {
            continue;
        };
        let Some(target_idx) = symbol_index.get(&target_id).copied() else {
            continue;
        };

        let annotation_name = method.as_str().to_ascii_lowercase();
        let annotation_args = if path_arg.is_empty() {
            Vec::new()
        } else {
            vec![format!("\"{path_arg}\"")]
        };
        let already_stamped = symbols[target_idx]
            .annotations
            .iter()
            .any(|ann| ann.name.eq_ignore_ascii_case(&annotation_name));
        if !already_stamped {
            symbols[target_idx].annotations.push(Annotation {
                name: annotation_name,
                args: annotation_args,
            });
        }
    }

    // ── 6. Build source-symbol lookup by (file_rel, line) ────────────────────
    // Used to find which symbol owns a given call site.
    let sym_by_file_rel: HashMap<String, Vec<&SemanticSymbol>> = {
        let mut m: HashMap<String, Vec<&SemanticSymbol>> = HashMap::new();
        for sym in &symbols {
            m.entry(sym.file_path.clone()).or_default().push(sym);
        }
        m
    };

    // ── 7. Convert refs into edges ───────────────────────────────────────────
    let mut edges: Vec<SemanticEdge> = Vec::new();
    let mut unresolved: Vec<UnresolvedRef> = Vec::new();
    let mut seen_edges: HashSet<(String, String, String)> = HashSet::new();

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
                push_resolved_edge(
                    &mut edges,
                    &mut seen_edges,
                    ResolvedEdgeInput {
                        source: source_id.clone(),
                        target: target_id.clone(),
                        kind: edge_kind.clone(),
                        origin: if matches!(r.kind, RefKind::Import) {
                            EdgeOrigin::Import
                        } else {
                            EdgeOrigin::Lsp
                        },
                    },
                    idx,
                );
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
                push_resolved_edge(
                    &mut edges,
                    &mut seen_edges,
                    ResolvedEdgeInput {
                        source: source_id.clone(),
                        target: target_sym.symbol_id.clone(),
                        kind: edge_kind.clone(),
                        origin: if matches!(r.kind, RefKind::Import) {
                            EdgeOrigin::Import
                        } else {
                            EdgeOrigin::Lsp
                        },
                    },
                    idx,
                );
                continue;
            }
        }

        if let Some(target_id) = resolve_symbol_name(
            &r.text,
            &src_rel,
            Some(&source_id),
            &name_to_ids,
            &slug_to_ids,
        ) {
            push_resolved_edge(
                &mut edges,
                &mut seen_edges,
                ResolvedEdgeInput {
                    source: source_id.clone(),
                    target: target_id,
                    kind: edge_kind.clone(),
                    origin: EdgeOrigin::BareName,
                },
                idx,
            );
            continue;
        }

        // Nothing worked — record as unresolved.
        unresolved.push(UnresolvedRef {
            source: source_id,
            text: r.text.clone(),
            kind: edge_kind,
        });
    }

    // ── 8. Add dependency / inheritance hints from declaration signatures ───
    for file in &bundle.files {
        let file_rel = rel_from_base(&file.path, scan_parent);
        let Some(lines) = source_lines_by_rel.get(&file_rel) else {
            continue;
        };
        for decl in &file.decls {
            let Some(source_id) = local_id_to_symbol_id
                .get(&(file_rel.clone(), decl.local_id.clone()))
                .cloned()
            else {
                continue;
            };
            let edge_source = decl
                .parent_local_id
                .as_ref()
                .and_then(|parent_local_id| {
                    local_id_to_symbol_id
                        .get(&(file_rel.clone(), parent_local_id.clone()))
                        .cloned()
                })
                .unwrap_or_else(|| source_id.clone());

            let signature_text = decl_signature_text(lines, decl);
            let dependency_names = extract_dependency_names(&signature_text, &file.language);
            for dep_name in dependency_names {
                let Some(target_id) = resolve_symbol_name(
                    &dep_name,
                    &file_rel,
                    Some(&edge_source),
                    &name_to_ids,
                    &slug_to_ids,
                ) else {
                    unresolved.push(UnresolvedRef {
                        source: edge_source.clone(),
                        text: dep_name,
                        kind: EdgeKind::DependsOn,
                    });
                    continue;
                };
                push_resolved_edge(
                    &mut edges,
                    &mut seen_edges,
                    ResolvedEdgeInput {
                        source: edge_source.clone(),
                        target: target_id,
                        kind: EdgeKind::DependsOn,
                        origin: EdgeOrigin::BareName,
                    },
                    10_000 + decl.signature_span.start as usize,
                );
            }

            let (extends_names, implements_names) =
                extract_inheritance_names(&signature_text, &decl.name, &file.language, &decl.kind);
            for base_name in extends_names {
                let Some(target_id) = resolve_symbol_name(
                    &base_name,
                    &file_rel,
                    Some(&source_id),
                    &name_to_ids,
                    &slug_to_ids,
                ) else {
                    unresolved.push(UnresolvedRef {
                        source: source_id.clone(),
                        text: base_name,
                        kind: EdgeKind::Extends,
                    });
                    continue;
                };
                push_resolved_edge(
                    &mut edges,
                    &mut seen_edges,
                    ResolvedEdgeInput {
                        source: source_id.clone(),
                        target: target_id,
                        kind: EdgeKind::Extends,
                        origin: EdgeOrigin::BareName,
                    },
                    20_000 + decl.signature_span.start as usize,
                );
            }
            for impl_name in implements_names {
                let Some(target_id) = resolve_symbol_name(
                    &impl_name,
                    &file_rel,
                    Some(&source_id),
                    &name_to_ids,
                    &slug_to_ids,
                ) else {
                    unresolved.push(UnresolvedRef {
                        source: source_id.clone(),
                        text: impl_name,
                        kind: EdgeKind::Implements,
                    });
                    continue;
                };
                push_resolved_edge(
                    &mut edges,
                    &mut seen_edges,
                    ResolvedEdgeInput {
                        source: source_id.clone(),
                        target: target_id,
                        kind: EdgeKind::Implements,
                        origin: EdgeOrigin::BareName,
                    },
                    30_000 + decl.signature_span.start as usize,
                );
            }
        }
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

fn push_resolved_edge(
    edges: &mut Vec<SemanticEdge>,
    seen: &mut HashSet<(String, String, String)>,
    edge: ResolvedEdgeInput,
    order_index: usize,
) {
    let ResolvedEdgeInput {
        source,
        target,
        kind,
        origin,
    } = edge;
    if source == target {
        return;
    }
    let kind_key = format!("{kind:?}");
    if !seen.insert((source.clone(), target.clone(), kind_key)) {
        return;
    }
    let cross_boundary = target_rel_differs_file(&source, &target);
    edges.push(SemanticEdge {
        source,
        target: EdgeTarget::Resolved(target),
        kind,
        origin,
        order_index,
        cross_boundary,
    });
}

struct ResolvedEdgeInput {
    source: SymbolId,
    target: SymbolId,
    kind: EdgeKind,
    origin: EdgeOrigin,
}

fn resolve_symbol_name(
    name: &str,
    src_rel: &str,
    source_id: Option<&str>,
    name_to_ids: &HashMap<String, Vec<SymbolId>>,
    slug_to_ids: &HashMap<String, Vec<SymbolId>>,
) -> Option<SymbolId> {
    resolve_candidates(name_to_ids.get(name), src_rel, source_id).or_else(|| {
        let slug = slugify(name);
        resolve_candidates(slug_to_ids.get(&slug), src_rel, source_id)
    })
}

fn resolve_candidates(
    candidates: Option<&Vec<SymbolId>>,
    src_rel: &str,
    source_id: Option<&str>,
) -> Option<SymbolId> {
    let candidates = candidates?;
    let filtered: Vec<&SymbolId> = candidates
        .iter()
        .filter(|candidate| source_id.is_none_or(|source| candidate.as_str() != source))
        .collect();
    if filtered.is_empty() {
        return None;
    }

    let same_file: Vec<&SymbolId> = filtered
        .iter()
        .copied()
        .filter(|candidate| id_file_part(candidate) == src_rel)
        .collect();
    if same_file.len() == 1 {
        return Some(same_file[0].clone());
    }

    let src_module = module_scope(src_rel);
    let same_module: Vec<&SymbolId> = filtered
        .iter()
        .copied()
        .filter(|candidate| module_scope(id_file_part(candidate)) == src_module)
        .collect();
    if same_module.len() == 1 {
        return Some(same_module[0].clone());
    }

    if filtered.len() == 1 {
        return Some(filtered[0].clone());
    }

    None
}

fn module_scope(file_rel: &str) -> String {
    let parent = Path::new(file_rel)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let mut kept = Vec::new();
    for segment in parent.split('/') {
        if segment.is_empty()
            || matches!(
                segment,
                "src"
                    | "lib"
                    | "app"
                    | "api"
                    | "core"
                    | "internal"
                    | "pkg"
                    | "conduit"
                    | "modules"
                    | "domain"
                    | "infrastructure"
                    | "interfaces"
                    | "services"
                    | "repositories"
                    | "repository"
            )
        {
            continue;
        }
        kept.push(segment);
    }
    if kept.is_empty() {
        parent.to_string()
    } else {
        kept.join("/")
    }
}

fn route_call_details(line: &str, method_name: &str) -> Option<(String, String)> {
    let args = call_arguments_from_line(line, method_name)?;
    if args.len() < 2 {
        return None;
    }
    let path = first_string_literal(&args[0]).unwrap_or_default();
    let handler = args
        .iter()
        .rev()
        .find_map(|arg| terminal_identifier(arg))
        .unwrap_or_default();
    if handler.is_empty() {
        return None;
    }
    Some((path, handler))
}

fn call_arguments_from_line(line: &str, method_name: &str) -> Option<Vec<String>> {
    let needle = format!("{method_name}(");
    let start = line.find(&needle)? + needle.len();
    let mut depth = 1_i32;
    let mut end = start;
    let chars: Vec<char> = line.chars().collect();
    for (idx, ch) in chars.iter().enumerate().skip(start) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = idx;
                    break;
                }
            }
            _ => {}
        }
    }
    if end <= start {
        return None;
    }
    Some(split_top_level_args(&line[start..end]))
}

fn split_top_level_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut paren = 0_i32;
    let mut bracket = 0_i32;
    let mut brace = 0_i32;
    let mut in_quote = false;
    let mut quote_char = '\0';

    for ch in args.chars() {
        if in_quote {
            current.push(ch);
            if ch == quote_char {
                in_quote = false;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                in_quote = true;
                quote_char = ch;
                current.push(ch);
            }
            '(' => {
                paren += 1;
                current.push(ch);
            }
            ')' => {
                paren -= 1;
                current.push(ch);
            }
            '[' => {
                bracket += 1;
                current.push(ch);
            }
            ']' => {
                bracket -= 1;
                current.push(ch);
            }
            '{' => {
                brace += 1;
                current.push(ch);
            }
            '}' => {
                brace -= 1;
                current.push(ch);
            }
            ',' if paren == 0 && bracket == 0 && brace == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

fn first_string_literal(arg: &str) -> Option<String> {
    let trimmed = arg.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        return Some(trimmed[1..trimmed.len() - 1].to_string());
    }
    None
}

fn terminal_identifier(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut token = String::new();
    for ch in trimmed.chars().rev() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else if !token.is_empty() {
            break;
        }
    }
    if token.is_empty() {
        return None;
    }
    Some(token.chars().rev().collect())
}

fn decl_signature_text(lines: &[String], decl: &SyntaxDecl) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let start = decl.signature_span.start.max(1) as usize - 1;
    let mut end = decl.signature_span.end.max(decl.signature_span.start);
    end = end
        .max(decl.span.start)
        .min(decl.span.end.max(decl.signature_span.start));
    end = end.max(decl.signature_span.start + 4);
    let end = usize::try_from(end)
        .unwrap_or(lines.len())
        .min(lines.len());
    if start >= end || start >= lines.len() {
        return lines.get(start).cloned().unwrap_or_default();
    }
    lines[start..end].join("\n")
}

fn extract_dependency_names(signature: &str, language: &str) -> Vec<String> {
    let Some(param_text) = between_top_level(signature, '(', ')') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for arg in split_top_level_args(&param_text) {
        if arg.trim().is_empty() || matches!(arg.trim(), "self" | "&self" | "cls" | "this") {
            continue;
        }
        if let Some(depends_body) = extract_call_body(&arg, "Depends") {
            if let Some(name) = terminal_identifier(&depends_body) {
                out.push(name);
            }
            continue;
        }
        if let Some((_, rhs)) = arg.split_once(':') {
            out.extend(type_identifiers(rhs));
            continue;
        }
        if language == "java" || language == "go" || language == "cpp" {
            let parts: Vec<&str> = arg.split_whitespace().collect();
            if parts.len() >= 2 {
                let type_part = parts[..parts.len() - 1].join(" ");
                out.extend(type_identifiers(&type_part));
            }
        }
    }
    dedupe_strings(out)
}

fn extract_inheritance_names(
    signature: &str,
    decl_name: &str,
    language: &str,
    decl_kind: &crate::analyzer::syntax::types::DeclKind,
) -> (Vec<String>, Vec<String>) {
    let mut extends = Vec::new();
    let mut implements = Vec::new();
    let lower = signature.to_ascii_lowercase();

    if let Some(after_extends) = slice_after_ci(signature, &lower, " extends ") {
        let head = cut_at_keywords(after_extends, &["implements", "{", "where", ":"]);
        extends.extend(split_relation_targets(head));
    }
    if let Some(after_implements) = slice_after_ci(signature, &lower, " implements ") {
        let head = cut_at_keywords(after_implements, &["{", "where", ":"]);
        implements.extend(split_relation_targets(head));
    }

    if language == "python"
        && matches!(decl_kind, crate::analyzer::syntax::types::DeclKind::Class)
        && let Some(class_pos) = lower.find("class ")
    {
        let after_class = &signature[class_pos + 6..];
        if let Some(paren_pos) = after_class.find('(')
            && let Some(bases) = between_top_level(&after_class[paren_pos..], '(', ')')
        {
            let mut base_names = split_relation_targets(&bases);
            if let Some(first) = base_names.first().cloned() {
                extends.push(first);
                if base_names.len() > 1 {
                    implements.extend(base_names.drain(1..));
                }
            }
        }
    }

    if language == "rust"
        && matches!(decl_kind, crate::analyzer::syntax::types::DeclKind::Trait)
        && let Some(after_colon) = signature.split_once(':').map(|(_, rhs)| rhs)
    {
        implements.extend(split_relation_targets(cut_at_keywords(
            after_colon,
            &["where", "{", ";"],
        )));
    }

    extends.retain(|name| name != decl_name);
    implements.retain(|name| name != decl_name);
    (dedupe_strings(extends), dedupe_strings(implements))
}

fn between_top_level(text: &str, open: char, close: char) -> Option<String> {
    let mut depth = 0_i32;
    let mut start = None;
    for (idx, ch) in text.char_indices() {
        if ch == open {
            if depth == 0 {
                start = Some(idx + ch.len_utf8());
            }
            depth += 1;
        } else if ch == close {
            depth -= 1;
            if depth == 0 {
                let start = start?;
                return Some(text[start..idx].to_string());
            }
        }
    }
    None
}

fn extract_call_body(text: &str, func_name: &str) -> Option<String> {
    let needle = format!("{func_name}(");
    let start = text.find(&needle)? + needle.len() - 1;
    between_top_level(&text[start..], '(', ')')
}

fn type_identifiers(type_expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in type_expr.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            current.push(ch);
        } else if !current.is_empty() {
            maybe_push_type_token(&mut out, &current);
            current.clear();
        }
    }
    if !current.is_empty() {
        maybe_push_type_token(&mut out, &current);
    }
    out
}

fn maybe_push_type_token(out: &mut Vec<String>, token: &str) {
    let lower = token.to_ascii_lowercase();
    if token.is_empty()
        || matches!(
            lower.as_str(),
            "self"
                | "cls"
                | "string"
                | "str"
                | "int"
                | "i32"
                | "i64"
                | "u32"
                | "u64"
                | "usize"
                | "bool"
                | "float"
                | "f32"
                | "f64"
                | "vec"
                | "option"
                | "result"
                | "list"
                | "dict"
                | "map"
                | "set"
                | "hashmap"
                | "arc"
                | "box"
                | "impl"
                | "dyn"
                | "mut"
                | "const"
                | "pub"
        )
    {
        return;
    }
    if token
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        || token.contains("Service")
        || token.contains("Repository")
        || token.contains("Controller")
        || token.contains("View")
        || token.contains("Client")
        || token.contains("Handler")
    {
        out.push(token.to_string());
    }
}

fn slice_after_ci<'a>(original: &'a str, lower: &str, needle: &str) -> Option<&'a str> {
    let idx = lower.find(needle)?;
    Some(&original[idx + needle.len()..])
}

fn cut_at_keywords<'a>(text: &'a str, keywords: &[&str]) -> &'a str {
    let lower = text.to_ascii_lowercase();
    let mut cut = text.len();
    for keyword in keywords {
        if let Some(idx) = lower.find(&keyword.to_ascii_lowercase()) {
            cut = cut.min(idx);
        }
    }
    &text[..cut]
}

fn split_relation_targets(text: &str) -> Vec<String> {
    split_top_level_args(text)
        .into_iter()
        .flat_map(|part| type_identifiers(&part))
        .collect()
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
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
