use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use tree_sitter::Node;

pub fn parse(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    walk_node(node, source, path, result);
}

fn walk_node(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "class_declaration" | "interface_declaration" => {
            append_type_declaration(node, source, path, result)
        }
        "method_declaration" | "constructor_declaration" => append_method(node, source, path, result),
        "import_declaration" => append_import(node, source, path, result),
        "method_invocation" => append_call(node, source, path, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_node(&child, source, path, result);
    }
}

fn append_type_declaration(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
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

fn append_method(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(name_node) = node.child_by_field_name("name") {
        result.symbols.push(Symbol {
            name: name_node.utf8_text(source).unwrap_or_default().to_string(),
            kind: "method".to_string(),
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
    if let Some(name_node) = node.child_by_field_name("name") {
        let text = name_node.utf8_text(source).unwrap_or_default().trim();
        let name = text.rsplit('.').next().unwrap_or(text).to_string();
        if !name.is_empty() {
             result.refs.push(Ref {
                name,
                kind: "call".to_string(),
                target_path: String::new(),
                file_path: path.to_string(),
                line: (name_node.start_position().row + 1) as i32,
                column: (name_node.start_position().column + 1) as i32,
            });
        }
    }
}

fn append_import(node: &Node, source: &[u8], file_path: &str, result: &mut AnalysisResult) {
    // import com.foo.bar -> we want 'bar' or the whole string? Let's take 'bar' for name.
    let text = node.utf8_text(source).unwrap_or_default();
    let import_path = text.trim_start_matches("import ").trim_end_matches(';').trim();
    if !import_path.is_empty() {
        let name = import_path.rsplit('.').next().unwrap_or(import_path).to_string();
        result.refs.push(Ref {
            name,
            kind: "import".to_string(),
            target_path: import_path.to_string(),
            file_path: file_path.to_string(),
            line: (node.start_position().row + 1) as i32,
            column: (node.start_position().column + 1) as i32,
        });
    }
}
