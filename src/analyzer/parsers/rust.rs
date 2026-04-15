use crate::analyzer::types::{AnalysisResult, Ref, Symbol};
use tree_sitter::{Language, Node};

pub fn parse(node: &Node, source: &[u8], path: &str, _language: &Language, result: &mut AnalysisResult) {
    walk_node(node, source, path, None, result);
}

fn walk_node(
    node: &Node,
    source: &[u8],
    path: &str,
    impl_parent: Option<&str>,
    result: &mut AnalysisResult,
) {
    match node.kind() {
        "function_item" => {
            append_function(node, source, path, impl_parent, result);
            // Walk body for nested calls but don't recurse into the full tree
            // to avoid double-counting nested functions
            if let Some(body) = node.child_by_field_name("body") {
                walk_calls_only(&body, source, path, result);
            }
            return;
        }
        "struct_item" => append_type_item(node, source, path, "struct", result),
        "enum_item" => append_type_item(node, source, path, "enum", result),
        "trait_item" => append_type_item(node, source, path, "trait", result),
        "type_item" => append_type_item(node, source, path, "type", result),
        "use_declaration" => append_use(node, source, path, result),
        "impl_item" => {
            // Determine the type name from the impl's type field
            let parent = impl_type_name(node, source);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_node(&child, source, path, parent.as_deref(), result);
            }
            return;
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_node(&child, source, path, impl_parent, result);
    }
}

/// Walk only looking for call and method call expressions (used inside function bodies).
fn walk_calls_only(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    match node.kind() {
        "call_expression" => {
            append_call(node, source, path, result);
        }
        "method_call_expression" => {
            append_method_call(node, source, path, result);
        }
        // Don't descend into nested function/closure bodies to avoid duplicates
        "closure_expression" => return,
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk_calls_only(&child, source, path, result);
    }
}

fn append_function(
    node: &Node,
    source: &[u8],
    path: &str,
    parent: Option<&str>,
    result: &mut AnalysisResult,
) {
    if let Some(name_node) = node.child_by_field_name("name") {
        let name = name_node.utf8_text(source).unwrap_or_default().to_string();
        let kind = if parent.is_some() {
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
            description: find_doc_comment(node, source),
            parent: parent.unwrap_or("").to_string(),
            technology: String::new(),
        });
    }
}

fn append_type_item(
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
            description: find_doc_comment(node, source),
            parent: String::new(),
            technology: String::new(),
        });
    }
}

/// Extract the self type name from an impl item (e.g. `impl Foo` → "Foo", `impl<T> Bar<T>` → "Bar").
fn impl_type_name(node: &Node, source: &[u8]) -> Option<String> {
    // Rust tree-sitter: impl_item has a "type" field for the implementing type
    node.child_by_field_name("type").map(|t| {
        // type_identifier for simple types; scoped_type_identifier for qualified ones
        // Just grab the first type_identifier descendant
        type_name_from_node(&t, source)
    })
}

fn type_name_from_node(node: &Node, source: &[u8]) -> String {
    match node.kind() {
        "type_identifier" | "identifier" => node.utf8_text(source).unwrap_or_default().to_string(),
        "generic_type" => {
            // `Foo<T>` → take the `type_identifier` child
            node.child_by_field_name("type")
                .or_else(|| {
                    let mut cursor = node.walk();
                    node.named_children(&mut cursor)
                        .find(|c| c.kind() == "type_identifier")
                })
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default()
        }
        "scoped_type_identifier" => {
            // `foo::Bar` → take the last segment
            let mut cursor = node.walk();
            node.named_children(&mut cursor)
                .filter(|c| c.kind() == "type_identifier")
                .last()
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default()
        }
        _ => {
            // Fallback: grab text and strip generics
            let text = node.utf8_text(source).unwrap_or_default();
            text.split('<').next().unwrap_or(text).trim().to_string()
        }
    }
}

fn append_call(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    // call_expression: function field points to the callee
    if let Some(func_node) = node.child_by_field_name("function") {
        let name = rust_call_name(&func_node, source);
        if !name.is_empty() {
            result.refs.push(Ref {
                name,
                kind: "call".to_string(),
                target_path: String::new(),
                file_path: path.to_string(),
                line: (func_node.start_position().row + 1) as i32,
                column: (func_node.start_position().column + 1) as i32,
            });
        }
    }
}

fn append_method_call(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    // method_call_expression: method field is the method name identifier
    if let Some(method_node) = node.child_by_field_name("method") {
        let name = method_node
            .utf8_text(source)
            .unwrap_or_default()
            .to_string();
        if !name.is_empty() {
            result.refs.push(Ref {
                name,
                kind: "call".to_string(),
                target_path: String::new(),
                file_path: path.to_string(),
                line: (method_node.start_position().row + 1) as i32,
                column: (method_node.start_position().column + 1) as i32,
            });
        }
    }
}

/// Resolve the terminal callable name from a call expression's function node.
fn rust_call_name(node: &Node, source: &[u8]) -> String {
    match node.kind() {
        "identifier" => node.utf8_text(source).unwrap_or_default().to_string(),
        "field_expression" => {
            // e.g. `self.foo` or `obj.method` — terminal field
            node.child_by_field_name("field")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_default()
        }
        "scoped_identifier" => {
            // e.g. `Foo::bar` or `std::mem::drop` — last segment
            node.child_by_field_name("name")
                .map(|n| n.utf8_text(source).unwrap_or_default().to_string())
                .unwrap_or_else(|| {
                    // Fallback: take text after last `::`
                    let text = node.utf8_text(source).unwrap_or_default();
                    text.rsplit("::").next().unwrap_or(text).trim().to_string()
                })
        }
        "generic_function" => {
            // e.g. `foo::<T>()` — function field
            node.child_by_field_name("function")
                .map(|n| rust_call_name(&n, source))
                .unwrap_or_default()
        }
        _ => {
            let text = node.utf8_text(source).unwrap_or_default();
            // Best-effort: last segment after `::`
            let text = text.rsplit("::").next().unwrap_or(text);
            text.split('<').next().unwrap_or(text).trim().to_string()
        }
    }
}

fn append_use(node: &Node, source: &[u8], path: &str, result: &mut AnalysisResult) {
    // use_declaration has an "argument" field containing a use_list, scoped_identifier, etc.
    if let Some(arg) = node.child_by_field_name("argument") {
        collect_use_paths(&arg, source, path, result);
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
                });
            }
        }
        "use_wildcard" => {
            // `use foo::*` — emit the prefix as import
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
                });
            }
        }
        _ => {}
    }
}

fn find_doc_comment(node: &Node, source: &[u8]) -> String {
    // Check for line_comment or block_comment siblings immediately above
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
            // Strip `///`, `//!`, `//`, `/*`, `*/`
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
