pub mod business;
pub mod data_flow;
pub mod structural;

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
