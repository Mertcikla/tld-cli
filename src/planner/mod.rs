use crate::workspace::Workspace;
use crate::client::diagv1::{ApplyPlanRequest, PlanElement, PlanViewPlacement, PlanConnector};
use crate::error::TldError;
use prost_types::Timestamp;

pub struct Plan {
    pub request: ApplyPlanRequest,
}

pub fn build(ws: &Workspace, recreate_ids: bool) -> Result<Plan, TldError> {
    let mut req = ApplyPlanRequest {
        org_id: ws.config.org_id.clone(),
        api_key: None,
        dry_run: Some(true),
        ..Default::default()
    };

    let synth_root = "root".to_string();

    // Elements
    for (ref_name, element) in &ws.elements {
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
            file_path: some_if_not_empty(&element.file_path),
            view_label: some_if_not_empty(&element.view_label),
            has_view: element.has_view,
            ..Default::default()
        };

        for placement in &element.placements {
            let mut p = PlanViewPlacement {
                parent_ref: if placement.parent_ref.is_empty() { synth_root.clone() } else { placement.parent_ref.clone() },
                ..Default::default()
            };
            if placement.position_x != 0.0 { p.position_x = Some(placement.position_x); }
            if placement.position_y != 0.0 { p.position_y = Some(placement.position_y); }
            plan_el.placements.push(p);
        }

        if !recreate_ids {
            if let Some(meta) = &ws.meta {
                if let Some(m) = meta.elements.get(ref_name) {
                    plan_el.id = Some(m.id);
                    plan_el.updated_at = Some(Timestamp {
                        seconds: m.updated_at.timestamp(),
                        nanos: m.updated_at.timestamp_subsec_nanos() as i32,
                    });
                }
                if element.has_view {
                    if let Some(m) = meta.views.get(ref_name) {
                        plan_el.view_id = Some(m.id);
                        plan_el.view_updated_at = Some(Timestamp {
                            seconds: m.updated_at.timestamp(),
                            nanos: m.updated_at.timestamp_subsec_nanos() as i32,
                        });
                    }
                }
            }
        }

        req.elements.push(plan_el);
    }

    // Connectors
    for (ref_name, connector) in &ws.connectors {
        let mut plan_conn = PlanConnector {
            r#ref: ref_name.clone(),
            view_ref: if connector.view.is_empty() { synth_root.clone() } else { connector.view.clone() },
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

        if !recreate_ids {
            if let Some(meta) = &ws.meta {
                if let Some(m) = meta.connectors.get(ref_name) {
                    plan_conn.id = Some(m.id);
                    plan_conn.updated_at = Some(Timestamp {
                        seconds: m.updated_at.timestamp(),
                        nanos: m.updated_at.timestamp_subsec_nanos() as i32,
                    });
                }
            }
        }

        req.connectors.push(plan_conn);
    }

    Ok(Plan { request: req })
}

fn some_if_not_empty(s: &str) -> Option<String> {
    if s.is_empty() { None } else { Some(s.to_string()) }
}
