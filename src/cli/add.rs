use crate::error::TldError;
use crate::output;
use crate::workspace::{self, Element, ViewPlacement};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct AddArgs {
    /// Name of the element to add or update
    pub name: String,
    /// Element kind (e.g. service, database, function)
    #[arg(long)]
    pub kind: Option<String>,
    /// Technology tag
    #[arg(long)]
    pub technology: Option<String>,
    /// Short description of the element
    #[arg(long)]
    pub description: Option<String>,
    /// URL for external documentation or source
    #[arg(long)]
    pub url: Option<String>,
    /// Parent element ref to nest this element under
    #[arg(long)]
    pub parent: Option<String>,
}

pub async fn exec(args: AddArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;
    let ref_name = workspace::slugify(&args.name);

    let mut element = ws
        .elements
        .get(&ref_name)
        .cloned()
        .unwrap_or_else(|| Element {
            name: args.name.clone(),
            ..Default::default()
        });

    if let Some(kind) = args.kind {
        element.kind = kind;
    }
    if let Some(tech) = args.technology {
        element.technology = tech;
    }
    if let Some(desc) = args.description {
        element.description = desc;
    }
    if let Some(url) = args.url {
        element.url = url;
    }

    if let Some(parent) = args.parent {
        // Simple case: overwrite/set single placement.
        // Go's add might have more complex placement logic but this is a good start.
        element.placements = vec![ViewPlacement {
            parent_ref: parent,
            ..Default::default()
        }];
    }

    ws.upsert_element(ref_name.clone(), element)?;
    workspace::save(&ws)?;

    output::print_ok(&format!(
        "Added/updated element '{}' in elements.yaml",
        ref_name
    ));
    output::print_info("Run 'tld apply' to push changes to the server.");

    Ok(())
}
