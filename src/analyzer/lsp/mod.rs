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
    m.insert(
        "go".to_string(),
        vec![ServerCommand {
            executable: "gopls".to_string(),
            args: vec![],
        }],
    );
    m.insert(
        "python".to_string(),
        vec![ServerCommand {
            executable: "pyright-langserver".to_string(),
            args: vec!["--stdio".to_string()],
        }],
    );
    m.insert(
        "rust".to_string(),
        vec![ServerCommand {
            executable: "rust-analyzer".to_string(),
            args: vec![],
        }],
    );
    m.insert(
        "typescript".to_string(),
        vec![ServerCommand {
            executable: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
        }],
    );
    m.insert(
        "javascript".to_string(),
        vec![ServerCommand {
            executable: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
        }],
    );
    m.insert(
        "java".to_string(),
        vec![
            // Eclipse JDT LS (most common)
            ServerCommand {
                executable: "jdtls".to_string(),
                args: vec![],
            },
            // Palantir's java-language-server
            ServerCommand {
                executable: "java-language-server".to_string(),
                args: vec![],
            },
        ],
    );
    m.insert(
        "c".to_string(),
        vec![ServerCommand {
            executable: "clangd".to_string(),
            args: vec![],
        }],
    );
    m.insert(
        "cpp".to_string(),
        vec![ServerCommand {
            executable: "clangd".to_string(),
            args: vec![],
        }],
    );
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
///
/// For each `Ref` with `kind == "call"` and no `target_path`, issue a
/// `textDocument/definition` request to the appropriate language server.
/// Resolved paths are cached by `(file, line, column)` to avoid duplicate
/// requests for the same call site. Silently skips languages where no LSP
/// server is found in PATH.
pub async fn resolve_calls_with_lsp(
    refs: &mut [crate::analyzer::types::Ref],
    root_dir: &str,
    unique_langs: &[&str],
) -> Result<(), crate::error::TldError> {
    use crate::analyzer::lsp::session::Session;
    use serde_json::json;

    let root_uri = format!("file://{}", root_dir.trim_end_matches('/'));

    for lang in unique_langs {
        let Some(cmd) = resolve_command(lang) else {
            continue;
        };

        let Ok(session) = Session::start(&cmd.path, &cmd.args, root_dir) else {
            continue;
        };

        if session.initialize(root_uri.clone()).await.is_err() {
            continue;
        }

        // Small delay to let the LSP server index the workspace.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Cache resolved target paths by (file_path, line, column) so that
        // duplicate call sites are looked up only once per session.
        let mut cache: std::collections::HashMap<(String, i32, i32), String> =
            std::collections::HashMap::new();

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

            let cache_key = (r.file_path.clone(), r.line, r.column);
            if let Some(cached) = cache.get(&cache_key) {
                r.target_path.clone_from(cached);
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
                    let resolved = loc
                        .get("uri")
                        .or_else(|| loc.get("targetUri"))
                        .and_then(|u| u.as_str())
                        .map(|uri| uri.trim_start_matches("file://").to_string());

                    if let Some(path) = resolved {
                        cache.insert(cache_key, path.clone());
                        r.target_path = path;
                    }
                }
            }
        }

        let _ = session.shutdown().await;
    }

    Ok(())
}
