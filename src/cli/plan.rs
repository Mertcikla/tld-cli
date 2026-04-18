use crate::cli::repository_root;
use crate::client;
use crate::client::diagv1::ApplyPlanResponse;
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::Args;
use std::fmt::Write;
use std::fs;

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

#[expect(clippy::print_stdout)]
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
    if ws.config.org_id.is_empty() {
        return Err(TldError::Generic(
            "No org ID found. Run 'tld login' first, or set TLD_ORG_ID environment variable."
                .to_string(),
        ));
    }

    output::print_info("Building plan...");
    let spinner = output::new_spinner("Contacting server for dry-run...");
    let mut ws_client =
        client::new_workspace_client(&ws.config.server_url, &ws.config.api_key).await?;

    let mut ws = ws;
    let resp = repository_root::run_dry_run_with_repository_root_sync(
        &mut ws,
        &mut ws_client,
        args.recreate_ids,
        true,
    )
    .await?;
    spinner.finish_and_clear();

    output::print_ok("Plan built successfully.");

    let report = render_plan_report(&resp, args.verbose);
    if let Some(output_path) = args.output {
        fs::write(&output_path, report)?;
        output::print_ok(&format!("Plan report written to {output_path}"));
    } else {
        print!("{report}");
    }

    Ok(())
}

fn render_plan_report(resp: &ApplyPlanResponse, verbose: bool) -> String {
    let mut out = String::new();

    // Summary of counts
    if let Some(summary) = &resp.summary {
        out.push_str("Plan Summary:\n");
        let _ = writeln!(out, "  Elements:   {:3} planned", summary.elements_planned);
        let _ = writeln!(out, "  Views:      {:3} planned", summary.views_planned);
        let _ = writeln!(
            out,
            "  Connectors: {:3} planned",
            summary.connectors_planned
        );
    }

    // Proposed Changes Table
    if verbose && (!resp.element_results.is_empty() || !resp.connector_results.is_empty()) {
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
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str("Proposed Changes:\n");
            out.push_str(&Table::new(rows).to_string());
            out.push('\n');
        }
    }

    // Conflicts
    if !resp.conflicts.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        let _ = writeln!(out, "{} conflicts detected!", resp.conflicts.len());
        for conflict in &resp.conflicts {
            let _ = writeln!(
                out,
                "  * {} \"{}\": {}",
                conflict.resource_type, conflict.r#ref, conflict.resolution_hint
            );
        }
    }

    // Drift
    if !resp.drift.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        let _ = writeln!(out, "{} drift items detected.", resp.drift.len());
        for drift in &resp.drift {
            let _ = writeln!(
                out,
                "  * {} \"{}\": {}",
                drift.resource_type, drift.r#ref, drift.reason
            );
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::render_plan_report;
    use crate::client::diagv1::{ApplyPlanResponse, PlanConflictItem, PlanDriftItem, PlanSummary};

    #[test]
    fn render_plan_report_includes_summary_conflicts_and_drift() {
        let resp = ApplyPlanResponse {
            summary: Some(PlanSummary {
                elements_planned: 2,
                views_planned: 1,
                connectors_planned: 3,
                ..Default::default()
            }),
            conflicts: vec![PlanConflictItem {
                resource_type: "element".to_string(),
                r#ref: "backend".to_string(),
                resolution_hint: "pull before apply".to_string(),
                ..Default::default()
            }],
            drift: vec![PlanDriftItem {
                resource_type: "connector".to_string(),
                r#ref: "root:api:db:reads".to_string(),
                reason: "label changed on server".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let text = render_plan_report(&resp, false);

        assert!(text.contains("Plan Summary:"));
        assert!(text.contains("1 conflicts detected!"));
        assert!(text.contains("1 drift items detected."));
        assert!(text.contains("backend"));
        assert!(text.contains("root:api:db:reads"));
    }
}
