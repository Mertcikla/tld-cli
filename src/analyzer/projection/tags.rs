//! Semantic tag assignment for analyzer-produced elements.
//!
//! The `tld analyze` pipeline already computes rich semantic data (role,
//! domain, endpoint annotations, external flag, salience). This module maps
//! that data to namespaced tag names on `Element.tags`, so the tlDiagram UI's
//! tag-filter feature has something meaningful to filter on.
//!
//! All auto-tags use the `dimension:value` naming scheme (or bare when
//! single-valued). Example: `role:orchestrator`, `domain:user`,
//! `endpoint:http-get`, `external`, `signal:high`.

use super::domain_for_symbol;
use crate::analyzer::semantic::{
    endpoints::detect_endpoint, roles::DerivedRole, types::SemanticSymbol,
};
use crate::workspace::types::Element;
use std::collections::HashMap;

/// Which auto-tag dimensions the analyzer should emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoTagOptions {
    pub role: bool,
    pub domain: bool,
    pub endpoint: bool,
    pub external: bool,
    pub signal: bool,
}

impl AutoTagOptions {
    /// Default dimensions: role, domain, endpoint, external. `signal` is opt-in
    /// because it duplicates what `role` already communicates.
    pub fn default_set() -> Self {
        Self {
            role: true,
            domain: true,
            endpoint: true,
            external: true,
            signal: false,
        }
    }

    pub fn all_enabled() -> Self {
        Self {
            role: true,
            domain: true,
            endpoint: true,
            external: true,
            signal: true,
        }
    }

    pub fn none() -> Self {
        Self {
            role: false,
            domain: false,
            endpoint: false,
            external: false,
            signal: false,
        }
    }

    /// Parse a CSV like "role,domain" or reserved words "all"/"none".
    /// Empty / unrecognised input falls back to the default set.
    pub fn parse(csv: &str) -> Self {
        let trimmed = csv.trim();
        if trimmed.is_empty() {
            return Self::default_set();
        }
        let lower = trimmed.to_ascii_lowercase();
        if matches!(lower.as_str(), "none" | "off" | "false" | "0") {
            return Self::none();
        }
        if lower == "all" {
            return Self::all_enabled();
        }

        let mut opts = Self::none();
        let mut recognized = false;
        for token in lower.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            match token {
                "role" | "roles" => {
                    opts.role = true;
                    recognized = true;
                }
                "domain" | "domains" => {
                    opts.domain = true;
                    recognized = true;
                }
                "endpoint" | "endpoints" => {
                    opts.endpoint = true;
                    recognized = true;
                }
                "external" => {
                    opts.external = true;
                    recognized = true;
                }
                "signal" | "salience" => {
                    opts.signal = true;
                    recognized = true;
                }
                _ => {}
            }
        }
        if recognized {
            opts
        } else {
            Self::default_set()
        }
    }
}

impl Default for AutoTagOptions {
    fn default() -> Self {
        Self::default_set()
    }
}

/// Push namespaced semantic tags onto `element.tags` based on `opts`.
/// Existing tags are preserved; duplicates are not added.
pub fn assign_semantic_tags(
    element: &mut Element,
    symbol: &SemanticSymbol,
    role: Option<&DerivedRole>,
    salience_score: i32,
    opts: &AutoTagOptions,
) {
    if opts.role {
        if let Some(r) = role {
            if !matches!(r, DerivedRole::LowSignal) {
                push_unique(&mut element.tags, format!("role:{}", role_slug(r)));
            }
        }
    }

    if opts.domain {
        let domain = domain_for_symbol(symbol);
        if !domain.is_empty() {
            push_unique(&mut element.tags, format!("domain:{domain}"));
        }
    }

    if opts.endpoint {
        for annotation in &symbol.annotations {
            if let Some(ep) = detect_endpoint(annotation) {
                let method = ep.method.as_str().to_ascii_lowercase();
                push_unique(&mut element.tags, format!("endpoint:http-{method}"));
                break;
            }
        }
    }

    if opts.external && symbol.external {
        push_unique(&mut element.tags, "external".to_string());
    }

    if opts.signal {
        if salience_score > 0 {
            push_unique(&mut element.tags, "signal:high".to_string());
        } else {
            push_unique(&mut element.tags, "signal:medium".to_string());
        }
    }
}

/// Whether a tag string was emitted by the auto-tagger (vs. a user tag).
#[allow(dead_code)]
pub fn is_auto_tag(tag: &str) -> bool {
    tag == "external"
        || tag.starts_with("role:")
        || tag.starts_with("domain:")
        || tag.starts_with("endpoint:")
        || tag.starts_with("signal:")
}

/// Return the default hex color for a known auto-tag, or `None` for user tags.
#[allow(dead_code)]
pub fn known_tag_color(tag: &str) -> Option<String> {
    let fixed = match tag {
        "role:entrypoint" => "#22C55E",
        "role:orchestrator" => "#3B82F6",
        "role:adapter" => "#F97316",
        "role:bootstrap" => "#A855F7",
        "role:interface" => "#06B6D4",
        "role:data-carrier" => "#9CA3AF",
        "role:domain-type" => "#6366F1",
        "role:utility" => "#64748B",
        "external" => "#7C3AED",
        "endpoint:http-get" => "#10B981",
        "endpoint:http-post" => "#F59E0B",
        "endpoint:http-put" => "#EAB308",
        "endpoint:http-delete" => "#EF4444",
        "endpoint:http-patch" => "#F472B6",
        "endpoint:http-head" => "#8B5CF6",
        "endpoint:http-options" => "#8B5CF6",
        "endpoint:http-any" => "#8B5CF6",
        "signal:high" => "#FACC15",
        "signal:medium" => "#E5E7EB",
        other if other.starts_with("domain:") => {
            return Some(domain_color(&other["domain:".len()..]));
        }
        _ => return None,
    };
    Some(fixed.to_string())
}

/// Deterministic color for an arbitrary domain name. Hashes the name into a
/// fixed pastel palette so the same domain gets the same color across runs.
#[allow(dead_code)]
pub fn domain_color(name: &str) -> String {
    const PALETTE: &[&str] = &[
        "#F87171", "#FB923C", "#FBBF24", "#A3E635", "#4ADE80", "#2DD4BF", "#38BDF8", "#818CF8",
        "#A78BFA", "#E879F9", "#F472B6", "#FB7185",
    ];
    let hash = name.bytes().fold(0u32, |acc, b| {
        acc.wrapping_mul(31).wrapping_add(u32::from(b))
    });
    PALETTE[(hash as usize) % PALETTE.len()].to_string()
}

fn role_slug(role: &DerivedRole) -> String {
    role.as_str().replace('_', "-")
}

fn push_unique(tags: &mut Vec<String>, tag: String) {
    if !tags.contains(&tag) {
        tags.push(tag);
    }
}

/// Remove auto-generated tags that are attached to fewer than `min_elements`
/// elements. User-authored tags are always preserved.
pub fn prune_sparse_auto_tags(elements: &mut HashMap<String, Element>, min_elements: usize) {
    if min_elements <= 1 {
        return;
    }

    let mut counts: HashMap<String, usize> = HashMap::new();
    for element in elements.values() {
        for tag in &element.tags {
            if is_auto_tag(tag) {
                *counts.entry(tag.clone()).or_default() += 1;
            }
        }
    }

    for element in elements.values_mut() {
        element.tags.retain(|tag| {
            !is_auto_tag(tag) || counts.get(tag).copied().unwrap_or_default() >= min_elements
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::semantic::types::{
        ControlMetrics, SemanticSymbol, SymbolSpans, Visibility,
    };
    use crate::analyzer::syntax::types::DeclKind;
    use crate::analyzer::types::Annotation;
    use crate::workspace::types::Element;

    fn sym(name: &str, file_path: &str) -> SemanticSymbol {
        SemanticSymbol {
            symbol_id: format!("repo:{file_path}:{name}"),
            repo_name: "repo".to_string(),
            file_path: file_path.to_string(),
            name: name.to_string(),
            kind: DeclKind::Function,
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
    fn parse_default_when_empty() {
        assert_eq!(AutoTagOptions::parse(""), AutoTagOptions::default_set());
        assert_eq!(AutoTagOptions::parse("   "), AutoTagOptions::default_set());
    }

    #[test]
    fn parse_none_disables_all() {
        let opts = AutoTagOptions::parse("none");
        assert_eq!(opts, AutoTagOptions::none());
    }

    #[test]
    fn parse_all_enables_all_including_signal() {
        let opts = AutoTagOptions::parse("all");
        assert!(opts.role && opts.domain && opts.endpoint && opts.external && opts.signal);
    }

    #[test]
    fn parse_csv_enables_only_named_dimensions() {
        let opts = AutoTagOptions::parse("role, domain");
        assert!(opts.role);
        assert!(opts.domain);
        assert!(!opts.endpoint);
        assert!(!opts.external);
        assert!(!opts.signal);
    }

    #[test]
    fn parse_unknown_dimensions_falls_back_to_default() {
        assert_eq!(
            AutoTagOptions::parse("garbage"),
            AutoTagOptions::default_set()
        );
    }

    #[test]
    fn assigns_role_tag_with_prefix() {
        let mut el = Element::default();
        let s = sym("Handle", "pkg/service.go");
        assign_semantic_tags(
            &mut el,
            &s,
            Some(&DerivedRole::Orchestrator),
            0,
            &AutoTagOptions::default_set(),
        );
        assert!(el.tags.contains(&"role:orchestrator".to_string()));
    }

    #[test]
    fn skips_low_signal_role() {
        let mut el = Element::default();
        let s = sym("trivial", "pkg/util.go");
        assign_semantic_tags(
            &mut el,
            &s,
            Some(&DerivedRole::LowSignal),
            -10,
            &AutoTagOptions::default_set(),
        );
        assert!(!el.tags.iter().any(|t| t.starts_with("role:")));
    }

    #[test]
    fn role_data_carrier_uses_dashed_slug() {
        let mut el = Element::default();
        let s = sym("User", "pkg/model.go");
        assign_semantic_tags(
            &mut el,
            &s,
            Some(&DerivedRole::DataCarrier),
            0,
            &AutoTagOptions::default_set(),
        );
        assert!(el.tags.contains(&"role:data-carrier".to_string()));
    }

    #[test]
    fn assigns_endpoint_tag_from_annotation() {
        let mut el = Element::default();
        let mut s = sym("list_articles", "api/articles.py");
        s.annotations.push(Annotation {
            name: "router.get".to_string(),
            args: vec!["\"/articles\"".to_string()],
        });
        assign_semantic_tags(
            &mut el,
            &s,
            Some(&DerivedRole::Entrypoint),
            5,
            &AutoTagOptions::default_set(),
        );
        assert!(el.tags.contains(&"endpoint:http-get".to_string()));
    }

    #[test]
    fn assigns_external_tag_when_symbol_external() {
        let mut el = Element::default();
        let mut s = sym("postgres", "");
        s.external = true;
        assign_semantic_tags(&mut el, &s, None, 0, &AutoTagOptions::default_set());
        assert!(el.tags.contains(&"external".to_string()));
    }

    #[test]
    fn respects_disabled_dimensions() {
        let mut el = Element::default();
        let s = sym("Handle", "pkg/service.go");
        let opts = AutoTagOptions {
            role: false,
            ..AutoTagOptions::default_set()
        };
        assign_semantic_tags(&mut el, &s, Some(&DerivedRole::Orchestrator), 5, &opts);
        assert!(!el.tags.iter().any(|t| t.starts_with("role:")));
    }

    #[test]
    fn preserves_pre_existing_tags_without_duplicating() {
        let mut el = Element::default();
        el.tags.push("user-tag".to_string());
        el.tags.push("role:orchestrator".to_string());
        let s = sym("Handle", "pkg/service.go");
        assign_semantic_tags(
            &mut el,
            &s,
            Some(&DerivedRole::Orchestrator),
            0,
            &AutoTagOptions::default_set(),
        );
        assert_eq!(
            el.tags.iter().filter(|t| *t == "role:orchestrator").count(),
            1
        );
        assert!(el.tags.contains(&"user-tag".to_string()));
    }

    #[test]
    fn signal_opt_in_emits_high_when_score_positive() {
        let mut el = Element::default();
        let s = sym("Handle", "pkg/service.go");
        let opts = AutoTagOptions::all_enabled();
        assign_semantic_tags(&mut el, &s, Some(&DerivedRole::Orchestrator), 3, &opts);
        assert!(el.tags.contains(&"signal:high".to_string()));
    }

    #[test]
    fn is_auto_tag_recognises_namespace() {
        assert!(is_auto_tag("role:entrypoint"));
        assert!(is_auto_tag("domain:user"));
        assert!(is_auto_tag("endpoint:http-get"));
        assert!(is_auto_tag("external"));
        assert!(is_auto_tag("signal:high"));
        assert!(!is_auto_tag("user-tag"));
        assert!(!is_auto_tag("wip"));
    }

    #[test]
    fn known_tag_color_returns_hex_for_fixed_tags() {
        assert_eq!(
            known_tag_color("role:orchestrator").as_deref(),
            Some("#3B82F6")
        );
        assert_eq!(known_tag_color("external").as_deref(), Some("#7C3AED"));
    }

    #[test]
    fn known_tag_color_returns_none_for_user_tag() {
        assert!(known_tag_color("wip").is_none());
        assert!(known_tag_color("owner-team-x").is_none());
    }

    #[test]
    fn domain_color_is_deterministic() {
        assert_eq!(domain_color("user"), domain_color("user"));
        assert_eq!(
            known_tag_color("domain:article"),
            known_tag_color("domain:article")
        );
    }

    #[test]
    fn prunes_auto_tags_below_minimum_support() {
        let mut elements = HashMap::from([
            (
                "a".to_string(),
                Element {
                    tags: vec!["role:orchestrator".to_string(), "wip".to_string()],
                    ..Default::default()
                },
            ),
            (
                "b".to_string(),
                Element {
                    tags: vec!["role:orchestrator".to_string()],
                    ..Default::default()
                },
            ),
        ]);

        prune_sparse_auto_tags(&mut elements, 3);

        assert!(
            !elements["a"]
                .tags
                .contains(&"role:orchestrator".to_string())
        );
        assert!(
            !elements["b"]
                .tags
                .contains(&"role:orchestrator".to_string())
        );
        assert!(elements["a"].tags.contains(&"wip".to_string()));
    }

    #[test]
    fn keeps_auto_tags_at_minimum_support() {
        let mut elements = HashMap::from([
            (
                "a".to_string(),
                Element {
                    tags: vec!["role:orchestrator".to_string()],
                    ..Default::default()
                },
            ),
            (
                "b".to_string(),
                Element {
                    tags: vec!["role:orchestrator".to_string()],
                    ..Default::default()
                },
            ),
            (
                "c".to_string(),
                Element {
                    tags: vec!["role:orchestrator".to_string()],
                    ..Default::default()
                },
            ),
        ]);

        prune_sparse_auto_tags(&mut elements, 3);

        assert!(
            elements
                .values()
                .all(|element| element.tags.contains(&"role:orchestrator".to_string()))
        );
    }
}
