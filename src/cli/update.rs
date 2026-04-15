use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::{Args, Subcommand};

#[derive(Args, Debug, Clone)]
pub struct UpdateArgs {
    #[command(subcommand)]
    pub resource: UpdateResource,
}

#[derive(Subcommand, Debug, Clone)]
pub enum UpdateResource {
    /// Update an element field.
    /// Valid fields: name, kind, technology, description, url, repo, branch, language, file_path, symbol, has_view, view_label
    Element {
        /// Element ref to update
        r#ref: String,
        /// Field name to update
        field: Option<String>,
        /// New value
        value: Option<String>,
    },
    /// Update a connector field.
    /// Valid fields: view, source, target, label, description, relationship, direction, style, url
    Connector {
        /// Connector canonical ref (view:source:target:label)
        r#ref: String,
        /// Field name to update
        field: Option<String>,
        /// New value
        value: Option<String>,
    },
}

#[expect(clippy::needless_pass_by_value)]
pub fn exec(args: UpdateArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;

    match args.resource {
        UpdateResource::Element {
            r#ref,
            field,
            value,
        } => {
            if let (Some(f), Some(v)) = (field, value) {
                ws.update_element_field(&r#ref, &f, &v)?;
                workspace::save(&ws)?;
                output::print_ok(&format!("Updated element '{ref}' field '{f}' to '{v}'"));
            } else {
                let el = ws
                    .elements
                    .get(&r#ref)
                    .ok_or_else(|| TldError::Generic(format!("Element '{ref}' not found")))?;
                output::print_info(&format!("Available fields for element '{ref}':"));
                output::print_kv_table(vec![
                    ("name", el.name.clone()),
                    ("kind", el.kind.clone()),
                    ("technology", el.technology.clone()),
                    ("description", el.description.clone()),
                    ("url", el.url.clone()),
                    ("repo", el.repo.clone()),
                    ("branch", el.branch.clone()),
                    ("language", el.language.clone()),
                    ("file_path", el.file_path.clone()),
                    ("symbol", el.symbol.clone()),
                    ("has_view", el.has_view.to_string()),
                    ("view_label", el.view_label.clone()),
                ]);
            }
        }
        UpdateResource::Connector {
            r#ref,
            field,
            value,
        } => {
            if let (Some(f), Some(v)) = (field, value) {
                ws.update_connector_field(&r#ref, &f, &v)?;
                workspace::save(&ws)?;
                output::print_ok(&format!("Updated connector '{ref}' field '{f}' to '{v}'"));
            } else {
                let conn = ws
                    .connectors
                    .get(&r#ref)
                    .ok_or_else(|| TldError::Generic(format!("Connector '{ref}' not found")))?;
                output::print_info(&format!("Available fields for connector '{ref}':"));
                output::print_kv_table(vec![
                    ("view", conn.view.clone()),
                    ("source", conn.source.clone()),
                    ("target", conn.target.clone()),
                    ("label", conn.label.clone()),
                    ("description", conn.description.clone()),
                    ("relationship", conn.relationship.clone()),
                    ("direction", conn.direction.clone()),
                    ("style", conn.style.clone()),
                    ("url", conn.url.clone()),
                ]);
            }
        }
    }

    output::print_info("Change recorded locally. Run 'tld apply' to push to cloud.");
    Ok(())
}
