use crate::cli::repository_root;
use crate::client;
use crate::error::TldError;
use crate::output;
use crate::planner;
use crate::workspace;
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

    Ok(())
}
