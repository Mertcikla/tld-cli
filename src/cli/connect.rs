use crate::error::TldError;
use crate::output;
use crate::workspace::{self, Connector};
use clap::Args;

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

#[expect(clippy::needless_pass_by_value)]
pub fn exec(args: ConnectArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;

    // 1. Check existence and suggest
    let check_exists = |name: &str, ws: &workspace::Workspace| -> Result<(), TldError> {
        if !ws.elements.contains_key(name) {
            let mut best_match = None;
            let mut best_score = 0.0;
            for key in ws.elements.keys() {
                let score = strsim::jaro_winkler(name, key);
                if score > 0.8 && score > best_score {
                    best_score = score;
                    best_match = Some(key);
                }
            }

            let msg = if let Some(m) = best_match {
                format!("Element '{name}' not found. Did you mean '{m}'?")
            } else {
                format!("Element '{name}' not found")
            };
            return Err(TldError::Generic(msg));
        }
        Ok(())
    };

    check_exists(&args.source, &ws)?;
    check_exists(&args.target, &ws)?;

    // 2. Warn if > 20 connections
    let conn_count = ws
        .connectors
        .values()
        .filter(|c| {
            c.source == args.source
                || c.target == args.source
                || c.source == args.target
                || c.target == args.target
        })
        .count();
    if conn_count >= 20 {
        output::print_warn(&format!(
            "Element '{}' or '{}' has {} connections. This might affect diagram readability.",
            args.source, args.target, conn_count
        ));
    }

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

    // 3 & 4. Duplicate/Update check
    let ref_name = ws.upsert_connector(connector)?;
    workspace::save(&ws)?;

    output::print_ok(&format!(
        "Processed connector '{ref_name}' in connectors.yaml"
    ));
    output::print_info("Run 'tld apply' to push changes to the server.");

    Ok(())
}
