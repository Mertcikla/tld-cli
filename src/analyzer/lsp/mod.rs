pub mod session;
use std::collections::HashMap;

pub struct ServerCommand {
    pub executable: String,
    pub args: Vec<String>,
}

pub struct ResolvedCommand {
    pub path: String,
    pub args: Vec<String>,
}

pub fn get_default_commands() -> HashMap<String, Vec<ServerCommand>> {
    let mut m = HashMap::new();
    m.insert("go".to_string(), vec![ServerCommand { executable: "gopls".to_string(), args: vec![] }]);
    m.insert("python".to_string(), vec![ServerCommand { executable: "pyright-langserver".to_string(), args: vec!["--stdio".to_string()] }]);
    m.insert("rust".to_string(), vec![ServerCommand { executable: "rust-analyzer".to_string(), args: vec![] }]);
    m.insert("typescript".to_string(), vec![ServerCommand { executable: "typescript-language-server".to_string(), args: vec!["--stdio".to_string()] }]);
    m.insert("javascript".to_string(), vec![ServerCommand { executable: "typescript-language-server".to_string(), args: vec!["--stdio".to_string()] }]);
    m.insert("c".to_string(), vec![ServerCommand { executable: "clangd".to_string(), args: vec![] }]);
    m.insert("cpp".to_string(), vec![ServerCommand { executable: "clangd".to_string(), args: vec![] }]);
    m
}

pub fn resolve_command(language: &str) -> Option<ResolvedCommand> {
    let defaults = get_default_commands();
    let commands = defaults.get(language)?;

    for cmd in commands {
        if let Ok(path) = which::which(&cmd.executable) {
            return Some(ResolvedCommand {
                path: path.to_string_lossy().to_string(),
                args: cmd.args.clone(),
            });
        }
    }
    None
}

/// Optionally enrich call refs with LSP definition lookups.
/// For each `Ref` with `kind == "call"` and no `target_path`, issue a
/// `textDocument/definition` request to the appropriate language server.
/// Silently skips languages where no LSP server is found.
pub async fn resolve_calls_with_lsp(
    refs: &mut Vec<crate::analyzer::types::Ref>,
    root_dir: &str,
    unique_langs: &[&str],
) -> Result<(), crate::error::TldError> {
    use crate::analyzer::lsp::session::Session;
    use serde_json::json;

    let root_uri = format!(
        "file://{}",
        root_dir.trim_end_matches('/')
    );

    for lang in unique_langs {
        let cmd = match resolve_command(lang) {
            Some(c) => c,
            None => continue,
        };

        let session = match Session::start(&cmd.path, &cmd.args, root_dir).await {
            Ok(s) => s,
            Err(_) => continue, // LSP not available
        };

        if session.initialize(root_uri.clone()).await.is_err() {
            continue;
        }

        // Small delay to let the LSP server initialize.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        for r in refs.iter_mut() {
            if r.kind != "call" || !r.target_path.is_empty() {
                continue;
            }

            // Only process refs from files matching this language.
            let file_ext = std::path::Path::new(&r.file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let file_lang = match file_ext {
                "go" => "go",
                "py" => "python",
                "ts" | "tsx" => "typescript",
                "js" | "jsx" => "javascript",
                "java" => "java",
                "cpp" | "cc" | "hpp" | "h" => "cpp",
                "rs" => "rust",
                _ => continue,
            };
            if file_lang != *lang {
                continue;
            }

            let file_uri = format!("file://{}", r.file_path);
            let request = json!({
                "method": "textDocument/definition",
                "params": {
                    "textDocument": {"uri": file_uri},
                    "position": {"line": r.line - 1, "character": r.column - 1}
                }
            });

            if let Ok(response) = session.send_request(request).await {
                // The result can be Location, Location[], or LocationLink[]
                let result = &response["result"];
                let loc = if result.is_array() {
                    result.get(0)
                } else {
                    Some(result)
                };

                if let Some(loc) = loc {
                    if let Some(uri) = loc.get("uri").and_then(|u| u.as_str()) {
                        let def_path = uri.trim_start_matches("file://");
                        r.target_path = def_path.to_string();
                    } else if let Some(target_uri) = loc.get("targetUri").and_then(|u| u.as_str()) {
                        let def_path = target_uri.trim_start_matches("file://");
                        r.target_path = def_path.to_string();
                    }
                }
            }
        }

        let _ = session.shutdown().await;
    }

    Ok(())
}
