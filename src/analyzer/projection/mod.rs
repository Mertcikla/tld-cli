pub mod business;
pub mod data_flow;
pub mod structural;
pub mod tags;

use crate::analyzer::semantic::{
    endpoints::detect_endpoint,
    roles::DerivedRole,
    types::{EdgeKind, SemanticSymbol},
};
use crate::analyzer::syntax::types::DeclKind;
use crate::workspace::{slugify, types::Connector};
use std::collections::HashMap;

/// Which projection view to use when generating workspace output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ViewMode {
    /// Current inventory-style output: files, folders, symbols, bare-name connectors.
    #[default]
    Structural,
    /// Semantic projection: high-salience symbols and LSP-resolved connections only.
    Business,
    /// Data-flow projection: symbols on traced flow paths around entrypoints.
    DataFlow,
}

impl ViewMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "structural" | "structure" => Some(Self::Structural),
            "business" | "biz" => Some(Self::Business),
            "data-flow" | "dataflow" | "data_flow" => Some(Self::DataFlow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ElementPresentation {
    pub kind: String,
    pub technology: String,
    pub symbol_kind: String,
}

pub fn present_symbol(sym: &SemanticSymbol, role: Option<&DerivedRole>) -> ElementPresentation {
    let symbol_kind = if sym.external {
        String::new()
    } else {
        sym.kind.as_str().to_string()
    };

    if sym.external {
        return ElementPresentation {
            kind: sym
                .description
                .strip_prefix("infra:")
                .and_then(|rest| rest.split(':').next())
                .unwrap_or("external")
                .to_string(),
            technology: sym.name.clone(),
            symbol_kind,
        };
    }

    let endpoint_like = sym
        .annotations
        .iter()
        .any(|annotation| detect_endpoint(annotation).is_some());

    let (kind, technology) = match role {
        Some(DerivedRole::Entrypoint) => {
            if endpoint_like {
                ("endpoint".to_string(), "HTTP Endpoint".to_string())
            } else {
                ("entrypoint".to_string(), "Entrypoint".to_string())
            }
        }
        Some(DerivedRole::Orchestrator) => ("service".to_string(), "Service".to_string()),
        Some(DerivedRole::Adapter) => {
            let kind = if looks_like_repository(sym) {
                "repository"
            } else if looks_like_cache_client(sym) {
                "cache-client"
            } else {
                "adapter"
            };
            (kind.to_string(), "Adapter".to_string())
        }
        Some(DerivedRole::DataCarrier | DerivedRole::DomainType) => {
            ("model".to_string(), "Domain Model".to_string())
        }
        Some(DerivedRole::Bootstrap) => {
            ("entrypoint-bootstrap".to_string(), "Bootstrap".to_string())
        }
        Some(DerivedRole::Interface) => ("interface".to_string(), "Interface".to_string()),
        Some(DerivedRole::Utility | DerivedRole::LowSignal) | None => fallback_presentation(sym),
    };

    ElementPresentation {
        kind,
        technology,
        symbol_kind,
    }
}

pub fn unique_slug(name: &str, file_path: &str, registry: &mut HashMap<String, usize>) -> String {
    let base = slugify(name);
    let count = registry.entry(base.clone()).or_insert(0);
    if *count == 0 {
        *count += 1;
        base
    } else {
        *count += 1;
        let stem = std::path::Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("x");
        format!("{}-{}", slugify(stem), base)
    }
}

pub fn domain_for_symbol(sym: &SemanticSymbol) -> String {
    if sym.external {
        return "infrastructure".to_string();
    }
    for annotation in &sym.annotations {
        if let Some(endpoint) = detect_endpoint(annotation)
            && !endpoint.path.is_empty()
            && let Some(domain) = domain_from_path_template(&endpoint.path)
        {
            return domain;
        }
    }
    domain_from_file_path(&sym.file_path)
        .or_else(|| domain_from_symbol_name(&sym.name))
        .unwrap_or_else(|| "misc".to_string())
}

pub fn edge_labels(kind: &EdgeKind) -> (&'static str, &'static str) {
    match kind {
        EdgeKind::Calls => ("calls", "uses"),
        EdgeKind::Imports => ("references", "depends_on"),
        EdgeKind::DependsOn => ("depends_on", "depends_on"),
        EdgeKind::Constructs => ("constructs", "creates"),
        EdgeKind::Reads => ("reads", "reads"),
        EdgeKind::Writes => ("writes", "mutates"),
        EdgeKind::Returns => ("returns", "provides"),
        EdgeKind::Throws => ("throws", "raises"),
        EdgeKind::Extends => ("extends", "extends"),
        EdgeKind::Implements => ("implements", "implements"),
    }
}

pub fn collapse_connectors(connectors: Vec<Connector>) -> Vec<Connector> {
    type ConnectorKey = (String, String, String, String);
    type CollapsedConnector = (Connector, usize);

    let mut collapsed: HashMap<ConnectorKey, CollapsedConnector> = HashMap::new();

    for connector in connectors {
        let key = (
            connector.view.clone(),
            connector.source.clone(),
            connector.target.clone(),
            connector.label.clone(),
        );
        let entry = collapsed
            .entry(key)
            .or_insert_with(|| (connector.clone(), 0));
        entry.1 += 1;
        if entry.0.relationship.is_empty() {
            entry.0.relationship = connector.relationship;
        }
        if entry.0.direction.is_empty() {
            entry.0.direction = connector.direction;
        }
        if entry.0.style.is_empty() {
            entry.0.style = connector.style;
        }
        if entry.0.url.is_empty() {
            entry.0.url = connector.url;
        }
    }

    let mut out: Vec<Connector> = collapsed
        .into_values()
        .map(|(mut connector, count)| {
            if count > 1 {
                connector.description = if connector.description.is_empty() {
                    format!("collapsed_edges={count}")
                } else {
                    format!("{}, collapsed_edges={count}", connector.description)
                };
            }
            connector
        })
        .collect();
    out.sort_by(|a, b| {
        (&a.view, &a.source, &a.target, &a.label).cmp(&(&b.view, &b.source, &b.target, &b.label))
    });
    out
}

fn fallback_presentation(sym: &SemanticSymbol) -> (String, String) {
    match sym.kind {
        DeclKind::Interface | DeclKind::Trait => ("interface".to_string(), "Interface".to_string()),
        DeclKind::Class | DeclKind::Struct | DeclKind::Enum | DeclKind::Type => {
            ("model".to_string(), "Component".to_string())
        }
        DeclKind::Constructor | DeclKind::Destructor => {
            ("entrypoint-bootstrap".to_string(), "Bootstrap".to_string())
        }
        DeclKind::Function | DeclKind::Method => ("utility".to_string(), "Utility".to_string()),
        _ => ("utility".to_string(), "Symbol".to_string()),
    }
}

fn domain_from_path_template(path: &str) -> Option<String> {
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    for segment in segments {
        if matches!(segment, "api" | "v1" | "v2") || segment.starts_with('{') {
            continue;
        }
        return Some(normalize_domain(segment));
    }
    None
}

fn domain_from_file_path(file_path: &str) -> Option<String> {
    let path = std::path::Path::new(file_path);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let generic = [
        "index",
        "main",
        "app",
        "api",
        "core",
        "service",
        "services",
        "repository",
        "repositories",
        "model",
        "models",
        "controller",
        "controllers",
        "route",
        "routes",
        "router",
        "routers",
        "handler",
        "handlers",
        "view",
        "views",
        "schema",
        "schemas",
        "serializer",
        "serializers",
        "validator",
        "validators",
        "dependency",
        "dependencies",
        "middleware",
        "middlewares",
        "utils",
        "helpers",
        "impl",
    ];

    if !stem.is_empty() && !generic.contains(&stem) {
        return Some(domain_from_symbol_name(stem).unwrap_or_else(|| normalize_domain(stem)));
    }

    for component in path.ancestors().skip(1) {
        let Some(name) = component.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !generic.contains(&name) && !name.is_empty() {
            return Some(normalize_domain(name));
        }
    }
    None
}

fn domain_from_symbol_name(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let suffixes = [
        "service",
        "repository",
        "controller",
        "handler",
        "model",
        "dto",
        "request",
        "response",
    ];
    for suffix in suffixes {
        if let Some(prefix) = lower.strip_suffix(suffix)
            && !prefix.is_empty()
        {
            return Some(normalize_domain(prefix));
        }
    }
    if let Some((prefix, _)) = lower.split_once('_')
        && !prefix.is_empty()
    {
        return Some(normalize_domain(prefix));
    }
    None
}

fn normalize_domain(raw: &str) -> String {
    match raw
        .trim_matches('_')
        .trim_matches('-')
        .to_ascii_lowercase()
        .as_str()
    {
        "users" => "user".to_string(),
        "profiles" => "profile".to_string(),
        "articles" => "article".to_string(),
        "comments" => "comment".to_string(),
        "favorites" => "favorite".to_string(),
        "follows" => "follow".to_string(),
        "tags" => "tag".to_string(),
        "auth" | "authentication" => "auth".to_string(),
        other if other.ends_with('s') && other.len() > 3 => other.trim_end_matches('s').to_string(),
        other => other.to_string(),
    }
}

fn looks_like_repository(sym: &SemanticSymbol) -> bool {
    let lower_name = sym.name.to_ascii_lowercase();
    let lower_path = sym.file_path.to_ascii_lowercase();
    lower_name.contains("repo")
        || lower_name.contains("repository")
        || lower_path.contains("/repository")
        || lower_path.contains("/repositories/")
        || lower_path.contains("/persistent/")
        || lower_path.contains("/persistence/")
}

fn looks_like_cache_client(sym: &SemanticSymbol) -> bool {
    let lower_name = sym.name.to_ascii_lowercase();
    let lower_path = sym.file_path.to_ascii_lowercase();
    lower_name.contains("cache")
        || lower_name.contains("redis")
        || lower_path.contains("/cache")
        || lower_path.contains("/redis")
}
