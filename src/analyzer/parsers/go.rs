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
    let query_src = r#"
(function_declaration
  name: (identifier) @fn_name) @fn

(method_declaration
  receiver: (parameter_list
    (parameter_declaration
      type: [
        (type_identifier) @recv_type
        (pointer_type (type_identifier) @recv_type)
      ]))
  name: (field_identifier) @method_name) @method

(type_spec
  name: (type_identifier) @struct_name
  type: (struct_type)) @struct_decl

(type_spec
  name: (type_identifier) @iface_name
  type: (interface_type)) @iface_decl

(type_spec
  name: (type_identifier) @type_name) @type_decl

(type_alias
  name: (type_identifier) @alias_name) @alias_decl
"#;

    let query = match Query::new(language, query_src) {
        Ok(q) => q,
        Err(_) => return,
    };

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

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;
            let name = cap_node.utf8_text(source).unwrap_or_default().to_string();

            if idx == fn_idx {
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
                });
            } else if idx == method_idx {
                let recv_type = m
                    .captures
                    .iter()
                    .find(|c| c.index == recv_idx)
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
                });
            } else if idx == struct_idx {
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
                });
            } else if idx == iface_idx {
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
                });
            } else if idx == type_idx {
                // Check whether it's already covered by struct/iface captures.
                let outer = find_ancestor_of_kind(&cap_node, "type_spec").unwrap_or(cap_node);
                let type_val = outer.child_by_field_name("type");
                let is_struct_or_iface = type_val
                    .map(|t| matches!(t.kind(), "struct_type" | "interface_type"))
                    .unwrap_or(false);
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
                    });
                }
            } else if idx == alias_idx {
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
    let query_src = r#"
(import_spec path: (interpreted_string_literal) @import_path)
(call_expression function: _ @callee)
"#;

    let query = match Query::new(language, query_src) {
        Ok(q) => q,
        Err(_) => return,
    };

    let import_idx = query
        .capture_index_for_name("import_path")
        .unwrap_or(u32::MAX);
    let callee_idx = query.capture_index_for_name("callee").unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == import_idx {
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
                    });
                }
            } else if idx == callee_idx {
                let name = go_call_name(&cap_node, source);
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
    if let Some(prev) = node.prev_named_sibling()
        && prev.kind() == "comment"
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
    String::new()
}

fn go_call_name(node: &Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "field_identifier" | "type_identifier" => {
            node.utf8_text(source).unwrap_or_default().to_string()
        }
        "selector_expression" => {
            if let Some(field_node) = node.child_by_field_name("field") {
                field_node.utf8_text(source).unwrap_or_default().to_string()
            } else {
                String::new()
            }
        }
        "parenthesized_expression" => {
            let mut cursor = node.walk();
            if let Some(child) = node.named_children(&mut cursor).next() {
                go_call_name(&child, source)
            } else {
                String::new()
            }
        }
        _ => {
            let text = node.utf8_text(source).unwrap_or_default().trim();
            if text.is_empty() {
                return String::new();
            }
            let mut text = text.to_string();
            if let Some(index) = text.rfind('.') {
                text = text[index + 1..].to_string();
            }
            if let Some(index) = text.find('[') {
                text = text[..index].to_string();
            }
            text.trim().to_string()
        }
    }
}
