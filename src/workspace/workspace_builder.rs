use crate::analyzer::types::AnalysisResult;
use crate::workspace::{
    slugify,
    types::{Connector, Element, ViewPlacement},
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

pub struct BuildContext {
    /// Project name (from .tld.yaml or dir basename). Used as the repository element name.
    pub repo_name: String,
    /// Git branch (or "main" fallback).
    pub branch: String,
    /// Owner — same as repo_name for local workspaces.
    pub owner: String,
    /// Absolute path of the directory that was analyzed.
    pub scan_root: String,
}

pub struct BuildOutput {
    pub elements: HashMap<String, Element>,
    pub connectors: Vec<Connector>,
}

/// Convert an `AnalysisResult` into workspace elements and connectors.
pub fn build(result: &AnalysisResult, ctx: &BuildContext) -> BuildOutput {
    let mut elements: HashMap<String, Element> = HashMap::new();
    let mut connectors: Vec<Connector> = Vec::new();

    // --- Phase 1: collect all relative file paths ---
    // Relative from scan_root's *parent*, so the top-level dir name appears in the path.
    // E.g.: scan_root = /repo/tests/test-codebase/go
    //       file abs  = /repo/tests/test-codebase/go/internal/service/order_service.go
    //       rel       = go/internal/service/order_service.go
    let scan_parent = Path::new(&ctx.scan_root)
        .parent()
        .map_or("", |p| p.to_str().unwrap_or(""))
        .to_string();

    // Deduplicated relative file paths (only source-code files we might analyze).
    let mut file_rel_paths: Vec<String> = Vec::new();
    let mut abs_to_rel: HashMap<String, String> = HashMap::new();
    let mut seen_files: HashSet<String> = HashSet::new();

    for abs_path in &result.files_scanned {
        let rel = rel_from_base(abs_path, &scan_parent);
        // Skip non-code files (no extension or common non-code files).
        if should_skip_file(&rel) {
            continue;
        }
        if seen_files.insert(rel.clone()) {
            abs_to_rel.insert(abs_path.clone(), rel.clone());
            file_rel_paths.push(rel);
        }
    }

    // Also ensure files referenced by symbols are included (in case they weren't in files_scanned).
    for sym in &result.symbols {
        let rel = rel_from_base(&sym.file_path, &scan_parent);
        if seen_files.insert(rel.clone()) {
            abs_to_rel.insert(sym.file_path.clone(), rel.clone());
            file_rel_paths.push(rel);
        }
    }

    // --- Phase 2: build folder tree ---
    let folder_rel_paths = collect_folder_rel_paths(&file_rel_paths);

    // Assign slugs, handling collisions with numeric suffixes.
    let mut folder_slug_by_path: HashMap<String, String> = HashMap::new();
    let mut used_folder_slugs: HashMap<String, String> = HashMap::new(); // slug -> first path that claimed it

    for folder_path in &folder_rel_paths {
        let base_name = Path::new(folder_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(folder_path.as_str());
        let base_slug = slugify(base_name);

        let slug = if let Some(existing_path) = used_folder_slugs.get(&base_slug) {
            if existing_path == folder_path {
                base_slug.clone()
            } else {
                // Collision: find a unique suffix.
                let mut n = 2;
                loop {
                    let candidate = format!("{base_slug}-{n}");
                    if !used_folder_slugs.contains_key(&candidate) {
                        break candidate;
                    }
                    n += 1;
                }
            }
        } else {
            base_slug.clone()
        };

        used_folder_slugs
            .entry(slug.clone())
            .or_insert(folder_path.clone());
        folder_slug_by_path.insert(folder_path.clone(), slug);
    }

    // --- Phase 3: repository element ---
    let repo_slug = slugify(&ctx.repo_name);
    elements.insert(
        repo_slug.clone(),
        Element {
            name: ctx.repo_name.clone(),
            kind: "repository".to_string(),
            technology: "Git Repository".to_string(),
            owner: ctx.owner.clone(),
            branch: ctx.branch.clone(),
            has_view: true,
            view_label: ctx.repo_name.clone(),
            placements: vec![ViewPlacement {
                parent_ref: "root".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        },
    );

    // --- Phase 4: folder elements (shallowest first) ---
    let mut sorted_folders: Vec<&String> = folder_rel_paths.iter().collect();
    sorted_folders.sort_by_key(|p| p.matches('/').count());

    for folder_path in &sorted_folders {
        let folder_name = Path::new(folder_path.as_str())
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(folder_path.as_str())
            .to_string();

        let slug = folder_slug_by_path[folder_path.as_str()].clone();

        let parent_slug = folder_parent_slug(folder_path, &folder_slug_by_path, &repo_slug);

        elements.insert(
            slug,
            Element {
                name: folder_name,
                kind: "folder".to_string(),
                technology: "Folder".to_string(),
                owner: ctx.owner.clone(),
                branch: ctx.branch.clone(),
                file_path: (*folder_path).clone(),
                placements: vec![ViewPlacement {
                    parent_ref: parent_slug,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
    }

    // --- Phase 5: file elements ---
    // Only create file elements for files that have at least one symbol.
    let files_with_symbols: std::collections::HashSet<String> = result
        .symbols
        .iter()
        .map(|s| rel_from_base(&s.file_path, &scan_parent))
        .collect();

    // file_slug_by_rel: rel_path -> slug
    let mut file_slug_by_rel: HashMap<String, String> = HashMap::new();
    let mut used_file_slugs: HashMap<String, String> = HashMap::new();

    for rel_path in &file_rel_paths {
        let file_name = Path::new(rel_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(rel_path.as_str());
        let base_slug = slugify(file_name);

        let slug = if let Some(existing_path) = used_file_slugs.get(&base_slug) {
            if existing_path == rel_path {
                base_slug.clone()
            } else {
                let mut n = 2;
                loop {
                    let candidate = format!("{base_slug}-{n}");
                    if !used_file_slugs.contains_key(&candidate) {
                        break candidate;
                    }
                    n += 1;
                }
            }
        } else {
            base_slug.clone()
        };

        used_file_slugs
            .entry(slug.clone())
            .or_insert(rel_path.clone());
        file_slug_by_rel.insert(rel_path.clone(), slug.clone());

        let parent_dir = Path::new(rel_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        let parent_slug = if parent_dir.is_empty() {
            repo_slug.clone()
        } else {
            folder_slug_by_path
                .get(parent_dir)
                .cloned()
                .unwrap_or_else(|| repo_slug.clone())
        };

        let technology = detect_file_technology(rel_path);

        // Only create a file element if this file contains at least one symbol.
        if files_with_symbols.contains(rel_path) {
            elements.insert(
                slug,
                Element {
                    name: file_name.to_string(),
                    kind: "file".to_string(),
                    technology,
                    owner: ctx.owner.clone(),
                    branch: ctx.branch.clone(),
                    file_path: rel_path.clone(),
                    view_label: file_name.to_string(),
                    placements: vec![ViewPlacement {
                        parent_ref: parent_slug,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            );
        }
    }

    // --- Phase 6: symbol elements (sort by file+line so classes come before methods) ---
    let mut symbols_sorted = result.symbols.clone();
    symbols_sorted.sort_by(|a, b| (&a.file_path, a.line).cmp(&(&b.file_path, b.line)));

    for sym in &symbols_sorted {
        let rel = rel_from_base(&sym.file_path, &scan_parent);
        let file_slug = file_slug_by_rel
            .get(&rel)
            .cloned()
            .unwrap_or_else(|| slugify(&rel));

        let parent_slug = symbol_parent_slug(sym, &file_slug, &elements);
        let base_sym_slug = slugify(&sym.name);

        // If slug collides with an existing element, use a compound slug:
        // {file_base_slug}-{symbol_slug}  (where file_base = stem of filename)
        let (sym_slug, compound_name) = if elements.contains_key(&base_sym_slug) {
            let file_base = Path::new(&rel)
                .file_stem()
                .and_then(|s| s.to_str())
                .map_or_else(|| file_slug.clone(), slugify);
            let compound_slug = format!("{file_base}-{base_sym_slug}");
            // When using compound slug, use ClassName.methodName as the display name
            // if we have a parent class name available (but not for destructors).
            let cname = if !sym.parent.is_empty() && !sym.name.starts_with('~') {
                format!("{}.{}", sym.parent, sym.name)
            } else {
                sym.name.clone()
            };
            // If compound also collides, add numeric suffix.
            let final_slug = if elements.contains_key(&compound_slug) {
                let mut n = 2;
                loop {
                    let candidate = format!("{compound_slug}-{n}");
                    if !elements.contains_key(&candidate) {
                        break candidate;
                    }
                    n += 1;
                }
            } else {
                compound_slug
            };
            (final_slug, cname)
        } else {
            (base_sym_slug, sym.name.clone())
        };

        elements.insert(
            sym_slug,
            Element {
                name: compound_name,
                kind: sym.kind.clone(),
                technology: sym.technology.clone(),
                owner: ctx.owner.clone(),
                branch: ctx.branch.clone(),
                file_path: rel.clone(),
                symbol: sym.name.clone(),
                description: sym.description.clone(),
                placements: vec![ViewPlacement {
                    parent_ref: parent_slug,
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
    }

    // --- Phase 7: import connectors (depends_on) ---
    let mut seen_connectors: HashSet<String> = HashSet::new();

    for r in &result.refs {
        if r.kind != "import" {
            continue;
        }

        let src_rel = rel_from_base(&r.file_path, &scan_parent);
        let src_file_slug = match file_slug_by_rel.get(&src_rel) {
            Some(s) => s.clone(),
            None => continue,
        };

        // Resolve import target to a file or folder slug.
        let target_slug = resolve_import_target(
            &r.target_path,
            &r.file_path,
            &ResolveContext {
                scan_parent: &scan_parent,
                file_slug_by_rel: &file_slug_by_rel,
                folder_slug_by_path: &folder_slug_by_path,
                elements: &elements,
            },
        );

        let target_slug = match target_slug {
            Some(s) if s != src_file_slug => s,
            _ => continue,
        };

        // View = nearest common ancestor folder.
        let tgt_rel_path = elements
            .get(&target_slug)
            .map(|e| e.file_path.clone())
            .unwrap_or_default();
        let view = nearest_common_ancestor_folder_slug(
            &src_rel,
            &tgt_rel_path,
            &folder_slug_by_path,
            &repo_slug,
        );

        let conn = Connector {
            view,
            source: src_file_slug,
            target: target_slug,
            label: "references".to_string(),
            relationship: "depends_on".to_string(),
            direction: "forward".to_string(),
            ..Default::default()
        };
        let key = conn.resource_ref();
        if seen_connectors.insert(key) {
            connectors.push(conn);
        }
    }

    // --- Phase 8: call connectors (uses) ---
    for r in &result.refs {
        if r.kind != "call" {
            continue;
        }

        // Find the narrowest containing symbol.
        let containing = find_containing_symbol(&symbols_sorted, &r.file_path, r.line);
        let src_slug = match containing {
            Some(sym) => slugify(&sym.name),
            None => continue,
        };

        if !elements.contains_key(&src_slug) {
            continue;
        }

        let tgt_slug = slugify(&r.name);
        if !elements.contains_key(&tgt_slug) || tgt_slug == src_slug {
            continue;
        }

        let src_rel = elements
            .get(&src_slug)
            .map(|e| e.file_path.clone())
            .unwrap_or_default();
        let tgt_rel = elements
            .get(&tgt_slug)
            .map(|e| e.file_path.clone())
            .unwrap_or_default();

        let view = nearest_common_ancestor_folder_slug(
            &src_rel,
            &tgt_rel,
            &folder_slug_by_path,
            &repo_slug,
        );

        let conn = Connector {
            view,
            source: src_slug,
            target: tgt_slug,
            label: "calls".to_string(),
            relationship: "uses".to_string(),
            direction: "forward".to_string(),
            ..Default::default()
        };
        let key = conn.resource_ref();
        if seen_connectors.insert(key) {
            connectors.push(conn);
        }
    }

    BuildOutput {
        elements,
        connectors,
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Compute relative path from a base directory.
fn rel_from_base(abs: &str, base: &str) -> String {
    if base.is_empty() {
        return abs.to_string();
    }
    let base_path = Path::new(base);
    Path::new(abs).strip_prefix(base_path).map_or_else(
        |_| abs.to_string(),
        |p| p.to_str().unwrap_or(abs).to_string(),
    )
}

fn should_skip_file(rel: &str) -> bool {
    let ext = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    // Skip lock files, YAML configs, markdown, build files, etc.
    matches!(
        ext,
        "lock"
            | "toml"
            | "json"
            | "md"
            | "txt"
            | "yaml"
            | "yml"
            | "sum"
            | "mod"
            | "gitignore"
            | "xml"
            | "gradle"
            | "properties"
    ) || rel.contains(".git/")
        || rel.contains("node_modules/")
        || rel.contains("target/")
}

fn collect_folder_rel_paths(file_rel_paths: &[String]) -> BTreeSet<String> {
    let mut folders: BTreeSet<String> = BTreeSet::new();
    for p in file_rel_paths {
        let mut current = Path::new(p).parent();
        while let Some(parent) = current {
            let s = parent.to_str().unwrap_or("");
            if s.is_empty() {
                break;
            }
            folders.insert(s.to_string());
            current = parent.parent();
        }
    }
    folders
}

fn folder_parent_slug(
    folder_path: &str,
    folder_slug_by_path: &HashMap<String, String>,
    repo_slug: &str,
) -> String {
    let parent = Path::new(folder_path)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    if parent.is_empty() {
        return repo_slug.to_string();
    }
    folder_slug_by_path
        .get(parent)
        .cloned()
        .unwrap_or_else(|| repo_slug.to_string())
}

fn detect_file_technology(rel_path: &str) -> String {
    let ext = Path::new(rel_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "go" => "go",
        "rs" => "rust",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "java" => "java",
        "cpp" | "cc" | "cxx" | "h" | "hpp" => "cpp",
        _ => "File",
    }
    .to_string()
}

/// Determine the parent slug for a symbol element.
/// TypeScript, JavaScript, Java, Python: methods go under their parent class.
/// Go, Rust: everything goes under the file.
/// C++: constructors and destructors go under their parent class; regular methods under the file.
fn symbol_parent_slug(
    sym: &crate::analyzer::types::Symbol,
    file_slug: &str,
    elements: &HashMap<String, Element>,
) -> String {
    let uses_class_parent = matches!(
        sym.technology.as_str(),
        "typescript" | "javascript" | "java" | "python"
    );
    let cpp_class_member =
        sym.technology == "cpp" && matches!(sym.kind.as_str(), "constructor" | "destructor");

    if (uses_class_parent || cpp_class_member) && !sym.parent.is_empty() {
        let parent_slug = slugify(&sym.parent);
        if elements.contains_key(&parent_slug) {
            return parent_slug;
        }
    }

    file_slug.to_string()
}

/// Find the narrowest symbol (by line range) that contains the given file+line.
fn find_containing_symbol<'a>(
    symbols: &'a [crate::analyzer::types::Symbol],
    file_path: &str,
    line: i32,
) -> Option<&'a crate::analyzer::types::Symbol> {
    symbols
        .iter()
        .filter(|s| {
            s.file_path == file_path && s.line <= line && (s.end_line == 0 || line <= s.end_line)
        })
        .min_by_key(|s| {
            // Narrowest range = most specific. Prefer non-zero end_line.
            if s.end_line > 0 {
                s.end_line - s.line
            } else {
                i32::MAX
            }
        })
}

struct ResolveContext<'a> {
    scan_parent: &'a str,
    file_slug_by_rel: &'a HashMap<String, String>,
    folder_slug_by_path: &'a HashMap<String, String>,
    elements: &'a HashMap<String, Element>,
}

/// Attempt to resolve an import target_path to a known file or folder slug.
fn resolve_import_target(
    target_path: &str,
    source_file_abs: &str,
    ctx: &ResolveContext,
) -> Option<String> {
    if target_path.is_empty() {
        return None;
    }

    // Case 1: relative path (TypeScript, C++ local includes)
    if target_path.starts_with("./") || target_path.starts_with("../") {
        let source_dir = Path::new(source_file_abs).parent()?.to_str()?.to_string();
        // Join source dir with the relative target
        let joined = format!(
            "{}/{}",
            source_dir,
            target_path.strip_prefix("./").unwrap_or(target_path)
        );
        let normalized = normalize_path(&joined);
        let rel = rel_from_base(&normalized, ctx.scan_parent);

        // Try with common extensions if no extension given
        for candidate in candidate_paths(&rel) {
            if let Some(slug) = ctx.file_slug_by_rel.get(&candidate) {
                return Some(slug.clone());
            }
        }
        return None;
    }

    // Case 2: package/module path — try matching the last component against known folders/files.
    // Go: "github.com/example/myproject/internal/service" -> last component "service"
    // Python: "models", "services" -> match against file slugs
    let last_component = target_path.rsplit(['/', '.']).next().unwrap_or(target_path);

    // Try as file slug (e.g., Python "models" -> "models-py" file)
    let file_slug = slugify(last_component);
    if ctx.elements.contains_key(&file_slug) {
        let el = &ctx.elements[&file_slug];
        if el.kind == "file" {
            return Some(file_slug);
        }
    }

    // Try folder slug
    if ctx.folder_slug_by_path.values().any(|s| *s == file_slug) {
        return Some(file_slug);
    }

    // Try matching target_path against any folder rel path by suffix
    let target_norm = target_path.replace('.', "/");
    for (folder_path, folder_slug) in ctx.folder_slug_by_path {
        if folder_path.ends_with(&target_norm) || folder_path.ends_with(last_component) {
            return Some(folder_slug.clone());
        }
    }

    // Try matching against file rel paths by stem
    for (file_rel, file_slug) in ctx.file_slug_by_rel {
        let stem = Path::new(file_rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if stem == last_component {
            return Some(file_slug.clone());
        }
    }

    None
}

fn candidate_paths(rel_without_ext: &str) -> Vec<String> {
    let exts = ["", ".go", ".ts", ".py", ".java", ".cpp", ".rs", ".js"];
    exts.iter()
        .map(|e| format!("{rel_without_ext}{e}"))
        .collect()
}

fn normalize_path(path: &str) -> String {
    let mut components: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                components.pop();
            }
            p => components.push(p),
        }
    }
    components.join("/")
}

/// Find the nearest common ancestor folder slug for two file rel paths.
fn nearest_common_ancestor_folder_slug(
    src_file_rel: &str,
    tgt_file_rel: &str,
    folder_slug_by_path: &HashMap<String, String>,
    repo_slug: &str,
) -> String {
    // Get directory parts (strip filenames).
    let src_dir = Path::new(src_file_rel)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let tgt_dir = Path::new(tgt_file_rel)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");

    let src_parts: Vec<&str> = if src_dir.is_empty() {
        vec![]
    } else {
        src_dir.split('/').collect()
    };
    let tgt_parts: Vec<&str> = if tgt_dir.is_empty() {
        vec![]
    } else {
        tgt_dir.split('/').collect()
    };

    // Find longest common prefix.
    let mut common: Vec<&str> = Vec::new();
    for (a, b) in src_parts.iter().zip(tgt_parts.iter()) {
        if a == b {
            common.push(a);
        } else {
            break;
        }
    }

    if common.is_empty() {
        return repo_slug.to_string();
    }

    let common_path = common.join("/");
    if let Some(slug) = folder_slug_by_path.get(&common_path) {
        return slug.clone();
    }

    // Fallback: slugify last common component.
    slugify(common.last().copied().unwrap_or(repo_slug))
}
