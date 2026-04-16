#![allow(dead_code)]
//! Business-flow extraction — traces ordered call paths from entrypoints through
//! high-salience orchestration nodes to external or persistence boundaries.

use super::graph::SemanticGraph;
use super::roles::DerivedRole;
use super::types::{EdgeKind, SemanticSymbol, SymbolId};
use std::collections::{HashMap, HashSet};

/// A single step in a business-flow trace.
#[derive(Debug, Clone)]
pub struct FlowStep {
    pub symbol_id: SymbolId,
    pub name: String,
    /// Neutral stage label derived from the edge kind.
    pub stage_label: String,
    /// Order index within the flow (source-code order when available).
    pub order: usize,
}

/// An extracted business-flow trace rooted at one entrypoint.
#[derive(Debug, Clone)]
pub struct BusinessFlow {
    pub entrypoint: SymbolId,
    pub entrypoint_name: String,
    pub steps: Vec<FlowStep>,
}

/// Extract ordered business flows starting from every Entrypoint or Orchestrator.
pub fn extract_flows(
    graph: &SemanticGraph,
    roles: &HashMap<SymbolId, DerivedRole>,
    scores: &HashMap<SymbolId, i32>,
    min_salience: i32,
    max_depth: usize,
) -> Vec<BusinessFlow> {
    let mut flows = Vec::new();

    let entrypoints: Vec<&SemanticSymbol> = graph
        .nodes
        .values()
        .filter(|s| {
            matches!(
                roles.get(&s.symbol_id),
                Some(DerivedRole::Entrypoint | DerivedRole::Orchestrator)
            )
        })
        .collect();

    for ep in entrypoints {
        let steps = walk_flow(graph, roles, scores, &ep.symbol_id, min_salience, max_depth);
        if !steps.is_empty() {
            flows.push(BusinessFlow {
                entrypoint: ep.symbol_id.clone(),
                entrypoint_name: ep.name.clone(),
                steps,
            });
        }
    }

    // Sort flows by entrypoint name for determinism.
    flows.sort_by(|a, b| a.entrypoint_name.cmp(&b.entrypoint_name));
    flows
}

#[expect(clippy::too_many_arguments)]
fn walk_flow(
    graph: &SemanticGraph,
    roles: &HashMap<SymbolId, DerivedRole>,
    scores: &HashMap<SymbolId, i32>,
    start: &SymbolId,
    min_salience: i32,
    max_depth: usize,
) -> Vec<FlowStep> {
    let mut steps = Vec::new();
    let mut visited: HashSet<SymbolId> = HashSet::new();
    visited.insert(start.clone());

    walk_recursive(
        graph,
        roles,
        scores,
        start,
        min_salience,
        max_depth,
        0,
        &mut visited,
        &mut steps,
    );

    steps
}

#[expect(clippy::too_many_arguments)]
fn walk_recursive(
    graph: &SemanticGraph,
    roles: &HashMap<SymbolId, DerivedRole>,
    scores: &HashMap<SymbolId, i32>,
    current: &SymbolId,
    min_salience: i32,
    max_depth: usize,
    depth: usize,
    visited: &mut HashSet<SymbolId>,
    steps: &mut Vec<FlowStep>,
) {
    if depth >= max_depth {
        return;
    }

    let mut out_edges: Vec<_> = graph.outgoing_edges(current).collect();
    // Sort by source-order index so we trace in code order.
    out_edges.sort_by_key(|e| e.order_index);

    for edge in out_edges {
        let Some(target_id) = edge.target.resolved_id() else {
            continue;
        };

        if visited.contains(target_id) {
            continue;
        }

        let target_score = scores.get(target_id).copied().unwrap_or(0);
        if target_score <= min_salience {
            continue;
        }

        let Some(target_sym) = graph.nodes.get(target_id) else {
            continue;
        };

        let stage_label = edge_stage_label(&edge.kind);

        visited.insert(target_id.to_string());
        steps.push(FlowStep {
            symbol_id: target_id.to_string(),
            name: target_sym.name.clone(),
            stage_label,
            order: edge.order_index,
        });

        // Stop descending at external or LowSignal nodes.
        if !matches!(
            roles.get(target_id),
            Some(DerivedRole::LowSignal | DerivedRole::DataCarrier)
        ) {
            let target_id_owned = target_id.to_string();
            walk_recursive(
                graph,
                roles,
                scores,
                &target_id_owned,
                min_salience,
                max_depth,
                depth + 1,
                visited,
                steps,
            );
        }
    }
}

fn edge_stage_label(kind: &EdgeKind) -> String {
    match kind {
        EdgeKind::Constructs => "construct",
        EdgeKind::Reads => "read",
        EdgeKind::Writes => "write",
        EdgeKind::Returns => "return",
        EdgeKind::Throws => "external",
        EdgeKind::Calls | EdgeKind::Imports | EdgeKind::Implements => "call",
    }
    .to_string()
}
