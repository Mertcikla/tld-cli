//! Structural projector — preserves the file/folder hierarchy while normalizing
//! symbol elements to architectural kinds and preserving the raw declaration kind.

use super::present_symbol;
use super::tags::{self, AutoTagOptions};
use crate::analyzer::{
    semantic::{graph::SemanticGraph, infra, resolver, roles, salience},
    types::AnalysisResult,
};
use crate::workspace::workspace_builder::{self, BuildContext, BuildOutput};
use std::collections::{HashMap, HashSet};

/// Project an `AnalysisResult` into workspace elements and connectors using the
/// structural (file/folder/symbol) representation, but with role-driven symbol
/// kinds and `symbol_kind` preserving the original declaration kind.
pub fn project(
    result: &AnalysisResult,
    ctx: &BuildContext,
    auto_tags: &AutoTagOptions,
) -> BuildOutput {
    let mut output = workspace_builder::build(result, ctx);

    let mut bundle = resolver::resolve(result, &ctx.repo_name, &ctx.scan_root);
    infra::synthesize(&mut bundle);

    let graph = SemanticGraph::build(&bundle);
    let roles = roles::infer_roles(&graph);
    let scores = salience::score_all(&graph, &roles);

    let symbols_by_display: HashMap<(String, String, String), _> = bundle
        .symbols
        .iter()
        .map(|symbol| {
            (
                (
                    symbol.file_path.clone(),
                    structural_display_name(symbol, &bundle),
                    symbol.name.clone(),
                ),
                symbol,
            )
        })
        .collect();

    let mut symbols_by_file_and_name: HashMap<(String, String), Vec<_>> = HashMap::new();
    for symbol in &bundle.symbols {
        symbols_by_file_and_name
            .entry((symbol.file_path.clone(), symbol.name.clone()))
            .or_default()
            .push(symbol);
    }

    let mut high_signal_slugs: HashSet<String> = HashSet::new();

    for (slug, element) in output.elements.iter_mut() {
        if element.symbol.is_empty() || element.file_path.is_empty() {
            continue;
        }

        let matched = symbols_by_display
            .get(&(
                element.file_path.clone(),
                element.name.clone(),
                element.symbol.clone(),
            ))
            .copied()
            .or_else(|| {
                let candidates = symbols_by_file_and_name
                    .get(&(element.file_path.clone(), element.symbol.clone()))?;
                (candidates.len() == 1).then_some(candidates[0])
            });

        let Some(symbol) = matched else {
            if element.symbol_kind.is_empty() {
                element.symbol_kind = element.kind.clone();
            }
            continue;
        };

        let role = roles.get(&symbol.symbol_id);
        let score = scores.get(&symbol.symbol_id).copied().unwrap_or_default();

        let presentation = present_symbol(symbol, role);
        element.symbol_kind = presentation.symbol_kind;
        element.kind = presentation.kind;
        if !presentation.technology.is_empty() {
            element.technology = presentation.technology;
        }

        if score > 0 {
            high_signal_slugs.insert(slug.clone());
        }

        tags::assign_semantic_tags(element, symbol, role, score, auto_tags);
    }

    prune_utility_sinks(&mut output, &high_signal_slugs);
    tags::prune_sparse_auto_tags(&mut output.elements, 3);

    output
}

/// Drop `calls` connectors pointing at utility-sink elements — targets with
/// many callers but zero outgoing calls and no positive salience score. These
/// are typically stdlib-shaped methods (`new`, `ok`, `as_str`, `unwrap`, …)
/// that the bare-name matcher over-attaches, and they add fan-in noise
/// without architectural signal.
fn prune_utility_sinks(output: &mut BuildOutput, high_signal_slugs: &HashSet<String>) {
    const FAN_IN_THRESHOLD: usize = 3;
    let mut fan_in: HashMap<String, usize> = HashMap::new();
    let mut fan_out: HashMap<String, usize> = HashMap::new();
    for connector in &output.connectors {
        if connector.label != "calls" {
            continue;
        }
        *fan_in.entry(connector.target.clone()).or_insert(0) += 1;
        *fan_out.entry(connector.source.clone()).or_insert(0) += 1;
    }

    output.connectors.retain(|connector| {
        if connector.label != "calls" {
            return true;
        }
        if high_signal_slugs.contains(&connector.target) {
            return true;
        }
        let incoming = fan_in.get(&connector.target).copied().unwrap_or(0);
        let outgoing = fan_out.get(&connector.target).copied().unwrap_or(0);
        !(outgoing == 0 && incoming >= FAN_IN_THRESHOLD)
    });
}

fn structural_display_name(
    symbol: &crate::analyzer::semantic::types::SemanticSymbol,
    bundle: &crate::analyzer::semantic::types::SemanticBundle,
) -> String {
    if let Some(owner_id) = &symbol.owner
        && !symbol.name.starts_with('~')
        && let Some(owner) = bundle
            .symbols
            .iter()
            .find(|candidate| &candidate.symbol_id == owner_id)
    {
        return format!("{}.{}", owner.name, symbol.name);
    }
    symbol.name.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::types::{Connector, Element};

    #[test]
    fn prune_utility_sinks_keeps_high_signal_targets() {
        let mut output = BuildOutput {
            elements: HashMap::from([
                ("entry".to_string(), Element::default()),
                ("a".to_string(), Element::default()),
                ("b".to_string(), Element::default()),
                ("sink".to_string(), Element::default()),
            ]),
            connectors: vec![
                Connector {
                    source: "entry".to_string(),
                    target: "sink".to_string(),
                    label: "calls".to_string(),
                    ..Default::default()
                },
                Connector {
                    source: "a".to_string(),
                    target: "sink".to_string(),
                    label: "calls".to_string(),
                    ..Default::default()
                },
                Connector {
                    source: "b".to_string(),
                    target: "sink".to_string(),
                    label: "calls".to_string(),
                    ..Default::default()
                },
            ],
        };

        prune_utility_sinks(&mut output, &HashSet::from(["sink".to_string()]));

        assert_eq!(output.connectors.len(), 3);
    }

    #[test]
    fn prune_utility_sinks_removes_unscored_fan_in_noise() {
        let mut output = BuildOutput {
            elements: HashMap::from([
                ("entry".to_string(), Element::default()),
                ("a".to_string(), Element::default()),
                ("b".to_string(), Element::default()),
                ("sink".to_string(), Element::default()),
            ]),
            connectors: vec![
                Connector {
                    source: "entry".to_string(),
                    target: "sink".to_string(),
                    label: "calls".to_string(),
                    ..Default::default()
                },
                Connector {
                    source: "a".to_string(),
                    target: "sink".to_string(),
                    label: "calls".to_string(),
                    ..Default::default()
                },
                Connector {
                    source: "b".to_string(),
                    target: "sink".to_string(),
                    label: "calls".to_string(),
                    ..Default::default()
                },
            ],
        };

        prune_utility_sinks(&mut output, &HashSet::new());

        assert!(output.connectors.is_empty());
    }
}
