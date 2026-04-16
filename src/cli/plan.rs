use crate::client;
use crate::error::TldError;
use crate::output;
use crate::planner;
use crate::workspace;
use clap::Args;
use tonic::Request;

#[derive(Args, Debug, Clone)]
pub struct PlanArgs {
    /// Write plan to file instead of stdout
    #[arg(short, long)]
    pub output: Option<String>,
    /// Ignore existing resource IDs and let the server generate new ones
    #[arg(long, default_value = "false")]
    pub recreate_ids: bool,
    /// show detailed resource reporting (elements, diagrams, connectors)
    #[arg(short, long, default_value = "false")]
    pub verbose: bool,
}

#[expect(clippy::print_stdout, clippy::items_after_statements)]
pub async fn exec(args: PlanArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;

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

    output::print_info("Building plan...");
    let plan = planner::build(&ws, args.recreate_ids)?;

    let spinner = output::new_spinner("Contacting server for dry-run...");
    let mut ws_client =
        client::new_workspace_client(&ws.config.server_url, &ws.config.api_key).await?;

    let resp = ws_client
        .apply_workspace_plan(Request::new(plan.request))
        .await?
        .into_inner();
    spinner.finish_and_clear();

    output::print_ok("Plan built successfully.");

    // Summary of counts
    if let Some(summary) = &resp.summary {
        println!("\nPlan Summary:");
        println!("  Elements:   {:3} planned", summary.elements_planned);
        println!("  Views:      {:3} planned", summary.views_planned);
        println!("  Connectors: {:3} planned", summary.connectors_planned);
    }

    // Proposed Changes Table
    if !resp.element_results.is_empty() || !resp.connector_results.is_empty() {
        println!("\nProposed Changes:");
        use tabled::{Table, Tabled};

        #[derive(Tabled)]
        struct ChangeRow {
            #[tabled(rename = "Type")]
            resource_type: String,
            #[tabled(rename = "Ref")]
            ref_name: String,
            #[tabled(rename = "Action")]
            action: String,
        }

        let mut rows = Vec::new();
        for res in &resp.element_results {
            rows.push(ChangeRow {
                resource_type: "element".to_string(),
                ref_name: res.r#ref.clone(),
                action: "UPSERT".to_string(),
            });
        }
        for res in &resp.connector_results {
            rows.push(ChangeRow {
                resource_type: "connector".to_string(),
                ref_name: res.r#ref.clone(),
                action: "UPSERT".to_string(),
            });
        }

        if !rows.is_empty() {
            let table = Table::new(rows).to_string();
            println!("{table}");
        }
    }

    // Conflicts
    if !resp.conflicts.is_empty() {
        println!();
        output::print_warn(&format!("{} conflicts detected!", resp.conflicts.len()));
        for conflict in &resp.conflicts {
            println!(
                "  * {} \"{}\": {}",
                conflict.resource_type, conflict.r#ref, conflict.resolution_hint
            );
        }
    }

    // Drift
    if !resp.drift.is_empty() {
        println!();
        output::print_info(&format!("{} drift items detected.", resp.drift.len()));
        for drift in &resp.drift {
            println!(
                "  * {} \"{}\": {}",
                drift.resource_type, drift.r#ref, drift.reason
            );
        }
    }

    Ok(())
}
