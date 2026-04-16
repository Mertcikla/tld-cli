//! Data-flow projector — emits a flow-focused view of how data moves between
//! high-salience symbols around business transactions.
//!
//! Uses the flow extraction from `semantic::flows` to build the view.

use crate::analyzer::semantic::{
    flows, graph::SemanticGraph, prune, resolver, roles, salience, types::SemanticBundle,
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

/// Project a `SyntaxBundle` as a data-flow diagram.
/// Emits one connector per resolved flow step; flow sources/targets are the
/// high-salience symbols on the traced path.
pub fn project(
    syntax: &SyntaxBundle,
    ctx: &BuildContext,
    noise_threshold: i32,
) -> (BuildOutput, ProjectionStats) {
    let scan_parent = std::path::Path::new(&ctx.scan_root)
        .parent()
        .map_or("", |p| p.to_str().unwrap_or(""))
        .to_string();

    let bundle: SemanticBundle = resolver::resolve_syntax(syntax, &scan_parent);
    let graph = SemanticGraph::build(&bundle);
    let role_map = roles::infer_roles(&graph);
    let score_map = salience::score_all(&graph, &role_map);
    let pruned = prune::prune(&bundle, &score_map, noise_threshold);
    let flow_list = flows::extract_flows(&graph, &role_map, &score_map, noise_threshold, 8);

    let repo_slug = slugify(&ctx.repo_name);
    let mut elements: HashMap<String, Element> = HashMap::new();
    let mut connectors: Vec<Connector> = Vec::new();
    let mut seen_conn: HashSet<String> = HashSet::new();
    let mut slug_registry: HashMap<String, usize> = HashMap::new();
    let mut sym_id_to_slug: HashMap<String, String> = HashMap::new();

    elements.insert(
        repo_slug.clone(),
        Element {
            name: ctx.repo_name.clone(),
            kind: "repository".to_string(),
            technology: "Git Repository".to_string(),
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

    // Emit symbols that appear in any flow.
    let flow_symbol_ids: HashSet<&str> = flow_list
        .iter()
        .flat_map(|f| {
            std::iter::once(f.entrypoint.as_str())
                .chain(f.steps.iter().map(|s| s.symbol_id.as_str()))
        })
        .collect();

    for sym in &pruned.symbols {
        if !flow_symbol_ids.contains(sym.symbol_id.as_str()) {
            continue;
        }
        let base = slugify(&sym.name);
        let count = slug_registry.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 {
            base.clone()
        } else {
            let stem = std::path::Path::new(&sym.file_path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("x");
            format!("{}-{}", slugify(stem), base)
        };
        *count += 1;
        sym_id_to_slug.insert(sym.symbol_id.clone(), slug.clone());

        elements.insert(
            slug,
            Element {
                name: sym.name.clone(),
                kind: sym.kind.as_str().to_string(),
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
    }

    // Emit connectors between consecutive flow steps.
    for flow in &flow_list {
        let ep_slug = sym_id_to_slug.get(&flow.entrypoint);
        let mut prev = ep_slug.cloned();

        for step in &flow.steps {
            let step_slug = sym_id_to_slug.get(&step.symbol_id).cloned();
            if let (Some(src), Some(tgt)) = (prev.clone(), step_slug.clone())
                && src != tgt
            {
                let conn = Connector {
                    view: repo_slug.clone(),
                    source: src,
                    target: tgt.clone(),
                    label: step.stage_label.clone(),
                    relationship: "uses".to_string(),
                    direction: "forward".to_string(),
                    ..Default::default()
                };
                let key = conn.resource_ref();
                if seen_conn.insert(key) {
                    connectors.push(conn);
                }
            }
            prev = step_slug;
        }
    }

    let stats = ProjectionStats {
        flow_count: flow_list.len(),
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
