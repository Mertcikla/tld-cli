#![allow(dead_code)]
//! Semantic graph — adjacency structure over `SemanticBundle`.
//!
//! Derives per-node metrics (fan-in, fan-out, cross-boundary reach, etc.) that
//! the role and salience layers consume.

use super::types::{EdgeOrigin, SemanticBundle, SemanticEdge, SemanticSymbol, SymbolId};
use std::collections::{HashMap, HashSet};

/// Derived metrics for one node in the graph.
#[derive(Debug, Clone, Default)]
pub struct NodeMetrics {
    /// Number of resolved edges arriving at this symbol.
    pub fan_in: usize,
    /// Number of resolved edges leaving this symbol.
    pub fan_out: usize,
    /// Number of resolved outgoing edges that cross a file boundary.
    pub cross_file_out: usize,
    /// Number of resolved incoming edges that cross a file boundary.
    pub cross_file_in: usize,
    /// True when any outgoing edge has origin == Lsp (reliable resolution).
    pub has_lsp_edges: bool,
}

/// The fully-materialized graph.
pub struct SemanticGraph {
    /// Indexed by SymbolId.
    pub nodes: HashMap<SymbolId, SemanticSymbol>,
    /// All edges.
    pub edges: Vec<SemanticEdge>,
    /// Derived per-node metrics.
    pub metrics: HashMap<SymbolId, NodeMetrics>,
    /// Outgoing adjacency: source → list of edge indices.
    pub outgoing: HashMap<SymbolId, Vec<usize>>,
    /// Incoming adjacency: target → list of edge indices.
    pub incoming: HashMap<SymbolId, Vec<usize>>,
}

impl SemanticGraph {
    /// Build a graph from a `SemanticBundle`.
    pub fn build(bundle: &SemanticBundle) -> Self {
        let nodes: HashMap<SymbolId, SemanticSymbol> = bundle
            .symbols
            .iter()
            .map(|s| (s.symbol_id.clone(), s.clone()))
            .collect();

        let mut outgoing: HashMap<SymbolId, Vec<usize>> = HashMap::new();
        let mut incoming: HashMap<SymbolId, Vec<usize>> = HashMap::new();

        for (i, edge) in bundle.edges.iter().enumerate() {
            outgoing.entry(edge.source.clone()).or_default().push(i);
            if let Some(target_id) = edge.target.resolved_id() {
                incoming.entry(target_id.to_string()).or_default().push(i);
            }
        }

        let mut metrics: HashMap<SymbolId, NodeMetrics> = HashMap::new();
        for sym in &bundle.symbols {
            let id = &sym.symbol_id;
            let out_edges = outgoing.get(id).map_or(&[] as &[_], Vec::as_slice);
            let in_edges = incoming.get(id).map_or(&[] as &[_], Vec::as_slice);

            let fan_out = out_edges.len();
            let fan_in = in_edges.len();
            let cross_file_out = out_edges
                .iter()
                .filter(|&&i| bundle.edges[i].cross_boundary)
                .count();
            let cross_file_in = in_edges
                .iter()
                .filter(|&&i| bundle.edges[i].cross_boundary)
                .count();
            let has_lsp_edges = out_edges
                .iter()
                .any(|&i| bundle.edges[i].origin == EdgeOrigin::Lsp);

            metrics.insert(
                id.clone(),
                NodeMetrics {
                    fan_in,
                    fan_out,
                    cross_file_out,
                    cross_file_in,
                    has_lsp_edges,
                },
            );
        }

        SemanticGraph {
            nodes,
            edges: bundle.edges.clone(),
            metrics,
            outgoing,
            incoming,
        }
    }

    pub fn metrics_for(&self, id: &str) -> &NodeMetrics {
        static EMPTY: NodeMetrics = NodeMetrics {
            fan_in: 0,
            fan_out: 0,
            cross_file_out: 0,
            cross_file_in: 0,
            has_lsp_edges: false,
        };
        self.metrics.get(id).unwrap_or(&EMPTY)
    }

    /// All outgoing edges from `source`.
    pub fn outgoing_edges(&self, source: &str) -> impl Iterator<Item = &SemanticEdge> {
        self.outgoing
            .get(source)
            .map_or(&[] as &[_], Vec::as_slice)
            .iter()
            .map(|&i| &self.edges[i])
    }

    /// Symbols that have no resolved incoming edges from within the scanned set.
    pub fn entrypoint_candidates(&self) -> impl Iterator<Item = &SemanticSymbol> {
        self.nodes
            .values()
            .filter(|sym| self.incoming.get(&sym.symbol_id).is_none_or(Vec::is_empty))
    }

    /// All symbol IDs reachable from `start` following resolved outgoing edges.
    pub fn reachable_from(&self, start: &str) -> HashSet<SymbolId> {
        let mut visited = HashSet::new();
        let mut stack = vec![start.to_string()];
        while let Some(cur) = stack.pop() {
            if !visited.insert(cur.clone()) {
                continue;
            }
            for edge in self.outgoing_edges(&cur) {
                if let Some(tid) = edge.target.resolved_id() {
                    stack.push(tid.to_string());
                }
            }
        }
        visited.remove(start);
        visited
    }
}
