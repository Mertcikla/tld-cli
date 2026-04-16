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
    /// Tag to apply to this element (can be specified multiple times)
    #[arg(long = "tag")]
    pub tags: Vec<String>,
}

#[expect(clippy::needless_pass_by_value)]
pub fn exec(args: AddArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;
    let ref_name = workspace::slugify(&args.name);

    let existing = ws.elements.get(&ref_name).cloned();
    let mut element = existing.clone().unwrap_or_else(|| Element {
        name: args.name.clone(),
        ..Default::default()
    });

    // Conflict detection helper
    let check_conflict = |field_name: &str,
                          new_val: &Option<String>,
                          old_val: &str|
     -> Result<(), TldError> {
        if let Some(new) = new_val
            && !old_val.is_empty()
            && old_val != new
        {
            return Err(TldError::Generic(format!(
                "Conflict: field '{field_name}' for element '{ref_name}' is already set to '{old_val}'. Use 'tld update' to change it."
            )));
        }
        Ok(())
    };

    if existing.is_some() {
        check_conflict("kind", &args.kind, &element.kind)?;
        check_conflict("technology", &args.technology, &element.technology)?;
        check_conflict("description", &args.description, &element.description)?;
        check_conflict("url", &args.url, &element.url)?;
    }

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
        // For parent, we check if it's already set in placements
        let has_different_parent = element
            .placements
            .iter()
            .any(|p| !p.parent_ref.is_empty() && p.parent_ref != parent);
        if has_different_parent && existing.is_some() {
            return Err(TldError::Generic(format!(
                "Conflict: element '{ref_name}' already has a different parent. Use 'tld update' to change it."
            )));
        }

        element.placements = vec![ViewPlacement {
            parent_ref: parent,
            ..Default::default()
        }];
    }

    // Validation
    if element.name.trim().is_empty() {
        return Err(TldError::Generic(
            "Element name cannot be empty".to_string(),
        ));
    }

    // Apply tags (upsert: add if not already present)
    for tag in &args.tags {
        let tag = tag.trim().to_string();
        if !tag.is_empty() && !element.tags.contains(&tag) {
            element.tags.push(tag);
        }
    }

    ws.upsert_element(ref_name.clone(), element);
    workspace::save(&ws)?;

    output::print_ok(&format!(
        "Added/updated element '{ref_name}' in elements.yaml"
    ));
    output::print_info("Run 'tld apply' to push changes to the server.");

    Ok(())
}
