pub mod session;
use crate::output;
use std::collections::HashMap;

pub struct ServerCommand {
    pub executable: String,
    pub args: Vec<String>,
    pub install_cmd: String,
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
            install_cmd: "go install golang.org/x/tools/gopls@latest".to_string(),
        }],
    );
    m.insert(
        "python".to_string(),
        vec![ServerCommand {
            executable: "pyright-langserver".to_string(),
            args: vec!["--stdio".to_string()],
            install_cmd: "npm install -g pyright".to_string(),
        }],
    );
    m.insert(
        "rust".to_string(),
        vec![ServerCommand {
            executable: "rust-analyzer".to_string(),
            args: vec![],
            install_cmd: "rustup component add rust-analyzer".to_string(),
        }],
    );
    m.insert(
        "typescript".to_string(),
        vec![ServerCommand {
            executable: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            install_cmd: "npm install -g typescript typescript-language-server".to_string(),
        }],
    );
    m.insert(
        "javascript".to_string(),
        vec![ServerCommand {
            executable: "typescript-language-server".to_string(),
            args: vec!["--stdio".to_string()],
            install_cmd: "npm install -g typescript typescript-language-server".to_string(),
        }],
    );
    m.insert(
        "java".to_string(),
        vec![
            // Eclipse JDT LS (most common)
            ServerCommand {
                executable: "jdtls".to_string(),
                args: vec![],
                install_cmd: "brew install jdtls".to_string(),
            },
        ],
    );
    m.insert(
        "c".to_string(),
        vec![ServerCommand {
            executable: "clangd".to_string(),
            args: vec![],
            install_cmd: "brew install llvm".to_string(),
        }],
    );
    m.insert(
        "cpp".to_string(),
        vec![ServerCommand {
            executable: "clangd".to_string(),
            args: vec![],
            install_cmd: "brew install llvm".to_string(),
        }],
    );
    m
}

pub fn resolve_command(language: &str) -> Result<ResolvedCommand, crate::error::TldError> {
    let defaults = get_default_commands();
    let commands = defaults.get(language).ok_or_else(|| {
        crate::error::TldError::Generic(format!("No LSP configured for {language}"))
    })?;

    for cmd in commands {
        if let Ok(path) = which::which(&cmd.executable) {
            return Ok(ResolvedCommand {
                path: path.to_string_lossy().to_string(),
                args: cmd.args.clone(),
            });
        }
    }

    let first = &commands[0];
    Err(crate::error::TldError::LspInstallRequired {
        lang: language.to_string(),
        executable: first.executable.clone(),
        install_cmd: first.install_cmd.clone(),
    })
}

/// Optionally enrich call refs with LSP definition lookups.
///
/// For each `Ref` with `kind == "call"` and no `target_path`, issue a
/// `textDocument/definition` request to the appropriate language server.
/// Resolved paths are cached by `(file, line, column)` to avoid duplicate
/// requests for the same call site. Warns and continues when a language server
/// cannot be found, started, or initialized.
pub async fn resolve_calls_with_lsp(
    refs: &mut [crate::analyzer::types::Ref],
    root_dir: &str,
    unique_langs: &[&str],
    progress: &indicatif::ProgressBar,
) -> Result<(), crate::error::TldError> {
    use crate::analyzer::lsp::session::Session;
    use crate::error::TldError;

    let root_uri = format!("file://{}", root_dir.trim_end_matches('/'));

    for lang in unique_langs {
        let lang_pending = count_pending_refs_for_language(refs, lang);
        if lang_pending == 0 {
            continue;
        }
        progress.set_message(format!("Indexing {lang} definitions"));

        let cmd = match resolve_command(lang) {
            Ok(c) => c,
            Err(TldError::LspInstallRequired {
                lang: l,
                executable,
                install_cmd,
            }) => {
                if let Some(c) = prompt_install(&l, &executable, &install_cmd, progress)? {
                    c
                } else {
                    progress.inc(lang_pending);
                    continue; // Continue without LSP
                }
            }
            Err(e) => {
                progress.suspend(|| {
                    output::print_warn(&format!(
                        "No LSP server found for '{lang}' ({e}); analysis accuracy may drop."
                    ));
                });
                progress.inc(lang_pending);
                continue;
            }
        };

        let session_res = Session::start(&cmd.path, &cmd.args, root_dir);
        let Ok(session) = session_res else {
            progress.suspend(|| {
                output::print_warn(&format!(
                    "Could not start the '{lang}' LSP server; analysis accuracy may drop."
                ));
            });
            progress.inc(lang_pending);
            continue;
        };

        if session.initialize(root_uri.clone()).await.is_err() {
            progress.suspend(|| {
                output::print_warn(&format!(
                    "Could not initialize the '{lang}' LSP server; analysis accuracy may drop."
                ));
            });
            progress.inc(lang_pending);
            continue;
        }

        // Small delay to let the LSP server index the workspace.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        // Cache resolved target paths by (file_path, line, column) so that
        // duplicate call sites are looked up only once per session.
        let mut cache: std::collections::HashMap<(String, i32, i32), String> =
            std::collections::HashMap::new();
        let mut opened_files = std::collections::HashSet::new();

        for r in refs.iter_mut() {
            if r.kind != "call" || !r.target_path.is_empty() {
                continue;
            }

            // Only process refs from files matching this language.
            let Some(file_lang) = language_for_path(&r.file_path) else {
                continue;
            };
            if file_lang != *lang {
                continue;
            }

            progress.set_message(format!("Resolving {lang}: {}::{}", r.file_path, r.name));
            let cache_key = (r.file_path.clone(), r.line, r.column);
            if let Some(cached) = cache.get(&cache_key) {
                r.target_path.clone_from(cached);
                progress.inc(1);
                continue;
            }

            let file_uri = format!("file://{}", r.file_path);
            if opened_files.insert(r.file_path.clone())
                && let Ok(text) = std::fs::read_to_string(&r.file_path)
            {
                let _ = session.did_open(&file_uri, lang, &text).await;
                let _ = session.document_symbols(&file_uri).await;
            }

            let response = session.definition(&file_uri, r.line, r.column).await.ok();
            let response = if response.is_some() {
                response
            } else {
                session
                    .implementation(&file_uri, r.line, r.column)
                    .await
                    .ok()
            };

            if let Some(path) = response.and_then(|resp| extract_location_path(&resp)) {
                cache.insert(cache_key, path.clone());
                r.target_path = path;
            }
            progress.inc(1);
        }

        for file_path in opened_files {
            let file_uri = format!("file://{file_path}");
            let _ = session.did_close(&file_uri).await;
        }

        let _ = session.shutdown().await;
    }

    Ok(())
}

fn extract_location_path(response: &serde_json::Value) -> Option<String> {
    let result = &response["result"];
    let loc = if result.is_array() {
        result.get(0)
    } else {
        Some(result)
    };

    loc.and_then(|loc| {
        loc.get("uri")
            .or_else(|| loc.get("targetUri"))
            .and_then(|u| u.as_str())
            .map(|uri| uri.trim_start_matches("file://").to_string())
    })
}

fn count_pending_refs_for_language(refs: &[crate::analyzer::types::Ref], lang: &str) -> u64 {
    refs.iter()
        .filter(|r| r.kind == "call" && r.target_path.is_empty())
        .filter(|r| language_for_path(&r.file_path) == Some(lang))
        .count() as u64
}

fn language_for_path(path: &str) -> Option<&'static str> {
    match std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
    {
        "go" => Some("go"),
        "py" => Some("python"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" => Some("javascript"),
        "java" => Some("java"),
        "cpp" | "cc" | "hpp" | "h" => Some("cpp"),
        "rs" => Some("rust"),
        _ => None,
    }
}

#[expect(clippy::print_stdout)]
fn prompt_install(
    lang: &str,
    executable: &str,
    install_cmd: &str,
    progress: &indicatif::ProgressBar,
) -> Result<Option<ResolvedCommand>, crate::error::TldError> {
    use std::io::{Write, stdin, stdout};

    progress.suspend(|| {
        println!("\nThe '{lang}' LSP server ({executable}) is not installed.");
        print!("Would you like to install it now using `{install_cmd}`? [y/N]: ");
        stdout().flush().ok();

        let mut input = String::new();
        stdin().read_line(&mut input).ok();

        if input.trim().eq_ignore_ascii_case("y") {
            output::print_info(&format!("Installing {executable}..."));
            let status = std::process::Command::new("sh")
                .arg("-c")
                .arg(install_cmd)
                .status()
                .map_err(|e| {
                    crate::error::TldError::Generic(format!("Failed to run install command: {e}"))
                })?;

            if status.success() {
                output::print_ok(&format!("{executable} installed successfully."));
                // Try to resolve again
                return resolve_command(lang).map(Some);
            }
            output::print_warn(&format!("Installation of {executable} failed."));
        }

        // If not installed or failed, ask Abort or Continue
        print!("Abort analysis or Continue without LSP enrichment? [a/C]: ");
        stdout().flush().ok();
        input.clear();
        stdin().read_line(&mut input).ok();

        if input.trim().eq_ignore_ascii_case("a") {
            return Err(crate::error::TldError::Generic(
                "Analysis aborted.".to_string(),
            ));
        }

        Ok(None)
    })
}
