//! Role inference — assigns architectural roles to graph nodes using structural
#![allow(dead_code)]
//! evidence rather than filename patterns or framework conventions.

use super::graph::SemanticGraph;
use super::types::SymbolId;
use crate::analyzer::syntax::types::DeclKind;
use std::collections::HashMap;

/// High-level architectural role of a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DerivedRole {
    /// Zero in-repo callers; likely an API handler, main, or callback registration target.
    Entrypoint,
    /// High outbound fan-out, multiple cross-boundary edges, branching control flow.
    Orchestrator,
    /// Primarily communicates with external or unresolved targets.
    Adapter,
    /// Class/struct with no meaningful behavior — pure data shape.
    DataCarrier,
    /// Mostly constructs objects and wires dependencies; little domain branching.
    Bootstrap,
    /// Low fan-in domain impact, no cross-boundary state, reused by many callers.
    Utility,
    /// Class, interface, or trait that primarily defines contracts.
    Interface,
    /// Carries domain meaning but not primarily behavioral.
    DomainType,
    /// Low-confidence structural contribution; likely a constructor or trivial wrapper.
    LowSignal,
}

impl DerivedRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Entrypoint => "entrypoint",
            Self::Orchestrator => "orchestrator",
            Self::Adapter => "adapter",
            Self::DataCarrier => "data_carrier",
            Self::Bootstrap => "bootstrap",
            Self::Utility => "utility",
            Self::Interface => "interface",
            Self::DomainType => "domain_type",
            Self::LowSignal => "low_signal",
        }
    }
}

/// Compute a `DerivedRole` for every node in the graph.
pub fn infer_roles(graph: &SemanticGraph) -> HashMap<SymbolId, DerivedRole> {
    let mut roles = HashMap::new();

    for (id, sym) in &graph.nodes {
        let m = graph.metrics_for(id);
        let role = classify(sym, m, graph);
        roles.insert(id.clone(), role);
    }

    roles
}

fn classify(
    sym: &super::types::SemanticSymbol,
    m: &super::graph::NodeMetrics,
    graph: &SemanticGraph,
) -> DerivedRole {
    // ── Hard rules based on DeclKind ─────────────────────────────────────────

    // Pure data shapes with no call edges out → DataCarrier.
    if sym.kind.is_data_shape() && m.fan_out == 0 {
        return DerivedRole::DataCarrier;
    }

    // Interface / trait containers with no call edges → Interface.
    if matches!(sym.kind, DeclKind::Interface | DeclKind::Trait) && m.fan_out == 0 {
        return DerivedRole::Interface;
    }

    // Constructors that only assign (fan-out ≤ 1, no cross-file) → LowSignal.
    if sym.kind == DeclKind::Constructor && m.fan_out <= 1 && m.cross_file_out == 0 {
        return DerivedRole::LowSignal;
    }

    // Destructors → always LowSignal.
    if sym.kind == DeclKind::Destructor {
        return DerivedRole::LowSignal;
    }

    // ── Graph-structure rules ─────────────────────────────────────────────────

    // Entrypoint: no callers inside the scanned set.
    if m.fan_in == 0 && m.fan_out > 0 {
        // But exclude constructors and trivial wrappers.
        if !matches!(sym.kind, DeclKind::Constructor | DeclKind::Destructor) {
            // If it also has high fan-out → Orchestrator.
            if m.fan_out >= 3 && m.cross_file_out >= 1 {
                return DerivedRole::Orchestrator;
            }
            return DerivedRole::Entrypoint;
        }
    }

    // Orchestrator: high outbound fan-out with cross-file edges.
    if m.fan_out >= 4 && m.cross_file_out >= 2 {
        return DerivedRole::Orchestrator;
    }
    if m.fan_out >= 3 && m.cross_file_out >= 1 {
        return DerivedRole::Orchestrator;
    }

    // Adapter: mostly outbound to unresolved / external targets.
    let unresolved_out = count_unresolved_out(graph, &sym.symbol_id);
    if unresolved_out >= 2 && m.fan_in <= 2 {
        return DerivedRole::Adapter;
    }

    // Bootstrap: container-level symbol that mostly constructs things.
    if sym.kind == DeclKind::Constructor && m.fan_out >= 2 {
        return DerivedRole::Bootstrap;
    }

    // Utility: high fan-in relative to fan-out, no cross-file edges out.
    if m.fan_in >= 3 && m.cross_file_out == 0 {
        return DerivedRole::Utility;
    }

    // Trivial wrapper: single downstream call, no cross-file activity → LowSignal.
    if m.fan_out == 1 && m.cross_file_out == 0 && m.cross_file_in == 0 {
        return DerivedRole::LowSignal;
    }

    // Container with some behavior.
    if sym.kind.is_container() {
        if m.fan_out >= 1 {
            return DerivedRole::DomainType;
        }
        return DerivedRole::DataCarrier;
    }

    // Default for functions / methods with some activity.
    if m.fan_out >= 2 {
        return DerivedRole::DomainType;
    }

    // Single-call functions with no context → LowSignal.
    if m.fan_out <= 1 && m.fan_in <= 1 {
        return DerivedRole::LowSignal;
    }

    DerivedRole::DomainType
}

fn count_unresolved_out(graph: &SemanticGraph, source: &str) -> usize {
    graph
        .outgoing_edges(source)
        .filter(|e| !e.target.is_resolved())
        .count()
}
