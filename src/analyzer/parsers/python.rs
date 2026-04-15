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
    let decl_query_src = r#"
(class_definition
  name: (identifier) @class_name
  body: (block) @class_body) @class_def

(function_definition
  name: (identifier) @fn_name) @fn_def
"#;

    let query = match Query::new(language, decl_query_src) {
        Ok(q) => q,
        Err(_) => return,
    };

    let class_name_idx = query
        .capture_index_for_name("class_name")
        .unwrap_or(u32::MAX);
    let class_body_idx = query
        .capture_index_for_name("class_body")
        .unwrap_or(u32::MAX);
    let fn_idx = query.capture_index_for_name("fn_name").unwrap_or(u32::MAX);

    // Collect class body byte ranges to skip functions nested inside classes
    // when processing top-level function matches.
    let mut class_body_ranges: Vec<(usize, usize)> = Vec::new();

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *node, source);

    while let Some(m) = matches.next() {
        let mut class_name_node: Option<Node> = None;
        let mut class_body_node: Option<Node> = None;
        let mut fn_name_node: Option<Node> = None;

        for cap in m.captures {
            match cap.index {
                i if i == class_name_idx => class_name_node = Some(cap.node),
                i if i == class_body_idx => class_body_node = Some(cap.node),
                i if i == fn_idx => fn_name_node = Some(cap.node),
                _ => {}
            }
        }

        if let Some(name_node) = class_name_node {
            let class_name = name_node.utf8_text(source).unwrap_or_default().to_string();
            let outer = name_node.parent().unwrap_or(name_node);
            let description = class_body_node
                .map(|b| extract_docstring(&b, source))
                .unwrap_or_default();
            result.symbols.push(Symbol {
                name: class_name.clone(),
                kind: "class".to_string(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                end_line: (outer.end_position().row + 1) as i32,
                description,
                parent: String::new(),
                technology: String::new(),
            });

            if let Some(body_node) = class_body_node {
                class_body_ranges.push((body_node.start_byte(), body_node.end_byte()));
                parse_class_methods(&body_node, source, path, language, &class_name, result);
            }
        } else if let Some(name_node) = fn_name_node {
            // Only emit if not inside a class body.
            let fn_start = name_node.start_byte();
            let inside_class = class_body_ranges
                .iter()
                .any(|(s, e)| fn_start >= *s && fn_start < *e);
            if !inside_class {
                let outer = name_node.parent().unwrap_or(name_node);
                let body = outer.child_by_field_name("body");
                let description = body
                    .map(|b| extract_docstring(&b, source))
                    .unwrap_or_default();
                result.symbols.push(Symbol {
                    name: name_node.utf8_text(source).unwrap_or_default().to_string(),
                    kind: "function".to_string(),
                    file_path: path.to_string(),
                    line: (name_node.start_position().row + 1) as i32,
                    end_line: (outer.end_position().row + 1) as i32,
                    description,
                    parent: String::new(),
                    technology: String::new(),
                });
            }
        }
    }
}

fn parse_class_methods(
    body_node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    class_name: &str,
    result: &mut AnalysisResult,
) {
    let method_query_src = r#"
(function_definition
  name: (identifier) @method_name) @method_def
"#;

    let query = match Query::new(language, method_query_src) {
        Ok(q) => q,
        Err(_) => return,
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
                // Only direct children of class body (not nested class methods).
                // Also accept decorated_definition wrappers (e.g. @staticmethod).
                let fn_node = name_node.parent().unwrap_or(name_node);
                let fn_parent = fn_node.parent();
                let is_direct = fn_parent
                    .map(|p| {
                        if p.id() == body_node.id() {
                            true
                        } else if p.kind() == "decorated_definition" {
                            p.parent()
                                .map(|gp| gp.id() == body_node.id())
                                .unwrap_or(false)
                        } else {
                            false
                        }
                    })
                    .unwrap_or(false);
                if !is_direct {
                    continue;
                }

                let name = name_node.utf8_text(source).unwrap_or_default().to_string();
                let kind = if name == "__init__" {
                    "constructor"
                } else {
                    "method"
                };
                let fn_body = fn_node.child_by_field_name("body");
                let description = fn_body
                    .map(|b| extract_docstring(&b, source))
                    .unwrap_or_default();
                result.symbols.push(Symbol {
                    name,
                    kind: kind.to_string(),
                    file_path: path.to_string(),
                    line: (name_node.start_position().row + 1) as i32,
                    end_line: (fn_node.end_position().row + 1) as i32,
                    description,
                    parent: class_name.to_string(),
                    technology: String::new(),
                });
            }
        }
    }
}

fn extract_docstring(body_node: &Node, source: &[u8]) -> String {
    let mut cursor = body_node.walk();
    for child in body_node.named_children(&mut cursor) {
        if child.kind() == "expression_statement" {
            let mut c2 = child.walk();
            for inner in child.named_children(&mut c2) {
                if inner.kind() == "string" {
                    let text = inner.utf8_text(source).unwrap_or_default();
                    let text = text.trim_start_matches("\"\"\"").trim_end_matches("\"\"\"");
                    let text = text.trim_start_matches("'''").trim_end_matches("'''");
                    let text = text.trim_matches('"').trim_matches('\'');
                    return text.trim().to_string();
                }
            }
        }
        break; // Only check first statement
    }
    String::new()
}

fn parse_refs(
    node: &Node,
    source: &[u8],
    path: &str,
    language: &Language,
    result: &mut AnalysisResult,
) {
    let ref_query_src = r#"
(import_from_statement
  module_name: _ @module)

(import_statement
  name: (dotted_name) @import_name)

(call
  function: _ @callee)
"#;

    let query = match Query::new(language, ref_query_src) {
        Ok(q) => q,
        Err(_) => return,
    };

    let module_idx = query.capture_index_for_name("module").unwrap_or(u32::MAX);
    let import_idx = query
        .capture_index_for_name("import_name")
        .unwrap_or(u32::MAX);
    let callee_idx = query.capture_index_for_name("callee").unwrap_or(u32::MAX);

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, *node, source);

    while let Some(m) = matches.next() {
        for cap in m.captures {
            let cap_node = cap.node;
            let idx = cap.index;

            if idx == module_idx || idx == import_idx {
                let module = cap_node.utf8_text(source).unwrap_or_default().to_string();
                if !module.is_empty() {
                    result.refs.push(Ref {
                        name: module.clone(),
                        kind: "import".to_string(),
                        target_path: module,
                        file_path: path.to_string(),
                        line: (cap_node.start_position().row + 1) as i32,
                        column: (cap_node.start_position().column + 1) as i32,
                    });
                }
            } else if idx == callee_idx {
                let text = cap_node.utf8_text(source).unwrap_or_default();
                let terminal_name = text.rsplit('.').next().unwrap_or(text).to_string();
                if !terminal_name.is_empty() && terminal_name != "self" {
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
