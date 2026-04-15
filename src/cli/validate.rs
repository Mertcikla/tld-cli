use crate::error::TldError;
use crate::output;
use crate::workspace::{self, ValidationOptions};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct ValidateArgs {
    /// Skip symbol validation checks
    #[arg(long = "skip-symbols", default_value = "false")]
    pub skip_symbols: bool,
}

pub async fn exec(args: ValidateArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;

    let opts = ValidationOptions {
        skip_symbols: args.skip_symbols,
    };

    let errs = ws.validate(&opts);

    if errs.is_empty() {
        output::print_ok("Workspace is valid.");
    } else {
        output::print_err(&format!("Workspace has {} validation errors:", errs.len()));
        for err in errs {
            println!("  - {}", err);
        }
        return Err(TldError::Generic("Validation failed".to_string()));
    }

    Ok(())
}
