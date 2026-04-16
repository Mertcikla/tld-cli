#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::expect_used
)]
use crate::analyzer::queries;
use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use std::sync::OnceLock;
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator};

struct DeclQuery {
    query: Query,
    class_name_idx: u32,
    class_body_idx: u32,
    iface_idx: u32,
    alias_idx: u32,
    fn_idx: u32,
}

static TS_DECL_QUERY: OnceLock<DeclQuery> = OnceLock::new();
static JS_DECL_QUERY: OnceLock<DeclQuery> = OnceLock::new();

pub fn parse(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    parse_declarations(node, source, path, language, result);
    parse_refs(node, source, path, language, result);
}

fn parse_declarations(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    let decl = decl_query(language, path);

    // Collect all classes first so we can process methods with parent context.
    let mut classes: Vec<(String, Node, i32, i32)> = Vec::new();
    let mut other_symbols: Vec<Symbol> = Vec::new();

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&decl.query, *node, source);

    while let Some(m) = matches.next() {
        let mut class_name_node: Option<Node> = None;
        let mut class_body_node: Option<Node> = None;
        let mut iface_name_node: Option<Node> = None;
        let mut alias_name_node: Option<Node> = None;
        let mut fn_name_node: Option<Node> = None;

        for cap in m.captures {
            match cap.index {
                i if i == decl.class_name_idx => class_name_node = Some(cap.node),
                i if i == decl.class_body_idx => class_body_node = Some(cap.node),
                i if i == decl.iface_idx => iface_name_node = Some(cap.node),
                i if i == decl.alias_idx => alias_name_node = Some(cap.node),
                i if i == decl.fn_idx => fn_name_node = Some(cap.node),
                _ => {}
            }
        }

        if let Some(name_node) = class_name_node {
            let class_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer_line = class_body_node
                .map_or((name_node.end_position().row + 1) as i32, |b| {
                    (b.end_position().row + 1) as i32
                });
            let name_line = (name_node.start_position().row + 1) as i32;
            if let Some(body) = class_body_node {
                classes.push((class_name, body, outer_line, name_line));
            } else {
                other_symbols.push(Symbol {
                    name: class_name,
                    kind: "class".to_string(),
                    file_path: path.to_string(),
                    line: name_line,
                    end_line: outer_line,
                    description: String::new(),
                    parent: String::new(),
                    technology: String::new(),
                });
            }
        } else if let Some(name_node) = iface_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            other_symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "interface".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
        } else if let Some(name_node) = alias_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            other_symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "type".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
        } else if let Some(name_node) = fn_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            other_symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "function".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
        }
    }

    // Emit class symbols first (so methods can reference them by slug).
    for (class_name, body_node, outer_line, name_line) in &classes {
        result.symbols.push(Symbol {
            name: class_name.clone(),
            kind: "class".to_string(),
            file_path: path.to_string(),
            line: *name_line,
            end_line: *outer_line,
            description: String::new(),
            parent: String::new(),
            technology: String::new(),
        });
        parse_class_members(body_node, source, path, language, class_name, result);
    }

    result.symbols.extend(other_symbols);
}

struct MemberQuery {
    query: Query,
    method_idx: u32,
}

static TS_MEMBER_QUERY: OnceLock<MemberQuery> = OnceLock::new();
static JS_MEMBER_QUERY: OnceLock<MemberQuery> = OnceLock::new();

fn parse_class_members(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    class_name: &str,
    result: &mut AnalysisResult,
) {
    let mq = member_query(language, path);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&mq.query, *body_node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if cap.index == mq.method_idx {
                let name_node = cap.node;
                let outer = name_node.parent().unwrap_or(name_node);
                let name = name_node.utf8_text(source).unwrap_or_default().to_string();
                let kind = if name == "constructor" {
                    "constructor"
                } else {
                    "method"
                };
                result.symbols.push(Symbol {
                    name,
                    kind: kind.to_string(),
                    file_path: path.to_string(),
                    line: (name_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: String::new(),
                    parent: class_name.to_string(),
                    technology: String::new(),
                });
            }
        }
    }
}

struct RefQuery {
    query: Query,
    import_idx: u32,
    callee_idx: u32,
}

static TS_REF_QUERY: OnceLock<RefQuery> = OnceLock::new();
static JS_REF_QUERY: OnceLock<RefQuery> = OnceLock::new();

fn is_javascript_path(path: &str) -> bool {
    matches!(
        std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str()),
        Some("js" | "jsx")
    )
}

fn build_decl_query(language: &Language, decl_query_src: &str) -> DeclQuery {
    let query = Query::new(language, decl_query_src).expect("Failed to compile TS decl query");
    let class_name_idx = query
        .capture_index_for_name("class_name")
        .unwrap_or(u32::MAX);
    let class_body_idx = query
        .capture_index_for_name("class_body")
        .unwrap_or(u32::MAX);
    let iface_idx = query
        .capture_index_for_name("iface_name")
        .unwrap_or(u32::MAX);
    let alias_idx = query
        .capture_index_for_name("alias_name")
        .unwrap_or(u32::MAX);
    let fn_idx = query.capture_index_for_name("fn_name").unwrap_or(u32::MAX);

    DeclQuery {
        query,
        class_name_idx,
        class_body_idx,
        iface_idx,
        alias_idx,
        fn_idx,
    }
}

fn build_member_query(language: &Language, member_query_src: &str) -> MemberQuery {
    let query = Query::new(language, member_query_src).expect("Failed to compile TS member query");
    let method_idx = query
        .capture_index_for_name("method_name")
        .unwrap_or(u32::MAX);
    MemberQuery { query, method_idx }
}

fn build_ref_query(language: &Language, ref_query_src: &str) -> RefQuery {
    let query = Query::new(language, ref_query_src).expect("Failed to compile TS ref query");
    let import_idx = query
        .capture_index_for_name("import_src")
        .unwrap_or(u32::MAX);
    let callee_idx = query.capture_index_for_name("callee").unwrap_or(u32::MAX);
    RefQuery {
        query,
        import_idx,
        callee_idx,
    }
}

fn decl_query(language: &Language, path: &str) -> &'static DeclQuery {
    if is_javascript_path(path) {
        JS_DECL_QUERY.get_or_init(|| build_decl_query(language, queries::javascript_declarations()))
    } else {
        TS_DECL_QUERY.get_or_init(|| build_decl_query(language, queries::typescript_declarations()))
    }
}

fn member_query(language: &Language, path: &str) -> &'static MemberQuery {
    if is_javascript_path(path) {
        JS_MEMBER_QUERY.get_or_init(|| build_member_query(language, queries::javascript_members()))
    } else {
        TS_MEMBER_QUERY.get_or_init(|| build_member_query(language, queries::typescript_members()))
    }
}

fn ref_query(language: &Language, path: &str) -> &'static RefQuery {
    if is_javascript_path(path) {
        JS_REF_QUERY.get_or_init(|| build_ref_query(language, queries::javascript_references()))
    } else {
        TS_REF_QUERY.get_or_init(|| build_ref_query(language, queries::typescript_references()))
    }
}

fn parse_refs(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    let rq = ref_query(language, path);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&rq.query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == rq.import_idx {
                let import_path = cap_node.utf8_text(source).unwrap_or_default().to_string();
                if !import_path.is_empty() {
                    result.refs.push(Ref {
                        name: import_path.clone(),
                        kind: "import".to_string(),
                        target_path: import_path,
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        column: (cap_node.start_position().column + 1) as i32,
                    });
                }
            } else if idx == rq.callee_idx {
                let text = cap_node.utf8_text(source).unwrap_or_default();
                let terminal_name = text.rsplit('.').next().unwrap_or(text).to_string();
                if !terminal_name.is_empty() {
                    result.refs.push(Ref {
                        name: terminal_name,
                        kind: "call".to_string(),
                        target_path: String::new(),
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        column: (cap_node.start_position().column + 1) as i32,
                    });
                }
            }
        }
    }
}
