#![allow(dead_code)]
//! Business projector — emits only architecturally significant symbols and
//! their resolved connections.
//!
//! Default behaviour:
//! - No file or folder elements.
//! - Only symbols with salience > threshold are emitted.
//! - Only edges between kept symbols are included.
//! - One flat view rooted at the repository element.

use crate::analyzer::semantic::{
    graph::SemanticGraph,
    prune, resolver, roles, salience,
    types::{EdgeKind, SemanticBundle},
};
use crate::analyzer::syntax::types::{DeclKind, SyntaxBundle};
use crate::workspace::{
    slugify,
    types::{Connector, Element, ViewPlacement},
    workspace_builder::{BuildContext, BuildOutput},
};
use std::collections::{HashMap, HashSet};

/// Observability counters reported back to the CLI.
pub struct ProjectionStats {
    pub symbols_total: usize,
    pub symbols_hidden: usize,
    pub connectors_emitted: usize,
    pub unresolved_refs: usize,
    pub resolved_call_edges: usize,
    pub lsp_resolved_edges: usize,
}

/// Project a `SyntaxBundle` using the semantic pipeline, emitting only
/// high-salience architectural symbols and their resolved connections.
pub fn project(
    syntax: &SyntaxBundle,
    ctx: &BuildContext,
    noise_threshold: i32,
) -> (BuildOutput, ProjectionStats) {
    // ── 1. Semantic resolution ────────────────────────────────────────────────
    let scan_parent = std::path::Path::new(&ctx.scan_root)
        .parent()
        .map_or("", |p| p.to_str().unwrap_or(""))
        .to_string();

    let bundle: SemanticBundle = resolver::resolve_syntax(syntax, &scan_parent);

    // ── 2. Graph + roles + salience ───────────────────────────────────────────
    let graph = SemanticGraph::build(&bundle);
    let role_map = roles::infer_roles(&graph);
    let score_map = salience::score_all(&graph, &role_map);

    // ── 3. Pruning ────────────────────────────────────────────────────────────
    let pruned = prune::prune(&bundle, &score_map, noise_threshold);

    // ── 4. Build workspace elements ───────────────────────────────────────────
    let repo_slug = slugify(&ctx.repo_name);

    let mut elements: HashMap<String, Element> = HashMap::new();

    // Repository root element.
    elements.insert(
        repo_slug.clone(),
        Element {
            name: ctx.repo_name.clone(),
            kind: "repository".to_string(),
            technology: "Git Repository".to_string(),
            repo: ctx.repo_url.clone().unwrap_or_default(),
            owner: ctx.owner.clone(),
            branch: ctx.branch.clone(),
            has_view: true,
            view_label: ctx.repo_name.clone(),
            placements: vec![ViewPlacement {
                parent_ref: "root".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
    );

    // Track slug → symbol for connector resolution.
    let mut sym_id_to_slug: HashMap<String, String> = HashMap::new();
    let mut slug_registry: HashMap<String, usize> = HashMap::new();

    for sym in &pruned.symbols {
        let role = role_map.get(&sym.symbol_id);
        let slug = unique_slug(&sym.name, &sym.file_path, &mut slug_registry);
        sym_id_to_slug.insert(sym.symbol_id.clone(), slug.clone());

        let technology = technology_label(&sym.kind, &sym.symbol_id);
        let kind_str = sym.kind.as_str().to_string();

        elements.insert(
            slug,
            Element {
                name: sym.name.clone(),
                kind: kind_str,
                technology,
                owner: ctx.owner.clone(),
                branch: ctx.branch.clone(),
                file_path: sym.file_path.clone(),
                symbol: sym.name.clone(),
                description: sym.description.clone(),
                placements: vec![ViewPlacement {
                    parent_ref: repo_slug.clone(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let _ = role; // Role annotation is preserved via the element kind for now.
    }

    // ── 5. Build connectors ───────────────────────────────────────────────────
    let mut connectors: Vec<Connector> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for edge in &pruned.edges {
        let src_slug = sym_id_to_slug.get(&edge.source);
        let tgt_slug = edge
            .target
            .resolved_id()
            .and_then(|t| sym_id_to_slug.get(t));

        let (src_slug, tgt_slug) = match (src_slug, tgt_slug) {
            (Some(s), Some(t)) => (s.clone(), t.clone()),
            _ => continue,
        };

        if src_slug == tgt_slug {
            continue;
        }

        let (label, relationship) = edge_labels(&edge.kind);

        let conn = Connector {
            view: repo_slug.clone(),
            source: src_slug,
            target: tgt_slug,
            label: label.to_string(),
            relationship: relationship.to_string(),
            direction: "forward".to_string(),
            ..Default::default()
        };

        let key = conn.resource_ref();
        if seen.insert(key) {
            connectors.push(conn);
        }
    }

    let stats = ProjectionStats {
        symbols_total: bundle.symbols.len(),
        symbols_hidden: pruned.hidden_count,
        connectors_emitted: connectors.len(),
        unresolved_refs: bundle.unresolved_refs.len(),
        resolved_call_edges: bundle
            .edges
            .iter()
            .filter(|e| matches!(e.kind, EdgeKind::Calls) && e.target.is_resolved())
            .count(),
        lsp_resolved_edges: bundle
            .edges
            .iter()
            .filter(|e| {
                matches!(e.kind, EdgeKind::Calls)
                    && e.origin == crate::analyzer::semantic::types::EdgeOrigin::Lsp
            })
            .count(),
    };

    (
        BuildOutput {
            elements,
            connectors,
        },
        stats,
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn unique_slug(name: &str, file_path: &str, registry: &mut HashMap<String, usize>) -> String {
    let base = slugify(name);
    let count = registry.entry(base.clone()).or_insert(0);
    if *count == 0 {
        *count += 1;
        base
    } else {
        *count += 1;
        // Disambiguate using the file stem.
        let stem = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("x");
        format!("{}-{}", slugify(stem), base)
    }
}

fn technology_label(kind: &DeclKind, _symbol_id: &str) -> String {
    match kind {
        DeclKind::Class | DeclKind::Struct => "Component",
        DeclKind::Interface | DeclKind::Trait => "Interface",
        DeclKind::Function | DeclKind::Method => "Function",
        DeclKind::Enum => "Enum",
        _ => "Symbol",
    }
    .to_string()
}

fn edge_labels(kind: &EdgeKind) -> (&'static str, &'static str) {
    match kind {
        EdgeKind::Calls => ("calls", "uses"),
        EdgeKind::Imports => ("references", "depends_on"),
        EdgeKind::Constructs => ("constructs", "creates"),
        EdgeKind::Reads => ("reads", "reads"),
        EdgeKind::Writes => ("writes", "mutates"),
        EdgeKind::Returns => ("returns", "provides"),
        EdgeKind::Throws => ("throws", "raises"),
        EdgeKind::Implements => ("implements", "implements"),
    }
}
