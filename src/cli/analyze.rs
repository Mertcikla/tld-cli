use crate::analyzer::projection::{self, ViewMode};
use crate::analyzer::scope;
use crate::analyzer::syntax;
use crate::analyzer::{Rules, Service, TreeSitterService};
use crate::error::TldError;
use crate::output;
use crate::workspace;
use crate::workspace::workspace_builder::BuildContext;
use clap::Args;
use std::path::Path;

#[expect(clippy::struct_excessive_bools)]
#[derive(Args, Debug, Clone)]
pub struct AnalyzeArgs {
    /// Path to analyze (file or directory)
    pub path: String,

    /// Expand scope to the git repository root for cross-file call resolution
    #[arg(long, default_value = "false")]
    pub deep: bool,

    /// Print what would be written without modifying workspace
    #[arg(long = "dry-run", default_value = "false")]
    pub dry_run: bool,

    /// Only re-analyse files changed since this git SHA or branch ref
    #[arg(long = "changed-since")]
    pub changed_since: Option<String>,

    /// Download tree-sitter parsers for specific languages before analyzing.
    /// Accepts a comma-separated list: --download rust,python
    #[arg(long = "download")]
    pub download: Option<String>,

    /// Enable LSP for enhanced cross-file call resolution (requires language server in PATH)
    #[arg(long, default_value = "false")]
    pub lsp: bool,

    /// Diagram view to generate: structural | business | data-flow
    ///
    /// - structural: current inventory-style (files, folders, symbols) — default
    /// - business: semantic projection showing high-salience orchestration symbols only
    /// - data-flow: traces dominant flow paths from entrypoints
    #[arg(long = "view", default_value = "structural")]
    pub view: String,

    /// Salience threshold for business/data-flow views. Symbols with scores at or
    /// below this value are hidden. Default -1 (hides constructors, DTOs, and
    /// trivial wrappers while keeping symbols called by orchestration paths).
    #[arg(long = "noise-threshold", default_value = "-1")]
    pub noise_threshold: i32,

    /// Include low-signal nodes that would normally be pruned (business view only)
    #[arg(long = "include-low-signal", default_value = "false")]
    pub include_low_signal: bool,
}

#[expect(clippy::print_stdout, clippy::print_stderr)]
pub async fn exec(args: AnalyzeArgs, wdir: String) -> Result<(), TldError> {
    // ── Pre-download requested parsers ────────────────────────────────────────
    if let Some(ref langs_csv) = args.download {
        let langs: Vec<&str> = langs_csv.split(',').map(str::trim).collect();
        let spinner = output::new_spinner(&format!("Downloading parsers: {langs_csv}..."));
        match ts_pack_core::download(&langs) {
            Ok(count) => {
                spinner.finish_and_clear();
                if count > 0 {
                    output::print_ok(&format!("Downloaded {count} parser(s)."));
                } else {
                    output::print_info("All requested parsers already cached.");
                }
            }
            Err(e) => {
                spinner.finish_and_clear();
                return Err(TldError::Generic(format!("Parser download failed: {e}")));
            }
        }
    }

    // ── Resolve view mode ─────────────────────────────────────────────────────
    let view_mode = ViewMode::from_str(&args.view).ok_or_else(|| {
        TldError::Generic(format!(
            "Unknown view '{}'. Valid values: structural, business, data-flow",
            args.view
        ))
    })?;

    // ── Load workspace config ─────────────────────────────────────────────────
    let ws = workspace::load(&wdir)?;
    let scan_path = args.path.clone();

    let abs_scan_path = Path::new(&scan_path)
        .canonicalize()
        .map_err(|e| TldError::Generic(format!("Failed to resolve path {scan_path}: {e}")))?;

    output::print_info(&format!("Analyzing {scan_path}..."));

    let mut exclude = ws
        .ws_config
        .as_ref()
        .map(|c| c.exclude.clone())
        .unwrap_or_default();

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

    // ── Build scan plan (scope) ───────────────────────────────────────────────
    let scan_scope = scope::plan(
        abs_scan_path.to_str().unwrap_or(""),
        &scope::PlanOptions {
            workspace_dir: &ws.dir,
            ws_config: ws.ws_config.as_ref(),
            repo_name: &derive_repo_name(&ws, &abs_scan_path),
            deep: args.deep,
            changed_since: args.changed_since.as_deref(),
            exclude: &exclude,
        },
    )?;

    output::print_info(&format!(
        "Scope: root={}, repositories={}, constrained_files={}",
        scan_scope.root_dir,
        scan_scope.repositories.len(),
        scan_scope.files.len()
    ));

    // The effective root may have been expanded to the git repo root by --deep.
    let effective_scan_root = scan_scope.root_dir.clone();

    // ── Run tree-sitter analysis ──────────────────────────────────────────────
    let analyzer_service = TreeSitterService::new();

    let spinner = output::new_spinner("Scanning files...");

    let mut result = if scan_scope.files.is_empty() {
        let mut merged = crate::analyzer::types::AnalysisResult::default();
        for repo in &scan_scope.repositories {
            let repo_rules = Rules::new(repo.exclude.clone());
            match analyzer_service.extract_path(
                &repo.root_dir,
                &repo_rules,
                Some(&|path, _is_dir| {
                    spinner.set_message(format!("Scanning {path}"));
                }),
            ) {
                Ok(r) => merged.merge(r),
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
                    eprint!("Download the '{lang}' parser now? [y/N]: ");
                    let mut input = String::new();
                    std::io::stdin().read_line(&mut input).ok();
                    if input.trim().eq_ignore_ascii_case("y") {
                        let lang_clone = lang.clone();
                        let dl_spinner =
                            output::new_spinner(&format!("Downloading '{lang_clone}' parser..."));
                        match ts_pack_core::download(&[lang_clone.as_str()]) {
                            Ok(_) => {
                                dl_spinner.finish_and_clear();
                                output::print_ok("Download complete. Re-running analysis...");
                                let retry = analyzer_service.extract_path(
                                    &repo.root_dir,
                                    &repo_rules,
                                    None,
                                )?;
                                merged.merge(retry);
                            }
                            Err(e) => {
                                dl_spinner.finish_and_clear();
                                return Err(TldError::Generic(format!("Download failed: {e}")));
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
            }
        }
        merged
    } else {
        // --changed-since: analyze only the listed files.
        let mut merged = crate::analyzer::types::AnalysisResult::default();
        for file in &scan_scope.files {
            spinner.set_message(format!("Scanning {}", file.abs_path));
            match TreeSitterService::extract_file(&file.abs_path) {
                Ok(mut r) => {
                    r.files_scanned.push(file.abs_path.clone());
                    merged.merge(r);
                }
                Err(TldError::UnsupportedLanguage(_) | TldError::ParserNotImplemented(_)) => {}
                Err(e) => {
                    spinner.finish_and_clear();
                    return Err(e);
                }
            }
        }
        merged
    };

    spinner.finish_and_clear();

    // ── Dry-run reporting ─────────────────────────────────────────────────────
    if args.dry_run {
        output::print_info(&format!(
            "Dry run: {} files scanned, {} symbols found, effective root {}",
            result.files_scanned.len(),
            result.symbols.len(),
            effective_scan_root
        ));
        for sym in &result.symbols {
            println!(
                "  {} ({}) in {} [parent: {}]",
                sym.name, sym.kind, sym.file_path, sym.parent
            );
        }
        return Ok(());
    }

    // ── Guard: nothing found ──────────────────────────────────────────────────
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

    // ── Optional LSP enrichment ───────────────────────────────────────────────
    if args.lsp {
        let unique_langs: Vec<&str> = {
            let mut seen = std::collections::HashSet::new();
            result
                .symbols
                .iter()
                .map(|s| s.technology.as_str())
                .filter(|t| !t.is_empty() && seen.insert(*t))
                .collect()
        };
        if !unique_langs.is_empty() {
            let lsp_spinner = output::new_spinner("Running LSP definition lookup...");
            match crate::analyzer::lsp::resolve_calls_with_lsp(
                &mut result.refs,
                &effective_scan_root,
                &unique_langs,
            )
            .await
            {
                Ok(()) => {
                    lsp_spinner.finish_and_clear();
                    let resolved = result
                        .refs
                        .iter()
                        .filter(|r| !r.target_path.is_empty())
                        .count();
                    output::print_info(&format!(
                        "LSP enrichment complete ({resolved} calls resolved)."
                    ));
                }
                Err(e) => {
                    lsp_spinner.finish_and_clear();
                    output::print_info(&format!("LSP enrichment skipped: {e}"));
                }
            }
        }
    }

    let syntax_bundle = match view_mode {
        ViewMode::Structural => None,
        ViewMode::Business | ViewMode::DataFlow => Some(syntax::from_analysis_result(
            &result,
            &repo_name_hint(&ws, &abs_scan_path),
        )),
    };

    // ── Build workspace output via the chosen projection ──────────────────────
    let scan_root = effective_scan_root.clone();
    let repo_name = derive_repo_name(&ws, &abs_scan_path);
    let branch =
        detect_git_branch(Path::new(&effective_scan_root)).unwrap_or_else(|| "main".to_string());

    let ctx = BuildContext {
        repo_name: repo_name.clone(),
        branch,
        owner: repo_name.clone(),
        scan_root,
    };

    let noise_threshold = if args.include_low_signal {
        i32::MIN
    } else {
        args.noise_threshold
    };

    let semantic_syntax = syntax_bundle.as_ref();

    let (build_output, stats_msg) = match view_mode {
        ViewMode::Structural => {
            let out = projection::structural::project(&result, &ctx);
            let msg = format!(
                "{} elements written, {} connectors created (structural view).",
                out.elements.len(),
                out.connectors.len()
            );
            (out, msg)
        }
        ViewMode::Business => {
            let syntax = semantic_syntax.ok_or_else(|| {
                TldError::Generic("semantic views should build a syntax bundle".to_string())
            })?;
            let (out, stats) = projection::business::project(
                syntax,
                &ctx,
                noise_threshold,
            );
            let msg = format!(
                "{} elements written, {} connectors created, {} low-signal symbols hidden, {} unresolved refs, {} resolved call edges ({} via LSP) (business view).",
                out.elements.len(),
                out.connectors.len(),
                stats.symbols_hidden,
                stats.unresolved_refs,
                stats.resolved_call_edges,
                stats.lsp_resolved_edges,
            );
            (out, msg)
        }
        ViewMode::DataFlow => {
            let syntax = semantic_syntax.ok_or_else(|| {
                TldError::Generic("semantic views should build a syntax bundle".to_string())
            })?;
            let (out, stats) = projection::data_flow::project(
                syntax,
                &ctx,
                noise_threshold,
            );
            let msg = format!(
                "{} elements written, {} connectors created, {} flows synthesized, {} low-signal symbols hidden, {} unresolved refs (data-flow view).",
                out.elements.len(),
                out.connectors.len(),
                stats.flow_count,
                stats.symbols_hidden,
                stats.unresolved_refs,
            );
            (out, msg)
        }
    };

    // ── Persist to workspace ──────────────────────────────────────────────────
    let mut ws = workspace::load(&wdir)?;
    for (slug, el) in build_output.elements {
        ws.elements.insert(slug, el);
    }
    for conn in build_output.connectors {
        ws.connectors.insert(conn.resource_ref(), conn);
    }

    workspace::save(&ws)?;
    output::print_ok(&format!("Analysis complete. {stats_msg}"));

    Ok(())
}

fn repo_name_hint(ws: &workspace::Workspace, scan_path: &Path) -> String {
    derive_repo_name(ws, scan_path)
}

fn derive_repo_name(ws: &workspace::Workspace, scan_path: &Path) -> String {
    ws.ws_config
        .as_ref()
        .map(|c| c.project_name.clone())
        .filter(|s: &String| !s.is_empty())
        .unwrap_or_else(|| {
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
