use crate::client::{AuthedWorkspaceClient, diagv1};
use crate::error::TldError;
use crate::output;
use crate::planner;
use crate::workspace::{self, RepositoryRootBinding, ResourceMetadata, Workspace};
use chrono::{DateTime, TimeZone, Utc};
use std::collections::HashSet;

pub async fn run_dry_run_with_repository_root_sync(
    ws: &mut Workspace,
    client: &mut AuthedWorkspaceClient,
    recreate_ids: bool,
    announce: bool,
) -> Result<diagv1::ApplyPlanResponse, TldError> {
    let mut root_binding = workspace::ensure_default_repository_root(ws)?;
    persist_root_changes(
        ws,
        root_binding
            .as_ref()
            .is_some_and(|binding| binding.workspace_changed),
        root_binding
            .as_ref()
            .is_some_and(|binding| binding.config_changed),
    )?;

    if let Some(binding) = root_binding.as_mut() {
        let mut workspace_changed = false;

        if ws
            .meta
            .as_ref()
            .is_some_and(|meta| meta.elements.contains_key(&binding.local_ref))
        {
            if announce {
                output::print_info(&format!(
                    "Repository root '{}' already exists on tlDiagram.",
                    binding.project_name
                ));
            }
        } else if let Some(existing_root) =
            find_remote_repository_root(client, &ws.config.org_id, &binding.project_name).await?
        {
            if announce {
                output::print_info(&format!(
                    "Repository root '{}' already exists on tlDiagram.",
                    binding.project_name
                ));
            }
            bind_remote_root(ws, &binding.local_ref, &existing_root);
            workspace_changed = true;
        } else if announce {
            output::print_info(&format!(
                "Repository root '{}' will be created in workspace-root.",
                binding.project_name
            ));
        }

        persist_root_changes(ws, workspace_changed, false)?;
    }

    let mut response = execute_dry_run(ws, client, recreate_ids).await?;

    if let Some(binding) = root_binding.as_mut()
        && canonicalize_root_ref(ws, binding, &response, announce)?
    {
        persist_root_changes(ws, true, true)?;
        response = execute_dry_run(ws, client, recreate_ids).await?;
    }

    Ok(response)
}

async fn execute_dry_run(
    ws: &Workspace,
    client: &mut AuthedWorkspaceClient,
    recreate_ids: bool,
) -> Result<diagv1::ApplyPlanResponse, TldError> {
    let plan = planner::build(ws, recreate_ids)?;
    client.apply_workspace_plan(plan.request).await
}

fn persist_root_changes(
    ws: &Workspace,
    save_workspace: bool,
    save_config: bool,
) -> Result<(), TldError> {
    if save_workspace {
        workspace::save(ws)?;
    }
    if save_config && let Some(config) = &ws.ws_config {
        workspace::save_workspace_config(&ws.dir, config)?;
    }
    Ok(())
}

fn bind_remote_root(ws: &mut Workspace, local_ref: &str, remote: &RemoteRootMatch) {
    let meta = ws.meta.get_or_insert_with(Default::default);
    meta.elements.insert(
        local_ref.to_string(),
        ResourceMetadata {
            id: remote.id,
            updated_at: remote.updated_at,
            conflict: false,
        },
    );
}

fn canonicalize_root_ref(
    ws: &mut Workspace,
    binding: &mut RepositoryRootBinding,
    response: &diagv1::ApplyPlanResponse,
    announce: bool,
) -> Result<bool, TldError> {
    let Some(result) = response
        .element_results
        .iter()
        .find(|result| result.r#ref == binding.local_ref)
    else {
        return Ok(false);
    };

    if result.canonical_ref.is_empty() || result.canonical_ref == binding.local_ref {
        return Ok(false);
    }

    if announce {
        output::print_info(&format!(
            "Repository root '{}' will use canonical ref '{}'.",
            binding.project_name, result.canonical_ref
        ));
    }

    ws.rename_element(&binding.local_ref, &result.canonical_ref);
    if let Some(config) = ws.ws_config.as_mut()
        && let Some(repo) = config.repositories.get_mut(&binding.repo_key)
    {
        repo.root.clone_from(&result.canonical_ref);
    } else {
        return Err(TldError::Generic(format!(
            "Workspace repository '{}' is no longer configured.",
            binding.repo_key
        )));
    }

    binding.local_ref = result.canonical_ref.clone();
    Ok(true)
}

#[derive(Debug, Clone)]
struct RemoteRootMatch {
    id: i32,
    updated_at: DateTime<Utc>,
    has_view: bool,
}

async fn find_remote_repository_root(
    client: &mut AuthedWorkspaceClient,
    org_id: &str,
    project_name: &str,
) -> Result<Option<RemoteRootMatch>, TldError> {
    let root_view_ids = list_root_view_ids(client, org_id).await?;
    if root_view_ids.is_empty() {
        return Ok(None);
    }

    let response = client
        .list_elements(diagv1::ListElementsRequest {
            limit: 100,
            offset: 0,
            search: project_name.to_string(),
        })
        .await?;

    let mut candidates = Vec::new();
    for element in response.elements {
        if !element.name.eq_ignore_ascii_case(project_name) {
            continue;
        }

        let placements = client
            .list_element_placements(diagv1::ListElementPlacementsRequest {
                element_id: element.id,
            })
            .await?;

        if !placements
            .placements
            .iter()
            .any(|placement| root_view_ids.contains(&placement.view_id))
        {
            continue;
        }

        let Some(updated_at) = timestamp_to_utc(element.updated_at.as_ref()) else {
            continue;
        };

        candidates.push(RemoteRootMatch {
            id: element.id,
            updated_at,
            has_view: element.has_view,
        });
    }

    candidates.sort_by_key(|candidate| (!candidate.has_view, candidate.id));
    Ok(candidates.into_iter().next())
}

async fn list_root_view_ids(
    client: &mut AuthedWorkspaceClient,
    org_id: &str,
) -> Result<HashSet<i32>, TldError> {
    let response = client
        .get_workspace(diagv1::GetWorkspaceRequest {
            org_id: org_id.to_string(),
            parent_id: None,
            level: None,
            search: None,
            limit: 1000,
            offset: 0,
            include_content: false,
            api_key: None,
        })
        .await?;

    Ok(response
        .views
        .into_iter()
        .filter(|view| view.owner_element_id.is_none() && view.parent_view_id.is_none())
        .map(|view| view.id)
        .collect())
}

fn timestamp_to_utc(ts: Option<&prost_types::Timestamp>) -> Option<DateTime<Utc>> {
    let timestamp = ts?;
    Utc.timestamp_opt(timestamp.seconds, timestamp.nanos.cast_unsigned())
        .single()
}
