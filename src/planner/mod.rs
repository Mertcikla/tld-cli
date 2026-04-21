use crate::client::diagv1::{ApplyPlanRequest, PlanConnector, PlanElement, PlanViewPlacement};
use crate::error::TldError;
use crate::workspace::Workspace;
use prost_types::Timestamp;
use std::collections::HashSet;

pub struct Plan {
    pub request: ApplyPlanRequest,
}

#[expect(clippy::unnecessary_wraps)]
pub fn build(ws: &Workspace, recreate_ids: bool) -> Result<Plan, TldError> {
    let mut req = ApplyPlanRequest {
        org_id: ws.config.org_id.clone(),
        api_key: None,
        dry_run: Some(true),
        ..Default::default()
    };

    let synth_root = "root".to_string();

    // Any element used as a placement parent or connector view must have a
    // canonical view in the plan, otherwise backend apply fails.
    let mut required_views: HashSet<String> = HashSet::new();
    for element in ws.elements.values() {
        for placement in &element.placements {
            if !placement.parent_ref.is_empty() && placement.parent_ref != synth_root {
                required_views.insert(placement.parent_ref.clone());
            }
        }
    }
    for connector in ws.connectors.values() {
        if !connector.view.is_empty() && connector.view != synth_root {
            required_views.insert(connector.view.clone());
        }
    }

    // Elements
    for (ref_name, element) in &ws.elements {
        let has_view = element.has_view || required_views.contains(ref_name);

        let mut plan_el = PlanElement {
            r#ref: ref_name.clone(),
            name: element.name.clone(),
            kind: Some(element.kind.clone()),
            description: some_if_not_empty(&element.description),
            technology: some_if_not_empty(&element.technology),
            url: some_if_not_empty(&element.url),
            logo_url: some_if_not_empty(&element.logo_url),
            repo: some_if_not_empty(&element.repo),
            branch: some_if_not_empty(&element.branch),
            language: some_if_not_empty(&element.language),
            file_path: if element.symbol.is_empty() {
                some_if_not_empty(&element.file_path)
            } else {
                let anchor = serde_json::json!({
                    "name": element.symbol,
                    "type": element.symbol_kind
                });
                Some(format!("{}#{}", element.file_path, anchor))
            },
            view_label: some_if_not_empty(&element.view_label),
            has_view,
            tags: element.tags.clone(),
            ..Default::default()
        };

        for placement in &element.placements {
            let mut p = PlanViewPlacement {
                parent_ref: if placement.parent_ref.is_empty() {
                    synth_root.clone()
                } else {
                    placement.parent_ref.clone()
                },
                ..Default::default()
            };
            if placement.position_x != 0.0 {
                p.position_x = Some(placement.position_x);
            }
            if placement.position_y != 0.0 {
                p.position_y = Some(placement.position_y);
            }
            plan_el.placements.push(p);
        }

        if !recreate_ids && let Some(meta) = &ws.meta {
            if let Some(m) = meta.elements.get(ref_name) {
                plan_el.id = Some(m.id);
                plan_el.updated_at = Some(Timestamp {
                    seconds: m.updated_at.timestamp(),
                    nanos: m.updated_at.timestamp_subsec_nanos().cast_signed(),
                });
            }
            if has_view && let Some(m) = meta.views.get(ref_name) {
                plan_el.view_id = Some(m.id);
                plan_el.view_updated_at = Some(Timestamp {
                    seconds: m.updated_at.timestamp(),
                    nanos: m.updated_at.timestamp_subsec_nanos().cast_signed(),
                });
            }
        }

        req.elements.push(plan_el);
    }

    // Connectors
    for (ref_name, connector) in &ws.connectors {
        let mut plan_conn = PlanConnector {
            r#ref: ref_name.clone(),
            view_ref: if connector.view.is_empty() {
                synth_root.clone()
            } else {
                connector.view.clone()
            },
            source_element_ref: connector.source.clone(),
            target_element_ref: connector.target.clone(),
            label: some_if_not_empty(&connector.label),
            description: some_if_not_empty(&connector.description),
            relationship: some_if_not_empty(&connector.relationship),
            direction: some_if_not_empty(&connector.direction),
            style: some_if_not_empty(&connector.style),
            url: some_if_not_empty(&connector.url),
            ..Default::default()
        };

        if !recreate_ids
            && let Some(meta) = &ws.meta
            && let Some(m) = meta.connectors.get(ref_name)
        {
            plan_conn.id = Some(m.id);
            plan_conn.updated_at = Some(Timestamp {
                seconds: m.updated_at.timestamp(),
                nanos: m.updated_at.timestamp_subsec_nanos().cast_signed(),
            });
        }

        req.connectors.push(plan_conn);
    }

    Ok(Plan { request: req })
}

fn some_if_not_empty(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::workspace::{Config, Connector, Element, ViewPlacement, Workspace};
    use std::collections::HashMap;

    #[test]
    fn auto_enables_views_for_referenced_parents() {
        let mut elements = HashMap::new();
        elements.insert(
            "parent".to_string(),
            Element {
                name: "Parent".to_string(),
                kind: "file".to_string(),
                has_view: false,
                ..Default::default()
            },
        );
        elements.insert(
            "child".to_string(),
            Element {
                name: "Child".to_string(),
                kind: "function".to_string(),
                placements: vec![ViewPlacement {
                    parent_ref: "parent".to_string(),
                    position_x: 0.0,
                    position_y: 0.0,
                }],
                ..Default::default()
            },
        );

        let ws = Workspace {
            dir: ".".to_string(),
            config: Config {
                org_id: "org-id".to_string(),
                ..Default::default()
            },
            ws_config: None,
            elements,
            connectors: HashMap::<String, Connector>::new(),
            meta: None,
        };

        let plan = build(&ws, false).expect("build plan");
        let parent = plan
            .request
            .elements
            .iter()
            .find(|e| e.r#ref == "parent")
            .expect("parent element in plan");

        assert!(parent.has_view);
    }

    #[test]
    fn auto_enables_views_for_connector_view_refs() {
        let mut elements = HashMap::new();
        elements.insert(
            "view-owner".to_string(),
            Element {
                name: "View Owner".to_string(),
                kind: "service".to_string(),
                has_view: false,
                ..Default::default()
            },
        );
        elements.insert(
            "source".to_string(),
            Element {
                name: "Source".to_string(),
                kind: "service".to_string(),
                ..Default::default()
            },
        );
        elements.insert(
            "target".to_string(),
            Element {
                name: "Target".to_string(),
                kind: "service".to_string(),
                ..Default::default()
            },
        );

        let mut connectors = HashMap::new();
        connectors.insert(
            "view-owner:source:target:uses".to_string(),
            Connector {
                view: "view-owner".to_string(),
                source: "source".to_string(),
                target: "target".to_string(),
                label: "uses".to_string(),
                ..Default::default()
            },
        );

        let ws = Workspace {
            dir: ".".to_string(),
            config: Config {
                org_id: "org-id".to_string(),
                ..Default::default()
            },
            ws_config: None,
            elements,
            connectors,
            meta: None,
        };

        let plan = build(&ws, false).expect("build plan");
        let owner = plan
            .request
            .elements
            .iter()
            .find(|e| e.r#ref == "view-owner")
            .expect("view owner element in plan");

        assert!(owner.has_view);
    }

    #[test]
    fn encodes_symbol_editor_anchor_in_frontend_format() {
        let mut elements = HashMap::new();
        elements.insert(
            "entrypoint".to_string(),
            Element {
                name: "handleRequest".to_string(),
                kind: "entrypoint".to_string(),
                language: "typescript".to_string(),
                file_path: "src/server.ts".to_string(),
                symbol: "handleRequest".to_string(),
                symbol_kind: "function".to_string(),
                symbol_line: 42,
                ..Default::default()
            },
        );

        let ws = Workspace {
            dir: ".".to_string(),
            config: Config {
                org_id: "org-id".to_string(),
                ..Default::default()
            },
            ws_config: None,
            elements,
            connectors: HashMap::<String, Connector>::new(),
            meta: None,
        };

        let plan = build(&ws, false).expect("build plan");
        let entrypoint = plan
            .request
            .elements
            .iter()
            .find(|e| e.r#ref == "entrypoint")
            .expect("entrypoint element in plan");

        assert_eq!(
            entrypoint.file_path.as_deref(),
            Some("src/server.ts#{\"name\":\"handleRequest\",\"type\":\"function\"}")
        );
        assert_eq!(entrypoint.language.as_deref(), Some("typescript"));
    }
}
