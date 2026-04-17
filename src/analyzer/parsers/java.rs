#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::expect_used,
    clippy::collapsible_if
)]
use crate::analyzer::queries;
use crate::analyzer::types::{AnalysisResult, Annotation, Ref, Symbol};
use std::sync::OnceLock;
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator};

struct DeclQuery {
    query: Query,
    class_name_idx: u32,
    class_body_idx: u32,
    iface_name_idx: u32,
    iface_body_idx: u32,
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
        let decl_query_src = queries::java_declarations();
        let query =
            Query::new(language, decl_query_src).expect("Failed to compile Java decl query");
        let class_name_idx = query
            .capture_index_for_name("class_name")
            .unwrap_or(u32::MAX);
        let class_body_idx = query
            .capture_index_for_name("class_body")
            .unwrap_or(u32::MAX);
        let iface_name_idx = query
            .capture_index_for_name("iface_name")
            .unwrap_or(u32::MAX);
        let iface_body_idx = query
            .capture_index_for_name("iface_body")
            .unwrap_or(u32::MAX);

        DeclQuery {
            query,
            class_name_idx,
            class_body_idx,
            iface_name_idx,
            iface_body_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&decl.query, *node, source);

    while let Some(m) = matches.next() {
        let mut class_name_node: Option<Node> = None;
        let mut class_body_node: Option<Node> = None;
        let mut iface_name_node: Option<Node> = None;
        let mut iface_body_node: Option<Node> = None;

        for cap in m.captures {
            match cap.index {
                i if i == decl.class_name_idx => class_name_node = Some(cap.node),
                i if i == decl.class_body_idx => class_body_node = Some(cap.node),
                i if i == decl.iface_name_idx => iface_name_node = Some(cap.node),
                i if i == decl.iface_body_idx => iface_body_node = Some(cap.node),
                _ => {}
            }
        }

        if let Some(name_node) = class_name_node {
            let class_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: class_name.clone(),
                kind: "class".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_java_annotations(&outer, source),
            });

            if let Some(body_node) = class_body_node {
                parse_class_members(&body_node, source, path, language, &class_name, result);
            }
        } else if let Some(name_node) = iface_name_node {
            let iface_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer = name_node.parent().unwrap_or(name_node);
            result.symbols.push(Symbol {
                name: iface_name.clone(),
                kind: "interface".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description: find_comment(&outer, source),
                parent: String::new(),
                technology: String::new(),
                annotations: extract_java_annotations(&outer, source),
            });

            if let Some(body_node) = iface_body_node {
                parse_interface_members(&body_node, source, path, language, &iface_name, result);
            }
        }
    }
}

struct MemberQuery {
    query: Query,
    method_idx: u32,
    ctor_idx: u32,
}

static MEMBER_QUERY: OnceLock<MemberQuery> = OnceLock::new();

fn parse_class_members(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    class_name: &str,
    result: &mut AnalysisResult,
) {
    let mq = MEMBER_QUERY.get_or_init(|| {
        let member_query_src = queries::java_class_members();
        let query =
            Query::new(language, member_query_src).expect("Failed to compile Java member query");
        let method_idx = query
            .capture_index_for_name("method_name")
            .unwrap_or(u32::MAX);
        let ctor_idx = query
            .capture_index_for_name("ctor_name")
            .unwrap_or(u32::MAX);
        MemberQuery {
            query,
            method_idx,
            ctor_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&mq.query, *body_node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == mq.method_idx {
                let decl_node = cap_node.parent().unwrap_or(cap_node);
                // Only direct children of class body.
                if decl_node.parent().is_none_or(|p| p.id() != body_node.id()) {
                    continue;
                }
                result.symbols.push(Symbol {
                    name: cap_node.utf8_text(source).unwrap_or_default().to_string(),
                    kind: "method".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (decl_node.end_position().row + 1) as i32,
                    description: find_comment(&decl_node, source),
                    parent: class_name.to_string(),
                    technology: String::new(),
                    annotations: extract_java_annotations(&decl_node, source),
                });
            } else if idx == mq.ctor_idx {
                let decl_node = cap_node.parent().unwrap_or(cap_node);
                if decl_node.parent().is_none_or(|p| p.id() != body_node.id()) {
                    continue;
                }
                result.symbols.push(Symbol {
                    name: cap_node.utf8_text(source).unwrap_or_default().to_string(),
                    kind: "constructor".to_string(),
                    file_path: path.to_string(),
                    line: (cap_node.start_position().row + 1) as i32,
                    end_line: (decl_node.end_position().row + 1) as i32,
                    description: find_comment(&decl_node, source),
                    parent: class_name.to_string(),
                    technology: String::new(),
                    annotations: extract_java_annotations(&decl_node, source),
                });
            }
        }
    }
}

struct InterfaceMemberQuery {
    query: Query,
    method_idx: u32,
}

static INTERFACE_MEMBER_QUERY: OnceLock<InterfaceMemberQuery> = OnceLock::new();

fn parse_interface_members(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    iface_name: &str,
    result: &mut AnalysisResult,
) {
    let imq = INTERFACE_MEMBER_QUERY.get_or_init(|| {
        let method_query_src = queries::java_interface_members();
        let query = Query::new(language, method_query_src)
            .expect("Failed to compile Java interface member query");
        let method_idx = query
            .capture_index_for_name("method_name")
            .unwrap_or(u32::MAX);
        InterfaceMemberQuery { query, method_idx }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&imq.query, *body_node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if cap.index == imq.method_idx {
                let name_node = cap.node;
                let decl_node = name_node.parent().unwrap_or(name_node);
                result.symbols.push(Symbol {
                    name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                    kind: "method".to_string(),
                    file_path: path.to_string(),
                    line: (name_node.start_position().row + 1) as i32,
                    end_line: (decl_node.end_position().row + 1) as i32,
                    description: String::new(),
                    parent: iface_name.to_string(),
                    technology: String::new(),
                    annotations: extract_java_annotations(&decl_node, source),
                });
            }
        }
    }
}

struct RefQuery {
    query: Query,
    import_idx: u32,
    call_idx: u32,
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
        let ref_query_src = queries::java_references();
        let query = Query::new(language, ref_query_src).expect("Failed to compile Java ref query");
        let import_idx = query.capture_index_for_name("import").unwrap_or(u32::MAX);
        let call_idx = query
            .capture_index_for_name("call_name")
            .unwrap_or(u32::MAX);
        RefQuery {
            query,
            import_idx,
            call_idx,
        }
    });

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&rq.query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == rq.import_idx {
                let text = cap_node.utf8_text(source).unwrap_or_default();
                let import_path = text
                    .trim_start_matches("import ")
                    .trim_start_matches("static ")
                    .trim_end_matches(';')
                    .trim();
                if !import_path.is_empty() {
                    let name = import_path
                        .rsplit('.')
                        .next()
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
            } else if idx == rq.call_idx {
                let name = cap_node.utf8_text(source).unwrap_or_default().to_string();
                let receiver = cap_node
                    .parent()
                    .and_then(|p| p.child_by_field_name("object"))
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

fn extract_java_annotations(decl_node: &Node, source: &[u8]) -> Vec<Annotation> {
    let mut out = Vec::new();
    let mut cursor = decl_node.walk();
    for child in decl_node.named_children(&mut cursor) {
        if child.kind() != "modifiers" {
            continue;
        }
        let mut mc = child.walk();
        for mchild in child.named_children(&mut mc) {
            match mchild.kind() {
                "marker_annotation" => {
                    let name = mchild
                        .child_by_field_name("name")
                        .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                        .unwrap_or_default();
                    if !name.is_empty() {
                        out.push(Annotation {
                            name,
                            args: Vec::new(),
                        });
                    }
                }
                "annotation" => {
                    let name = mchild
                        .child_by_field_name("name")
                        .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                        .unwrap_or_default();
                    let args = mchild
                        .child_by_field_name("arguments")
                        .map(|args_node| {
                            let mut ac = args_node.walk();
                            args_node
                                .named_children(&mut ac)
                                .map(|c| c.utf8_text(source).unwrap_or_default().to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    if !name.is_empty() {
                        out.push(Annotation { name, args });
                    }
                }
                _ => {}
            }
        }
    }
    out
}

fn find_comment(node: &Node, source: &[u8]) -> String {
    if let Some(prev) = node.prev_named_sibling() {
        if (prev.kind() == "line_comment" || prev.kind() == "block_comment")
            && node
                .start_position()
                .row
                .saturating_sub(prev.end_position().row)
                <= 1
        {
            let text = prev.utf8_text(source).unwrap_or_default().trim();
            let text = text.strip_prefix("//").unwrap_or(text);
            let text = text
                .strip_prefix("/*")
                .unwrap_or(text)
                .strip_suffix("*/")
                .unwrap_or(text);
            return text.trim().to_string();
        }
    }
    String::new()
}
