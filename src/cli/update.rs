use clap::{Args, Subcommand};
use crate::error::TldError;
use crate::workspace;
use crate::output;

#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    #[command(subcommand)]
    pub resource: UpdateResource,
}

#[derive(Subcommand, Debug, Clone)]
pub enum UpdateResource {
    /// Update an element field
    Element {
        /// Element ref to update
        r#ref: String,
        /// Field name (e.g. name, kind, technology, ref)
        field: String,
        /// New value
        value: String,
    },
    /// Update a connector field
    Connector {
        /// Connector canonical ref (view:source:target:label)
        r#ref: String,
        /// Field name (e.g. label, description, relationship)
        field: String,
        /// New value
        value: String,
    },
}

pub async fn exec(args: UpdateArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;

    match args.resource {
        UpdateResource::Element { r#ref, field, value } => {
            ws.update_element_field(&r#ref, &field, &value)?;
            workspace::save(&ws)?;
            output::print_ok(&format!("Updated element '{}' field '{}' to '{}'", r#ref, field, value));
        }
        UpdateResource::Connector { r#ref, field, value } => {
            ws.update_connector_field(&r#ref, &field, &value)?;
            workspace::save(&ws)?;
            output::print_ok(&format!("Updated connector '{}' field '{}' to '{}'", r#ref, field, value));
        }
    }

    output::print_info("Change recorded locally. Run 'tld apply' to push to cloud.");
    Ok(())
}
