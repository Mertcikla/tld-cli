use std::path::Path;
use tree_sitter::Node;
use crate::analyzer::types::{AnalysisResult, Symbol, Ref};

pub fn parse(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    walk_node(node, source, path, result);
}

fn walk_node(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "function_declaration" => append_function(node, source, path, "function", result),
        "method_declaration" => append_function(node, source, path, "method", result),
        "type_spec" => append_type_spec(node, source, path, result),
        "type_alias" => append_type_alias(node, source, path, result),
        "import_spec" => append_import(node, source, path, result),
        "call_expression" => append_call(node, source, path, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_node(&child, source, path, result);
    }
}

fn append_function(node: &Node, source: &[u8], path: &str, kind: &str, result: &mut AnalysisResult) {
    if let Some(name_node) = node.child_by_field_name("name") {
        result.symbols.push(Symbol {
            name: name_node.utf8_text(source).unwrap_or_default().to_string(),
            kind: kind.to_string(),
            file_path: path.to_string(),
            line: (name_node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            description: find_comment(node, source),
            parent: String::new(),
            technology: String::new(),
        });
    }
}

fn append_type_spec(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(name_node) = node.child_by_field_name("name") {
        let mut kind = "type";
        if let Some(type_node) = node.child_by_field_name("type") {
            kind = match type_node.kind() {
                "struct_type" => "struct",
                "interface_type" => "interface",
                _ => "type",
            };
        }
        result.symbols.push(Symbol {
            name: name_node.utf8_text(source).unwrap_or_default().to_string(),
            kind: kind.to_string(),
            file_path: path.to_string(),
            line: (name_node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            description: find_comment(node, source),
            parent: String::new(),
            technology: String::new(),
        });
    }
}

fn append_type_alias(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(name_node) = node.child_by_field_name("name") {
        result.symbols.push(Symbol {
            name: name_node.utf8_text(source).unwrap_or_default().to_string(),
            kind: "type".to_string(),
            file_path: path.to_string(),
            line: (name_node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            description: String::new(),
            parent: String::new(),
            technology: String::new(),
        });
    }
}

fn append_call(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(function_node) = node.child_by_field_name("function") {
        let name = go_call_name(&function_node, source);
        if !name.is_empty() {
            result.refs.push(Ref {
                name,
                kind: "call".to_string(),
                target_path: String::new(),
                file_path: path.to_string(),
                line: (function_node.start_position().row + 1) as i32,
                column: (function_node.start_position().column + 1) as i32,
            });
        }
    }
}

fn append_import(node: &Node, source: &[u8], file_path: &str, result: &mut AnalysisResult) {
    if let Some(path_node) = node.child_by_field_name("path") {
        let text = path_node.utf8_text(source).unwrap_or_default().trim();
        let import_path = text.trim_matches('\"');
        if !import_path.is_empty() {
            let name = Path::new(import_path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(import_path)
                .to_string();
            
            result.refs.push(Ref {
                name,
                kind: "import".to_string(),
                target_path: import_path.to_string(),
                file_path: file_path.to_string(),
                line: (path_node.start_position().row + 1) as i32,
                column: (path_node.start_position().column + 1) as i32,
            });
        }
    }
}

fn find_comment(node: &Node, source: &[u8]) -> String {
    if let Some(prev) = node.prev_named_sibling() {
        if prev.kind() == "comment" {
            // Check if it's immediately above
            if node.start_position().row - prev.end_position().row <= 1 {
                let text = prev.utf8_text(source).unwrap_or_default().trim();
                let text = text.strip_prefix("//").unwrap_or(text);
                let text = text.strip_prefix("/*").unwrap_or(text);
                let text = text.strip_suffix("*/").unwrap_or(text);
                return text.trim().to_string();
            }
        }
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
