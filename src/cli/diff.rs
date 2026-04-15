use crate::client;
use crate::client::diagv1;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::Args;
use std::fs;
use std::process::Command;
use tonic::Request;

#[derive(Args, Debug, Clone)]
pub struct DiffArgs {
    /// Skip symbol verification
    #[arg(long, default_value = "false")]
    pub skip_symbols: bool,
}

#[expect(clippy::expect_used)]
pub async fn exec(_args: DiffArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;

    if ws.config.org_id.is_empty() {
        return Err(TldError::Generic(
            "Org ID required in .tld.yaml".to_string(),
        ));
    }

    output::print_info("Fetching server state for comparison...");

    // 1. Create temp dir
    let temp_dir = std::env::temp_dir().join(format!("tld-diff-{}", std::process::id()));
    fs::create_dir_all(&temp_dir)?;

    let mut client =
        client::new_workspace_client(&ws.config.server_url, &ws.config.api_key).await?;
    let req = Request::new(diagv1::ExportOrganizationRequest {
        org_id: ws.config.org_id.clone(),
        api_key: None,
    });

    let resp = client.export_workspace(req).await?.into_inner();

    let server_ws_dir = temp_dir
        .to_str()
        .expect("temp path should be valid utf8")
        .to_string();
    let server_ws = workspace::conversion::from_export_response(
        &server_ws_dir,
        ws.config.clone(),
        ws.ws_config.clone(),
        ws.meta.as_ref(),
        resp,
    );

    workspace::save(&server_ws)?;

    // 2. Run git diff
    output::print_info("Calculating differences (+ local addition, - server has it)...");

    let mut diff_cmd = Command::new("git");
    diff_cmd.args([
        "diff",
        "--no-index",
        "--color=always",
        &server_ws_dir,
        &wdir,
    ]);

    // We don't check exit status because git diff --no-index returns 1 if differences exist.
    let _ = diff_cmd.status();

    // 3. Cleanup
    let _ = fs::remove_dir_all(&temp_dir);

    Ok(())
}
