use crate::client;
use crate::client::diagv1;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::Args;
use tonic::Request;

#[derive(Args, Debug, Clone)]
pub struct PullArgs {
    /// Apply without prompting for confirmation
    #[arg(long, default_value = "false")]
    pub force: bool,
    /// show what would be pulled without writing
    #[arg(long, default_value = "false")]
    pub dry_run: bool,
}

pub async fn exec(args: PullArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;

    if ws.config.org_id.is_empty() {
        return Err(TldError::Generic(
            "Org ID required in .tld.yaml".to_string(),
        ));
    }

    if ws.config.api_key.is_empty() {
        return Err(TldError::Generic(
            "No API key found. Run 'tld login' first.".to_string(),
        ));
    }

    output::print_info("Pulling latest state from server...");

    let mut client =
        client::new_workspace_client(&ws.config.server_url, &ws.config.api_key).await?;

    let req = Request::new(diagv1::ExportOrganizationRequest {
        org_id: ws.config.org_id.clone(),
        api_key: None,
    });

    let spinner = output::new_spinner("Contacting server...");
    let resp = client.export_workspace(req).await?.into_inner();
    spinner.finish_and_clear();

    let server_ws = workspace::conversion::from_export_response(
        &wdir,
        ws.config.clone(),
        ws.workspace_config.clone(),
        ws.meta.as_ref(),
        resp,
    );

    // Load last sync state from lockfile for 3-way merge
    let lock_file = workspace::load_lock_file(&wdir)?.unwrap_or_default();
    let last_sync_meta = lock_file.to_meta();
    let current_meta = ws.meta.clone().unwrap_or_default();

    if args.dry_run {
        output::print_info(&format!(
            "Would pull {} elements and {} connectors.",
            server_ws.elements.len(),
            server_ws.connectors.len()
        ));
        return Ok(());
    }

    if args.force {
        workspace::save(&server_ws)?;
        output::print_ok("Workspace updated (forced overwrite).");
    } else {
        workspace::merger::merge_workspace(&wdir, &server_ws, &last_sync_meta, &current_meta)?;
        output::print_ok("Workspace merged with server state.");
    }

    // Update lockfile
    let lf = workspace::lockfile::LockFile::from_workspace(&server_ws);
    workspace::lockfile::save_lock_file(&wdir, &lf)?;

    Ok(())
}
