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
    /// Best-effort cyclomatic complexity signal.
    ///
    /// Phase 2 uses a language-agnostic heuristic derived from body span length
    /// and interaction count. Phase 3 can replace this with parser-populated
    /// control-region counts without changing downstream salience consumers.
    pub cyclomatic_complexity: u32,
    /// Best-effort cognitive complexity signal.
    pub cognitive_complexity: u32,
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
            let (cyclomatic_complexity, cognitive_complexity) = estimate_complexity(sym, fan_out);

            metrics.insert(
                id.clone(),
                NodeMetrics {
                    fan_in,
                    fan_out,
                    cross_file_out,
                    cross_file_in,
                    has_lsp_edges,
                    cyclomatic_complexity,
                    cognitive_complexity,
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
            cyclomatic_complexity: 0,
            cognitive_complexity: 0,
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

fn estimate_complexity(sym: &SemanticSymbol, fan_out: usize) -> (u32, u32) {
    use crate::analyzer::syntax::types::DeclKind;

    if !matches!(
        sym.kind,
        DeclKind::Function | DeclKind::Method | DeclKind::Constructor | DeclKind::Destructor
    ) {
        return (0, 0);
    }

    let control_regions = sym.control.branches + sym.control.loops + sym.control.tries;
    if control_regions > 0 || sym.control.early_returns > 0 {
        let interaction_branches = u32::try_from(fan_out.saturating_sub(1)).unwrap_or(u32::MAX);
        let cyclomatic_complexity = 1 + control_regions + interaction_branches;
        let cognitive_complexity = cyclomatic_complexity
            + sym.control.branches
            + sym.control.loops
            + sym.control.early_returns;
        return (cyclomatic_complexity, cognitive_complexity);
    }

    let body_lines = sym.spans.body_end.saturating_sub(sym.spans.body_start);
    let structural_branches = body_lines / 12;
    let interaction_branches = u32::try_from(fan_out.saturating_sub(1)).unwrap_or(u32::MAX);
    let cyclomatic_complexity = 1 + structural_branches + interaction_branches;
    let cognitive_complexity = cyclomatic_complexity + (body_lines / 20);

    (cyclomatic_complexity, cognitive_complexity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::semantic::types::{
        ControlMetrics, EdgeKind, EdgeOrigin, EdgeTarget, SemanticBundle, SemanticEdge,
        SemanticSymbol, SymbolSpans, Visibility,
    };
    use crate::analyzer::syntax::types::DeclKind;

    fn make_symbol(id: &str, name: &str, kind: DeclKind, start: u32, end: u32) -> SemanticSymbol {
        SemanticSymbol {
            symbol_id: id.to_string(),
            repo_name: "repo".to_string(),
            file_path: "src/file.ts".to_string(),
            name: name.to_string(),
            kind,
            owner: None,
            visibility: Visibility::Unknown,
            external: false,
            description: String::new(),
            spans: SymbolSpans {
                body_start: start,
                body_end: end,
                sig_line: start,
            },
            control: ControlMetrics::default(),
            annotations: Vec::new(),
        }
    }

    #[test]
    fn graph_metrics_estimate_complexity_for_behavioral_symbols() {
        let bundle = SemanticBundle {
            symbols: vec![
                make_symbol(
                    "repo:src/file.ts:orchestrate",
                    "orchestrate",
                    DeclKind::Function,
                    1,
                    48,
                ),
                make_symbol(
                    "repo:src/file.ts:step1",
                    "step1",
                    DeclKind::Function,
                    50,
                    52,
                ),
                make_symbol(
                    "repo:src/file.ts:step2",
                    "step2",
                    DeclKind::Function,
                    54,
                    56,
                ),
                make_symbol(
                    "repo:src/file.ts:OrderService",
                    "OrderService",
                    DeclKind::Class,
                    60,
                    120,
                ),
            ],
            edges: vec![
                SemanticEdge {
                    source: "repo:src/file.ts:orchestrate".to_string(),
                    target: EdgeTarget::Resolved("repo:src/file.ts:step1".to_string()),
                    kind: EdgeKind::Calls,
                    origin: EdgeOrigin::BareName,
                    order_index: 0,
                    cross_boundary: false,
                },
                SemanticEdge {
                    source: "repo:src/file.ts:orchestrate".to_string(),
                    target: EdgeTarget::Resolved("repo:src/file.ts:step2".to_string()),
                    kind: EdgeKind::Calls,
                    origin: EdgeOrigin::BareName,
                    order_index: 1,
                    cross_boundary: false,
                },
            ],
            unresolved_refs: vec![],
        };

        let graph = SemanticGraph::build(&bundle);
        let orchestrate = graph.metrics_for("repo:src/file.ts:orchestrate");
        let container = graph.metrics_for("repo:src/file.ts:OrderService");

        assert!(orchestrate.cyclomatic_complexity >= 5);
        assert!(orchestrate.cognitive_complexity >= orchestrate.cyclomatic_complexity);
        assert_eq!(container.cyclomatic_complexity, 0);
        assert_eq!(container.cognitive_complexity, 0);
    }

    #[test]
    fn graph_metrics_prefer_parser_backed_control_counts() {
        let bundle = SemanticBundle {
            symbols: vec![SemanticSymbol {
                symbol_id: "repo:src/file.ts:orchestrate".to_string(),
                repo_name: "repo".to_string(),
                file_path: "src/file.ts".to_string(),
                name: "orchestrate".to_string(),
                kind: DeclKind::Function,
                owner: None,
                visibility: Visibility::Unknown,
                external: false,
                description: String::new(),
                spans: SymbolSpans {
                    body_start: 1,
                    body_end: 6,
                    sig_line: 1,
                },
                control: ControlMetrics {
                    branches: 2,
                    loops: 1,
                    tries: 1,
                    early_returns: 2,
                },
                annotations: Vec::new(),
            }],
            edges: vec![],
            unresolved_refs: vec![],
        };

        let graph = SemanticGraph::build(&bundle);
        let metrics = graph.metrics_for("repo:src/file.ts:orchestrate");

        assert_eq!(metrics.cyclomatic_complexity, 5);
        assert_eq!(metrics.cognitive_complexity, 10);
    }
}
