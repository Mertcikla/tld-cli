use clap::Args;
use crate::error::TldError;
use crate::workspace::{self, Connector};
use crate::output;

#[derive(Args, Debug, Clone)]
pub struct ConnectArgs {
    /// Source element ref
    pub source: String,
    /// Target element ref
    pub target: String,
    /// Parent view ref for the connector (inferred if missing)
    #[arg(long)]
    pub view: Option<String>,
    /// Optional label for the connector
    #[arg(long)]
    pub label: Option<String>,
    /// Relationship type (e.g. uses, calls, depends-on)
    #[arg(long)]
    pub relationship: Option<String>,
    /// Direction of the connector (forward, backward, both, none)
    #[arg(long, default_value = "forward")]
    pub direction: String,
    /// Visual style (bezier, straight, step, smoothstep)
    #[arg(long, default_value = "bezier")]
    pub style: String,
}

pub async fn exec(args: ConnectArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;

    let connector = Connector {
        source: args.source,
        target: args.target,
        view: args.view.unwrap_or_default(),
        label: args.label.unwrap_or_default(),
        relationship: args.relationship.unwrap_or_default(),
        direction: args.direction,
        style: args.style,
        ..Default::default()
    };

    let ref_name = ws.append_connector(connector)?;
    workspace::save(&ws)?;

    output::print_ok(&format!("Appended connector '{}' to connectors.yaml", ref_name));
    output::print_info("Run 'tld apply' to push changes to the server.");

    Ok(())
}
