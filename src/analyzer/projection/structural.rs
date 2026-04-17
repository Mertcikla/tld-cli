//! Structural projector — preserves the file/folder hierarchy while normalizing
//! symbol elements to architectural kinds and preserving the raw declaration kind.

use super::present_symbol;
use crate::analyzer::{
    semantic::{graph::SemanticGraph, infra, resolver, roles, salience},
    types::AnalysisResult,
};
use crate::workspace::workspace_builder::{self, BuildContext, BuildOutput};
use std::collections::HashMap;

/// Project an `AnalysisResult` into workspace elements and connectors using the
/// structural (file/folder/symbol) representation, but with role-driven symbol
/// kinds and `symbol_kind` preserving the original declaration kind.
pub fn project(result: &AnalysisResult, ctx: &BuildContext) -> BuildOutput {
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

    for element in output.elements.values_mut() {
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

        let presentation = present_symbol(symbol, roles.get(&symbol.symbol_id));
        element.symbol_kind = presentation.symbol_kind;
        element.kind = presentation.kind;
        if !presentation.technology.is_empty() {
            element.technology = presentation.technology;
        }

        if scores.get(&symbol.symbol_id).copied().unwrap_or_default() > 0 {
            if !element.tags.contains(&"high-signal".to_string()) {
                element.tags.push("high-signal".to_string());
            }
        }
    }

    output
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
