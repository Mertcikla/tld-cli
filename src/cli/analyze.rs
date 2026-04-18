use crate::analyzer::projection;
use crate::analyzer::projection::collapse::CollapseConfig;
use crate::analyzer::projection::tags::AutoTagOptions;
use crate::analyzer::{AnalysisResult, Rules, Service, TreeSitterService};
use crate::error::TldError;
use crate::output;
use crate::workspace;
use crate::workspace::workspace_builder::BuildContext;
use clap::Args;
use indicatif::ProgressBar;
use std::path::{Path, PathBuf};

/// Patterns excluded from analysis by default.
///
/// Matched by `Rules::should_ignore_path` (see `analyzer::ignore`), which does
/// both whole-path glob matching and per-segment matching. Directory patterns
/// end with `/` (matched against any path segment); file patterns are globs
/// (matched against base names).
const DEFAULT_EXCLUDES: &[&str] = &[
    // Build / tooling directories
    "target/",
    ".git/",
    "node_modules/",
    "build/",
    ".tld/",
    ".claude/",
    "workdir/",
    "dist/",
    "out/",
    "vendor/",
    ".next/",
    ".nuxt/",
    ".svelte-kit/",
    "coverage/",
    ".cache/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".venv/",
    "venv/",
    "__pycache__/",
    ".gradle/",
    ".idea/",
    ".vscode/",
    // Test directories
    "tests/",
    "test/",
    "__tests__/",
    "spec/",
    "specs/",
    "fixtures/",
    "testdata/",
    "mocks/",
    "__mocks__/",
    "e2e/",
    "cypress/",
    // Migrations / generated schemas
    "migrations/",
    "alembic/",
    "migrate/",
    // Static assets / docs
    "static/",
    "staticfiles/",
    "public/",
    "assets/",
    "docs/",
    "templates/",
    // Minified / generated / declaration file globs
    "*.min.js",
    "*.min.css",
    "*.bundle.js",
    "*.d.ts",
    "*.pb.go",
    "*_pb2.py",
    "*_pb2_grpc.py",
    "*.generated.*",
    "generated/",
    // Test files by naming convention
    "*_test.go",
    "*.test.ts",
    "*.test.tsx",
    "*.test.js",
    "*.test.jsx",
    "*.spec.ts",
    "*.spec.tsx",
    "*.spec.js",
    "*.spec.jsx",
    "test_*.py",
    "*_test.py",
    "tests.py",
    "conftest.py",
    "*_spec.py",
    "spec_*.py",
    "*.gen.go",
];

const HARD_MAX_ELEMENTS: usize = 10_000;
const DEFAULT_TARGET_ELEMENTS: usize = HARD_MAX_ELEMENTS;
const LSP_AUTO_MAX_PENDING_CALLS: usize = 5_000;
const LSP_AUTO_MAX_SYMBOLS: usize = 8_000;
const LSP_AUTO_MAX_FILES: usize = 1_500;

#[derive(Args, Debug, Clone)]
pub struct AnalyzeArgs {
    /// Path to analyze (file or directory)
    pub path: String,

    /// Print what would be written without modifying workspace
    #[arg(long = "dry-run", default_value = "false")]
    pub dry_run: bool,

    /// Download tree-sitter parsers for specific languages before analyzing.
    /// Accepts a comma-separated list: --download rust,python
    #[arg(long = "download")]
    pub download: Option<String>,

    /// Salience threshold for structural view noise pruning.
    /// Symbol elements with scores at or below this value are hidden.
    /// Default -4 keeps almost everything and only removes absolute low-signal noise.
    #[arg(long = "noise-threshold", default_value = "-4")]
    pub noise_threshold: i32,

    /// Include low-signal nodes that would normally be pruned
    #[arg(long = "include-low-signal", default_value = "false")]
    pub include_low_signal: bool,

    /// Auto-assign semantic tags to analyzed elements: none | all | csv dimensions
    ///
    /// Supported dimensions: role, domain, endpoint, external, signal
    #[arg(long = "auto-tag")]
    pub auto_tag: Option<String>,

    /// LSP enrichment mode: auto | on | off
    #[arg(long = "lsp", default_value = "auto")]
    pub lsp: String,

    /// Target number of visible elements after auto-collapse.
    ///
    /// The analyzer enforces a hard maximum of 10000 elements regardless of this value.
    #[arg(long = "max-elements", default_value_t = DEFAULT_TARGET_ELEMENTS)]
    pub max_elements: usize,
}

#[expect(clippy::print_stdout, clippy::print_stderr)]
pub async fn exec(args: AnalyzeArgs, wdir: String, verbose: bool) -> Result<(), TldError> {
    if args.max_elements == 0 {
        return Err(TldError::Generic(
            "--max-elements must be greater than 0".to_string(),
        ));
    }
    if args.max_elements > HARD_MAX_ELEMENTS {
        return Err(TldError::Generic(format!(
            "--max-elements must be <= {HARD_MAX_ELEMENTS}"
        )));
    }

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

    // ── Parse options ─────────────────────────────────────────────────────────
    let lsp_mode = LspMode::from_str(&args.lsp).ok_or_else(|| {
        TldError::Generic(format!(
            "Unknown LSP mode '{}'. Valid values: auto, on, off",
            args.lsp
        ))
    })?;

    // ── Load workspace config ─────────────────────────────────────────────────
    let ws = workspace::load(&wdir)?;
    let scan_path = args.path.clone();
    let auto_tag_opts = args
        .auto_tag
        .as_deref()
        .or_else(|| {
            ws.ws_config
                .as_ref()
                .and_then(|config| config.auto_tag.as_deref())
        })
        .map_or_else(AutoTagOptions::default_set, AutoTagOptions::parse);

    let abs_scan_path = Path::new(&scan_path)
        .canonicalize()
        .map_err(|e| TldError::Generic(format!("Failed to resolve path {scan_path}: {e}")))?;
    let effective_scan_root = if abs_scan_path.is_file() {
        abs_scan_path
            .parent()
            .map_or_else(|| abs_scan_path.clone(), Path::to_path_buf)
    } else {
        abs_scan_path.clone()
    };

    output::print_info(&format!("Analyzing {scan_path}..."));

    let mut exclude = ws
        .ws_config
        .as_ref()
        .map(|c| c.exclude.clone())
        .unwrap_or_default();

    for default_exclude in DEFAULT_EXCLUDES {
        let de = default_exclude.to_string();
        if !exclude.contains(&de) {
            exclude.push(de);
        }
    }

    // ── Run tree-sitter analysis ──────────────────────────────────────────────
    let analyzer_service = TreeSitterService::new();
    let scan_rules = Rules::new(exclude);
    let scan_total =
        TreeSitterService::count_path(abs_scan_path.to_str().unwrap_or(""), &scan_rules)?;
    let scan_progress = progress_bar_for_total(scan_total, "Scanning files...");

    let progress = scan_progress.clone();
    let mut result = match analyzer_service.extract_path(
        abs_scan_path.to_str().unwrap_or(""),
        &scan_rules,
        Some(&|path, is_dir| {
            if !is_dir {
                update_progress_message(&progress, path, "Scanning");
                progress.inc(1);
            }
        }),
    ) {
        Ok(result) => result,
        Err(TldError::ParserDownloadRequired { ref lang, .. }) => {
            scan_progress.finish_and_clear();
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
                        scan_progress.set_position(0);
                        analyzer_service.extract_path(
                            abs_scan_path.to_str().unwrap_or(""),
                            &scan_rules,
                            Some(&|path, is_dir| {
                                if !is_dir {
                                    update_progress_message(&progress, path, "Scanning");
                                    progress.inc(1);
                                }
                            }),
                        )?
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
            scan_progress.finish_and_clear();
            return Err(e);
        }
    };

    scan_progress.finish_and_clear();

    // ── Dry-run reporting ─────────────────────────────────────────────────────
    if args.dry_run {
        output::print_info(&format!(
            "Dry run: {} files scanned, {} symbols found, effective root {}",
            result.files_scanned.len(),
            result.symbols.len(),
            effective_scan_root.to_string_lossy()
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

    // ── LSP enrichment ───────────────────────────────────────────────────────
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
        let pending_lsp_lookups = result
            .refs
            .iter()
            .filter(|r| r.kind == "call" && r.target_path.is_empty())
            .count();
        match lsp_decision(lsp_mode, &result, pending_lsp_lookups) {
            LspDecision::Run => {
                let lsp_progress = progress_bar_for_total(
                    pending_lsp_lookups as u64,
                    "Running LSP definition lookup...",
                );
                match crate::analyzer::lsp::resolve_calls_with_lsp(
                    &mut result.refs,
                    effective_scan_root.to_str().unwrap_or(""),
                    &unique_langs,
                    &lsp_progress,
                )
                .await
                {
                    Ok(()) => {
                        lsp_progress.finish_and_clear();
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
                        lsp_progress.finish_and_clear();
                        output::print_warn(&format!(
                            "LSP enrichment unavailable ({e}); analysis accuracy may drop, but continuing."
                        ));
                    }
                }
            }
            LspDecision::Skip(reason) => {
                output::print_info(&format!("Skipping LSP enrichment ({reason})."));
            }
        }
    }

    // ── Build workspace output ────────────────────────────────────────────────
    let scan_root = effective_scan_root.to_string_lossy().to_string();
    let repo_identity = derive_repo_identity(&ws, &abs_scan_path, Some(&scan_root));
    let branch = detect_git_branch(&effective_scan_root).unwrap_or_else(|| "main".to_string());

    let ctx = BuildContext {
        repo_name: repo_identity.name,
        branch,
        owner: repo_identity.owner,
        repo_url: repo_identity.remote_url,
        scan_root,
    };

    let noise_threshold = if args.include_low_signal {
        i32::MIN
    } else {
        args.noise_threshold
    };
    let collapse_config = CollapseConfig::new(args.max_elements, HARD_MAX_ELEMENTS);

    let (build_output, stats) = projection::structural::project_with_threshold(
        &result,
        &ctx,
        auto_tag_opts,
        collapse_config,
        noise_threshold,
    );
    if verbose {
        print_salience_scores(&stats.salience_entries, noise_threshold);
    }
    let stats_msg = format!(
        "{} elements written, {} connectors created, {} low-signal symbols hidden ({} symbols scored; structural map).",
        build_output.elements.len(),
        build_output.connectors.len(),
        stats.symbols_pruned,
        stats.symbols_scored,
    );

    // ── Persist to workspace ──────────────────────────────────────────────────
    // analyze is the sole producer of derived elements/connectors, so replace
    // both collections wholesale — merging leaves stale entries whenever source
    // code renames, deletes, or prunes a symbol.
    let mut ws = workspace::load(&wdir)?;
    ws.elements.clear();
    ws.connectors.clear();
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LspMode {
    Auto,
    On,
    Off,
}

impl LspMode {
    fn from_str(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "on" => Some(Self::On),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

enum LspDecision {
    Run,
    Skip(String),
}

fn lsp_decision(mode: LspMode, result: &AnalysisResult, pending_lsp_lookups: usize) -> LspDecision {
    match mode {
        LspMode::On => {
            if pending_lsp_lookups == 0 {
                LspDecision::Skip("no unresolved call sites".to_string())
            } else {
                LspDecision::Run
            }
        }
        LspMode::Off => LspDecision::Skip("disabled by --lsp off".to_string()),
        LspMode::Auto => {
            if pending_lsp_lookups == 0 {
                return LspDecision::Skip("no unresolved call sites".to_string());
            }
            if pending_lsp_lookups > LSP_AUTO_MAX_PENDING_CALLS {
                return LspDecision::Skip(format!(
                    "auto mode budget exceeded: {pending_lsp_lookups} pending calls"
                ));
            }
            if result.symbols.len() > LSP_AUTO_MAX_SYMBOLS {
                return LspDecision::Skip(format!(
                    "auto mode budget exceeded: {} symbols",
                    result.symbols.len()
                ));
            }
            if result.files_scanned.len() > LSP_AUTO_MAX_FILES {
                return LspDecision::Skip(format!(
                    "auto mode budget exceeded: {} files",
                    result.files_scanned.len()
                ));
            }
            LspDecision::Run
        }
    }
}

fn print_salience_scores(entries: &[projection::structural::SalienceEntry], noise_threshold: i32) {
    output::print_info(&format!(
        "Salience scores ({} symbols, pruning <= {noise_threshold}):",
        entries.len()
    ));
    let mut rows = entries.to_vec();
    rows.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.file_path.cmp(&b.file_path))
            .then_with(|| a.name.cmp(&b.name))
    });
    for row in rows {
        let status = if row.kept { "keep" } else { "drop" };
        output::print_info(&format!(
            "    {score:>3} {status:>4}  {file}::{name}",
            score = row.score,
            file = row.file_path,
            name = row.name
        ));
    }
}

#[derive(Debug, Clone)]
struct RepoIdentity {
    name: String,
    owner: String,
    remote_url: Option<String>,
}

fn derive_repo_identity(
    ws: &workspace::Workspace,
    scan_path: &Path,
    effective_root: Option<&str>,
) -> RepoIdentity {
    let configured_name = ws
        .ws_config
        .as_ref()
        .map(|c| c.project_name.trim().to_string())
        .filter(|s| !s.is_empty());

    let git_root = find_git_root_from(scan_path)
        .or_else(|| effective_root.map(Path::new).and_then(find_git_root_from));

    let fallback_name = if scan_path.is_dir() {
        scan_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("codebase")
            .to_string()
    } else {
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
    };

    let git_root_name = git_root
        .as_ref()
        .and_then(|root| root.file_name())
        .and_then(|s| s.to_str())
        .map(ToString::to_string);

    let name = configured_name.or(git_root_name).unwrap_or(fallback_name);

    let remote_url = git_root
        .as_ref()
        .and_then(|root| detect_git_remote_url(root));

    RepoIdentity {
        owner: name.clone(),
        name,
        remote_url,
    }
}

fn find_git_root_from(path: &Path) -> Option<PathBuf> {
    let dir = if path.is_file() { path.parent()? } else { path };

    let marker = dir.join(".git");
    if marker.exists() {
        Some(dir.to_path_buf())
    } else {
        None
    }
}

fn detect_git_remote_url(path: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let remote = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if remote.is_empty() {
        None
    } else {
        Some(remote)
    }
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

fn progress_bar_for_total(total: u64, msg: &str) -> ProgressBar {
    if total > 0 {
        output::new_progress_bar(total, msg)
    } else {
        output::new_spinner(msg)
    }
}

fn update_progress_message(progress: &ProgressBar, path: &str, action: &str) {
    progress.set_message(format!("{action} {path}"));
}

#[cfg(test)]
mod tests {
    use super::{derive_repo_identity, find_git_root_from};
    use crate::workspace::types::Workspace;

    #[test]
    fn repo_identity_uses_directory_name_when_not_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let project_dir = dir.path().join("digitaltwin-poc");
        std::fs::create_dir_all(&project_dir).expect("create project dir");

        let ws = Workspace::default();
        let identity = derive_repo_identity(&ws, &project_dir, None);

        assert_eq!(identity.name, "digitaltwin-poc");
        assert_eq!(identity.owner, "digitaltwin-poc");
    }

    #[test]
    fn find_git_root_respects_nearest_git_marker() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path().join("digitaltwin-poc");
        let nested = repo.join("src/ui");
        std::fs::create_dir_all(&nested).expect("create nested");
        std::fs::create_dir_all(repo.join(".git")).expect("create git marker");

        let detected_nested = find_git_root_from(&nested);
        assert!(detected_nested.is_none());

        let detected_repo = find_git_root_from(&repo).expect("git root should be found");
        assert_eq!(detected_repo, repo);
    }
}
