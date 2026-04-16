//! Semantic types — the result of enriching syntax facts with LSP resolution.
#![allow(dead_code)]

use crate::analyzer::syntax::types::DeclKind;

/// Stable, globally-unique identifier for a symbol.
/// Format: `{repo_name}:{file_rel}:{name}` or `{repo_name}:{file_rel}:{parent}::{name}`.
pub type SymbolId = String;

/// Visibility of a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Protected,
    Private,
    PackagePrivate,
    Unknown,
}

/// Where an edge's resolution came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeOrigin {
    /// Resolved by the LSP (call hierarchy, definition, or references).
    Lsp,
    /// Resolved by matching the target_path set from an import statement.
    Import,
    /// Resolved by bare-name slug matching (lowest confidence).
    BareName,
    /// The edge target is outside the scanned repository.
    External,
}

/// Kind of semantic relationship on an edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeKind {
    Calls,
    Imports,
    Constructs,
    Reads,
    Writes,
    Returns,
    Throws,
    Implements,
}

/// Target of a semantic edge — either a resolved internal symbol or an external name.
#[derive(Debug, Clone)]
pub enum EdgeTarget {
    Resolved(SymbolId),
    Unresolved(String),
    External(String),
}

impl EdgeTarget {
    pub fn resolved_id(&self) -> Option<&str> {
        match self {
            Self::Resolved(id) => Some(id.as_str()),
            _ => None,
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::Resolved(_))
    }
}

/// A span of body and signature lines for a symbol.
#[derive(Debug, Clone, Default)]
pub struct SymbolSpans {
    pub body_start: u32,
    pub body_end: u32,
    pub sig_line: u32,
}

/// Per-symbol control-flow counts derived from syntax blocks.
#[derive(Debug, Clone, Default)]
pub struct ControlMetrics {
    pub branches: u32,
    pub loops: u32,
    pub tries: u32,
    pub early_returns: u32,
}

/// A fully-resolved semantic symbol.
#[derive(Debug, Clone)]
pub struct SemanticSymbol {
    pub symbol_id: SymbolId,
    pub repo_name: String,
    pub file_path: String,
    pub name: String,
    pub kind: DeclKind,
    pub owner: Option<SymbolId>,
    pub visibility: Visibility,
    /// True when the symbol target is outside the scanned repositories.
    pub external: bool,
    pub description: String,
    pub spans: SymbolSpans,
    pub control: ControlMetrics,
}

/// A directed edge in the semantic graph.
#[derive(Debug, Clone)]
pub struct SemanticEdge {
    pub source: SymbolId,
    pub target: EdgeTarget,
    pub kind: EdgeKind,
    pub origin: EdgeOrigin,
    /// Call-site order within the source body.
    pub order_index: usize,
    /// True when source and target are in different files or packages.
    pub cross_boundary: bool,
}

/// An unresolved reference kept for observability.
#[derive(Debug, Clone)]
pub struct UnresolvedRef {
    pub source: SymbolId,
    pub text: String,
    pub kind: EdgeKind,
}

/// The output of the semantic enrichment stage.
#[derive(Debug, Clone, Default)]
pub struct SemanticBundle {
    pub symbols: Vec<SemanticSymbol>,
    pub edges: Vec<SemanticEdge>,
    pub unresolved_refs: Vec<UnresolvedRef>,
}

impl SemanticBundle {
    pub fn merge(&mut self, other: SemanticBundle) {
        self.symbols.extend(other.symbols);
        self.edges.extend(other.edges);
        self.unresolved_refs.extend(other.unresolved_refs);
    }
}
