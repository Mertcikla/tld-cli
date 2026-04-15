use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use tree_sitter::Node;

pub fn parse(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    walk_node(node, source, path, result);
}

fn walk_node(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "function_declaration" | "method_definition" | "lexical_declaration" => {
            append_declaration(node, source, path, result)
        }
        "class_declaration" => append_class(node, source, path, result),
        "import_statement" => append_import(node, source, path, result),
        "call_expression" => append_call(node, source, path, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_node(&child, source, path, result);
    }
}

fn append_declaration(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    // Basic function or variable/const declaration.
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = name_node.utf8_text(source).unwrap_or_default().to_string();
        let kind = if node.kind() == "method_definition" {
            "method"
        } else {
            "function"
        };
        result.symbols.push(Symbol {
            name,
            kind: kind.to_string(),
            file_path: path.to_string(),
            line: (name_node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            description: String::new(),
            parent: String::new(),
            technology: String::new(),
        });
    }
}

fn append_class(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(name_node) = node.child_by_field_name("name") {
        result.symbols.push(Symbol {
            name: name_node.utf8_text(source).unwrap_or_default().to_string(),
            kind: "class".to_string(),
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
        let name = function_node.utf8_text(source).unwrap_or_default();
        if !name.is_empty() {
             // Take last segment for namespaced calls
            let terminal_name = name.rsplit('.').next().unwrap_or(name).to_string();
            result.refs.push(Ref {
                name: terminal_name,
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
    // import { foo } from './bar' -> source field has './bar'
    if let Some(source_node) = node.child_by_field_name("source") {
        let import_path = source_node.utf8_text(source).unwrap_or_default().trim_matches('\'').trim_matches('\"');
        result.refs.push(Ref {
            name: import_path.to_string(),
            kind: "import".to_string(),
            target_path: import_path.to_string(),
            file_path: file_path.to_string(),
            line: (source_node.start_position().row + 1) as i32,
            column: (source_node.start_position().column + 1) as i32,
        });
    }
}
