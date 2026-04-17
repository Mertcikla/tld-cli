#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::expect_used
)]
use crate::analyzer::queries;
use crate::analyzer::types::{AnalysisResult, Annotation, Ref, Symbol};
use std::sync::OnceLock;
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator};

struct DeclQuery {
    query: Query,
    fn_idx: u32,
    struct_idx: u32,
    enum_idx: u32,
    trait_idx: u32,
    type_idx: u32,
    impl_idx: u32,
    impl_type_idx: u32,
    use_idx: u32,
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
        let decl_query_src = queries::rust_declarations();
        let query =
            Query::new(language, decl_query_src).expect("Failed to compile Rust decl query");
        let fn_idx = query.capture_index_for_name("fn_name").unwrap_or(u32::MAX);
        let struct_idx = query
            .capture_index_for_name("struct_name")
            .unwrap_or(u32::MAX);
        let enum_idx = query
            .capture_index_for_name("enum_name")
            .unwrap_or(u32::MAX);
        let trait_idx = query
            .capture_index_for_name("trait_name")
            .unwrap_or(u32::MAX);
        let type_idx = query
            .capture_index_for_name("type_name")
            .unwrap_or(u32::MAX);
        let impl_idx = query.capture_index_for_name("impl").unwrap_or(u32::MAX);
        let impl_type_idx = query
            .capture_index_for_name("impl_type")
            .unwrap_or(u32::MAX);
        let use_idx = query.capture_index_for_name("use_arg").unwrap_or(u32::MAX);

        DeclQuery {
            query,
            fn_idx,
            struct_idx,
            enum_idx,
            trait_idx,
            type_idx,
            impl_idx,
            impl_type_idx,
            use_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&decl.query, *node, source);

    while let Some(m) = matches.next() {
        let mut fn_name_node: Option<Node> = None;
        let mut struct_name_node: Option<Node> = None;
        let mut enum_name_node: Option<Node> = None;
        let mut trait_name_node: Option<Node> = None;
        let mut type_name_node: Option<Node> = None;
        let mut impl_node: Option<Node> = None;
        let mut impl_type_node: Option<Node> = None;
        let mut use_node: Option<Node> = None;

        for cap in m.captures {
            match cap.index {
                i if i == decl.fn_idx => fn_name_node = Some(cap.node),
                i if i == decl.struct_idx => struct_name_node = Some(cap.node),
                i if i == decl.enum_idx => enum_name_node = Some(cap.node),
                i if i == decl.trait_idx => trait_name_node = Some(cap.node),
                i if i == decl.type_idx => type_name_node = Some(cap.node),
                i if i == decl.impl_idx => impl_node = Some(cap.node),
                i if i == decl.impl_type_idx => impl_type_node = Some(cap.node),
                i if i == decl.use_idx => use_node = Some(cap.node),
                _ => {}
            }
        }

        if let Some(name_node) = fn_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            // Skip if inside an impl block (handled separately)
            if is_inside_impl(&outer) {
                continue;
            }
            result.symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "function".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_doc_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_rust_annotations(&outer, source),
            });
        } else if let Some(name_node) = struct_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "struct".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_doc_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_rust_annotations(&outer, source),
            });
        } else if let Some(name_node) = enum_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "enum".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_doc_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_rust_annotations(&outer, source),
            });
        } else if let Some(name_node) = trait_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "trait".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_doc_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_rust_annotations(&outer, source),
            });
        } else if let Some(name_node) = type_name_node {
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                kind: "type".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_doc_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_rust_annotations(&outer, source),
            });
        } else if let Some(node) = impl_node {
            if let Some(type_node) = impl_type_node {
                let parent = type_name_from_node(&type_node, source);
                parse_impl_members(&node, source, path, language, &parent, result);
            }
        } else if let Some(node) = use_node {
            collect_use_paths(&node, source, path, result);
        }
    }
}

fn is_inside_impl(node: &Node) -> bool {
    let mut current = node.parent();
    while let Some(p) = current {
        if p.kind() == "impl_item" {
            return true;
        }
        current = p.parent();
    }
    false
}

struct ImplMemberQuery {
    query: Query,
    name_idx: u32,
}

static IMPL_MEMBER_QUERY: OnceLock<ImplMemberQuery> = OnceLock::new();

fn parse_impl_members(
    impl_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    parent_type: &str,
    result: &mut AnalysisResult,
) {
    let imq = IMPL_MEMBER_QUERY.get_or_init(|| {
        let query_src = queries::rust_impl_members();
        let query =
            Query::new(language, query_src).expect("Failed to compile Rust impl member query");
        let name_idx = query.capture_index_for_name("fn_name").unwrap_or(u32::MAX);
        ImplMemberQuery { query, name_idx }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&imq.query, *impl_node, source);
    while let Some(m) = matches.next() {
        for cap in m.captures {
            if cap.index == imq.name_idx {
                let name_node = cap.node;
                let outer = name_node.parent().unwrap_or(name_node);
                result.symbols.push(Symbol {
                    name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                    kind: "method".to_string(),
                    file_path: path.to_string(),
                    line: (name_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description: find_doc_comment(&outer, source),
                    parent: parent_type.to_string(),
                    technology: String::new(),
                    annotations: extract_rust_annotations(&outer, source),
                });
            }
        }
    }
}

struct RefQuery {
    query: Query,
    call_idx: u32,
    method_idx: u32,
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
        let query_src = queries::rust_references();
        let query = Query::new(language, query_src).expect("Failed to compile Rust ref query");
        let call_idx = query.capture_index_for_name("callee").unwrap_or(u32::MAX);
        let method_idx = query
            .capture_index_for_name("method_name")
            .unwrap_or(u32::MAX);
        RefQuery {
            query,
            call_idx,
            method_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&rq.query, *node, source);
    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;
            if idx == rq.call_idx {
                let (name, receiver) = rust_call_info(&cap_node, source);
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
            } else if idx == rq.method_idx {
                let name = cap_node.utf8_text(source).unwrap_or_default().to_string();
                let receiver = cap_node
                    .parent()
                    .and_then(|p| p.child_by_field_name("value"))
                    .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                    .unwrap_or_default();
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

fn type_name_from_node(node: &Node, source: &[u8]) -> String {
    match node.kind() {
        "type_identifier" | "identifier" => node.utf8_text(source).unwrap_or_default().to_string(),
        "generic_type" => node
            .child_by_field_name("type")
            .or_else(|| {
                let mut cursor = node.walk();
                node.named_children(&mut cursor)
                    .find(|c| c.kind() == "type_identifier")
            })
            .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
            .unwrap_or_default(),
        "scoped_type_identifier" => {
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .filter(|c| c.kind() == "type_identifier")
                .last()
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default()
        }
        _ => {
            let text = node.utf8_text(source).unwrap_or_default();
            text.split('<').next().unwrap_or(text).trim().to_string()
        }
    }
}

fn rust_call_info(node: &Node, source: &[u8]) -> (String, String) {
    match node.kind() {
        "identifier" => (
            node.utf8_text(source).unwrap_or_default().to_string(),
            String::new(),
        ),
        "field_expression" => {
            let name = node
                .child_by_field_name("field")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            let receiver = node
                .child_by_field_name("value")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            (name, receiver)
        }
        "scoped_identifier" => {
            let name = node.child_by_field_name("name").map_or_else(
                || {
                    let text = node.utf8_text(source).unwrap_or_default();
                    text.rsplit("::").next().unwrap_or(text).trim().to_string()
                },
                |n| n.utf8_text(source).unwrap_or_default().to_string(),
            );
            let receiver = node
                .child_by_field_name("path")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default();
            (name, receiver)
        }
        "generic_function" => node
            .child_by_field_name("function")
            .map(|n| rust_call_info(&n, source))
            .unwrap_or_default(),
        _ => {
            let text = node.utf8_text(source).unwrap_or_default();
            let (receiver, name_part) = text.rsplit_once("::").map_or_else(
                || (String::new(), text.to_string()),
                |(r, n)| (r.to_string(), n.to_string()),
            );
            let name = name_part
                .split('<')
                .next()
                .unwrap_or(&name_part)
                .trim()
                .to_string();
            (name, receiver)
        }
    }
}

fn collect_use_paths(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect_use_paths(&child, source, path, result);
            }
        }
        "scoped_identifier" | "identifier" | "scoped_use_list" => {
            let text = node.utf8_text(source).unwrap_or_default();
            let target_path = text.replace("::", "/");
            let name = text.rsplit("::").next().unwrap_or(text).to_string();
            if !name.is_empty() && name != "_" && name != "self" {
                result.refs.push(Ref {
                    name,
                    kind: "import".to_string(),
                    target_path,
                    file_path: path.to_string(),
                    line: (node.start_position().row + 1) as i32,
                    column: (node.start_position().column + 1) as i32,
                    receiver: String::new(),
                });
            }
        }
        "use_wildcard" => {
            let text = node.utf8_text(source).unwrap_or_default();
            let text = text.trim_end_matches("::*").trim_end_matches("/*");
            if !text.is_empty() {
                let target_path = text.replace("::", "/");
                let name = text.rsplit("::").next().unwrap_or(text).to_string();
                result.refs.push(Ref {
                    name,
                    kind: "import".to_string(),
                    target_path,
                    file_path: path.to_string(),
                    line: (node.start_position().row + 1) as i32,
                    column: (node.start_position().column + 1) as i32,
                    receiver: String::new(),
                });
            }
        }
        _ => {}
    }
}

fn extract_rust_annotations(decl_node: &Node, source: &[u8]) -> Vec<Annotation> {
    let mut attrs: Vec<Node> = Vec::new();
    let mut current = decl_node.prev_named_sibling();
    while let Some(sib) = current {
        if sib.kind() == "attribute_item" || sib.kind() == "inner_attribute_item" {
            attrs.push(sib);
            current = sib.prev_named_sibling();
        } else {
            break;
        }
    }
    attrs.reverse();
    attrs
        .into_iter()
        .filter_map(|n| parse_rust_attribute(&n, source))
        .collect()
}

fn parse_rust_attribute(attr_item: &Node, source: &[u8]) -> Option<Annotation> {
    let text = attr_item.utf8_text(source).unwrap_or_default().trim();
    let inner = text
        .strip_prefix("#!")
        .or_else(|| text.strip_prefix('#'))?;
    let inner = inner.strip_prefix('[')?;
    let inner = inner.strip_suffix(']')?.trim();
    if let Some(paren_idx) = inner.find('(') {
        let name = inner[..paren_idx].trim().to_string();
        let args_text = &inner[paren_idx + 1..];
        let args_text = args_text.strip_suffix(')').unwrap_or(args_text);
        let args: Vec<String> = split_top_level_commas(args_text)
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if name.is_empty() {
            None
        } else {
            Some(Annotation { name, args })
        }
    } else {
        let name = inner.to_string();
        if name.is_empty() {
            None
        } else {
            Some(Annotation {
                name,
                args: Vec::new(),
            })
        }
    }
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth: i32 = 0;
    let mut start = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    if start <= s.len() {
        out.push(&s[start..]);
    }
    out
}

fn find_doc_comment(node: &Node, source: &[u8]) -> String {
    if let Some(prev) = node.prev_named_sibling() {
        let kind = prev.kind();
        if (kind == "line_comment" || kind == "block_comment" || kind == "doc_comment")
            && node
                .start_position()
                .row
                .saturating_sub(prev.end_position().row)
                <= 1
        {
            let text = prev.utf8_text(source).unwrap_or_default().trim();
            let text = text
                .lines()
                .map(|l| {
                    l.trim()
                        .strip_prefix("///")
                        .or_else(|| l.trim().strip_prefix("//!"))
                        .or_else(|| l.trim().strip_prefix("//"))
                        .or_else(|| l.trim().strip_prefix("/**"))
                        .or_else(|| l.trim().strip_prefix("/*"))
                        .unwrap_or(l.trim())
                        .trim_end_matches("*/")
                        .trim()
                })
                .collect::<Vec<_>>()
                .join(" ")
                .trim()
                .to_string();
            return text;
        }
    }
    String::new()
}
