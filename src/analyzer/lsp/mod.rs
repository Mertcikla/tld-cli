pub mod session;
use std::collections::HashMap;
use std::process::Command;

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
