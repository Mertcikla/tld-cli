use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use tree_sitter::Node;

pub fn parse(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    walk_node(node, source, path, result);
}

fn walk_node(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "function_definition" | "field_declaration" | "declaration" => {
            append_function(node, source, path, result)
        }
        "class_specifier" | "struct_specifier" => append_type_declaration(node, source, path, result),
        "preproc_include" => append_include(node, source, path, result),
        "call_expression" => append_call(node, source, path, result),
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_node(&child, source, path, result);
    }
}

fn append_function(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(declarator) = node.child_by_field_name("declarator") {
         let name = cpp_name(&declarator, source);
         if !name.is_empty() {
            result.symbols.push(Symbol {
                name,
                kind: "function".to_string(),
                file_path: path.to_string(),
                line: (declarator.start_position().row + 1) as i32,
                end_line: (node.end_position().row + 1) as i32,
                description: String::new(),
                parent: String::new(),
                technology: String::new(),
            });
         }
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

fn append_call(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    if let Some(name_node) = node.child_by_field_name("function") {
        let text = name_node.utf8_text(source).unwrap_or_default().trim();
        let name = text.rsplit("::").next().unwrap_or(text).rsplit('.').next().unwrap_or(text).to_string();
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

fn append_include(node: &Node, source: &[u8], file_path: &str, result: &mut AnalysisResult) {
     // #include "foo.h" -> path field has "foo.h"
    if let Some(path_node) = node.child_by_field_name("path") {
        let import_path = path_node.utf8_text(source).unwrap_or_default().trim_matches('<').trim_matches('>').trim_matches('\"');
        result.refs.push(Ref {
            name: import_path.to_string(),
            kind: "import".to_string(),
            target_path: import_path.to_string(),
            file_path: file_path.to_string(),
            line: (path_node.start_position().row + 1) as i32,
            column: (path_node.start_position().column + 1) as i32,
        });
    }
}

fn cpp_name(node: &Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" | "type_identifier" | "field_identifier" => node.utf8_text(source).unwrap_or_default().to_string(),
        "function_declarator" => {
             if let Some(decl) = node.child_by_field_name("declarator") {
                 cpp_name(&decl, source)
             } else {
                 String::new()
             }
        }
        "scoped_identifier" => {
             if let Some(name) = node.child_by_field_name("name") {
                 cpp_name(&name, source)
             } else {
                 String::new()
             }
        }
        _ => {
            let text = node.utf8_text(source).unwrap_or_default().trim();
            let name = text.rsplit("::").next().unwrap_or(text);
            name.rsplit('.').next().unwrap_or(name).trim().to_string()
        }
    }
}
