use crate::client::diagv1;
use crate::workspace::types::*;
use chrono::{TimeZone, Utc};
use std::collections::HashMap;

pub fn from_export_response(
    wdir: &str,
    global_cfg: Config,
    workspace_config: Option<WorkspaceConfig>,
    existing_meta: Option<&Meta>,
    msg: diagv1::ExportOrganizationResponse,
) -> Workspace {
    let mut new_ws = Workspace {
        dir: wdir.to_string(),
        config: global_cfg,
        elements: HashMap::new(),
        connectors: HashMap::new(),
        workspace_config,
        meta: Some(Meta {
            elements: HashMap::new(),
            views: HashMap::new(),
            connectors: HashMap::new(),
        }),
    };

    let mut existing_element_refs = HashMap::new();
    let mut existing_connector_refs = HashMap::new();

    if let Some(meta) = existing_meta {
        for (r, m) in &meta.elements {
            existing_element_refs.insert(m.id, r.clone());
        }
        for (r, m) in &meta.connectors {
            existing_connector_refs.insert(m.id, r.clone());
        }
    }

    let mut object_id_to_ref = HashMap::new();
    for e in msg.elements {
        let ref_name = existing_element_refs
            .get(&e.id)
            .cloned()
            .unwrap_or_else(|| crate::workspace::slugify(&e.name));

        object_id_to_ref.insert(e.id, ref_name.clone());

        let kind = e.kind.unwrap_or_else(|| "element".to_string());

        new_ws.elements.insert(
            ref_name.clone(),
            Element {
                name: e.name,
                kind,
                description: e.description.unwrap_or_default(),
                technology: e.technology.unwrap_or_default(),
                url: e.url.unwrap_or_default(),
                logo_url: e.logo_url.unwrap_or_default(),
                repo: e.repo.unwrap_or_default(),
                branch: e.branch.unwrap_or_default(),
                language: e.language.unwrap_or_default(),
                file_path: e.file_path.unwrap_or_default(),
                symbol: String::new(), // Protobuf Element doesn't have symbol yet?
                has_view: e.has_view,
                view_label: e.view_label.unwrap_or_default(),
                placements: Vec::new(),
                owner: String::new(),
            },
        );

        if let Some(meta) = &mut new_ws.meta && let Some(ts) = e.updated_at {
            meta.elements.insert(
                ref_name,
                ResourceMetadata {
                    id: e.id,
                    updated_at: Utc.timestamp_opt(ts.seconds, ts.nanos as u32).unwrap(),
                    conflict: false,
                },
            );
        }
    }

    let mut diagram_id_to_view_ref = HashMap::new();
    for d in msg.views {
        let mut owner_ref = "root".to_string();
        for (r, el) in &new_ws.elements {
            if el.name.to_lowercase() == d.name.to_lowercase() {
                owner_ref = r.clone();
                break;
            }
        }

        diagram_id_to_view_ref.insert(d.id, owner_ref.clone());

        if owner_ref != "root" {
            if let Some(el) = new_ws.elements.get_mut(&owner_ref) {
                el.has_view = true;
                if el.view_label.is_empty() {
                    el.view_label = d.name.clone();
                }
            }
            if let Some(meta) = &mut new_ws.meta && let Some(ts) = d.updated_at {
                meta.views.insert(
                    owner_ref,
                    ResourceMetadata {
                        id: d.id,
                        updated_at: Utc.timestamp_opt(ts.seconds, ts.nanos as u32).unwrap(),
                        conflict: false,
                    },
                );
            }
        }
    }

    for p in msg.placements {
        if let Some(element_ref) = object_id_to_ref.get(&p.element_id) {
            let parent_ref = diagram_id_to_view_ref
                .get(&p.view_id)
                .cloned()
                .unwrap_or_else(|| "root".to_string());
            if let Some(el) = new_ws.elements.get_mut(element_ref) {
                el.placements.push(ViewPlacement {
                    parent_ref,
                    position_x: p.position_x,
                    position_y: p.position_y,
                });
            }
        }
    }

    for c in msg.connectors {
        let view_ref = diagram_id_to_view_ref
            .get(&c.view_id)
            .cloned()
            .unwrap_or_else(|| "root".to_string());
        let src_ref = object_id_to_ref.get(&c.source_element_id);
        let tgt_ref = object_id_to_ref.get(&c.target_element_id);

        if let (Some(s), Some(t)) = (src_ref, tgt_ref) {
            let label = c.label.unwrap_or_default();
            let key = format!("{}:{}:{}:{}", view_ref, s, t, label);

            new_ws.connectors.insert(
                key.clone(),
                Connector {
                    view: view_ref,
                    source: s.clone(),
                    target: t.clone(),
                    label,
                    description: c.description.unwrap_or_default(),
                    relationship: c.relationship.unwrap_or_default(),
                    direction: c.direction,
                    style: c.style,
                    url: c.url.unwrap_or_default(),
                },
            );

            if let Some(meta) = &mut new_ws.meta && let Some(ts) = c.updated_at {
                meta.connectors.insert(
                    key,
                    ResourceMetadata {
                        id: c.id,
                        updated_at: Utc.timestamp_opt(ts.seconds, ts.nanos as u32).unwrap(),
                        conflict: false,
                    },
                );
            }
        }
    }

    new_ws
}
