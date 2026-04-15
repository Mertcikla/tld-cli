use serde::{Deserialize, Serialize};

/// Symbol is a named declaration found in a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file_path: String,
    pub line: i32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub end_line: i32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub parent: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub technology: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

fn is_zero(v: &i32) -> bool {
    *v == 0
}

/// Ref is a call-site reference to a named symbol found within a source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ref {
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub target_path: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file_path: String,
    pub line: i32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub column: i32,
}

/// Result holds the output of extracting symbols from one or more files.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub symbols: Vec<Symbol>,
    pub refs: Vec<Ref>,
    /// Absolute paths of every file visited during directory walks (including unsupported files).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files_scanned: Vec<String>,
}

impl AnalysisResult {
    pub fn merge(&mut self, other: AnalysisResult) {
        self.symbols.extend(other.symbols);
        self.refs.extend(other.refs);
        self.files_scanned.extend(other.files_scanned);
    }
}
