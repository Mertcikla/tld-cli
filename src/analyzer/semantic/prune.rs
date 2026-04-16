//! Pruning — removes low-salience nodes and their dangling edges from the
//! semantic graph for the business projection.

use super::types::{SemanticBundle, SemanticEdge, SemanticSymbol, SymbolId};
use std::collections::HashSet;

/// Pruning output: kept symbols, kept edges, and a summary of what was hidden.
pub struct PruneResult {
    pub symbols: Vec<SemanticSymbol>,
    pub edges: Vec<SemanticEdge>,
    pub hidden_count: usize,
}

/// Keep only symbols whose salience score is strictly above `threshold`.
/// All edges whose source or resolved target is hidden are also removed.
pub fn prune(
    bundle: &SemanticBundle,
    scores: &std::collections::HashMap<SymbolId, i32>,
    threshold: i32,
) -> PruneResult {
    let kept_ids: HashSet<&SymbolId> = bundle
        .symbols
        .iter()
        .filter(|s| scores.get(&s.symbol_id).copied().unwrap_or(0) > threshold)
        .map(|s| &s.symbol_id)
        .collect();

    let hidden_count = bundle.symbols.len() - kept_ids.len();

    let symbols: Vec<SemanticSymbol> = bundle
        .symbols
        .iter()
        .filter(|s| kept_ids.contains(&s.symbol_id))
        .cloned()
        .collect();

    let edges: Vec<SemanticEdge> = bundle
        .edges
        .iter()
        .filter(|e| {
            kept_ids.contains(&e.source)
                && e.target
                    .resolved_id()
                    .is_some_and(|t| kept_ids.contains(&t.to_string()))
        })
        .cloned()
        .collect();

    PruneResult {
        symbols,
        edges,
        hidden_count,
    }
}
