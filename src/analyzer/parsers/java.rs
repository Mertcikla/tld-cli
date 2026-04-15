#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::map_unwrap_or
)]
use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use tree_sitter::{Language, Node, Query, QueryCursor, StreamingIterator};

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
    let decl_query_src = r"
(class_declaration
  name: (identifier) @class_name
  body: (class_body) @class_body) @class_decl

(interface_declaration
  name: (identifier) @iface_name
  body: (interface_body) @iface_body) @iface_decl
";

    let Ok(query) = Query::new(language, decl_query_src) else {
        return;
    };

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

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *node, source);

    while let Some(m) = matches.next() {
        let mut class_name_node: Option<Node> = None;
        let mut class_body_node: Option<Node> = None;
        let mut iface_name_node: Option<Node> = None;
        let mut iface_body_node: Option<Node> = None;

        for cap in m.captures {
            match cap.index {
                i if i == class_name_idx => class_name_node = Some(cap.node),
                i if i == class_body_idx => class_body_node = Some(cap.node),
                i if i == iface_name_idx => iface_name_node = Some(cap.node),
                i if i == iface_body_idx => iface_body_node = Some(cap.node),
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
            });

            if let Some(body_node) = iface_body_node {
                parse_interface_members(&body_node, source, path, language, &iface_name, result);
            }
        }
    }
}

fn parse_class_members(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    class_name: &str,
    result: &mut AnalysisResult,
) {
    let member_query_src = r"
(method_declaration
  name: (identifier) @method_name) @method_decl

(constructor_declaration
  name: (identifier) @ctor_name) @ctor_decl
";

    let Ok(query) = Query::new(language, member_query_src) else {
        return;
    };

    let method_idx = query
        .capture_index_for_name("method_name")
        .unwrap_or(u32::MAX);
    let ctor_idx = query
        .capture_index_for_name("ctor_name")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *body_node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == method_idx {
                let decl_node = cap_node.parent().unwrap_or(cap_node);
                // Only direct children of class body.
                if decl_node
                    .parent()
                    .map(|p| p.id() != body_node.id())
                    .unwrap_or(true)
                {
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
                });
            } else if idx == ctor_idx {
                let decl_node = cap_node.parent().unwrap_or(cap_node);
                if decl_node
                    .parent()
                    .map(|p| p.id() != body_node.id())
                    .unwrap_or(true)
                {
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
                });
            }
        }
    }
}

fn parse_interface_members(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    iface_name: &str,
    result: &mut AnalysisResult,
) {
    let method_query_src = r"
(method_declaration
  name: (identifier) @method_name) @method_decl
";

    let Ok(query) = Query::new(language, method_query_src) else {
        return;
    };

    let method_idx = query
        .capture_index_for_name("method_name")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *body_node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if cap.index == method_idx {
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
                });
            }
        }
    }
}

fn parse_refs(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    let ref_query_src = r"
(import_declaration) @import

(method_invocation
  name: (identifier) @call_name)
";

    let Ok(query) = Query::new(language, ref_query_src) else {
        return;
    };

    let import_idx = query.capture_index_for_name("import").unwrap_or(u32::MAX);
    let call_idx = query
        .capture_index_for_name("call_name")
        .unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == import_idx {
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
                    });
                }
            } else if idx == call_idx {
                let name = cap_node.utf8_text(source).unwrap_or_default().to_string();
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

fn find_comment(node: &Node, source: &[u8]) -> String {
    if let Some(prev) = node.prev_named_sibling()
        && (prev.kind() == "line_comment" || prev.kind() == "block_comment")
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
    String::new()
}
