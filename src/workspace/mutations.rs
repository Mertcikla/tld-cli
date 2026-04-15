use crate::error::TldError;
use crate::workspace::types::*;
use std::collections::HashMap;

impl Workspace {
    /// Upserts an element in the workspace.
    pub fn upsert_element(
        &mut self,
        ref_name: String,
        mut element: Element,
    ) -> Result<(), TldError> {
        // Enforce some defaults if missing
        if element.kind.is_empty() {
            element.kind = "service".to_string();
        }

        self.elements.insert(ref_name, element);
        Ok(())
    }

    /// Removes an element and Cascades deletion to connectors and placements.
    pub fn remove_element(&mut self, ref_name: &str) -> Result<(), TldError> {
        if self.elements.remove(ref_name).is_some() {
            // 1. Remove connectors where this element is source or target
            self.connectors
                .retain(|_, c| c.source != ref_name && c.target != ref_name);

            // 2. Remove placements in other elements where this element is parent
            for el in self.elements.values_mut() {
                el.placements.retain(|p| p.parent_ref != ref_name);
            }

            // 3. Remove from metadata
            if let Some(meta) = &mut self.meta {
                meta.elements.remove(ref_name);
                meta.views.remove(ref_name);
            }
        }
        Ok(())
    }

    /// Renames an element ref and cascades to all references.
    pub fn rename_element(&mut self, old_ref: &str, new_ref: &str) -> Result<(), TldError> {
        if old_ref == new_ref {
            return Ok(());
        }

        if let Some(element) = self.elements.remove(old_ref) {
            self.elements.insert(new_ref.to_string(), element);

            // Cascade to connectors
            let mut new_connectors = HashMap::new();
            for (key, mut conn) in self.connectors.drain() {
                let mut changed = false;
                if conn.view == old_ref {
                    conn.view = new_ref.to_string();
                    changed = true;
                }
                if conn.source == old_ref {
                    conn.source = new_ref.to_string();
                    changed = true;
                }
                if conn.target == old_ref {
                    conn.target = new_ref.to_string();
                    changed = true;
                }

                if changed {
                    new_connectors.insert(conn.resource_ref(), conn);
                } else {
                    new_connectors.insert(key, conn);
                }
            }
            self.connectors = new_connectors;

            // Cascade to placements
            for el in self.elements.values_mut() {
                for p in &mut el.placements {
                    if p.parent_ref == old_ref {
                        p.parent_ref = new_ref.to_string();
                    }
                }
            }

            // Cascade to metadata
            if let Some(meta) = &mut self.meta {
                if let Some(m) = meta.elements.remove(old_ref) {
                    meta.elements.insert(new_ref.to_string(), m);
                }
                if let Some(m) = meta.views.remove(old_ref) {
                    meta.views.insert(new_ref.to_string(), m);
                }
            }
        }

        Ok(())
    }

    /// Appends a connector after resolving the view if not provided.
    pub fn append_connector(&mut self, mut connector: Connector) -> Result<String, TldError> {
        if connector.view.is_empty() {
            connector.view = self.infer_connector_view(&connector.source, &connector.target)?;
        }

        let ref_name = connector.resource_ref();
        self.connectors.insert(ref_name.clone(), connector);
        Ok(ref_name)
    }

    /// Infers the shared parent view for two elements.
    fn infer_connector_view(&self, source_ref: &str, target_ref: &str) -> Result<String, TldError> {
        let source = self.elements.get(source_ref).ok_or_else(|| {
            TldError::Generic(format!("Source element '{}' not found", source_ref))
        })?;
        let target = self.elements.get(target_ref).ok_or_else(|| {
            TldError::Generic(format!("Target element '{}' not found", target_ref))
        })?;

        let source_parents: Vec<_> = source
            .placements
            .iter()
            .map(|p| p.parent_ref.as_str())
            .collect();
        let target_parents: Vec<_> = target
            .placements
            .iter()
            .map(|p| p.parent_ref.as_str())
            .collect();

        for sp in &source_parents {
            if target_parents.contains(sp) {
                return Ok(sp.to_string());
            }
        }

        Ok("root".to_string())
    }

    /// Updates a specific field on an element.
    pub fn update_element_field(
        &mut self,
        ref_name: &str,
        field: &str,
        value: &str,
    ) -> Result<(), TldError> {
        if field == "ref" {
            return self.rename_element(ref_name, value);
        }

        let el = self
            .elements
            .get_mut(ref_name)
            .ok_or_else(|| TldError::Generic(format!("Element '{}' not found", ref_name)))?;

        match field {
            "name" => el.name = value.to_string(),
            "kind" => el.kind = value.to_string(),
            "owner" => el.owner = value.to_string(),
            "description" => el.description = value.to_string(),
            "technology" => el.technology = value.to_string(),
            "url" => el.url = value.to_string(),
            "logo_url" => el.logo_url = value.to_string(),
            "repo" => el.repo = value.to_string(),
            "branch" => el.branch = value.to_string(),
            "language" => el.language = value.to_string(),
            "file_path" => el.file_path = value.to_string(),
            "symbol" => el.symbol = value.to_string(),
            "has_view" => el.has_view = value.parse().unwrap_or(false),
            "view_label" => el.view_label = value.to_string(),
            _ => {
                return Err(TldError::Generic(format!(
                    "Unknown element field '{}'",
                    field
                )));
            }
        }

        Ok(())
    }

    /// Updates a specific field on a connector.
    pub fn update_connector_field(
        &mut self,
        ref_name: &str,
        field: &str,
        value: &str,
    ) -> Result<(), TldError> {
        let mut conn = self
            .connectors
            .remove(ref_name)
            .ok_or_else(|| TldError::Generic(format!("Connector '{}' not found", ref_name)))?;

        match field {
            "view" => conn.view = value.to_string(),
            "source" => conn.source = value.to_string(),
            "target" => conn.target = value.to_string(),
            "label" => conn.label = value.to_string(),
            "description" => conn.description = value.to_string(),
            "relationship" => conn.relationship = value.to_string(),
            "direction" => conn.direction = value.to_string(),
            "style" => conn.style = value.to_string(),
            "url" => conn.url = value.to_string(),
            _ => {
                self.connectors.insert(ref_name.to_string(), conn);
                return Err(TldError::Generic(format!(
                    "Unknown connector field '{}'",
                    field
                )));
            }
        }

        let new_ref = conn.resource_ref();

        // Update metadata if present
        if let Some(meta) = &mut self.meta
            && let Some(m) = meta.connectors.remove(ref_name)
        {
            meta.connectors.insert(new_ref.clone(), m);
        }

        self.connectors.insert(new_ref, conn);
        Ok(())
    }

    /// Removes a connector matching coordinates. Returns count of removed items.
    pub fn remove_connector(
        &mut self,
        view: &str,
        source: &str,
        target: &str,
    ) -> Result<usize, TldError> {
        let before = self.connectors.len();
        let mut removed_refs = Vec::new();

        self.connectors.retain(|ref_name, c| {
            if c.view == view && c.source == source && c.target == target {
                removed_refs.push(ref_name.clone());
                false
            } else {
                true
            }
        });

        let count = before - self.connectors.len();
        if count > 0 {
            // Cleanup metadata
            if let Some(meta) = &mut self.meta {
                for r in removed_refs {
                    meta.connectors.remove(&r);
                }
            }
        }

        Ok(count)
    }
}
