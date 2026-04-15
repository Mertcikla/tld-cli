use crate::error::TldError;
use crate::output;
use crate::workspace::{self, ValidationOptions};
use clap::Args;

#[derive(Args, Debug, Clone)]
pub struct CheckArgs {
    /// Exit non-zero when outdated diagrams are detected
    #[arg(long, default_value = "false")]
    pub strict: bool,
}

#[expect(clippy::needless_pass_by_value, clippy::print_stdout)]
pub fn exec(args: CheckArgs, wdir: String) -> Result<(), TldError> {
    let ws = workspace::load(&wdir)?;
    let mut all_passed = true;

    // 1. Validation (Schema + Symbols)
    output::print_info("Checking Workspace Validation...");
    let opts = ValidationOptions::default();
    let errs = ws.validate(&opts);
    if errs.is_empty() {
        output::print_ok("PASS  Validation");
    } else {
        output::print_err("FAIL  Validation");
        for err in errs {
            println!("      - {err}");
        }
        all_passed = false;
    }

    // 2. Outdated Diagrams
    output::print_info("Checking for Outdated Diagrams...");
    let outdated = ws.check_outdated();
    if outdated.is_empty() {
        output::print_ok("PASS  Outdated Diagrams");
    } else {
        if args.strict {
            output::print_err("FAIL  Outdated Diagrams");
            all_passed = false;
        } else {
            output::print_warn("WARN  Outdated Diagrams");
        }
        for msg in outdated {
            println!("      - {msg}");
        }
    }

    if !all_passed {
        return Err(TldError::Generic("Check failed".to_string()));
    }

    Ok(())
}
