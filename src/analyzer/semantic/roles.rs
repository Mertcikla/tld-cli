//! Role inference — assigns architectural roles to graph nodes using structural
#![allow(dead_code)]
//! evidence rather than filename patterns or framework conventions.

use super::endpoints::detect_endpoint;
use super::graph::SemanticGraph;
use super::types::SymbolId;
use crate::analyzer::syntax::types::DeclKind;
use std::collections::HashMap;
use std::path::Path;

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
    if sym.external {
        return DerivedRole::Adapter;
    }

    // ── Framework-annotation rules ───────────────────────────────────────────

    // Any symbol bearing an HTTP endpoint annotation is an Entrypoint, even if
    // graph metrics would otherwise classify it differently (framework-
    // dispatched handlers look structurally like dead code because the router
    // invokes them reflectively).
    if sym.annotations.iter().any(|a| detect_endpoint(a).is_some()) {
        return DerivedRole::Entrypoint;
    }

    // Name/path heuristics for common scaffolding that graph metrics alone
    // tend to overvalue in layered web backends.
    if looks_like_interface_contract(sym) {
        return DerivedRole::Interface;
    }
    if looks_like_bootstrap_wiring(sym) {
        return DerivedRole::Bootstrap;
    }
    if looks_like_repository(sym) {
        return DerivedRole::Adapter;
    }
    if looks_like_service(sym) {
        return DerivedRole::Orchestrator;
    }
    if looks_like_data_scaffolding(sym) {
        return DerivedRole::DataCarrier;
    }
    if looks_like_support_scaffolding(sym) {
        return DerivedRole::LowSignal;
    }

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

fn looks_like_interface_contract(sym: &super::types::SemanticSymbol) -> bool {
    if matches!(sym.kind, DeclKind::Interface | DeclKind::Trait) {
        return true;
    }

    let name = sym.name.to_ascii_lowercase();
    let path = sym.file_path.to_ascii_lowercase();
    let in_contract_path =
        has_path_segment(&path, &["interface", "interfaces", "contract", "contracts"]);
    in_contract_path
        && matches!(
            sym.kind,
            DeclKind::Class | DeclKind::Struct | DeclKind::Type
        )
        && (name.starts_with('i') || name.ends_with("interface") || name.ends_with("port"))
}

fn looks_like_data_scaffolding(sym: &super::types::SemanticSymbol) -> bool {
    let path = sym.file_path.to_ascii_lowercase();
    let name = sym.name.to_ascii_lowercase();
    let stem = file_stem_lower(&path);
    let scaffold_path = has_path_segment(
        &path,
        &[
            "dto",
            "dtos",
            "schema",
            "schemas",
            "serializer",
            "serializers",
            "validator",
            "validators",
            "request",
            "requests",
            "response",
            "responses",
            "payload",
            "payloads",
            "record",
            "records",
        ],
    ) || contains_any(
        &stem,
        &[
            "dto",
            "schema",
            "serializer",
            "validator",
            "request",
            "response",
        ],
    );

    let scaffold_name = contains_any(
        &name,
        &[
            "dto",
            "schema",
            "serializer",
            "validator",
            "request",
            "response",
            "payload",
            "record",
        ],
    );

    match sym.kind {
        DeclKind::Class
        | DeclKind::Struct
        | DeclKind::Enum
        | DeclKind::Type
        | DeclKind::Field
        | DeclKind::Variable => scaffold_path || scaffold_name,
        DeclKind::Method | DeclKind::Function => {
            scaffold_path
                && (name.starts_with("from_")
                    || name.starts_with("to_")
                    || name == "bind"
                    || name == "response")
        }
        _ => false,
    }
}

fn looks_like_repository(sym: &super::types::SemanticSymbol) -> bool {
    let path = sym.file_path.to_ascii_lowercase();
    let name = sym.name.to_ascii_lowercase();
    has_path_segment(&path, &["repository", "repositories", "repo", "repos"])
        || contains_any(&name, &["repository", "repo"])
}

fn looks_like_service(sym: &super::types::SemanticSymbol) -> bool {
    let path = sym.file_path.to_ascii_lowercase();
    let name = sym.name.to_ascii_lowercase();
    has_path_segment(&path, &["service", "services", "controller", "controllers"])
        || contains_any(&name, &["service", "controller"])
}

fn looks_like_bootstrap_wiring(sym: &super::types::SemanticSymbol) -> bool {
    let path = sym.file_path.to_ascii_lowercase();
    let name = sym.name.to_ascii_lowercase();
    let stem = file_stem_lower(&path);

    has_path_segment(
        &path,
        &[
            "config",
            "configs",
            "settings",
            "provider",
            "providers",
            "container",
            "containers",
            "bootstrap",
            "boot",
            "startup",
            "wiring",
        ],
    ) || contains_any(
        &stem,
        &[
            "config",
            "settings",
            "provider",
            "container",
            "bootstrap",
            "startup",
        ],
    ) || contains_any(
        &name,
        &[
            "create_app",
            "configure",
            "register",
            "provider",
            "container",
            "bootstrap",
        ],
    )
}

fn looks_like_support_scaffolding(sym: &super::types::SemanticSymbol) -> bool {
    let path = sym.file_path.to_ascii_lowercase();
    let name = sym.name.to_ascii_lowercase();
    let stem = file_stem_lower(&path);

    has_path_segment(
        &path,
        &[
            "exception",
            "exceptions",
            "error",
            "errors",
            "logging",
            "middleware",
            "middlewares",
            "auth",
            "security",
            "util",
            "utils",
            "helper",
            "helpers",
        ],
    ) || contains_any(
        &stem,
        &[
            "exception",
            "error",
            "logging",
            "middleware",
            "auth",
            "security",
            "util",
            "helper",
        ],
    ) || contains_any(
        &name,
        &[
            "exception",
            "error",
            "middleware",
            "token",
            "password",
            "hash",
            "helper",
        ],
    )
}

fn has_path_segment(path: &str, needles: &[&str]) -> bool {
    Path::new(path).components().any(|component| {
        let segment = component.as_os_str().to_string_lossy().to_ascii_lowercase();
        needles.iter().any(|needle| segment == *needle)
    })
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn file_stem_lower(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(path)
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::semantic::graph::SemanticGraph;
    use crate::analyzer::semantic::types::{
        ControlMetrics, SemanticBundle, SemanticSymbol, SymbolSpans, Visibility,
    };

    fn sym(name: &str, kind: DeclKind, file_path: &str) -> SemanticSymbol {
        SemanticSymbol {
            symbol_id: format!("repo:{file_path}:{name}"),
            repo_name: "repo".to_string(),
            file_path: file_path.to_string(),
            name: name.to_string(),
            kind,
            owner: None,
            visibility: Visibility::Unknown,
            external: false,
            description: String::new(),
            spans: SymbolSpans::default(),
            control: ControlMetrics::default(),
            annotations: Vec::new(),
        }
    }

    #[test]
    fn classify_marks_dto_classes_as_data_carriers() {
        let bundle = SemanticBundle {
            symbols: vec![sym(
                "UserLoginRequest",
                DeclKind::Class,
                "conduit/api/schemas/requests/user.py",
            )],
            ..Default::default()
        };
        let graph = SemanticGraph::build(&bundle);
        let roles = infer_roles(&graph);
        assert_eq!(
            roles["repo:conduit/api/schemas/requests/user.py:UserLoginRequest"],
            DerivedRole::DataCarrier
        );
    }

    #[test]
    fn classify_marks_container_wiring_as_bootstrap() {
        let bundle = SemanticBundle {
            symbols: vec![sym(
                "get_article_service",
                DeclKind::Function,
                "conduit/core/providers.py",
            )],
            ..Default::default()
        };
        let graph = SemanticGraph::build(&bundle);
        let roles = infer_roles(&graph);
        assert_eq!(
            roles["repo:conduit/core/providers.py:get_article_service"],
            DerivedRole::Bootstrap
        );
    }

    #[test]
    fn classify_marks_validators_as_low_signal() {
        let bundle = SemanticBundle {
            symbols: vec![sym("Bind", DeclKind::Method, "users/validators.go")],
            ..Default::default()
        };
        let graph = SemanticGraph::build(&bundle);
        let roles = infer_roles(&graph);
        assert!(matches!(
            roles["repo:users/validators.go:Bind"],
            DerivedRole::LowSignal | DerivedRole::DataCarrier
        ));
    }
}
