#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::expect_used,
    clippy::collapsible_if
)]
use crate::analyzer::queries;
use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use std::sync::OnceLock;
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator};

struct DeclQuery {
    query: Query,
    fn_idx: u32,
    method_idx: u32,
    recv_idx: u32,
    struct_idx: u32,
    iface_idx: u32,
    type_idx: u32,
    alias_idx: u32,
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
        let query_src = queries::go_declarations();
        let query = Query::new(language, query_src).expect("Failed to compile Go decl query");
        let fn_idx = query.capture_index_for_name("fn_name").unwrap_or(u32::MAX);
        let method_idx = query
            .capture_index_for_name("method_name")
            .unwrap_or(u32::MAX);
        let recv_idx = query
            .capture_index_for_name("recv_type")
            .unwrap_or(u32::MAX);
        let struct_idx = query
            .capture_index_for_name("struct_name")
            .unwrap_or(u32::MAX);
        let iface_idx = query
            .capture_index_for_name("iface_name")
            .unwrap_or(u32::MAX);
        let type_idx = query
            .capture_index_for_name("type_name")
            .unwrap_or(u32::MAX);
        let alias_idx = query
            .capture_index_for_name("alias_name")
            .unwrap_or(u32::MAX);

        DeclQuery {
            query,
            fn_idx,
            method_idx,
            recv_idx,
            struct_idx,
            iface_idx,
            type_idx,
            alias_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&decl.query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;
            let name = cap_node.utf8_text(source).unwrap_or_default().to_string();

            if idx == decl.fn_idx {
                let outer =
                    find_ancestor_of_kind(&cap_node, "function_declaration").unwrap_or(cap_node);
                result.symbols.push(Symbol {
                    name,
                    kind: "function".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: find_comment(&outer, source),
                    parent: String::new(),
                    technology: String::new(),
                    annotations: Vec::new(),
                });
            } else if idx == decl.method_idx {
                let recv_type = m
                    .captures
                    .iter()
                    .find(|c| c.index == decl.recv_idx)
                    .map(|c| {
                        let t = c.node.utf8_text(source).unwrap_or_default();
                        t.trim_start_matches('*').to_string()
                    })
                    .unwrap_or_default();
                let outer =
                    find_ancestor_of_kind(&cap_node, "method_declaration").unwrap_or(cap_node);
                result.symbols.push(Symbol {
                    name,
                    kind: "method".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: find_comment(&outer, source),
                    parent: recv_type,
                    technology: String::new(),
                    annotations: Vec::new(),
                });
            } else if idx == decl.struct_idx {
                let outer = find_ancestor_of_kind(&cap_node, "type_spec").unwrap_or(cap_node);
                result.symbols.push(Symbol {
                    name,
                    kind: "struct".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: find_comment(&outer, source),
                    parent: String::new(),
                    technology: String::new(),
                    annotations: Vec::new(),
                });
            } else if idx == decl.iface_idx {
                let outer = find_ancestor_of_kind(&cap_node, "type_spec").unwrap_or(cap_node);
                result.symbols.push(Symbol {
                    name,
                    kind: "interface".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: find_comment(&outer, source),
                    parent: String::new(),
                    technology: String::new(),
                    annotations: Vec::new(),
                });
            } else if idx == decl.type_idx {
                // Check whether it's already covered by struct/iface captures.
                let outer = find_ancestor_of_kind(&cap_node, "type_spec").unwrap_or(cap_node);
                let type_val = outer.child_by_field_name("type");
                let is_struct_or_iface =
                    type_val.is_some_and(|t| matches!(t.kind(), "struct_type" | "interface_type"));
                if !is_struct_or_iface {
                    result.symbols.push(Symbol {
                        name,
                        kind: "type".to_string(),
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        end_line: (outer.end_position().row + 1) as i32,
                        description: String::new(),
                        parent: String::new(),
                        technology: String::new(),
                        annotations: Vec::new(),
                    });
                }
            } else if idx == decl.alias_idx {
                let outer = find_ancestor_of_kind(&cap_node, "type_alias").unwrap_or(cap_node);
                result.symbols.push(Symbol {
                    name,
                    kind: "type".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: String::new(),
                    parent: String::new(),
                    technology: String::new(),
                    annotations: Vec::new(),
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

static REF_QUERY: OnceLock<RefQuery> = OnceLock::new();

fn parse_refs(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    let rq = REF_QUERY.get_or_init(|| {
        let query_src = queries::go_references();
        let query = Query::new(language, query_src).expect("Failed to compile Go ref query");
        let import_idx = query
            .capture_index_for_name("import_path")
            .unwrap_or(u32::MAX);
        let callee_idx = query.capture_index_for_name("callee").unwrap_or(u32::MAX);
        RefQuery {
            query,
            import_idx,
            callee_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&rq.query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == rq.import_idx {
                let text = cap_node.utf8_text(source).unwrap_or_default().trim();
                let import_path = text.trim_matches('"');
                if !import_path.is_empty() {
                    let name = std::path::Path::new(import_path)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(import_path)
                        .to_string();
                    result.refs.push(Ref {
                        name,
                        kind: "import".to_string(),
                        target_path: import_path.to_string(),
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        column: (cap_node.start_position().column + 1) as i32,
                        receiver: String::new(),
                    });
                }
            } else if idx == rq.callee_idx {
                let (name, receiver) = go_call_info(&cap_node, source);
                if !name.is_empty() {
                    result.refs.push(Ref {
                        name,
                        kind: "call".to_string(),
                        target_path: String::new(),
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        column: (cap_node.start_position().column + 1) as i32,
                        receiver,
                    });
                }
            }
        }
    }
}

/// Walk up the tree to find the first ancestor of the given node kind.
fn find_ancestor_of_kind<'a>(node: &Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut current = node.parent()?;
    loop {
        if current.kind() == kind {
            return Some(current);
        }
        current = current.parent()?;
    }
}

fn find_comment(node: &Node, source: &[u8]) -> String {
    if let Some(prev) = node.prev_named_sibling() {
        if prev.kind() == "comment"
            && node
                .start_position()
                .row
                .saturating_sub(prev.end_position().row)
                <= 1
        {
            let text = prev.utf8_text(source).unwrap_or_default().trim();
            let text = text.strip_prefix("//").unwrap_or(text).trim();
            let text = text.strip_prefix("/*").unwrap_or(text);
            let text = text.strip_suffix("*/").unwrap_or(text);
            return text.trim().to_string();
        }
    }
    String::new()
}

fn go_call_info(node: &Node, source: &[u8]) -> (String, String) {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" => (
            node.utf8_text(source).unwrap_or_default().to_string(),
            String::new(),
        ),
        "selector_expression" => {
            let name = node
                .child_by_field_name("field")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            let receiver = node
                .child_by_field_name("operand")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            (name, receiver)
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            if let Some(child) = node.named_children(&mut cursor).next() {
                go_call_info(&child, source)
            } else {
                (String::new(), String::new())
            }
        }
        _ => {
            let text = node.utf8_text(source).unwrap_or_default().trim();
            if text.is_empty() {
                return (String::new(), String::new());
            }
            let (receiver, name_part) = text.rsplit_once('.').map_or_else(
                || (String::new(), text.to_string()),
                |(r, n)| (r.to_string(), n.to_string()),
            );
            let name_part = name_part
                .split('[')
                .next()
                .unwrap_or(&name_part)
                .trim()
                .to_string();
            (name_part, receiver)
        }
    }
}
