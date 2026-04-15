use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use tree_sitter::Node;

pub fn parse(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    walk_node(node, source, path, result);
}

fn walk_node(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "function_definition" => append_function(node, source, path, "function", result),
        "class_definition" => append_function(node, source, path, "class", result),
        "import_from_statement" | "import_statement" => append_import(node, source, path, result),
        "call" => append_call(node, source, path, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_node(&child, source, path, result);
    }
}

fn append_function(
    node: &Node,
    source: &[u8],
    path: &str,
    kind: &str,
    result: &mut AnalysisResult,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        result.symbols.push(Symbol {
            name: name_node.utf8_text(source).unwrap_or_default().to_string(),
            kind: kind.to_string(),
            file_path: path.to_string(),
            line: (name_node.start_position().row + 1) as i32,
            end_line: (node.end_position().row + 1) as i32,
            description: String::new(), // Docstrings are inside body, skipping for simplicity
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
    // Basic import extraction. 
    // from foo import bar -> name: bar, target: foo
    // import baz -> name: baz, target: baz
    if node.kind() == "import_from_statement" {
        if let Some(module_node) = node.child_by_field_name("module_name") {
            let module = module_node.utf8_text(source).unwrap_or_default();
            result.refs.push(Ref {
                name: module.to_string(),
                kind: "import".to_string(),
                target_path: module.to_string(),
                file_path: file_path.to_string(),
                line: (module_node.start_position().row + 1) as i32,
                column: (module_node.start_position().column + 1) as i32,
            });
        }
    } else {
        // import_statement
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "dotted_name" {
                let name = child.utf8_text(source).unwrap_or_default();
                result.refs.push(Ref {
                    name: name.to_string(),
                    kind: "import".to_string(),
                    target_path: name.to_string(),
                    file_path: file_path.to_string(),
                    line: (child.start_position().row + 1) as i32,
                    column: (child.start_position().column + 1) as i32,
                });
            }
        }
    }
}
