use clap::Args;
use std::path::{Path};
use crate::error::TldError;
use crate::output;
use crate::workspace;
use crate::analyzer::{Service, TreeSitterService, Rules};

#[derive(Args, Debug, Clone)]
pub struct AnalyzeArgs {
    /// Path to analyze (file or directory)
    pub path: String,
    /// Scan entire git repo for cross-file call references (slower)
    #[arg(long, default_value = "false")]
    pub deep: bool,
    /// Print what would be written without modifying workspace
    #[arg(long = "dry-run", default_value = "false")]
    pub dry_run: bool,
    /// Only re-analyse files changed since this git SHA or ref
    #[arg(long = "changed-since")]
    pub changed_since: Option<String>,
}

pub async fn exec(args: AnalyzeArgs, wdir: String) -> Result<(), TldError> {
    let mut ws = workspace::load(&wdir)?;
    let scan_path = args.path.clone();

    let abs_scan_path = Path::new(&scan_path).canonicalize()
        .map_err(|e| TldError::Generic(format!("Failed to resolve path {}: {}", scan_path, e)))?;

    output::print_info(&format!("Analyzing {}...", scan_path));

    let rules = Rules::new(ws.workspace_config.as_ref().map(|c| c.exclude.clone()).unwrap_or_default());
    let analyzer_service = TreeSitterService::new();

    let spinner = output::new_spinner("Scanning files...");
    let result = analyzer_service.extract_path(
        abs_scan_path.to_str().unwrap_or(""),
        &rules,
        Some(&|path, _is_dir| {
            spinner.set_message(format!("Scanning {}", path));
        }),
    )?;
    spinner.finish_and_clear();

    if args.dry_run {
        output::print_info("Dry run: found the following symbols:");
        for sym in &result.symbols {
            println!("  {} ({}) in {}", sym.name, sym.kind, sym.file_path);
        }
        return Ok(());
    }

    let mut new_elements = 0;
    let mut updated_elements = 0;

    for sym in result.symbols {
        let ref_name = workspace::slugify(&sym.name);
        
        // Simple upsert logic
        if let Some(existing) = ws.elements.get_mut(&ref_name) {
            existing.technology = sym.technology;
            existing.kind = sym.kind;
            existing.file_path = sym.file_path;
            existing.symbol = sym.name;
            updated_elements += 1;
        } else {
            ws.elements.insert(ref_name, workspace::Element {
                name: sym.name.clone(),
                kind: sym.kind,
                technology: sym.technology,
                file_path: sym.file_path,
                symbol: sym.name,
                ..Default::default()
            });
            new_elements += 1;
        }
    }

    workspace::save(&ws)?;
    output::print_ok(&format!("Analysis complete. {} new, {} updated elements.", new_elements, updated_elements));

    Ok(())
}
