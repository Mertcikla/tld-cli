//! Infrastructure registry and external node synthesis.

use super::types::{
    ControlMetrics, EdgeKind, EdgeOrigin, EdgeTarget, SemanticBundle, SemanticEdge, SemanticSymbol,
    SymbolSpans, Visibility,
};
use crate::analyzer::syntax::types::DeclKind;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy)]
pub struct InfraMatch {
    pub technology: &'static str,
    pub kind: &'static str,
    pub category: &'static str,
}

const RULES: &[(&str, InfraMatch)] = &[
    (
        "postgres",
        InfraMatch {
            technology: "PostgreSQL",
            kind: "database",
            category: "sql",
        },
    ),
    (
        "psycopg",
        InfraMatch {
            technology: "PostgreSQL",
            kind: "database",
            category: "sql",
        },
    ),
    (
        "sqlalchemy",
        InfraMatch {
            technology: "SQL Database",
            kind: "database",
            category: "sql",
        },
    ),
    (
        "diesel",
        InfraMatch {
            technology: "SQL Database",
            kind: "database",
            category: "sql",
        },
    ),
    (
        "hibernate",
        InfraMatch {
            technology: "SQL Database",
            kind: "database",
            category: "sql",
        },
    ),
    (
        "jpa",
        InfraMatch {
            technology: "SQL Database",
            kind: "database",
            category: "sql",
        },
    ),
    (
        "redis",
        InfraMatch {
            technology: "Redis",
            kind: "cache",
            category: "kv",
        },
    ),
    (
        "mongo",
        InfraMatch {
            technology: "MongoDB",
            kind: "database",
            category: "doc",
        },
    ),
    (
        "odmantic",
        InfraMatch {
            technology: "MongoDB",
            kind: "database",
            category: "doc",
        },
    ),
    (
        "dynamodb",
        InfraMatch {
            technology: "DynamoDB",
            kind: "database",
            category: "kv",
        },
    ),
    (
        "boto3",
        InfraMatch {
            technology: "DynamoDB",
            kind: "database",
            category: "kv",
        },
    ),
    (
        "jwt",
        InfraMatch {
            technology: "JWT",
            kind: "external",
            category: "auth",
        },
    ),
    (
        "pyjwt",
        InfraMatch {
            technology: "JWT",
            kind: "external",
            category: "auth",
        },
    ),
    (
        "stripe",
        InfraMatch {
            technology: "Stripe",
            kind: "external",
            category: "payments",
        },
    ),
];

pub fn detect_infra(text: &str) -> Option<InfraMatch> {
    let lower = text.to_ascii_lowercase();
    RULES
        .iter()
        .find(|(pattern, _)| lower.contains(pattern))
        .map(|(_, m)| *m)
}

pub fn synthesize(bundle: &mut SemanticBundle) {
    let repo_name = bundle
        .symbols
        .first()
        .map_or_else(|| "repo".to_string(), |symbol| symbol.repo_name.clone());

    let mut ext_ids: HashMap<String, String> = HashMap::new();
    let mut seen_edges: HashSet<(String, String, EdgeKind)> = HashSet::new();
    let mut has_database_terminal = false;

    for unresolved in bundle.unresolved_refs.clone() {
        if let Some(infra) = detect_infra(&unresolved.text) {
            if infra.kind == "database" {
                has_database_terminal = true;
            }
            let ext_id = ensure_external_symbol(
                bundle,
                &repo_name,
                &mut ext_ids,
                ExternalSpec {
                    name: infra.technology,
                    kind: infra.kind,
                    description: &format!("infra:{}:{}", infra.kind, infra.category),
                },
            );

            let edge_kind = match unresolved.kind {
                EdgeKind::Imports => EdgeKind::Imports,
                _ => EdgeKind::DependsOn,
            };
            push_external_edge(
                bundle,
                &mut seen_edges,
                unresolved.source.clone(),
                ext_id,
                edge_kind,
            );
            continue;
        }

        if matches!(unresolved.kind, EdgeKind::Extends | EdgeKind::Implements)
            && let Some(name) = external_symbol_name(&unresolved.text)
        {
            let ext_id = ensure_external_symbol(
                bundle,
                &repo_name,
                &mut ext_ids,
                ExternalSpec {
                    name: &name,
                    kind: "external",
                    description: "framework:base",
                },
            );
            push_external_edge(
                bundle,
                &mut seen_edges,
                unresolved.source.clone(),
                ext_id,
                unresolved.kind.clone(),
            );
        }
    }

    if !has_database_terminal {
        let repository_sources: Vec<String> = bundle
            .symbols
            .iter()
            .filter(|symbol| looks_like_repository(symbol))
            .map(|symbol| symbol.symbol_id.clone())
            .collect();
        if !repository_sources.is_empty() {
            let inferred_db = infer_database_technology(bundle);
            let ext_id = ensure_external_symbol(
                bundle,
                &repo_name,
                &mut ext_ids,
                ExternalSpec {
                    name: inferred_db.technology,
                    kind: inferred_db.kind,
                    description: &format!("infra:{}:{}", inferred_db.kind, inferred_db.category),
                },
            );

            for source_id in repository_sources {
                push_external_edge(
                    bundle,
                    &mut seen_edges,
                    source_id,
                    ext_id.clone(),
                    EdgeKind::DependsOn,
                );
            }
        }
    }
}

#[derive(Clone, Copy)]
struct ExternalSpec<'a> {
    name: &'a str,
    kind: &'a str,
    description: &'a str,
}

fn ensure_external_symbol(
    bundle: &mut SemanticBundle,
    repo_name: &str,
    ext_ids: &mut HashMap<String, String>,
    spec: ExternalSpec<'_>,
) -> String {
    ext_ids
        .entry(spec.name.to_string())
        .or_insert_with(|| {
            let id = format!(
                "{repo_name}:__external__/{}:{}",
                spec.kind,
                spec.name.to_ascii_lowercase().replace(' ', "-")
            );
            bundle.symbols.push(SemanticSymbol {
                symbol_id: id.clone(),
                repo_name: repo_name.to_string(),
                file_path: format!("__external__/{name}", name = spec.name),
                name: spec.name.to_string(),
                kind: DeclKind::Class,
                owner: None,
                visibility: Visibility::Unknown,
                external: true,
                description: spec.description.to_string(),
                spans: SymbolSpans::default(),
                control: ControlMetrics::default(),
                annotations: Vec::new(),
            });
            id
        })
        .clone()
}

fn push_external_edge(
    bundle: &mut SemanticBundle,
    seen_edges: &mut HashSet<(String, String, EdgeKind)>,
    source: String,
    target: String,
    kind: EdgeKind,
) {
    if seen_edges.insert((source.clone(), target.clone(), kind.clone())) {
        bundle.edges.push(SemanticEdge {
            source,
            target: EdgeTarget::Resolved(target),
            kind,
            origin: EdgeOrigin::External,
            order_index: usize::MAX,
            cross_boundary: true,
        });
    }
}

fn external_symbol_name(text: &str) -> Option<String> {
    let trimmed = text.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    let candidate = trimmed
        .rsplit("::")
        .next()
        .unwrap_or(trimmed)
        .rsplit('.')
        .next()
        .unwrap_or(trimmed)
        .rsplit('/')
        .next()
        .unwrap_or(trimmed)
        .split('<')
        .next()
        .unwrap_or(trimmed)
        .split('(')
        .next()
        .unwrap_or(trimmed)
        .trim();

    if candidate.is_empty() {
        None
    } else {
        Some(candidate.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_adds_database_for_repository_symbols() {
        let mut bundle = SemanticBundle {
            symbols: vec![SemanticSymbol {
                symbol_id: "repo:src/repositories/user_repository.py:UserRepository".to_string(),
                repo_name: "repo".to_string(),
                file_path: "src/repositories/user_repository.py".to_string(),
                name: "UserRepository".to_string(),
                kind: DeclKind::Class,
                owner: None,
                visibility: Visibility::Unknown,
                external: false,
                description: String::new(),
                spans: SymbolSpans::default(),
                control: Default::default(),
                annotations: Vec::new(),
            }],
            edges: vec![],
            unresolved_refs: vec![],
        };

        synthesize(&mut bundle);

        assert!(
            bundle
                .symbols
                .iter()
                .any(|symbol| symbol.external && symbol.description.starts_with("infra:database:"))
        );
        assert!(
            bundle
                .edges
                .iter()
                .any(|edge| matches!(edge.kind, EdgeKind::DependsOn))
        );
    }

    #[test]
    fn synthesize_adds_external_framework_base_for_inheritance() {
        let mut bundle = SemanticBundle {
            symbols: vec![SemanticSymbol {
                symbol_id: "repo:src/views.py:ArticleView".to_string(),
                repo_name: "repo".to_string(),
                file_path: "src/views.py".to_string(),
                name: "ArticleView".to_string(),
                kind: DeclKind::Class,
                owner: None,
                visibility: Visibility::Unknown,
                external: false,
                description: String::new(),
                spans: SymbolSpans::default(),
                control: Default::default(),
                annotations: Vec::new(),
            }],
            edges: vec![],
            unresolved_refs: vec![super::super::types::UnresolvedRef {
                source: "repo:src/views.py:ArticleView".to_string(),
                text: "viewsets.ModelViewSet".to_string(),
                kind: EdgeKind::Extends,
            }],
        };

        synthesize(&mut bundle);

        assert!(
            bundle
                .symbols
                .iter()
                .any(|symbol| symbol.external && symbol.name == "ModelViewSet")
        );
        assert!(bundle.edges.iter().any(|edge| {
            matches!(edge.kind, EdgeKind::Extends)
                && matches!(&edge.target, EdgeTarget::Resolved(target) if target.contains("modelviewset"))
        }));
    }
}

fn looks_like_repository(symbol: &SemanticSymbol) -> bool {
    let lower_name = symbol.name.to_ascii_lowercase();
    let lower_path = symbol.file_path.to_ascii_lowercase();
    lower_name.contains("repository")
        || lower_name.contains("repo")
        || lower_path.contains("/repository")
        || lower_path.contains("/repositories/")
        || lower_path.contains("/persistent/")
        || lower_path.contains("/persistence/")
}

fn infer_database_technology(bundle: &SemanticBundle) -> InfraMatch {
    let mut haystacks: Vec<String> = bundle
        .unresolved_refs
        .iter()
        .map(|unresolved| unresolved.text.clone())
        .collect();
    haystacks.extend(bundle.symbols.iter().map(|symbol| symbol.file_path.clone()));
    haystacks.extend(bundle.symbols.iter().map(|symbol| symbol.name.clone()));

    let joined = haystacks.join(" ").to_ascii_lowercase();
    if joined.contains("dynamo") || joined.contains("ddb") || joined.contains("boto3") {
        return InfraMatch {
            technology: "DynamoDB",
            kind: "database",
            category: "kv",
        };
    }
    if joined.contains("mongo") || joined.contains("odmantic") {
        return InfraMatch {
            technology: "MongoDB",
            kind: "database",
            category: "doc",
        };
    }
    if joined.contains("postgres") || joined.contains("psycopg") || joined.contains("diesel") {
        return InfraMatch {
            technology: "PostgreSQL",
            kind: "database",
            category: "sql",
        };
    }
    InfraMatch {
        technology: "SQL Database",
        kind: "database",
        category: "sql",
    }
}
