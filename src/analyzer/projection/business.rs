#![allow(dead_code)]
//! Business projector — emits architecturally significant symbols grouped into
//! coarse business domains plus synthesized infrastructure terminals.

use super::tags::{self, AutoTagOptions};
use super::{collapse_connectors, domain_for_symbol, edge_labels, present_symbol, unique_slug};
use crate::analyzer::semantic::{
    graph::SemanticGraph,
    infra, prune, resolver, roles, salience,
    types::{EdgeKind, EdgeTarget, SemanticBundle, SemanticEdge, SemanticSymbol, SymbolId},
};
use crate::analyzer::syntax::types::SyntaxBundle;
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

pub fn project(
    syntax: &SyntaxBundle,
    ctx: &BuildContext,
    noise_threshold: i32,
    auto_tags: AutoTagOptions,
) -> (BuildOutput, ProjectionStats) {
    let scan_parent = std::path::Path::new(&ctx.scan_root)
        .parent()
        .map_or("", |p| p.to_str().unwrap_or(""))
        .to_string();

    let mut bundle: SemanticBundle = resolver::resolve_syntax(syntax, &scan_parent);
    infra::synthesize(&mut bundle);

    let graph = SemanticGraph::build(&bundle);
    let role_map = roles::infer_roles(&graph);
    let score_map = salience::score_all(&graph, &role_map);
    let pruned = prune::prune(&bundle, &score_map, noise_threshold);
    let mut visible_ids: HashSet<String> = pruned
        .symbols
        .iter()
        .map(|symbol| symbol.symbol_id.clone())
        .collect();
    let mut projected_symbols = pruned.symbols.clone();
    for symbol_id in forced_projection_symbol_ids(&bundle, &visible_ids) {
        if visible_ids.insert(symbol_id.clone())
            && let Some(symbol) = bundle
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == symbol_id)
        {
            projected_symbols.push(symbol.clone());
        }
    }
    let supplemental_edges = supplemental_projection_edges(&bundle, &visible_ids);

    let repo_slug = slugify(&ctx.repo_name);
    let mut elements: HashMap<String, Element> = HashMap::new();
    let mut sym_id_to_slug: HashMap<String, String> = HashMap::new();
    let mut slug_registry: HashMap<String, usize> = HashMap::new();
    let mut domain_registry: HashMap<String, String> = HashMap::new();

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

    let mut symbol_domains: HashMap<String, String> = HashMap::new();
    for sym in &projected_symbols {
        let domain = domain_for_symbol(sym);
        if !domain.is_empty() {
            symbol_domains.insert(sym.symbol_id.clone(), domain.clone());
            domain_registry.entry(domain.clone()).or_insert_with(|| {
                let slug = format!("domain-{}", slugify(&domain));
                let domain_tags = if auto_tags.domain {
                    vec![format!("domain:{domain}")]
                } else {
                    Vec::new()
                };
                elements.insert(
                    slug.clone(),
                    Element {
                        name: domain.clone(),
                        kind: "domain".to_string(),
                        technology: "Domain".to_string(),
                        owner: ctx.owner.clone(),
                        branch: ctx.branch.clone(),
                        tags: domain_tags,
                        placements: vec![ViewPlacement {
                            parent_ref: repo_slug.clone(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    },
                );
                slug
            });
        }
    }

    for sym in &projected_symbols {
        let slug = unique_slug(&sym.name, &sym.file_path, &mut slug_registry);
        sym_id_to_slug.insert(sym.symbol_id.clone(), slug.clone());

        let parent_ref = symbol_domains
            .get(&sym.symbol_id)
            .and_then(|domain| domain_registry.get(domain))
            .cloned()
            .unwrap_or_else(|| repo_slug.clone());
        let role = role_map.get(&sym.symbol_id);
        let score = score_map.get(&sym.symbol_id).copied().unwrap_or_default();
        let presentation = present_symbol(sym, role);

        let mut element = Element {
            name: sym.name.clone(),
            kind: presentation.kind,
            technology: presentation.technology,
            owner: ctx.owner.clone(),
            branch: ctx.branch.clone(),
            file_path: sym.file_path.clone(),
            symbol: sym.name.clone(),
            symbol_kind: presentation.symbol_kind,
            description: sym.description.clone(),
            placements: vec![ViewPlacement {
                parent_ref,
                ..Default::default()
            }],
            ..Default::default()
        };
        tags::assign_semantic_tags(&mut element, sym, role, score, auto_tags);
        elements.insert(slug, element);
    }

    let mut connectors: Vec<Connector> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for edge in pruned.edges.iter().chain(supplemental_edges.iter()) {
        let src_slug = sym_id_to_slug.get(&edge.source);
        let tgt_slug = edge
            .target
            .resolved_id()
            .and_then(|target| sym_id_to_slug.get(target));
        let (Some(src_slug), Some(tgt_slug)) = (src_slug, tgt_slug) else {
            continue;
        };

        let source_domain_slug = symbol_domains
            .get(&edge.source)
            .and_then(|domain| domain_registry.get(domain))
            .cloned();
        let target_domain_slug = edge
            .target
            .resolved_id()
            .and_then(|target| symbol_domains.get(target))
            .and_then(|domain| domain_registry.get(domain))
            .cloned();

        let (source_ref, target_ref) = if matches!(edge.kind, EdgeKind::Imports)
            && source_domain_slug.is_some()
            && target_domain_slug.is_some()
            && source_domain_slug != target_domain_slug
        {
            (
                source_domain_slug.unwrap_or_else(|| src_slug.clone()),
                target_domain_slug.unwrap_or_else(|| tgt_slug.clone()),
            )
        } else {
            (src_slug.clone(), tgt_slug.clone())
        };

        if source_ref == target_ref {
            continue;
        }

        let (label, relationship) = edge_labels(&edge.kind);
        let conn = Connector {
            view: repo_slug.clone(),
            source: source_ref,
            target: target_ref,
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

    let connectors = collapse_connectors(connectors);
    tags::prune_sparse_auto_tags(&mut elements, 3);
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

fn forced_projection_symbol_ids(
    bundle: &SemanticBundle,
    visible_ids: &HashSet<String>,
) -> HashSet<String> {
    let mut forced = HashSet::new();

    for edge in &bundle.edges {
        if !matches!(edge.kind, EdgeKind::Extends | EdgeKind::Implements) {
            continue;
        }
        let Some(target_id) = edge.target.resolved_id() else {
            continue;
        };
        if !visible_ids.contains(&edge.source) {
            continue;
        }
        if bundle
            .symbols
            .iter()
            .any(|symbol| symbol.symbol_id == target_id && symbol.external)
        {
            forced.insert(target_id.to_string());
        }
    }

    forced
}

fn supplemental_projection_edges(
    bundle: &SemanticBundle,
    visible_ids: &HashSet<String>,
) -> Vec<SemanticEdge> {
    let symbols_by_id: HashMap<&str, &SemanticSymbol> = bundle
        .symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.as_str(), symbol))
        .collect();
    let mut supplemental = Vec::new();
    let mut seen: HashSet<(String, String, EdgeKind)> = HashSet::new();

    for edge in &bundle.edges {
        match edge.kind {
            EdgeKind::Calls | EdgeKind::DependsOn => {
                let Some(target_id) = edge.target.resolved_id() else {
                    continue;
                };
                let Some(source_ref) =
                    architectural_symbol_id(&edge.source, &symbols_by_id, visible_ids)
                else {
                    continue;
                };
                let Some(target_ref) =
                    architectural_symbol_id(target_id, &symbols_by_id, visible_ids)
                else {
                    continue;
                };
                if source_ref == target_ref {
                    continue;
                }
                if seen.insert((source_ref.clone(), target_ref.clone(), EdgeKind::DependsOn)) {
                    supplemental.push(SemanticEdge {
                        source: source_ref,
                        target: EdgeTarget::Resolved(target_ref),
                        kind: EdgeKind::DependsOn,
                        origin: edge.origin.clone(),
                        order_index: edge.order_index,
                        cross_boundary: edge.cross_boundary,
                    });
                }
            }
            EdgeKind::Extends | EdgeKind::Implements => {
                let Some(target_id) = edge.target.resolved_id() else {
                    continue;
                };
                if !visible_ids.contains(&edge.source) || !visible_ids.contains(target_id) {
                    continue;
                }
                if seen.insert((
                    edge.source.clone(),
                    target_id.to_string(),
                    edge.kind.clone(),
                )) {
                    supplemental.push(SemanticEdge {
                        source: edge.source.clone(),
                        target: EdgeTarget::Resolved(target_id.to_string()),
                        kind: edge.kind.clone(),
                        origin: edge.origin.clone(),
                        order_index: edge.order_index,
                        cross_boundary: edge.cross_boundary,
                    });
                }
            }
            _ => {}
        }
    }

    supplemental
}

fn architectural_symbol_id(
    symbol_id: &str,
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    visible_ids: &HashSet<String>,
) -> Option<SymbolId> {
    let symbol = symbols_by_id.get(symbol_id)?;
    if let Some(owner) = &symbol.owner
        && visible_ids.contains(owner)
    {
        return Some(owner.clone());
    }
    visible_ids
        .contains(symbol_id)
        .then(|| symbol_id.to_string())
}
