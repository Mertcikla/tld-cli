use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JsonOutput {
    pub command: String,
    pub status: String,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub summary: HashMap<String, i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<JsonItem>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retries: Option<i32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diff_files: Vec<JsonDiffFile>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_modified: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JsonItem {
    pub r#ref: String,
    pub resource_type: String,
    pub action: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub reason: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct JsonDiffFile {
    pub path: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub hunks: Vec<String>,
}

impl JsonOutput {
    pub fn ok(command: &str) -> Self {
        Self {
            command: command.to_string(),
            status: "ok".to_string(),
            ..Default::default()
        }
    }

    pub fn error(command: &str, err: &str) -> Self {
        Self {
            command: command.to_string(),
            status: "error".to_string(),
            errors: vec![err.to_string()],
            ..Default::default()
        }
    }
}
