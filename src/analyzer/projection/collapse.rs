//! Auto-collapse for oversized projections.
//!
//! This module works on the already-projected workspace graph so it can be
//! reused by multiple views. It keeps the highest-signal anchors, preserves
//! enough ancestry to keep the layout coherent, and summarizes hidden detail
//! behind one synthetic node per visible container.

use super::collapse_connectors;
use crate::workspace::types::{Connector, Element, ViewPlacement};
use crate::workspace::workspace_builder::BuildOutput;
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};

const SUMMARY_TECHNOLOGY: &str = "Auto-collapsed";

#[derive(Debug, Clone, Copy)]
pub struct CollapseConfig {
    pub target_elements: usize,
    pub hard_limit: usize,
    pub max_root_children: usize,
    pub selection_ratio_percent: usize,
}

impl CollapseConfig {
    pub fn new(target_elements: usize, hard_limit: usize) -> Self {
        Self {
            target_elements,
            hard_limit,
            max_root_children: 40,
            selection_ratio_percent: 80,
        }
    }

    pub fn disabled(&self) -> bool {
        self.target_elements == 0
    }
}

#[derive(Debug, Clone, Default)]
struct NodeInfo {
    parent: Option<String>,
    children: Vec<String>,
    depth: usize,
    own_signal: i32,
    subtree_signal: i32,
    degree: usize,
    hidden_descendants: usize,
}

pub fn auto_collapse(
    output: BuildOutput,
    signal_by_slug: &HashMap<String, i32>,
    config: CollapseConfig,
) -> BuildOutput {
    if config.disabled() || output.elements.len() <= config.target_elements {
        return output;
    }

    let BuildOutput {
        elements,
        connectors,
    } = output;

    let mut infos = build_node_info(&elements, &connectors, signal_by_slug);
    let roots = root_slugs(&elements);
    if roots.is_empty() {
        return BuildOutput {
            elements,
            connectors,
        };
    }

    let top_level = top_level_children(&roots, &infos);
    let target_elements = config
        .target_elements
        .min(config.hard_limit)
        .max(roots.len());
    let selection_budget = selection_budget(target_elements, config.selection_ratio_percent);

    let priorities = compute_priorities(&elements, &infos, &top_level);
    let mut keep = HashSet::new();
    for root in &roots {
        keep.insert(root.clone());
    }

    let mut ordered_top_level = top_level
        .iter()
        .map(|slug| {
            (
                slug.clone(),
                priorities.get(slug).copied().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    ordered_top_level.sort_by_key(|(slug, priority)| (Reverse(*priority), slug.clone()));
    for (slug, _) in ordered_top_level
        .into_iter()
        .take(config.max_root_children.max(roots.len()))
    {
        insert_with_ancestors(&slug, &infos, &mut keep);
    }

    let mut candidates = priorities
        .iter()
        .filter(|(slug, _)| !keep.contains(*slug))
        .map(|(slug, priority)| (slug.clone(), *priority))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(slug, priority)| (Reverse(*priority), slug.clone()));

    for (slug, priority) in candidates {
        if priority <= 0 {
            break;
        }
        let added = ancestors_until_kept(&slug, &infos, &keep);
        if keep.len() + added.len() > selection_budget {
            continue;
        }
        for ancestor in added {
            keep.insert(ancestor);
        }
    }

    refresh_hidden_descendants(&mut infos, &keep, &roots);

    let mut summary_candidates = keep
        .iter()
        .filter_map(|slug| {
            let info = infos.get(slug)?;
            (info.hidden_descendants > 0).then_some((
                slug.clone(),
                info.hidden_descendants as i64 * 100 + i64::from(info.subtree_signal.max(0)),
            ))
        })
        .collect::<Vec<_>>();
    summary_candidates.sort_by_key(|(slug, priority)| (Reverse(*priority), slug.clone()));

    let mut visible_elements = HashMap::new();
    for slug in &keep {
        if let Some(element) = elements.get(slug) {
            visible_elements.insert(slug.clone(), element.clone());
        }
    }

    let mut summary_by_parent = HashMap::new();
    let mut remaining_slots = target_elements.saturating_sub(visible_elements.len());
    for (parent_slug, _) in summary_candidates {
        if remaining_slots == 0 {
            break;
        }
        let Some(parent) = visible_elements.get(&parent_slug) else {
            continue;
        };
        let summary_slug = unique_summary_slug(&parent_slug, &visible_elements);
        let hidden_descendants = infos
            .get(&parent_slug)
            .map(|info| info.hidden_descendants)
            .unwrap_or_default();
        let summary = Element {
            name: format!("{} detail", parent.name),
            kind: "component".to_string(),
            technology: SUMMARY_TECHNOLOGY.to_string(),
            owner: parent.owner.clone(),
            branch: parent.branch.clone(),
            description: format!("collapsed_elements={hidden_descendants}"),
            placements: vec![ViewPlacement {
                parent_ref: parent_slug.clone(),
                ..Default::default()
            }],
            ..Default::default()
        };
        visible_elements.insert(summary_slug.clone(), summary);
        summary_by_parent.insert(parent_slug, summary_slug);
        remaining_slots -= 1;
    }

    let representative_cache =
        build_representative_cache(&elements, &infos, &keep, &summary_by_parent);

    let mut visible_connectors = Vec::new();
    for connector in connectors {
        let Some(source) = representative_cache.get(&connector.source).cloned() else {
            continue;
        };
        let Some(target) = representative_cache.get(&connector.target).cloned() else {
            continue;
        };
        if source == target {
            continue;
        }

        let mapped_view = visible_view(&connector.view, &elements, &infos, &keep, &roots);
        let mut connector = connector;
        connector.view = mapped_view;
        connector.source = source;
        connector.target = target;
        visible_connectors.push(connector);
    }

    let mut collapsed = BuildOutput {
        elements: visible_elements,
        connectors: collapse_connectors(visible_connectors),
    };

    if collapsed.elements.len() > config.hard_limit {
        trim_to_hard_limit(
            &mut collapsed,
            &infos,
            &roots,
            &priorities,
            config.hard_limit,
        );
    }

    collapsed
}

fn build_node_info(
    elements: &HashMap<String, Element>,
    connectors: &[Connector],
    signal_by_slug: &HashMap<String, i32>,
) -> HashMap<String, NodeInfo> {
    let mut infos = HashMap::new();
    for (slug, element) in elements {
        let parent = element
            .placements
            .first()
            .map(|placement| placement.parent_ref.clone())
            .filter(|parent| !parent.is_empty());
        infos.insert(
            slug.clone(),
            NodeInfo {
                parent,
                own_signal: signal_by_slug.get(slug).copied().unwrap_or_default(),
                ..Default::default()
            },
        );
    }

    let keys = infos.keys().cloned().collect::<Vec<_>>();
    for slug in keys {
        if let Some(parent) = infos.get(&slug).and_then(|info| info.parent.clone())
            && let Some(parent_info) = infos.get_mut(&parent)
        {
            parent_info.children.push(slug);
        }
    }

    for connector in connectors {
        if let Some(info) = infos.get_mut(&connector.source) {
            info.degree += 1;
        }
        if let Some(info) = infos.get_mut(&connector.target) {
            info.degree += 1;
        }
    }

    let roots = root_slugs(elements);
    for root in &roots {
        populate_depth_and_signal(root, 0, &mut infos);
    }

    infos
}

fn root_slugs(elements: &HashMap<String, Element>) -> Vec<String> {
    let mut roots = elements
        .iter()
        .filter_map(|(slug, element)| {
            element
                .placements
                .first()
                .filter(|placement| placement.parent_ref == "root")
                .map(|_| slug.clone())
        })
        .collect::<Vec<_>>();
    roots.sort();
    roots
}

fn top_level_children(roots: &[String], infos: &HashMap<String, NodeInfo>) -> Vec<String> {
    let root_set = roots.iter().collect::<HashSet<_>>();
    let mut children = infos
        .iter()
        .filter_map(|(slug, info)| {
            info.parent
                .as_ref()
                .filter(|parent| root_set.contains(parent))
                .map(|_| slug.clone())
        })
        .collect::<Vec<_>>();
    children.sort();
    children
}

fn populate_depth_and_signal(
    slug: &str,
    depth: usize,
    infos: &mut HashMap<String, NodeInfo>,
) -> i32 {
    let children = infos
        .get(slug)
        .map(|info| info.children.clone())
        .unwrap_or_default();

    let mut subtree_signal = infos
        .get(slug)
        .map(|info| info.own_signal.max(0))
        .unwrap_or_default();

    for child in &children {
        subtree_signal += populate_depth_and_signal(child, depth + 1, infos);
    }

    if let Some(info) = infos.get_mut(slug) {
        info.depth = depth;
        info.subtree_signal = subtree_signal;
    }

    subtree_signal
}

fn compute_priorities(
    elements: &HashMap<String, Element>,
    infos: &HashMap<String, NodeInfo>,
    top_level: &[String],
) -> HashMap<String, i64> {
    let top_level_set = top_level.iter().collect::<HashSet<_>>();
    elements
        .iter()
        .map(|(slug, element)| {
            let info = infos.get(slug).cloned().unwrap_or_default();
            let own_signal = i64::from(info.own_signal);
            let subtree_signal = i64::from(info.subtree_signal);
            let degree = i64::try_from(info.degree).unwrap_or(i64::MAX);
            let depth = i64::try_from(info.depth).unwrap_or(i64::MAX);

            let kind_bonus = match element.kind.as_str() {
                "repository" => 100_000,
                "endpoint" | "entrypoint" => 30_000,
                "service" => 22_000,
                "adapter" | "repository-client" | "cache-client" => 18_000,
                "domain" => 14_000,
                "folder" => 8_000,
                "file" => 5_000,
                "model" => 4_000,
                "interface" => 3_500,
                "entrypoint-bootstrap" => 1_000,
                "utility" => -1_500,
                _ => 2_000,
            };

            let top_level_bonus = if top_level_set.contains(slug) {
                40_000
            } else {
                0
            };

            let priority = kind_bonus
                + top_level_bonus
                + own_signal * 1_000
                + subtree_signal * 45
                + degree * 20
                - depth * 150;

            (slug.clone(), priority)
        })
        .collect()
}

fn selection_budget(target: usize, ratio_percent: usize) -> usize {
    ((target * ratio_percent) / 100).max(1)
}

fn insert_with_ancestors(
    slug: &str,
    infos: &HashMap<String, NodeInfo>,
    keep: &mut HashSet<String>,
) {
    for ancestor in ancestors_until_kept(slug, infos, keep) {
        keep.insert(ancestor);
    }
}

fn ancestors_until_kept(
    slug: &str,
    infos: &HashMap<String, NodeInfo>,
    keep: &HashSet<String>,
) -> Vec<String> {
    let mut added = Vec::new();
    let mut current = Some(slug.to_string());
    while let Some(node) = current {
        if keep.contains(&node) {
            break;
        }
        added.push(node.clone());
        current = infos.get(&node).and_then(|info| info.parent.clone());
    }
    added.reverse();
    added
}

fn refresh_hidden_descendants(
    infos: &mut HashMap<String, NodeInfo>,
    keep: &HashSet<String>,
    roots: &[String],
) {
    for root in roots {
        let _ = count_hidden(root, infos, keep);
    }
}

fn count_hidden(
    slug: &str,
    infos: &mut HashMap<String, NodeInfo>,
    keep: &HashSet<String>,
) -> usize {
    let children = infos
        .get(slug)
        .map(|info| info.children.clone())
        .unwrap_or_default();
    let mut hidden = 0;
    for child in &children {
        hidden += count_hidden(child, infos, keep);
        if !keep.contains(child) {
            hidden += 1;
        }
    }
    if let Some(info) = infos.get_mut(slug) {
        info.hidden_descendants = hidden;
    }
    hidden
}

fn unique_summary_slug(parent_slug: &str, elements: &HashMap<String, Element>) -> String {
    let base = format!("{parent_slug}-detail");
    if !elements.contains_key(&base) {
        return base;
    }
    let mut n = 2;
    loop {
        let candidate = format!("{base}-{n}");
        if !elements.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn build_representative_cache(
    elements: &HashMap<String, Element>,
    infos: &HashMap<String, NodeInfo>,
    keep: &HashSet<String>,
    summary_by_parent: &HashMap<String, String>,
) -> HashMap<String, String> {
    let mut cache = HashMap::new();
    for slug in elements.keys() {
        let rep = representative_for(slug, infos, keep, summary_by_parent);
        cache.insert(slug.clone(), rep);
    }
    cache
}

fn representative_for(
    slug: &str,
    infos: &HashMap<String, NodeInfo>,
    keep: &HashSet<String>,
    summary_by_parent: &HashMap<String, String>,
) -> String {
    if keep.contains(slug) {
        return slug.to_string();
    }

    let mut current = infos.get(slug).and_then(|info| info.parent.clone());
    while let Some(parent) = current {
        if keep.contains(&parent) {
            return summary_by_parent.get(&parent).cloned().unwrap_or(parent);
        }
        current = infos.get(&parent).and_then(|info| info.parent.clone());
    }

    slug.to_string()
}

fn visible_view(
    view_slug: &str,
    elements: &HashMap<String, Element>,
    infos: &HashMap<String, NodeInfo>,
    keep: &HashSet<String>,
    roots: &[String],
) -> String {
    if keep.contains(view_slug) {
        return view_slug.to_string();
    }

    let mut current = Some(view_slug.to_string());
    while let Some(slug) = current {
        if keep.contains(&slug) {
            return slug;
        }
        current = infos.get(&slug).and_then(|info| info.parent.clone());
    }

    roots
        .first()
        .cloned()
        .unwrap_or_else(|| elements.keys().next().cloned().unwrap_or_default())
}

fn trim_to_hard_limit(
    output: &mut BuildOutput,
    infos: &HashMap<String, NodeInfo>,
    roots: &[String],
    priorities: &HashMap<String, i64>,
    hard_limit: usize,
) {
    if output.elements.len() <= hard_limit {
        return;
    }

    let root_set = roots.iter().collect::<HashSet<_>>();
    let mut removable = output
        .elements
        .keys()
        .filter(|slug| !root_set.contains(*slug))
        .cloned()
        .collect::<Vec<_>>();
    removable.sort_by_key(|slug| {
        (
            priorities.get(slug).copied().unwrap_or_default(),
            Reverse(infos.get(slug).map(|info| info.depth).unwrap_or_default()),
            slug.clone(),
        )
    });

    let mut removed = HashSet::new();
    for slug in removable {
        if output.elements.len().saturating_sub(removed.len()) <= hard_limit {
            break;
        }
        removed.insert(slug);
    }

    output.elements.retain(|slug, _| !removed.contains(slug));
    output.connectors.retain(|connector| {
        output.elements.contains_key(&connector.source)
            && output.elements.contains_key(&connector.target)
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_element(name: &str, kind: &str, parent: &str) -> Element {
        Element {
            name: name.to_string(),
            kind: kind.to_string(),
            technology: kind.to_string(),
            placements: vec![ViewPlacement {
                parent_ref: parent.to_string(),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    #[test]
    fn auto_collapse_reduces_large_workspace_to_budget() {
        let elements = HashMap::from([
            (
                "repo".to_string(),
                Element {
                    name: "repo".to_string(),
                    kind: "repository".to_string(),
                    technology: "Git Repository".to_string(),
                    has_view: true,
                    view_label: "repo".to_string(),
                    placements: vec![ViewPlacement {
                        parent_ref: "root".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ),
            (
                "clients".to_string(),
                make_element("clients", "folder", "repo"),
            ),
            (
                "streams".to_string(),
                make_element("streams", "folder", "repo"),
            ),
            (
                "api".to_string(),
                make_element("Api.java", "file", "clients"),
            ),
            (
                "impl".to_string(),
                make_element("Impl.java", "file", "streams"),
            ),
            (
                "entry".to_string(),
                make_element("handleRequest", "entrypoint", "api"),
            ),
            (
                "service".to_string(),
                make_element("process", "service", "impl"),
            ),
            ("util".to_string(), make_element("trim", "utility", "impl")),
            (
                "helper".to_string(),
                make_element("helper", "utility", "impl"),
            ),
        ]);

        let connectors = vec![
            Connector {
                view: "repo".to_string(),
                source: "entry".to_string(),
                target: "service".to_string(),
                label: "calls".to_string(),
                relationship: "uses".to_string(),
                direction: "forward".to_string(),
                ..Default::default()
            },
            Connector {
                view: "repo".to_string(),
                source: "service".to_string(),
                target: "util".to_string(),
                label: "calls".to_string(),
                relationship: "uses".to_string(),
                direction: "forward".to_string(),
                ..Default::default()
            },
            Connector {
                view: "repo".to_string(),
                source: "service".to_string(),
                target: "helper".to_string(),
                label: "calls".to_string(),
                relationship: "uses".to_string(),
                direction: "forward".to_string(),
                ..Default::default()
            },
        ];

        let output = BuildOutput {
            elements,
            connectors,
        };
        let signal = HashMap::from([
            ("entry".to_string(), 12),
            ("service".to_string(), 10),
            ("util".to_string(), -3),
            ("helper".to_string(), -4),
        ]);

        let collapsed = auto_collapse(output, &signal, CollapseConfig::new(8, 10_000));

        assert!(collapsed.elements.len() <= 8);
        assert!(collapsed.elements.contains_key("repo"));
        assert!(collapsed.elements.contains_key("clients"));
        assert!(collapsed.elements.contains_key("streams"));
        assert!(
            collapsed.elements.contains_key("entry") || collapsed.elements.contains_key("service")
        );
        assert!(
            collapsed
                .elements
                .values()
                .any(|element| element.technology == SUMMARY_TECHNOLOGY),
            "collapsed output should contain a summary node"
        );
        assert!(
            !collapsed.connectors.is_empty(),
            "collapsed output should preserve at least one aggregated connector"
        );
    }

    #[test]
    fn auto_collapse_is_noop_under_budget() {
        let output = BuildOutput {
            elements: HashMap::from([(
                "repo".to_string(),
                Element {
                    name: "repo".to_string(),
                    kind: "repository".to_string(),
                    placements: vec![ViewPlacement {
                        parent_ref: "root".to_string(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            )]),
            connectors: Vec::new(),
        };

        let collapsed = auto_collapse(output, &HashMap::new(), CollapseConfig::new(10, 10_000));
        assert_eq!(collapsed.elements.len(), 1);
        assert!(collapsed.connectors.is_empty());
    }
}
