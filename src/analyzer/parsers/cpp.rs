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
    struct_name_idx: u32,
    struct_body_idx: u32,
    enum_name_idx: u32,
    fn_decl_idx: u32,
}

static DECL_QUERY: OnceLock<DeclQuery> = OnceLock::new();

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
    let decl = DECL_QUERY.get_or_init(|| {
        let decl_query_src = queries::cpp_declarations();
        let query = Query::new(language, decl_query_src).expect("Failed to compile CPP decl query");
        let class_name_idx = query
            .capture_index_for_name("class_name")
            .unwrap_or(u32::MAX);
        let class_body_idx = query
            .capture_index_for_name("class_body")
            .unwrap_or(u32::MAX);
        let struct_name_idx = query
            .capture_index_for_name("struct_name")
            .unwrap_or(u32::MAX);
        let struct_body_idx = query
            .capture_index_for_name("struct_body")
            .unwrap_or(u32::MAX);
        let enum_name_idx = query
            .capture_index_for_name("enum_name")
            .unwrap_or(u32::MAX);
        let fn_decl_idx = query
            .capture_index_for_name("fn_declarator")
            .unwrap_or(u32::MAX);

        DeclQuery {
            query,
            class_name_idx,
            class_body_idx,
            struct_name_idx,
            struct_body_idx,
            enum_name_idx,
            fn_decl_idx,
        }
    });

    let mut type_body_ranges: Vec<(usize, usize)> = Vec::new();

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&decl.query, *node, source);

    while let Some(m) = matches.next() {
        let mut class_name_node: Option<Node> = None;
        let mut class_body_node: Option<Node> = None;
        let mut struct_name_node: Option<Node> = None;
        let mut struct_body_node: Option<Node> = None;
        let mut enum_name_node: Option<Node> = None;
        let mut fn_decl_node: Option<Node> = None;

        for cap in m.captures {
            match cap.index {
                i if i == decl.class_name_idx => class_name_node = Some(cap.node),
                i if i == decl.class_body_idx => class_body_node = Some(cap.node),
                i if i == decl.struct_name_idx => struct_name_node = Some(cap.node),
                i if i == decl.struct_body_idx => struct_body_node = Some(cap.node),
                i if i == decl.enum_name_idx => enum_name_node = Some(cap.node),
                i if i == decl.fn_decl_idx => fn_decl_node = Some(cap.node),
                _ => {}
            }
        }

        if let Some(name_node) = class_name_node {
            let type_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: type_name.clone(),
                kind: "class".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
            if let Some(body) = class_body_node {
                type_body_ranges.push((body.start_byte(), body.end_byte()));
                parse_type_members(&body, source, path, language, &type_name, result);
            }
        } else if let Some(name_node) = struct_name_node {
            let type_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: type_name.clone(),
                kind: "struct".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
            if let Some(body) = struct_body_node {
                type_body_ranges.push((body.start_byte(), body.end_byte()));
                parse_type_members(&body, source, path, language, &type_name, result);
            }
        } else if let Some(name_node) = enum_name_node {
            let enum_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: enum_name,
                kind: "enum".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
        } else if let Some(decl_node) = fn_decl_node {
            // Skip functions inside class/struct bodies.
            let fn_start = decl_node.start_byte();
            let inside_type = type_body_ranges
                .iter()
                .any(|(s, e)| fn_start >= *s && fn_start < *e);
            if inside_type {
                continue;
            }
            let outer = decl_node.parent().unwrap_or(decl_node);
            let name = cpp_name(&decl_node, source);
            if !name.is_empty() {
                // If the declarator is a scoped_identifier (e.g. ClassName::method),
                // treat it as a method with the scope as parent.
                let (kind, parent) = cpp_scoped_kind(&decl_node, source);
                result.symbols.push(Symbol {
                    name,
                    kind,
                    file_path: path.to_string(),
                    line: (decl_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: String::new(),
                    parent,
                    technology: String::new(),
                });
            }
        }
    }
}

struct MemberQuery {
    query: Query,
    decl_idx: u32,
    field_decl_idx: u32,
    ctor_decl_idx: u32,
}

static MEMBER_QUERY: OnceLock<MemberQuery> = OnceLock::new();

fn parse_type_members(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    type_name: &str,
    result: &mut AnalysisResult,
) {
    let mq = MEMBER_QUERY.get_or_init(|| {
        let method_query_src = queries::cpp_members();
        let query =
            Query::new(language, method_query_src).expect("Failed to compile CPP member query");
        let decl_idx = query
            .capture_index_for_name("method_declarator")
            .unwrap_or(u32::MAX);
        let field_decl_idx = query
            .capture_index_for_name("decl_member")
            .unwrap_or(u32::MAX);
        let ctor_decl_idx = query
            .capture_index_for_name("ctor_decl_member")
            .unwrap_or(u32::MAX);
        MemberQuery {
            query,
            decl_idx,
            field_decl_idx,
            ctor_decl_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&mq.query, *body_node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let (decl_node, from_field) = if cap.index == mq.decl_idx {
                (cap.node, false)
            } else if cap.index == mq.field_decl_idx || cap.index == mq.ctor_decl_idx {
                (cap.node, true)
            } else {
                continue;
            };

            let outer = decl_node.parent().unwrap_or(decl_node);
            let raw = decl_node.utf8_text(source).unwrap_or_default();
            let name = if from_field {
                raw.trim().to_string()
            } else {
                cpp_name(&decl_node, source)
            };

            if name.is_empty() {
                continue;
            }

            if from_field && !name.starts_with('~') && name != type_name {
                continue;
            }

            let kind = if name.starts_with('~') {
                "destructor"
            } else if name == type_name {
                "constructor"
            } else {
                "method"
            };

            result.symbols.push(Symbol {
                name,
                kind: kind.to_string(),
                file_path: path.to_string(),
                line: (decl_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: String::new(),
                parent: type_name.to_string(),
                technology: String::new(),
            });
        }
    }
}

fn cpp_scoped_kind(decl_node: &Node, source: &[u8]) -> (String, String) {
    let inner = if decl_node.kind() == "function_declarator" {
        decl_node
            .child_by_field_name("declarator")
            .unwrap_or(*decl_node)
    } else {
        *decl_node
    };

    if inner.kind() == "scoped_identifier" {
        let scope = inner
            .child_by_field_name("scope")
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("")
            .to_string();
        let name_part = inner
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok())
            .unwrap_or("")
            .trim()
            .to_string();

        if name_part.starts_with('~') {
            return ("destructor".to_string(), scope);
        }
        if !scope.is_empty() && name_part == scope {
            return ("constructor".to_string(), scope);
        }
        if !scope.is_empty() {
            return ("method".to_string(), scope);
        }
    }

    let raw = decl_node.utf8_text(source).unwrap_or_default().trim();
    if let Some(pos) = raw.find("::") {
        let scope = raw[..pos].trim().to_string();
        let after = raw[pos + 2..].trim();
        let name_part = after.split('(').next().unwrap_or(after).trim();
        if !scope.is_empty() && !name_part.is_empty() {
            if name_part.starts_with('~') {
                return ("destructor".to_string(), scope);
            }
            if name_part == scope {
                return ("constructor".to_string(), scope);
            }
            return ("method".to_string(), scope);
        }
    }

    ("function".to_string(), String::new())
}

struct RefQuery {
    query: Query,
    include_idx: u32,
    callee_idx: u32,
}

static REF_QUERY: OnceLock<RefQuery> = OnceLock::new();

fn parse_refs(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    let rq = REF_QUERY.get_or_init(|| {
        let ref_query_src = queries::cpp_references();
        let query = Query::new(language, ref_query_src).expect("Failed to compile CPP ref query");
        let include_idx = query
            .capture_index_for_name("include_path")
            .unwrap_or(u32::MAX);
        let callee_idx = query.capture_index_for_name("callee").unwrap_or(u32::MAX);
        RefQuery {
            query,
            include_idx,
            callee_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&rq.query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == rq.include_idx {
                let import_path = cap_node
                    .utf8_text(source)
                    .unwrap_or_default()
                    .trim_matches('<')
                    .trim_matches('>')
                    .trim_matches('"');
                if !import_path.is_empty() {
                    result.refs.push(Ref {
                        name: import_path.to_string(),
                        kind: "import".to_string(),
                        target_path: import_path.to_string(),
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        column: (cap_node.start_position().column + 1) as i32,
                    });
                }
            } else if idx == rq.callee_idx {
                let text = cap_node.utf8_text(source).unwrap_or_default().trim();
                let name = text
                    .rsplit("::")
                    .next()
                    .unwrap_or(text)
                    .rsplit('.')
                    .next()
                    .unwrap_or(text)
                    .to_string();
                if !name.is_empty() {
                    result.refs.push(Ref {
                        name,
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

fn cpp_name(node: &Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "type_identifier" | "field_identifier" => {
            node.utf8_text(source).unwrap_or_default().to_string()
        }
        "function_declarator" => {
            if let Some(decl) = node.child_by_field_name("declarator") {
                cpp_name(&decl, source)
            } else {
                String::new()
            }
        }
        "scoped_identifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                cpp_name(&name, source)
            } else {
                String::new()
            }
        }
        "pointer_declarator" | "reference_declarator" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                let name = cpp_name(&child, source);
                if !name.is_empty() {
                    return name;
                }
            }
            String::new()
        }
        _ => {
            let text = node.utf8_text(source).unwrap_or_default().trim();
            let name = text.rsplit("::").next().unwrap_or(text);
            name.rsplit('.').next().unwrap_or(name).trim().to_string()
        }
    }
}
