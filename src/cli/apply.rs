use clap::Args;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use crate::planner;
use crate::client;
use tonic::Request;

#[derive(Args, Debug, Clone)]
pub struct ApplyArgs {
    /// Skip the interactive approval prompt
    #[arg(short = 'f', long = "force", default_value = "false")]
    pub force: bool,
    /// Ignore existing resource IDs and let the server generate new ones
    #[arg(long, default_value = "false")]
    pub recreate_ids: bool,
}

pub async fn exec(args: ApplyArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;
    
    // Check for API key
    if ws.config.api_key.is_empty() {
        return Err(TldError::Generic("No API key found. Run 'tld login' first.".to_string()));
    }

    // 1. Build and show plan first (unless force is used)
    // For now we'll just go straight to apply if force is on, or do a simplified flow.
    
    output::print_info("Applying workspace to tlDiagram...");
    let mut plan = planner::build(&ws, args.recreate_ids)?;
    plan.request.dry_run = Some(false); // Actually apply

    let spinner = output::new_spinner("Contacting server...");
    let mut ws_client = client::new_workspace_client(&ws.config.server_url, &ws.config.api_key).await?;

    let resp = ws_client.apply_workspace_plan(Request::new(plan.request)).await?
        .into_inner();
    spinner.finish_and_clear();

    if let Some(summary) = &resp.summary {
        output::print_ok(&format!("Applied successfully! {} elements, {} views, {} connectors updated.", 
            summary.elements_planned, summary.views_planned, summary.connectors_planned));
    } else {
        output::print_ok("Applied successfully.");
    }

    if let Some(version) = &resp.version {
        println!("New workspace version: {}", version.version_id);
    }

    // Conflicts notification
    if !resp.conflicts.is_empty() {
        output::print_warn(&format!("Warning: {} conflicts were detected during apply.", resp.conflicts.len()));
    }

    Ok(())
}
