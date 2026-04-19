use crate::client;
use crate::client::diagv1;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct ExportArgs {
    /// Organisation ID to export (default: from .tld.yaml)
    pub org_id: Option<String>,
    /// Write export to this file (default: stdout)
    #[arg(short = 'o', long)]
    pub output_file: Option<String>,
}

pub async fn exec(args: ExportArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;

    let org_id = args
        .org_id
        .or_else(|| Some(ws.config.org_id.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            TldError::Generic(
                "Org ID required. Provide it as an argument or in .tld.yaml".to_string(),
            )
        })?;

    if ws.config.api_key.is_empty() {
        return Err(TldError::Generic(
            "No API key found. Run 'tld login' first.".to_string(),
        ));
    }

    output::print_info(&format!(
        "Exporting organization {} from {}...",
        org_id, ws.config.server_url
    ));

    let mut client = client::new_workspace_client(&ws.config.server_url, &ws.config.api_key)?;

    let req = diagv1::ExportOrganizationRequest {
        org_id: org_id.clone(),
        api_key: None,
    };

    let spinner = output::new_spinner("Contacting server...");
    let resp = client.export_workspace(req).await?;
    spinner.finish_and_clear();

    let new_ws = workspace::conversion::from_export_response(
        &wdir,
        ws.config.clone(),
        ws.ws_config.clone(),
        ws.meta.as_ref(),
        resp,
    );
    workspace::save(&new_ws)?;

    output::print_ok(&format!(
        "Exported {} elements and {} connectors to {}",
        new_ws.elements.len(),
        new_ws.connectors.len(),
        wdir
    ));

    Ok(())
}
