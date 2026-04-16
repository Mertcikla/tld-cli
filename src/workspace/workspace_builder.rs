use crate::analyzer::types::{AnalysisResult, Symbol as AnalyzerSymbol};
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
    let mut builder = WorkspaceBuilder::new(result, ctx);
    builder.build()
}

struct WorkspaceBuilder<'a> {
    result: &'a AnalysisResult,
    ctx: &'a BuildContext,
    elements: HashMap<String, Element>,
    connectors: Vec<Connector>,
    scan_parent: String,
    file_rel_paths: Vec<String>,
    folder_rel_paths: BTreeSet<String>,
    folder_slug_by_path: HashMap<String, String>,
    file_slug_by_rel: HashMap<String, String>,
    repo_slug: String,
}

impl<'a> WorkspaceBuilder<'a> {
    fn new(result: &'a AnalysisResult, ctx: &'a BuildContext) -> Self {
        let scan_parent = Path::new(&ctx.scan_root)
            .parent()
            .map_or("", |p| p.to_str().unwrap_or(""))
            .to_string();

        Self {
            result,
            ctx,
            elements: HashMap::new(),
            connectors: Vec::new(),
            scan_parent,
            file_rel_paths: Vec::new(),
            folder_rel_paths: BTreeSet::new(),
            folder_slug_by_path: HashMap::new(),
            file_slug_by_rel: HashMap::new(),
            repo_slug: String::new(),
        }
    }

    fn build(&mut self) -> BuildOutput {
        self.initialize_paths();
        self.build_folder_tree();
        self.add_repository_element();
        self.add_folder_elements();
        self.add_file_elements();
        self.add_symbol_elements();
        self.add_connectors();

        BuildOutput {
            elements: std::mem::take(&mut self.elements),
            connectors: std::mem::take(&mut self.connectors),
        }
    }

    fn initialize_paths(&mut self) {
        let mut seen_files = HashSet::new();

        for abs_path in &self.result.files_scanned {
            let rel = rel_from_base(abs_path, &self.scan_parent);
            if should_skip_file(&rel) {
                continue;
            }
            if seen_files.insert(rel.clone()) {
                self.file_rel_paths.push(rel);
            }
        }

        for sym in &self.result.symbols {
            let rel = rel_from_base(&sym.file_path, &self.scan_parent);
            if seen_files.insert(rel.clone()) {
                self.file_rel_paths.push(rel);
            }
        }

        self.folder_rel_paths = collect_folder_rel_paths(&self.file_rel_paths);
    }

    fn build_folder_tree(&mut self) {
        let mut registry = SlugRegistry::new();
        for folder_path in &self.folder_rel_paths {
            let base_name = Path::new(folder_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(folder_path.as_str());
            let slug = registry.claim(base_name, folder_path);
            self.folder_slug_by_path.insert(folder_path.clone(), slug);
        }
    }

    fn add_repository_element(&mut self) {
        self.repo_slug = slugify(&self.ctx.repo_name);
        self.elements.insert(
            self.repo_slug.clone(),
            Element {
                name: self.ctx.repo_name.clone(),
                kind: "repository".to_string(),
                technology: "Git Repository".to_string(),
                owner: self.ctx.owner.clone(),
                branch: self.ctx.branch.clone(),
                has_view: true,
                view_label: self.ctx.repo_name.clone(),
                placements: vec![ViewPlacement {
                    parent_ref: "root".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );
    }

    fn add_folder_elements(&mut self) {
        let mut sorted_folders: Vec<&String> = self.folder_rel_paths.iter().collect();
        sorted_folders.sort_by_key(|p| p.matches('/').count());

        for folder_path in sorted_folders {
            let folder_name = Path::new(folder_path.as_str())
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(folder_path.as_str())
                .to_string();

            let slug = self.folder_slug_by_path[folder_path.as_str()].clone();
            let parent_slug = self.folder_parent_slug(folder_path);

            self.elements.insert(
                slug,
                Element {
                    name: folder_name,
                    kind: "folder".to_string(),
                    technology: "Folder".to_string(),
                    owner: self.ctx.owner.clone(),
                    branch: self.ctx.branch.clone(),
                    file_path: (*folder_path).clone(),
                    placements: vec![ViewPlacement {
                        parent_ref: parent_slug,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            );
        }
    }

    fn add_file_elements(&mut self) {
        let files_with_symbols: HashSet<String> = self
            .result
            .symbols
            .iter()
            .map(|s| rel_from_base(&s.file_path, &self.scan_parent))
            .collect();

        let mut registry = SlugRegistry::new();
        for rel_path in &self.file_rel_paths {
            let file_name = Path::new(rel_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(rel_path.as_str());

            let slug = registry.claim(file_name, rel_path);
            self.file_slug_by_rel.insert(rel_path.clone(), slug.clone());

            if files_with_symbols.contains(rel_path) {
                let parent_dir = Path::new(rel_path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or("");
                let parent_slug = if parent_dir.is_empty() {
                    self.repo_slug.clone()
                } else {
                    self.folder_slug_by_path
                        .get(parent_dir)
                        .cloned()
                        .unwrap_or_else(|| self.repo_slug.clone())
                };

                self.elements.insert(
                    slug,
                    Element {
                        name: file_name.to_string(),
                        kind: "file".to_string(),
                        technology: detect_file_technology(rel_path),
                        owner: self.ctx.owner.clone(),
                        branch: self.ctx.branch.clone(),
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
    }

    fn add_symbol_elements(&mut self) {
        let mut symbols_sorted = self.result.symbols.clone();
        symbols_sorted.sort_by(|a, b| (&a.file_path, a.line).cmp(&(&b.file_path, b.line)));

        for sym in &symbols_sorted {
            let rel = rel_from_base(&sym.file_path, &self.scan_parent);
            let file_slug = self
                .file_slug_by_rel
                .get(&rel)
                .cloned()
                .unwrap_or_else(|| slugify(&rel));
            let parent_slug = self.symbol_parent_slug(sym, &file_slug);
            let base_sym_slug = slugify(&sym.name);

            let (sym_slug, compound_name) = if self.elements.contains_key(&base_sym_slug) {
                let file_base = Path::new(&rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map_or_else(|| file_slug.clone(), slugify);
                let compound_slug = format!("{file_base}-{base_sym_slug}");
                let cname = if !sym.parent.is_empty() && !sym.name.starts_with('~') {
                    format!("{}.{}", sym.parent, sym.name)
                } else {
                    sym.name.clone()
                };

                let final_slug = if self.elements.contains_key(&compound_slug) {
                    let mut n = 2;
                    loop {
                        let candidate = format!("{compound_slug}-{n}");
                        if !self.elements.contains_key(&candidate) {
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

            self.elements.insert(
                sym_slug,
                Element {
                    name: compound_name,
                    kind: sym.kind.clone(),
                    technology: sym.technology.clone(),
                    owner: self.ctx.owner.clone(),
                    branch: self.ctx.branch.clone(),
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
    }

    fn add_connectors(&mut self) {
        let mut seen_connectors = HashSet::new();
        let symbols_sorted = self.result.symbols.clone(); // For looking up containing symbol

        for r in &self.result.refs {
            let conn = if r.kind == "import" {
                self.build_import_connector(r)
            } else if r.kind == "call" {
                self.build_call_connector(r, &symbols_sorted)
            } else {
                None
            };

            if let Some(c) = conn {
                let key = c.resource_ref();
                if seen_connectors.insert(key) {
                    self.connectors.push(c);
                }
            }
        }
    }

    fn build_import_connector(&self, r: &crate::analyzer::types::Ref) -> Option<Connector> {
        let src_rel = rel_from_base(&r.file_path, &self.scan_parent);
        let src_file_slug = self.file_slug_by_rel.get(&src_rel)?.clone();

        let target_slug = resolve_import_target(
            &r.target_path,
            &r.file_path,
            &ResolveContext {
                scan_parent: &self.scan_parent,
                file_slug_by_rel: &self.file_slug_by_rel,
                folder_slug_by_path: &self.folder_slug_by_path,
                elements: &self.elements,
            },
        )?;

        if target_slug == src_file_slug {
            return None;
        }

        let tgt_rel_path = self.elements.get(&target_slug)?.file_path.clone();
        let view = nearest_common_ancestor_folder_slug(
            &src_rel,
            &tgt_rel_path,
            &self.folder_slug_by_path,
            &self.repo_slug,
        );

        Some(Connector {
            view,
            source: src_file_slug,
            target: target_slug,
            label: "references".to_string(),
            relationship: "depends_on".to_string(),
            direction: "forward".to_string(),
            ..Default::default()
        })
    }

    fn build_call_connector(
        &self,
        r: &crate::analyzer::types::Ref,
        symbols: &[AnalyzerSymbol],
    ) -> Option<Connector> {
        let containing = find_containing_symbol(symbols, &r.file_path, r.line)?;
        // Look up source slug by file + symbol name for higher accuracy.
        let src_slug =
            self.find_element_slug_for_symbol(&containing.name, &containing.file_path)?;

        // Priority 1: LSP gave us a resolved target_path — find the target symbol there.
        let tgt_slug = if r.target_path.is_empty() {
            // Fallback: bare slug name match (original behaviour).
            let slug = slugify(&r.name);
            if self.elements.contains_key(&slug) {
                Some(slug)
            } else {
                None
            }
        } else {
            let tgt_rel = rel_from_base(&r.target_path, &self.scan_parent);
            // Try to find a symbol element in the target file with the matching name.
            self.elements
                .iter()
                .find(|(_, el)| {
                    !el.symbol.is_empty() && el.symbol == r.name && el.file_path == tgt_rel
                })
                .map(|(k, _)| k.clone())
                // If we couldn't find the exact symbol, see if the file itself is an element.
                .or_else(|| self.file_slug_by_rel.get(&tgt_rel).cloned())
        }?;

        if tgt_slug == src_slug {
            return None;
        }

        let src_rel = self.elements.get(&src_slug)?.file_path.clone();
        let tgt_rel = self.elements.get(&tgt_slug)?.file_path.clone();

        let view = nearest_common_ancestor_folder_slug(
            &src_rel,
            &tgt_rel,
            &self.folder_slug_by_path,
            &self.repo_slug,
        );

        Some(Connector {
            view,
            source: src_slug,
            target: tgt_slug,
            label: "calls".to_string(),
            relationship: "uses".to_string(),
            direction: "forward".to_string(),
            ..Default::default()
        })
    }

    /// Find the element slug for a declaration by matching the `symbol` field and file path.
    fn find_element_slug_for_symbol(&self, sym_name: &str, abs_file: &str) -> Option<String> {
        let file_rel = rel_from_base(abs_file, &self.scan_parent);
        // Look for an element whose `symbol` field matches and whose file matches.
        let lsp_match = self
            .elements
            .iter()
            .find(|(_, el)| {
                !el.symbol.is_empty() && el.symbol == sym_name && el.file_path == file_rel
            })
            .map(|(k, _)| k.clone());

        if lsp_match.is_some() {
            return lsp_match;
        }

        // Fallback: slug-based match (original behaviour).
        let slug = slugify(sym_name);
        if self.elements.contains_key(&slug) {
            Some(slug)
        } else {
            None
        }
    }

    fn folder_parent_slug(&self, folder_path: &str) -> String {
        let parent = Path::new(folder_path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("");
        if parent.is_empty() {
            return self.repo_slug.clone();
        }
        self.folder_slug_by_path
            .get(parent)
            .cloned()
            .unwrap_or_else(|| self.repo_slug.clone())
    }

    fn symbol_parent_slug(&self, sym: &AnalyzerSymbol, file_slug: &str) -> String {
        let uses_class_parent = matches!(
            sym.technology.as_str(),
            "typescript" | "javascript" | "java" | "python"
        );
        let cpp_class_member =
            sym.technology == "cpp" && matches!(sym.kind.as_str(), "constructor" | "destructor");

        if (uses_class_parent || cpp_class_member) && !sym.parent.is_empty() {
            let parent_slug = slugify(&sym.parent);
            if self.elements.contains_key(&parent_slug) {
                return parent_slug;
            }
        }
        file_slug.to_string()
    }
}

struct SlugRegistry {
    used: HashMap<String, String>, // slug -> claimant key
}

impl SlugRegistry {
    fn new() -> Self {
        Self {
            used: HashMap::new(),
        }
    }

    fn claim(&mut self, base_name: &str, claimant_key: &str) -> String {
        let base_slug = slugify(base_name);
        if let Some(existing_key) = self.used.get(&base_slug) {
            if existing_key == claimant_key {
                return base_slug;
            }
            let mut n = 2;
            loop {
                let candidate = format!("{base_slug}-{n}");
                if !self.used.contains_key(&candidate) {
                    self.used
                        .insert(candidate.clone(), claimant_key.to_string());
                    return candidate;
                }
                n += 1;
            }
        }
        self.used
            .insert(base_slug.clone(), claimant_key.to_string());
        base_slug
    }
}

// ─── Free Helpers ────────────────────────────────────────────────────────────

fn rel_from_base(abs: &str, base: &str) -> String {
    if base.is_empty() {
        return abs.to_string();
    }
    Path::new(abs).strip_prefix(Path::new(base)).map_or_else(
        |_| abs.to_string(),
        |p| p.to_str().unwrap_or(abs).to_string(),
    )
}

fn should_skip_file(rel: &str) -> bool {
    let ext = Path::new(rel)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
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
    let mut folders = BTreeSet::new();
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

fn find_containing_symbol<'a>(
    symbols: &'a [AnalyzerSymbol],
    file_path: &str,
    line: i32,
) -> Option<&'a AnalyzerSymbol> {
    symbols
        .iter()
        .filter(|s| {
            s.file_path == file_path && s.line <= line && (s.end_line == 0 || line <= s.end_line)
        })
        .min_by_key(|s| {
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

fn resolve_import_target(
    target_path: &str,
    source_file_abs: &str,
    ctx: &ResolveContext,
) -> Option<String> {
    if target_path.is_empty() {
        return None;
    }
    if target_path.starts_with("./") || target_path.starts_with("../") {
        let source_dir = Path::new(source_file_abs).parent()?.to_str()?.to_string();
        let joined = format!(
            "{}/{}",
            source_dir,
            target_path.strip_prefix("./").unwrap_or(target_path)
        );
        let normalized = normalize_path(&joined);
        let rel = rel_from_base(&normalized, ctx.scan_parent);
        for candidate in candidate_paths(&rel) {
            if let Some(slug) = ctx.file_slug_by_rel.get(&candidate) {
                return Some(slug.clone());
            }
        }
        return None;
    }
    let last = target_path.rsplit(['/', '.']).next().unwrap_or(target_path);
    let fs = slugify(last);
    if ctx.elements.get(&fs).is_some_and(|el| el.kind == "file") {
        return Some(fs);
    }
    if ctx.folder_slug_by_path.values().any(|s| *s == fs) {
        return Some(fs);
    }
    let target_norm = target_path.replace('.', "/");
    for (fpath, fslug) in ctx.folder_slug_by_path {
        if fpath.ends_with(&target_norm) || fpath.ends_with(last) {
            return Some(fslug.clone());
        }
    }
    for (frel, fslug) in ctx.file_slug_by_rel {
        if Path::new(frel).file_stem().and_then(|s| s.to_str()) == Some(last) {
            return Some(fslug.clone());
        }
    }
    None
}

fn candidate_paths(rel_without_ext: &str) -> Vec<String> {
    ["", ".go", ".ts", ".py", ".java", ".cpp", ".rs", ".js"]
        .iter()
        .map(|e| format!("{rel_without_ext}{e}"))
        .collect()
}

fn normalize_path(path: &str) -> String {
    let mut components = Vec::new();
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

fn nearest_common_ancestor_folder_slug(
    src_file_rel: &str,
    tgt_file_rel: &str,
    folder_slug_by_path: &HashMap<String, String>,
    repo_slug: &str,
) -> String {
    let src_dir = Path::new(src_file_rel)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let tgt_dir = Path::new(tgt_file_rel)
        .parent()
        .and_then(|p| p.to_str())
        .unwrap_or("");
    let sp: Vec<&str> = if src_dir.is_empty() {
        vec![]
    } else {
        src_dir.split('/').collect()
    };
    let tp: Vec<&str> = if tgt_dir.is_empty() {
        vec![]
    } else {
        tgt_dir.split('/').collect()
    };
    let mut common = Vec::new();
    for (&a, &b) in sp.iter().zip(tp.iter()) {
        if a == b {
            common.push(a);
        } else {
            break;
        }
    }
    if common.is_empty() {
        return repo_slug.to_string();
    }
    let cp = common.join("/");
    folder_slug_by_path
        .get(&cp)
        .cloned()
        .unwrap_or_else(|| slugify(common.last().copied().unwrap_or(repo_slug)))
}
