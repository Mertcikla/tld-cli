use crate::cli::repository_root;
use crate::client;
use crate::client::diagv1;
use crate::error::TldError;
use crate::output;
use crate::planner;
use crate::workspace;
use chrono::{TimeZone, Utc};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct ApplyArgs {
    /// Skip the interactive approval prompt
    #[arg(short = 'f', long = "force", default_value = "false")]
    pub force: bool,
    /// Ignore existing resource IDs and let the server generate new ones
    #[arg(long, default_value = "false")]
    pub recreate_ids: bool,
}

#[expect(clippy::print_stdout)]
pub async fn exec(args: ApplyArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;

    // Check for server URL
    if ws.config.server_url.is_empty() {
        return Err(TldError::Generic(
            "No server URL configured. Run 'tld login' first, or set TLD_SERVER_URL environment variable.".to_string(),
        ));
    }

    // Check for API key
    if ws.config.api_key.is_empty() {
        return Err(TldError::Generic(
            "No API key found. Run 'tld login' first, or set TLD_API_KEY environment variable."
                .to_string(),
        ));
    }
    if ws.config.org_id.is_empty() {
        return Err(TldError::Generic(
            "No org ID found. Run 'tld login' first, or set TLD_ORG_ID environment variable."
                .to_string(),
        ));
    }

    let mut ws_client =
        client::new_workspace_client(&ws.config.server_url, &ws.config.api_key).await?;

    let prep_spinner = output::new_spinner("Preparing repository roots...");
    let _ = repository_root::run_dry_run_with_repository_root_sync(
        &mut ws,
        &mut ws_client,
        args.recreate_ids,
        true,
    )
    .await?;
    prep_spinner.finish_and_clear();

    output::print_info("Applying workspace to tlDiagram...");
    let mut plan = planner::build(&ws, args.recreate_ids)?;
    plan.request.dry_run = Some(false); // Actually apply

    let spinner = output::new_spinner("Contacting server...");
    let resp = ws_client.apply_workspace_plan(plan.request).await?;
    spinner.finish_and_clear();

    if let Some(summary) = &resp.summary {
        output::print_ok(&format!(
            "Applied successfully! {0} elements, {1} views, {2} connectors updated.",
            summary.elements_planned, summary.views_planned, summary.connectors_planned
        ));
    } else {
        output::print_ok("Applied successfully.");
    }

    if let Some(version) = &resp.version {
        println!("New workspace version: {0}", version.version_id);
    }

    // Conflicts notification
    if !resp.conflicts.is_empty() {
        output::print_warn(&format!(
            "Warning: {} conflicts were detected during apply.",
            resp.conflicts.len()
        ));
    }

    update_workspace_metadata_from_apply_response(&mut ws, &resp)?;

    Ok(())
}

fn update_workspace_metadata_from_apply_response(
    ws: &mut workspace::Workspace,
    resp: &diagv1::ApplyPlanResponse,
) -> Result<(), TldError> {
    let mut renamed_elements = std::collections::HashMap::new();
    for result in &resp.element_results {
        if !result.canonical_ref.is_empty() && result.canonical_ref != result.r#ref {
            renamed_elements.insert(result.r#ref.clone(), result.canonical_ref.clone());
            ws.rename_element(&result.r#ref, &result.canonical_ref);
        }
    }

    let meta = ws.meta.get_or_insert_with(Default::default);
    meta.elements.clear();
    meta.views.clear();
    meta.connectors.clear();

    for result in &resp.element_results {
        let key = element_result_ref(result);
        meta.elements.insert(
            key,
            workspace::ResourceMetadata {
                id: result.id,
                updated_at: timestamp_to_utc(result.updated_at.as_ref())?,
                conflict: false,
            },
        );
    }

    for (ref_name, resource) in &resp.view_metadata {
        let key = renamed_elements
            .get(ref_name)
            .cloned()
            .unwrap_or_else(|| ref_name.clone());
        meta.views.insert(
            key,
            workspace::ResourceMetadata {
                id: resource.id,
                updated_at: timestamp_to_utc(resource.updated_at.as_ref())?,
                conflict: false,
            },
        );
    }

    for result in &resp.connector_results {
        let key = connector_result_ref(result);
        meta.connectors.insert(
            key,
            workspace::ResourceMetadata {
                id: result.id,
                updated_at: timestamp_to_utc(result.updated_at.as_ref())?,
                conflict: false,
            },
        );
    }

    let lock_file = workspace::lockfile::LockFile::from_workspace(ws);
    workspace::lockfile::save_lock_file(&ws.dir, &lock_file)?;

    Ok(())
}

fn element_result_ref(result: &diagv1::ApplyPlanElementResult) -> String {
    if result.canonical_ref.is_empty() {
        result.r#ref.clone()
    } else {
        result.canonical_ref.clone()
    }
}

fn connector_result_ref(result: &diagv1::ApplyPlanConnectorResult) -> String {
    if result.canonical_ref.is_empty() {
        result.r#ref.clone()
    } else {
        result.canonical_ref.clone()
    }
}

fn timestamp_to_utc(
    ts: Option<&prost_types::Timestamp>,
) -> Result<chrono::DateTime<Utc>, TldError> {
    let ts =
        ts.ok_or_else(|| TldError::Generic("apply response missing updated_at".to_string()))?;
    Utc.timestamp_opt(ts.seconds, ts.nanos.cast_unsigned())
        .single()
        .ok_or_else(|| TldError::Generic("apply response contains invalid timestamp".to_string()))
}

#[cfg(test)]
mod tests {
    use super::update_workspace_metadata_from_apply_response;
    use crate::client::diagv1;
    use crate::workspace::{Config, Connector, Element, Workspace};
    use prost_types::Timestamp;
    use std::collections::HashMap;

    #[test]
    fn apply_response_renames_elements_and_persists_canonical_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut connectors = HashMap::new();
        connectors.insert(
            "root:old-ref:target:uses".to_string(),
            Connector {
                view: "root".to_string(),
                source: "old-ref".to_string(),
                target: "target".to_string(),
                label: "uses".to_string(),
                ..Default::default()
            },
        );

        let mut elements = HashMap::new();
        elements.insert(
            "old-ref".to_string(),
            Element {
                name: "Service".to_string(),
                ..Default::default()
            },
        );
        elements.insert(
            "target".to_string(),
            Element {
                name: "Target".to_string(),
                ..Default::default()
            },
        );

        let mut ws = Workspace {
            dir: dir.path().to_string_lossy().into_owned(),
            config: Config::default(),
            ws_config: None,
            elements,
            connectors,
            meta: None,
        };

        update_workspace_metadata_from_apply_response(
            &mut ws,
            &diagv1::ApplyPlanResponse {
                element_results: vec![
                    diagv1::ApplyPlanElementResult {
                        r#ref: "old-ref".to_string(),
                        canonical_ref: "canonical-ref".to_string(),
                        id: 10,
                        updated_at: Some(Timestamp {
                            seconds: 1,
                            nanos: 0,
                        }),
                    },
                    diagv1::ApplyPlanElementResult {
                        r#ref: "target".to_string(),
                        canonical_ref: "target".to_string(),
                        id: 11,
                        updated_at: Some(Timestamp {
                            seconds: 2,
                            nanos: 0,
                        }),
                    },
                ],
                connector_results: vec![diagv1::ApplyPlanConnectorResult {
                    r#ref: "root:old-ref:target:uses".to_string(),
                    canonical_ref: "root:canonical-ref:target:uses".to_string(),
                    id: 20,
                    updated_at: Some(Timestamp {
                        seconds: 3,
                        nanos: 0,
                    }),
                }],
                ..Default::default()
            },
        )
        .expect("update metadata");

        assert!(ws.elements.contains_key("canonical-ref"));
        assert!(!ws.elements.contains_key("old-ref"));
        assert!(ws.connectors.contains_key("root:canonical-ref:target:uses"));
        let meta = ws.meta.expect("meta");
        assert!(meta.elements.contains_key("canonical-ref"));
        assert!(
            meta.connectors
                .contains_key("root:canonical-ref:target:uses")
        );
    }
}
