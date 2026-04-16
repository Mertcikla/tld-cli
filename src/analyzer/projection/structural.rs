//! Structural projector — thin wrapper around the existing `workspace_builder`.
//!
//! Produces `elements.yaml` and `connectors.yaml` in the current (pre-redesign)
//! format. Kept as the default and debugging mode.

use crate::analyzer::types::AnalysisResult;
use crate::workspace::workspace_builder::{self, BuildContext, BuildOutput};

/// Project an `AnalysisResult` into workspace elements and connectors using the
/// current structural (file/folder/symbol) representation.
pub fn project(result: &AnalysisResult, ctx: &BuildContext) -> BuildOutput {
    workspace_builder::build(result, ctx)
}
