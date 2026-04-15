use clap::{Args, Subcommand};
use crate::error::TldError;
use crate::workspace;
use crate::output;

#[derive(Args, Debug, Clone)]
pub struct RemoveArgs {
    #[command(subcommand)]
    pub resource: RemoveResource,
}

#[derive(Subcommand, Debug, Clone)]
pub enum RemoveResource {
    /// Remove an element from elements.yaml
    Element {
        /// Element ref to remove
        r#ref: String,
    },
    /// Remove a connector from connectors.yaml
    Connector {
        /// Parent view ref (required)
        #[arg(long)]
        view: String,
        /// Source element ref (required)
        #[arg(long, rename_all = "lowercase")] // Go uses --from
        from: String,
        /// Target element ref (required)
        #[arg(long)]
        to: String,
    },
}

pub async fn exec(args: RemoveArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;

    match args.resource {
        RemoveResource::Element { r#ref } => {
            ws.remove_element(&r#ref)?;
            workspace::save(&ws)?;
            output::print_ok(&format!("Removed element '{}' and its associations", r#ref));
        }
        RemoveResource::Connector { view, from, to } => {
            let count = ws.remove_connector(&view, &from, &to)?;
            if count > 0 {
                workspace::save(&ws)?;
                output::print_ok(&format!("Removed {} connector(s) matching coordinates", count));
            } else {
                output::print_warn("No matching connectors found - nothing removed.");
            }
        }
    }

    output::print_info("Run 'tld apply' to push changes to the server.");
    Ok(())
}
