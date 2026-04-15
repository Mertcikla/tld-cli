use crate::error::TldError;
use crate::output::{self};
use crate::workspace;
use clap::Args;
use tabled::Tabled;

#[derive(Args, Debug, Clone)]
pub struct ViewsArgs {
    /// show only views in a specific parent view
    #[arg(long)]
    pub parent: Option<String>,
}

#[derive(Tabled)]
struct ViewRow {
    #[tabled(rename = "Ref")]
    r#ref: String,
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Label")]
    label: String,
}

#[expect(clippy::needless_pass_by_value)]
pub fn exec(args: ViewsArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;

    let mut rows = Vec::new();

    for (ref_name, element) in &ws.elements {
        if !element.has_view {
            continue;
        }

        if let Some(parent) = &args.parent {
            let in_parent = element.placements.iter().any(|p| p.parent_ref == *parent);
            if !in_parent {
                continue;
            }
        }

        rows.push(ViewRow {
            r#ref: ref_name.clone(),
            name: element.name.clone(),
            label: element.view_label.clone(),
        });
    }

    rows.sort_by(|a, b| a.r#ref.cmp(&b.r#ref));

    output::print_table(rows);

    Ok(())
}
