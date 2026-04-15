use clap::Args;
use crate::error::TldError;
use crate::workspace;
use crate::output;

#[derive(Args, Debug, Clone)]
pub struct StatusArgs {
    /// show full detailed status
    #[arg(long, default_value = "false")]
    pub long: bool,
}

pub async fn exec(args: StatusArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;
    let lock_file = workspace::load_lock_file(&wdir)?;

    match lock_file {
        Some(lf) => {
            output::print_ok("Workspace is initialized and has sync history.");
            
            let mut pairs: Vec<(&str, String)> = vec![
                ("Version ID", lf.version_id),
                ("Applied By", lf.applied_by),
            ];
            
            if let Some(last) = lf.last_apply {
                pairs.push(("Last Apply", last.to_rfc3339()));
            }

            if args.long {
                pairs.push(("Local Elements", ws.elements.len().to_string()));
                pairs.push(("Local Connectors", ws.connectors.len().to_string()));
            }

            output::print_kv_table(pairs);
        }
        None => {
            output::print_warn("No sync history found. Workspace has not been applied yet.");
            output::print_info("Run 'tld plan' or 'tld apply' to see sync status against server.");
        }
    }

    Ok(())
}
