use crate::error::TldError;
use crate::workspace::types::{Meta, ResourceMetadata, Workspace};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use yaml_rust2::{Yaml, YamlEmitter, YamlLoader};

/// Merges server state into local YAML files while preserving comments and formatting.
pub fn merge_workspace(
    wdir: &str,
    server_ws: &Workspace,
    last_sync: &Meta,
    current_meta: &Meta,
) -> Result<(), TldError> {
    let elements_path = Path::new(wdir).join("elements.yaml");
    let connectors_path = Path::new(wdir).join("connectors.yaml");

    let mut total_conflicts = 0;

    // Merge Elements
    if elements_path.exists() {
        total_conflicts += merge_file(
            &elements_path,
            &server_ws.elements,
            &server_ws
                .meta
                .as_ref()
                .map(|m| m.elements.clone())
                .unwrap_or_default(),
            &last_sync.elements,
            &current_meta.elements,
        )?;
    } else {
        crate::workspace::save(server_ws)?;
    }

    // Merge Connectors
    if connectors_path.exists() {
        total_conflicts += merge_file(
            &connectors_path,
            &server_ws.connectors,
            &server_ws
                .meta
                .as_ref()
                .map(|m| m.connectors.clone())
                .unwrap_or_default(),
            &last_sync.connectors,
            &current_meta.connectors,
        )?;
    }

    if total_conflicts > 0 {
        return Err(TldError::Generic(format!(
            "Merge complete with {} conflict(s). Please resolve them in elements.yaml/connectors.yaml and run 'tld apply' again.",
            total_conflicts
        )));
    }

    Ok(())
}

fn merge_file<T: serde::Serialize>(
    path: &Path,
    server_items: &HashMap<String, T>,
    server_meta: &HashMap<String, ResourceMetadata>,
    last_sync_meta: &HashMap<String, ResourceMetadata>,
    current_local_meta: &HashMap<String, ResourceMetadata>,
) -> Result<usize, TldError> {
    let content = fs::read_to_string(path).map_err(|e| TldError::Generic(e.to_string()))?;
    let docs = YamlLoader::load_from_str(&content).map_err(|e| TldError::Generic(e.to_string()))?;
    if docs.is_empty() {
        return Ok(0);
    }

    // We treat the first document as the mapping.
    let mut doc = docs[0].clone();
    let mut conflicts = 0;

    if let Yaml::Hash(ref mut h) = doc {
        let mut keys_to_remove = Vec::new();

        for (key, val) in h.iter_mut() {
            let key_str = match key.as_str() {
                Some(s) => s.to_string(),
                None => continue,
            };

            if key_str.starts_with("_meta") {
                continue;
            }

            let s_meta = server_meta.get(&key_str);
            let l_meta = last_sync_meta.get(&key_str);
            let c_meta = current_local_meta.get(&key_str);

            if let Some(server_item) = server_items.get(&key_str) {
                let local_changed = match (l_meta, c_meta) {
                    (Some(l), Some(c)) => c.updated_at > l.updated_at,
                    _ => false,
                };
                let server_changed = match (l_meta, s_meta) {
                    (Some(l), Some(s)) => s.updated_at > l.updated_at,
                    _ => false,
                };

                if local_changed && server_changed {
                    // CONFLICT
                    conflicts += 1;
                    let server_yaml_str = serde_yaml::to_string(server_item).unwrap_or_default();
                    let local_yaml_str = format!("{:?}", val); // Simplified local capture
                    let conflict_val = format!(
                        "<<< LOCAL\n{}\n===\n{}>>> SERVER",
                        local_yaml_str.trim(),
                        server_yaml_str.trim()
                    );
                    *val = Yaml::String(conflict_val);
                } else if server_changed {
                    // Update from server
                    let server_yaml_str = serde_yaml::to_string(server_item).unwrap_or_default();
                    let server_docs =
                        YamlLoader::load_from_str(&server_yaml_str).unwrap_or_default();
                    if !server_docs.is_empty() {
                        *val = server_docs[0].clone();
                    }
                }
            } else if l_meta.is_some() {
                // Deleted on server
                keys_to_remove.push(key.clone());
            }
        }

        for k in keys_to_remove {
            h.remove(&k);
        }

        // Add new items from server
        for (key, item) in server_items {
            let key_yaml = Yaml::String(key.clone());
            if h.get(&key_yaml).is_none() {
                let server_yaml_str = serde_yaml::to_string(item).unwrap_or_default();
                let server_docs = YamlLoader::load_from_str(&server_yaml_str).unwrap_or_default();
                if !server_docs.is_empty() {
                    h.insert(key_yaml, server_docs[0].clone());
                }
            }
        }
    }

    let mut out_str = String::new();
    let mut emitter = YamlEmitter::new(&mut out_str);
    emitter
        .dump(&doc)
        .map_err(|e| TldError::Generic(e.to_string()))?;
    fs::write(path, out_str).map_err(|e| TldError::Generic(e.to_string()))?;
    Ok(conflicts)
}
