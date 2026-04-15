use crate::analyzer::{Rules, Service, TreeSitterService};
use crate::error::TldError;
use crate::output;
use crate::workspace;
use crate::workspace::workspace_builder::{self, BuildContext};
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
    /// Enable LSP for enhanced cross-file call resolution (requires language server in PATH)
    #[arg(long, default_value = "false")]
    pub lsp: bool,
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

    let ws = workspace::load(&wdir)?;
    let scan_path = args.path.clone();

    let abs_scan_path = Path::new(&scan_path)
        .canonicalize()
        .map_err(|e| TldError::Generic(format!("Failed to resolve path {}: {}", scan_path, e)))?;

    output::print_info(&format!("Analyzing {}...", scan_path));

    let mut exclude = ws
        .workspace_config
        .as_ref()
        .map(|c| c.exclude.clone())
        .unwrap_or_default();

    // Default sensible excludes
    for default_exclude in &[
        "target/",
        ".git/",
        "node_modules/",
        "build/",
        ".tld/",
        ".claude/",
        "workdir/",
    ] {
        let de = default_exclude.to_string();
        if !exclude.contains(&de) {
            exclude.push(de);
        }
    }

    let rules = Rules::new(exclude);
    let analyzer_service = TreeSitterService::new();

    let spinner = output::new_spinner("Scanning files...");
    let mut result = match analyzer_service.extract_path(
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
        output::print_info(&format!(
            "Dry run: {} files scanned, {} symbols found",
            result.files_scanned.len(),
            result.symbols.len()
        ));
        for sym in &result.symbols {
            println!(
                "  {} ({}) in {} [parent: {}]",
                sym.name, sym.kind, sym.file_path, sym.parent
            );
        }
        return Ok(());
    }

    // Filter files_scanned to only code files (mirrors workspace_builder's should_skip_file).
    let has_code = result.files_scanned.iter().any(|p| {
        let ext = std::path::Path::new(p)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        !matches!(
            ext,
            "lock" | "toml" | "json" | "md" | "txt" | "yaml" | "yml" | "sum" | "mod" | "gitignore"
        )
    });
    if result.symbols.is_empty() && !has_code {
        output::print_info(
            "No symbols or architectural elements were found at the specified path.",
        );
        return Ok(());
    }

    // Optional LSP enrichment for better call resolution.
    if args.lsp {
        let unique_langs: Vec<&str> = {
            let mut seen = std::collections::HashSet::new();
            result.symbols.iter()
                .map(|s| s.technology.as_str())
                .filter(|t| !t.is_empty() && seen.insert(*t))
                .collect()
        };
        if !unique_langs.is_empty() {
            let lsp_spinner = output::new_spinner("Running LSP definition lookup...");
            if let Err(e) = crate::analyzer::lsp::resolve_calls_with_lsp(
                &mut result.refs,
                abs_scan_path.to_str().unwrap_or(""),
                &unique_langs,
            )
            .await
            {
                lsp_spinner.finish_and_clear();
                output::print_info(&format!("LSP enrichment skipped: {}", e));
            } else {
                lsp_spinner.finish_and_clear();
                output::print_info("LSP enrichment complete.");
            }
        }
    }

    // Build workspace elements and connectors from the analysis result.
    let scan_root = abs_scan_path.to_str().unwrap_or("").to_string();
    let repo_name = derive_repo_name(&ws, &abs_scan_path);
    let branch = detect_git_branch(&abs_scan_path).unwrap_or_else(|| "main".to_string());

    let ctx = BuildContext {
        repo_name: repo_name.clone(),
        branch,
        owner: repo_name,
        scan_root,
    };

    let output = workspace_builder::build(&result, &ctx);

    let element_count = output.elements.len();
    let connector_count = output.connectors.len();

    let mut ws = workspace::load(&wdir)?;
    for (slug, el) in output.elements {
        ws.elements.insert(slug, el);
    }
    for conn in output.connectors {
        ws.connectors.insert(conn.resource_ref(), conn);
    }

    workspace::save(&ws)?;
    output::print_ok(&format!(
        "Analysis complete. {} elements written, {} connectors created.",
        element_count, connector_count
    ));

    Ok(())
}

fn derive_repo_name(ws: &workspace::Workspace, scan_path: &Path) -> String {
    ws.workspace_config
        .as_ref()
        .map(|c| c.project_name.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            // Use the grandparent of scan_path if possible, otherwise the basename.
            scan_path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or_else(|| {
                    scan_path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("codebase")
                })
                .to_string()
        })
}

fn detect_git_branch(path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .ok()?;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !branch.is_empty() {
            return Some(branch);
        }
    }
    None
}
