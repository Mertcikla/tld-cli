#![allow(dead_code)]
//! Salience scoring — quantifies how architecturally significant each symbol is.
//!
//! Scores are integers. Positive scores indicate signal; negative scores indicate
//! low-value implementation detail. Default pruning threshold is 0 (hide ≤ 0).

use super::graph::SemanticGraph;
use super::roles::DerivedRole;
use super::types::SymbolId;
use crate::analyzer::syntax::types::DeclKind;
use std::collections::HashMap;

/// Compute a salience score for every node in the graph.
///
/// Score inputs (from the spec):
/// - `+4` two or more resolved cross-boundary outgoing edges
/// - `+3` mutates state or persists values (approximated by Write edges out)
/// - `+3` branching plus multiple downstream calls (approximated by fan-out ≥ 3)
/// - `+2` constructs domain objects later returned or persisted
/// - `+2` on a path from an entrypoint to an external side effect
/// - `+2` any callers exist (has non-zero fan-in)
/// - `+2` high cyclomatic complexity (generic Phase 2 heuristic)
/// - `+1` high cognitive complexity (generic Phase 2 heuristic)
/// - `-3` constructor with no meaningful branching or side effects
/// - `-3` pure data carrier with no behavior
/// - `-2` trivial wrapper: one downstream call, no mutation, no branching
/// - `-2` bootstrap-only wiring
/// - `-2` isolated: zero callers and zero callees
///
/// After individual scoring, a second pass propagates method scores to their
/// owner containers so that `OrderService` stays visible when `placeOrder` does.
pub fn score_all(
    graph: &SemanticGraph,
    roles: &HashMap<SymbolId, DerivedRole>,
) -> HashMap<SymbolId, i32> {
    let entrypoint_ids: Vec<&str> = roles
        .iter()
        .filter(|(_, r)| **r == DerivedRole::Entrypoint || **r == DerivedRole::Orchestrator)
        .map(|(id, _)| id.as_str())
        .collect();

    // Pre-compute which symbols are reachable from entrypoints.
    let mut reachable_from_entry: std::collections::HashSet<SymbolId> =
        std::collections::HashSet::new();
    for ep in &entrypoint_ids {
        reachable_from_entry.extend(graph.reachable_from(ep));
    }

    let mut scores: HashMap<SymbolId, i32> = HashMap::new();

    for (id, sym) in &graph.nodes {
        let m = graph.metrics_for(id);
        let role = roles.get(id).unwrap_or(&DerivedRole::LowSignal);
        let mut s: i32 = 0;
        let suppress_graph_bonuses = matches!(
            role,
            DerivedRole::LowSignal
                | DerivedRole::DataCarrier
                | DerivedRole::Bootstrap
                | DerivedRole::Interface
        );

        // ── Positive contributions ────────────────────────────────────────────
        if !suppress_graph_bonuses && m.cross_file_out >= 2 {
            s += 4;
        }
        // Write / mutation edges out (approximated by Write kind edges).
        let write_out = count_edge_kind_out(graph, id, |e| {
            matches!(e.kind, super::types::EdgeKind::Writes)
        });
        if !suppress_graph_bonuses && write_out > 0 {
            s += 3;
        }
        // High fan-out as a proxy for branching + multiple downstream calls.
        if !suppress_graph_bonuses && m.fan_out >= 3 {
            s += 3;
        } else if !suppress_graph_bonuses && m.fan_out >= 2 {
            s += 1;
        }
        // Constructs things and fan-out >= 2.
        let construct_out = count_edge_kind_out(graph, id, |e| {
            matches!(e.kind, super::types::EdgeKind::Constructs)
        });
        if !suppress_graph_bonuses && construct_out >= 1 && m.fan_out >= 2 {
            s += 2;
        }
        // Reachable from an entrypoint.
        if !suppress_graph_bonuses && reachable_from_entry.contains(id) {
            s += 2;
        }
        // Has callers — something depends on this symbol.
        if !suppress_graph_bonuses && m.fan_in >= 2 {
            s += 2;
        } else if !suppress_graph_bonuses && m.fan_in == 1 {
            s += 1;
        }
        if !suppress_graph_bonuses && m.cyclomatic_complexity >= 5 {
            s += 2;
        }
        if !suppress_graph_bonuses && m.cognitive_complexity >= 7 {
            s += 1;
        }
        // LSP-resolved edges are higher confidence.
        if !suppress_graph_bonuses && m.has_lsp_edges {
            s += 1;
        }

        // ── Negative contributions ────────────────────────────────────────────
        match role {
            DerivedRole::LowSignal | DerivedRole::DataCarrier => {
                s -= 4;
            }
            DerivedRole::Bootstrap => {
                s -= 4;
            }
            DerivedRole::Interface => {
                s -= 3;
            }
            DerivedRole::Utility => {
                s -= 2;
            }
            _ => {}
        }

        // Constructor with no cross-boundary activity.
        if sym.kind == DeclKind::Constructor && m.cross_file_out == 0 {
            s -= 3;
        }

        // Trivial wrapper: single call, no cross-boundary.
        if m.fan_out == 1 && m.cross_file_out == 0 && m.fan_in <= 1 {
            s -= 2;
        }

        // Isolated leaf: no callers, no callees.
        if m.fan_in == 0 && m.fan_out == 0 {
            s -= 2;
        }

        scores.insert(id.clone(), s);
    }

    // ── Second pass: propagate method scores to their owner containers ────────
    // A class/struct/trait inherits max(0, best_method_score / 2) so that
    // containers remain visible when their methods are architecturally significant,
    // even though the container node itself has no direct call edges.
    let owner_map: HashMap<SymbolId, SymbolId> = graph
        .nodes
        .values()
        .filter_map(|sym| {
            sym.owner
                .as_ref()
                .map(|o| (sym.symbol_id.clone(), o.clone()))
        })
        .collect();

    let child_scores: Vec<(SymbolId, i32)> = owner_map
        .iter()
        .filter_map(|(child, owner)| scores.get(child).copied().map(|s| (owner.clone(), s)))
        .collect();

    for (owner_id, child_score) in child_scores {
        if child_score > 0 {
            let entry = scores.entry(owner_id).or_insert(0);
            // Propagate half of child's positive score, at least +1.
            let bonus = (child_score / 2).max(1);
            if *entry < bonus {
                *entry = bonus;
            }
        }
    }

    scores
}

fn count_edge_kind_out<F>(graph: &SemanticGraph, source: &str, pred: F) -> usize
where
    F: Fn(&super::types::SemanticEdge) -> bool,
{
    graph.outgoing_edges(source).filter(|e| pred(e)).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::semantic::graph::SemanticGraph;
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
    fn salience_rewards_complex_orchestration_more_than_trivial_wrapper() {
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
                    "repo:src/file.ts:wrapper",
                    "wrapper",
                    DeclKind::Function,
                    60,
                    63,
                ),
                make_symbol("repo:src/file.ts:leaf", "leaf", DeclKind::Function, 65, 66),
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
                SemanticEdge {
                    source: "repo:src/file.ts:wrapper".to_string(),
                    target: EdgeTarget::Resolved("repo:src/file.ts:leaf".to_string()),
                    kind: EdgeKind::Calls,
                    origin: EdgeOrigin::BareName,
                    order_index: 2,
                    cross_boundary: false,
                },
            ],
            unresolved_refs: vec![],
        };

        let graph = SemanticGraph::build(&bundle);
        let roles = crate::analyzer::semantic::roles::infer_roles(&graph);
        let scores = score_all(&graph, &roles);

        assert!(scores["repo:src/file.ts:orchestrate"] > scores["repo:src/file.ts:wrapper"]);
        assert!(
            graph
                .metrics_for("repo:src/file.ts:orchestrate")
                .cyclomatic_complexity
                >= 5
        );
    }

    #[test]
    fn salience_penalizes_bootstrap_even_when_wiring_many_dependencies() {
        let bundle = SemanticBundle {
            symbols: vec![
                make_symbol(
                    "repo:src/container.ts:Container",
                    "Container",
                    DeclKind::Class,
                    1,
                    20,
                ),
                make_symbol(
                    "repo:src/container.ts:wire",
                    "wire",
                    DeclKind::Function,
                    22,
                    35,
                ),
                make_symbol(
                    "repo:src/user.ts:UserService",
                    "UserService",
                    DeclKind::Class,
                    40,
                    55,
                ),
                make_symbol(
                    "repo:src/article.ts:ArticleService",
                    "ArticleService",
                    DeclKind::Class,
                    57,
                    72,
                ),
                make_symbol(
                    "repo:src/profile.ts:ProfileService",
                    "ProfileService",
                    DeclKind::Class,
                    74,
                    89,
                ),
            ],
            edges: vec![
                SemanticEdge {
                    source: "repo:src/container.ts:wire".to_string(),
                    target: EdgeTarget::Resolved("repo:src/user.ts:UserService".to_string()),
                    kind: EdgeKind::Calls,
                    origin: EdgeOrigin::BareName,
                    order_index: 0,
                    cross_boundary: true,
                },
                SemanticEdge {
                    source: "repo:src/container.ts:wire".to_string(),
                    target: EdgeTarget::Resolved("repo:src/article.ts:ArticleService".to_string()),
                    kind: EdgeKind::Calls,
                    origin: EdgeOrigin::BareName,
                    order_index: 1,
                    cross_boundary: true,
                },
                SemanticEdge {
                    source: "repo:src/container.ts:wire".to_string(),
                    target: EdgeTarget::Resolved("repo:src/profile.ts:ProfileService".to_string()),
                    kind: EdgeKind::Calls,
                    origin: EdgeOrigin::BareName,
                    order_index: 2,
                    cross_boundary: true,
                },
            ],
            unresolved_refs: vec![],
        };

        let graph = SemanticGraph::build(&bundle);
        let mut roles = crate::analyzer::semantic::roles::infer_roles(&graph);
        roles.insert(
            "repo:src/container.ts:wire".to_string(),
            DerivedRole::Bootstrap,
        );

        let scores = score_all(&graph, &roles);
        assert!(scores["repo:src/container.ts:wire"] <= -1);
    }
}
