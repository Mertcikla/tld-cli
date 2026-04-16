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

        // ── Positive contributions ────────────────────────────────────────────
        if m.cross_file_out >= 2 {
            s += 4;
        }
        // Write / mutation edges out (approximated by Write kind edges).
        let write_out = count_edge_kind_out(graph, id, |e| {
            matches!(e.kind, super::types::EdgeKind::Writes)
        });
        if write_out > 0 {
            s += 3;
        }
        // High fan-out as a proxy for branching + multiple downstream calls.
        if m.fan_out >= 3 {
            s += 3;
        } else if m.fan_out >= 2 {
            s += 1;
        }
        // Constructs things and fan-out >= 2.
        let construct_out = count_edge_kind_out(graph, id, |e| {
            matches!(e.kind, super::types::EdgeKind::Constructs)
        });
        if construct_out >= 1 && m.fan_out >= 2 {
            s += 2;
        }
        // Reachable from an entrypoint.
        if reachable_from_entry.contains(id) {
            s += 2;
        }
        // Has callers — something depends on this symbol.
        if m.fan_in >= 2 {
            s += 2;
        } else if m.fan_in == 1 {
            s += 1;
        }
        // LSP-resolved edges are higher confidence.
        if m.has_lsp_edges {
            s += 1;
        }

        // ── Negative contributions ────────────────────────────────────────────
        match role {
            DerivedRole::LowSignal | DerivedRole::DataCarrier => {
                s -= 3;
            }
            DerivedRole::Bootstrap => {
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
