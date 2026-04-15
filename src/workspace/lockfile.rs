use crate::error::TldError;
use crate::workspace::types::{Meta, ResourceMetadata};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LockFile {
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_apply: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub applied_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_version: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub current_elements: HashMap<String, ResourceMetadata>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub current_views: HashMap<String, ResourceMetadata>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub current_connectors: HashMap<String, ResourceMetadata>,
}

impl LockFile {
    pub fn to_meta(&self) -> Meta {
        Meta {
            elements: self.current_elements.clone(),
            views: self.current_views.clone(),
            connectors: self.current_connectors.clone(),
        }
    }

    pub fn from_workspace(ws: &crate::workspace::types::Workspace) -> Self {
        let meta = ws.meta.clone().unwrap_or_default();
        Self {
            version: "v1".to_string(),
            current_elements: meta.elements,
            current_views: meta.views,
            current_connectors: meta.connectors,
            ..Default::default()
        }
    }
}

pub fn load_lock_file(dir: &str) -> Result<Option<LockFile>, TldError> {
    let path = Path::new(dir).join(".tld.lock");
    if !path.exists() {
        return Ok(None);
    }
    let data = fs::read_to_string(&path)?;
    let lock_file: LockFile =
        serde_yaml::from_str(&data).map_err(|e| TldError::Yaml(e.to_string()))?;
    Ok(Some(lock_file))
}

pub fn save_lock_file(dir: &str, lock_file: &LockFile) -> Result<(), TldError> {
    let path = Path::new(dir).join(".tld.lock");
    let data = serde_yaml::to_string(lock_file).map_err(|e| TldError::Yaml(e.to_string()))?;
    fs::write(path, data)?;
    Ok(())
}

pub fn load_metadata(dir: &str) -> Result<Meta, TldError> {
    let mut meta = Meta::default();

    if let Some(lock_file) = load_lock_file(dir)? {
        meta.elements = lock_file.current_elements;
        meta.views = lock_file.current_views;
        meta.connectors = lock_file.current_connectors;
    }

    // Fallback to YAML metadata sections in elements.yaml if lockfile is missing or incomplete
    // (This matches Go's LoadMetadata logic for backward compatibility/hybrid setups)
    if meta.elements.is_empty()
        && let Ok(m) = load_yaml_metadata_section(dir, "elements.yaml", "_meta_elements")
    {
        meta.elements = m;
    }
    if meta.views.is_empty()
        && let Ok(m) = load_yaml_metadata_section(dir, "elements.yaml", "_meta_views")
    {
        meta.views = m;
    }
    if meta.connectors.is_empty()
        && let Ok(m) = load_yaml_metadata_section(dir, "connectors.yaml", "_meta_connectors")
    {
        meta.connectors = m;
    }

    Ok(meta)
}

fn load_yaml_metadata_section(
    dir: &str,
    filename: &str,
    section: &str,
) -> Result<HashMap<String, ResourceMetadata>, TldError> {
    let path = Path::new(dir).join(filename);
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let data = fs::read_to_string(&path)?;
    let val: serde_yaml::Value =
        serde_yaml::from_str(&data).map_err(|e| TldError::Yaml(e.to_string()))?;

    if let Some(mapping) = val.as_mapping()
        && let Some(sec_val) = mapping.get(serde_yaml::Value::String(section.to_string()))
    {
        let m: HashMap<String, ResourceMetadata> =
            serde_yaml::from_value(sec_val.clone()).map_err(|e| TldError::Yaml(e.to_string()))?;
        return Ok(m);
    }
    Ok(HashMap::new())
}
