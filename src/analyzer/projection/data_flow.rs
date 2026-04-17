//! Data-flow projector — emits one representative request chain per endpoint
//! family, collapsed to architectural nodes such as endpoint/service/repository/db.

use super::{collapse_connectors, domain_for_symbol, edge_labels, present_symbol, unique_slug};
use crate::analyzer::semantic::{
    graph::SemanticGraph,
    infra, prune, resolver, roles, salience,
    types::{EdgeKind, SemanticBundle, SemanticSymbol, SymbolId},
};
use crate::analyzer::syntax::types::SyntaxBundle;
use crate::workspace::{
    slugify,
    types::{Connector, Element, ViewPlacement},
    workspace_builder::{BuildContext, BuildOutput},
};
use std::collections::{HashMap, HashSet};

pub struct ProjectionStats {
    pub flow_count: usize,
    pub symbols_hidden: usize,
    pub unresolved_refs: usize,
}

#[derive(Debug, Clone)]
struct ArchEdge {
    target: SymbolId,
    kind: EdgeKind,
}

#[derive(Debug, Clone)]
struct PathStep {
    symbol_id: SymbolId,
    via_kind: Option<EdgeKind>,
}

#[derive(Debug, Clone)]
struct FamilyChain {
    family: String,
    root_count: usize,
    path: Vec<PathStep>,
}

pub fn project(
    syntax: &SyntaxBundle,
    ctx: &BuildContext,
    noise_threshold: i32,
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

    let symbols_by_id: HashMap<&str, &SemanticSymbol> = bundle
        .symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.as_str(), symbol))
        .collect();

    let visible_ids: HashSet<String> = bundle
        .symbols
        .iter()
        .filter(|symbol| {
            is_architectural_symbol(
                symbol,
                role_map.get(&symbol.symbol_id),
                score_map
                    .get(&symbol.symbol_id)
                    .copied()
                    .unwrap_or_default(),
                noise_threshold,
            )
        })
        .map(|symbol| symbol.symbol_id.clone())
        .collect();

    let adjacency = build_architecture_edges(&bundle, &symbols_by_id, &visible_ids);
    let family_chains = select_family_chains(
        &bundle,
        &symbols_by_id,
        &role_map,
        &score_map,
        &visible_ids,
        &adjacency,
    )
    .into_iter()
    .take(10)
    .collect::<Vec<_>>();

    let repo_slug = slugify(&ctx.repo_name);
    let mut elements: HashMap<String, Element> = HashMap::new();
    let mut connectors: Vec<Connector> = Vec::new();
    let mut slug_registry: HashMap<String, usize> = HashMap::new();

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

    for chain in &family_chains {
        let family_slug = format!("flow-{}", slugify(&chain.family));
        elements.insert(
            family_slug.clone(),
            Element {
                name: chain.family.clone(),
                kind: "domain".to_string(),
                technology: "Request Flow".to_string(),
                owner: ctx.owner.clone(),
                branch: ctx.branch.clone(),
                description: if chain.root_count > 1 {
                    format!("representative_flow_for={} endpoints", chain.root_count)
                } else {
                    String::new()
                },
                placements: vec![ViewPlacement {
                    parent_ref: repo_slug.clone(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let mut prev_slug: Option<String> = None;
        for (idx, step) in chain.path.iter().enumerate() {
            let Some(symbol) = symbols_by_id.get(step.symbol_id.as_str()).copied() else {
                continue;
            };
            let presentation = present_symbol(symbol, role_map.get(&step.symbol_id));
            let element_slug = unique_slug(
                &format!("{}-{}-{}", chain.family, idx, symbol.name),
                &format!("{}/{}", chain.family, symbol.file_path),
                &mut slug_registry,
            );

            elements.insert(
                element_slug.clone(),
                Element {
                    name: symbol.name.clone(),
                    kind: presentation.kind,
                    technology: presentation.technology,
                    owner: ctx.owner.clone(),
                    branch: ctx.branch.clone(),
                    file_path: symbol.file_path.clone(),
                    symbol: symbol.name.clone(),
                    symbol_kind: presentation.symbol_kind,
                    description: symbol.description.clone(),
                    placements: vec![ViewPlacement {
                        parent_ref: family_slug.clone(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            );

            if let (Some(source), Some(edge_kind)) = (prev_slug.clone(), step.via_kind.as_ref()) {
                let (label, relationship) = edge_labels(edge_kind);
                connectors.push(Connector {
                    view: repo_slug.clone(),
                    source,
                    target: element_slug.clone(),
                    label: label.to_string(),
                    relationship: relationship.to_string(),
                    direction: "forward".to_string(),
                    ..Default::default()
                });
            }

            prev_slug = Some(element_slug);
        }
    }

    let connectors = collapse_connectors(connectors);
    let stats = ProjectionStats {
        flow_count: family_chains.len(),
        symbols_hidden: pruned.hidden_count,
        unresolved_refs: bundle.unresolved_refs.len(),
    };

    (
        BuildOutput {
            elements,
            connectors,
        },
        stats,
    )
}

fn is_architectural_symbol(
    symbol: &SemanticSymbol,
    role: Option<&roles::DerivedRole>,
    score: i32,
    noise_threshold: i32,
) -> bool {
    symbol.external
        || symbol.annotations.iter().any(|annotation| {
            crate::analyzer::semantic::endpoints::detect_endpoint(annotation).is_some()
        })
        || matches!(
            role,
            Some(
                roles::DerivedRole::Entrypoint
                    | roles::DerivedRole::Orchestrator
                    | roles::DerivedRole::Adapter
                    | roles::DerivedRole::Bootstrap
                    | roles::DerivedRole::Interface
            )
        )
        || score > noise_threshold
}

fn build_architecture_edges(
    bundle: &SemanticBundle,
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    visible_ids: &HashSet<String>,
) -> HashMap<SymbolId, Vec<ArchEdge>> {
    let mut adjacency: HashMap<SymbolId, Vec<ArchEdge>> = HashMap::new();
    let mut seen: HashSet<(String, String, EdgeKind)> = HashSet::new();

    for edge in &bundle.edges {
        if !matches!(
            edge.kind,
            EdgeKind::Calls | EdgeKind::DependsOn | EdgeKind::Imports | EdgeKind::Constructs
        ) {
            continue;
        }
        let Some(target_id) = edge.target.resolved_id() else {
            continue;
        };
        let Some(source_ref) = architectural_symbol_id(&edge.source, symbols_by_id, visible_ids)
        else {
            continue;
        };
        let Some(target_ref) = architectural_symbol_id(target_id, symbols_by_id, visible_ids)
        else {
            continue;
        };
        if source_ref == target_ref {
            continue;
        }
        if seen.insert((source_ref.clone(), target_ref.clone(), edge.kind.clone())) {
            adjacency.entry(source_ref).or_default().push(ArchEdge {
                target: target_ref,
                kind: edge.kind.clone(),
            });
        }
    }

    adjacency
}

fn select_family_chains(
    bundle: &SemanticBundle,
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    role_map: &HashMap<SymbolId, roles::DerivedRole>,
    score_map: &HashMap<SymbolId, i32>,
    visible_ids: &HashSet<String>,
    adjacency: &HashMap<SymbolId, Vec<ArchEdge>>,
) -> Vec<FamilyChain> {
    let mut roots_by_family: HashMap<String, Vec<SymbolId>> = HashMap::new();

    for symbol in &bundle.symbols {
        if !visible_ids.contains(&symbol.symbol_id) {
            continue;
        }
        let is_root = matches!(
            role_map.get(&symbol.symbol_id),
            Some(roles::DerivedRole::Entrypoint)
        ) || symbol.annotations.iter().any(|annotation| {
            crate::analyzer::semantic::endpoints::detect_endpoint(annotation).is_some()
        });
        if !is_root {
            continue;
        }
        let family = domain_for_symbol(symbol);
        if family == "infrastructure" || family == "misc" {
            continue;
        }
        if let Some(root_id) =
            architectural_symbol_id(&symbol.symbol_id, symbols_by_id, visible_ids)
        {
            roots_by_family.entry(family).or_default().push(root_id);
        }
    }

    if roots_by_family.is_empty() {
        for symbol in &bundle.symbols {
            if !visible_ids.contains(&symbol.symbol_id) {
                continue;
            }
            if !matches!(
                role_map.get(&symbol.symbol_id),
                Some(roles::DerivedRole::Orchestrator | roles::DerivedRole::Entrypoint)
            ) {
                continue;
            }
            let family = domain_for_symbol(symbol);
            if family == "infrastructure" {
                continue;
            }
            if let Some(root_id) =
                architectural_symbol_id(&symbol.symbol_id, symbols_by_id, visible_ids)
            {
                roots_by_family.entry(family).or_default().push(root_id);
            }
        }
    }

    let mut families = Vec::new();
    for (family, roots) in roots_by_family {
        let unique_roots: HashSet<_> = roots.iter().cloned().collect();
        let mut best_chain: Option<Vec<PathStep>> = None;
        let mut best_score = i32::MIN;

        for root in unique_roots {
            let chain = best_path_from(&root, adjacency, symbols_by_id, role_map, score_map, 5);
            let score = path_score(&chain, symbols_by_id, role_map, score_map);
            if score > best_score {
                best_score = score;
                best_chain = Some(chain);
            }
        }

        if let Some(path) = best_chain
            && path.len() >= 2
        {
            families.push(FamilyChain {
                family,
                root_count: roots.len(),
                path,
            });
        }
    }

    families.sort_by(|a, b| {
        b.root_count
            .cmp(&a.root_count)
            .then_with(|| a.family.cmp(&b.family))
    });
    families
}

fn best_path_from(
    start: &SymbolId,
    adjacency: &HashMap<SymbolId, Vec<ArchEdge>>,
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    role_map: &HashMap<SymbolId, roles::DerivedRole>,
    score_map: &HashMap<SymbolId, i32>,
    max_depth: usize,
) -> Vec<PathStep> {
    let mut best = vec![PathStep {
        symbol_id: start.clone(),
        via_kind: None,
    }];
    let mut visited = HashSet::from([start.clone()]);
    let mut path = best.clone();

    dfs_best_path(
        start,
        adjacency,
        symbols_by_id,
        role_map,
        score_map,
        max_depth,
        &mut visited,
        &mut path,
        &mut best,
    );

    best
}

#[expect(clippy::too_many_arguments)]
fn dfs_best_path(
    current: &SymbolId,
    adjacency: &HashMap<SymbolId, Vec<ArchEdge>>,
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    role_map: &HashMap<SymbolId, roles::DerivedRole>,
    score_map: &HashMap<SymbolId, i32>,
    remaining_depth: usize,
    visited: &mut HashSet<SymbolId>,
    path: &mut Vec<PathStep>,
    best: &mut Vec<PathStep>,
) {
    if path_score(path, symbols_by_id, role_map, score_map)
        > path_score(best, symbols_by_id, role_map, score_map)
    {
        *best = path.clone();
    }

    if remaining_depth == 0 {
        return;
    }

    let mut neighbors = adjacency.get(current).cloned().unwrap_or_default();
    neighbors.sort_by_key(|edge| {
        std::cmp::Reverse(node_priority(
            &edge.target,
            symbols_by_id,
            role_map,
            score_map,
        ))
    });

    for edge in neighbors {
        if visited.contains(&edge.target) {
            continue;
        }

        visited.insert(edge.target.clone());
        path.push(PathStep {
            symbol_id: edge.target.clone(),
            via_kind: Some(edge.kind.clone()),
        });

        let is_terminal = symbols_by_id
            .get(edge.target.as_str())
            .map(|symbol| symbol.external)
            .unwrap_or(false);

        if !is_terminal {
            dfs_best_path(
                &edge.target,
                adjacency,
                symbols_by_id,
                role_map,
                score_map,
                remaining_depth - 1,
                visited,
                path,
                best,
            );
        } else if path_score(path, symbols_by_id, role_map, score_map)
            > path_score(best, symbols_by_id, role_map, score_map)
        {
            *best = path.clone();
        }

        path.pop();
        visited.remove(&edge.target);
    }
}

fn path_score(
    path: &[PathStep],
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    role_map: &HashMap<SymbolId, roles::DerivedRole>,
    score_map: &HashMap<SymbolId, i32>,
) -> i32 {
    let mut score = (path.len() as i32) * 4;

    for step in path {
        let Some(symbol) = symbols_by_id.get(step.symbol_id.as_str()).copied() else {
            continue;
        };
        if symbol.external {
            score += 40;
            if symbol.description.starts_with("infra:database:") {
                score += 20;
            }
        }
        score += score_map
            .get(&step.symbol_id)
            .copied()
            .unwrap_or_default()
            .max(0);
        score += match role_map.get(&step.symbol_id) {
            Some(roles::DerivedRole::Entrypoint) => 10,
            Some(roles::DerivedRole::Orchestrator) => 14,
            Some(roles::DerivedRole::Adapter) => 18,
            Some(roles::DerivedRole::Bootstrap) => 8,
            _ => 0,
        };
    }

    score
}

fn node_priority(
    symbol_id: &str,
    symbols_by_id: &HashMap<&str, &SemanticSymbol>,
    role_map: &HashMap<SymbolId, roles::DerivedRole>,
    score_map: &HashMap<SymbolId, i32>,
) -> i32 {
    let Some(symbol) = symbols_by_id.get(symbol_id).copied() else {
        return 0;
    };
    if symbol.external {
        return if symbol.description.starts_with("infra:database:") {
            100
        } else {
            80
        };
    }

    score_map.get(symbol_id).copied().unwrap_or_default()
        + match role_map.get(symbol_id) {
            Some(roles::DerivedRole::Adapter) => 40,
            Some(roles::DerivedRole::Orchestrator) => 30,
            Some(roles::DerivedRole::Entrypoint) => 20,
            Some(roles::DerivedRole::Bootstrap) => 10,
            _ => 0,
        }
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
