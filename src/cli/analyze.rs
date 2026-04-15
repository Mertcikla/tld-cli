use crate::analyzer::{Rules, Service, TreeSitterService};
use crate::error::TldError;
use crate::output;
use crate::workspace;
use clap::Args;
use std::path::Path;

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
    /// Download tree-sitter parsers for specific languages before analyzing.
    /// Accepts a comma-separated list: --download rust,python
    #[arg(long = "download")]
    pub download: Option<String>,
}

pub async fn exec(args: AnalyzeArgs, wdir: String) -> Result<(), TldError> {
    // Pre-download requested parsers before doing any analysis.
    if let Some(ref langs_csv) = args.download {
        let langs: Vec<&str> = langs_csv.split(',').map(str::trim).collect();
        let spinner = output::new_spinner(&format!("Downloading parsers: {}...", langs_csv));
        match ts_pack_core::download(&langs) {
            Ok(count) => {
                spinner.finish_and_clear();
                if count > 0 {
                    output::print_ok(&format!("Downloaded {} parser(s).", count));
                } else {
                    output::print_info("All requested parsers already cached.");
                }
            }
            Err(e) => {
                spinner.finish_and_clear();
                return Err(TldError::Generic(format!("Parser download failed: {}", e)));
            }
        }
    }

    let mut ws = workspace::load(&wdir)?;
    let scan_path = args.path.clone();

    let abs_scan_path = Path::new(&scan_path)
        .canonicalize()
        .map_err(|e| TldError::Generic(format!("Failed to resolve path {}: {}", scan_path, e)))?;

    output::print_info(&format!("Analyzing {}...", scan_path));

    let rules = Rules::new(
        ws.workspace_config
            .as_ref()
            .map(|c| c.exclude.clone())
            .unwrap_or_default(),
    );
    let analyzer_service = TreeSitterService::new();

    let spinner = output::new_spinner("Scanning files...");
    let result = match analyzer_service.extract_path(
        abs_scan_path.to_str().unwrap_or(""),
        &rules,
        Some(&|path, _is_dir| {
            spinner.set_message(format!("Scanning {}", path));
        }),
    ) {
        Ok(r) => r,
        Err(TldError::ParserDownloadRequired { ref lang, .. }) => {
            spinner.finish_and_clear();
            eprintln!(
                "{}",
                TldError::ParserDownloadRequired {
                    lang: lang.clone(),
                    reason: "grammar not in local cache".to_string(),
                }
            );
            eprintln!();
            // Prompt user interactively.
            eprint!("Download the '{}' parser now? [y/N]: ", lang);
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            if input.trim().eq_ignore_ascii_case("y") {
                let lang_clone = lang.clone();
                let dl_spinner =
                    output::new_spinner(&format!("Downloading '{}' parser...", lang_clone));
                match ts_pack_core::download(&[lang_clone.as_str()]) {
                    Ok(_) => {
                        dl_spinner.finish_and_clear();
                        output::print_ok("Download complete. Re-running analysis...");
                        // Retry once after download.
                        analyzer_service.extract_path(
                            abs_scan_path.to_str().unwrap_or(""),
                            &rules,
                            None,
                        )?
                    }
                    Err(e) => {
                        dl_spinner.finish_and_clear();
                        return Err(TldError::Generic(format!("Download failed: {}", e)));
                    }
                }
            } else {
                return Err(TldError::Generic("Analysis aborted.".to_string()));
            }
        }
        Err(e) => {
            spinner.finish_and_clear();
            return Err(e);
        }
    };
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
            ws.elements.insert(
                ref_name,
                workspace::Element {
                    name: sym.name.clone(),
                    kind: sym.kind,
                    technology: sym.technology,
                    file_path: sym.file_path,
                    symbol: sym.name,
                    ..Default::default()
                },
            );
            new_elements += 1;
        }
    }

    workspace::save(&ws)?;
    output::print_ok(&format!(
        "Analysis complete. {} new, {} updated elements.",
        new_elements, updated_elements
    ));

    Ok(())
}
