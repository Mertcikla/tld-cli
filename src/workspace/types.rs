use chrono::{DateTime, Utc};
/// Workspace types – mirrors the Go `workspace` package structs.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Global user configuration (~/.config/tldiagram/tld.yaml).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub org_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationConfig>,
}

/// Per-workspace config (.tld/.tld.yaml).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub project_name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub repositories: HashMap<String, Repository>,
}

/// A repository entry in workspace config.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Repository {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(rename = "localDir", default, skip_serializing_if = "String::is_empty")]
    pub local_dir: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<RepositoryConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,
}

/// Per-repository behaviour.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub mode: String,
}

/// Workspace-level validation settings.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    pub level: i32,
    #[serde(default)]
    pub allow_low_insight: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub include_rules: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude_rules: Vec<String>,
}

/// An element placement inside a parent view.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ViewPlacement {
    #[serde(rename = "parent")]
    pub parent_ref: String,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub position_x: f64,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub position_y: f64,
}

#[expect(clippy::trivially_copy_pass_by_ref)]
fn is_zero_f64(v: &f64) -> bool {
    *v == 0.0
}

/// The primary workspace element resource.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Element {
    pub name: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub technology: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub owner: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub logo_url: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repo: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub branch: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub language: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub file_path: String,
    /// Named code symbol within `file_path` (e.g. "MyFunc").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub symbol: String,
    /// Original declaration kind preserved when `kind` is normalized by a projection.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub symbol_kind: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_view: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub view_label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placements: Vec<ViewPlacement>,
}

/// One entry in `connectors.yaml`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Connector {
    pub view: String,
    pub source: String,
    pub target: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub relationship: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub direction: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub style: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub url: String,
}

impl Connector {
    /// Returns the canonical ref for the connector (view:source:target:label).
    pub fn resource_ref(&self) -> String {
        format!(
            "{}:{}:{}:{}",
            self.view, self.source, self.target, self.label
        )
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ResourceMetadata {
    pub id: i32,
    pub updated_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub conflict: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Meta {
    #[serde(default)]
    pub elements: HashMap<String, ResourceMetadata>,
    #[serde(default)]
    pub views: HashMap<String, ResourceMetadata>,
    #[serde(default)]
    pub connectors: HashMap<String, ResourceMetadata>,
}

/// Fully loaded workspace state.
#[derive(Debug, Default, Clone)]
pub struct Workspace {
    pub dir: String,
    pub config: Config,
    pub ws_config: Option<WorkspaceConfig>,
    pub elements: HashMap<String, Element>,
    pub connectors: HashMap<String, Connector>,
    pub meta: Option<Meta>,
}
