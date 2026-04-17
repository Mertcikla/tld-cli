use crate::error::TldError;
use crate::output;
use crate::workspace::{self, git, types::Repository, types::WorkspaceConfig};
use clap::Args;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Workspace directory to initialise (default: .tld)
    pub dir: Option<String>,
    /// Run interactive setup wizard
    #[arg(long, default_value = "false")]
    pub wizard: bool,
}

pub fn exec(args: InitArgs, _wdir: String) -> Result<(), TldError> {
    let dir = args.dir.unwrap_or_else(|| ".tld".to_string());
    let path = Path::new(&dir);

    // Workspace root is the parent of the .tld directory
    let workspace_root = if let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            Path::new(".")
        } else {
            parent
        }
    } else {
        Path::new(".")
    };

    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    // Create empty YAML files if they don't exist
    let files = [("elements.yaml", "{}\n"), ("connectors.yaml", "{}\n")];

    for (filename, content) in files {
        let file_path = path.join(filename);
        if !file_path.exists() {
            fs::write(&file_path, content)?;
        }
    }

    let ws_config_path = path.join(".tld.yaml");
    if !ws_config_path.exists() {
        let mut config = WorkspaceConfig {
            exclude: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "vendor".to_string(),
                ".tld".to_string(),
                ".venv".to_string(),
                "build".to_string(),
                "out".to_string(),
            ],
            ..Default::default()
        };

        // Git detection
        if let Some(toplevel) = (git::is_git_repo(workspace_root))
            .then(|| git::get_toplevel(workspace_root))
            .flatten()
        {
            let canonical_root =
                fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
            let canonical_toplevel = fs::canonicalize(&toplevel).unwrap_or(toplevel);

            if let Some(name) = (canonical_root == canonical_toplevel)
                .then(|| canonical_toplevel.file_name().and_then(|n| n.to_str()))
                .flatten()
            {
                config.project_name = name.to_string();
            }

            let remotes = git::get_remotes(workspace_root);
            if !remotes.is_empty() {
                let mut repositories = HashMap::new();
                for (name, url) in remotes {
                    repositories.insert(
                        name,
                        Repository {
                            url,
                            local_dir: ".".to_string(),
                            ..Default::default()
                        },
                    );
                }
                config.repositories = repositories;
            }
        }

        // If project_name is still empty (not a git root), use the directory name
        if config.project_name.is_empty() {
            let abs_root = fs::canonicalize(workspace_root).unwrap_or_else(|_| {
                if workspace_root == Path::new(".") {
                    std::env::current_dir().unwrap_or_else(|_| workspace_root.to_path_buf())
                } else {
                    workspace_root.to_path_buf()
                }
            });
            if let Some(name) = abs_root.file_name().and_then(|n| n.to_str()) {
                config.project_name = name.to_string();
            }
        }

        let yaml = serde_yaml::to_string(&config)
            .map_err(|e| TldError::Generic(format!("Failed to serialize config: {e}")))?;
        fs::write(ws_config_path, yaml)?;
    }

    // Initialize global config if it doesn't exist
    let global_cfg_path = workspace::config_path()?;
    if !global_cfg_path.exists() {
        if let Some(parent) = global_cfg_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let default_global = "server_url: https://tldiagram.com\napi_key: \"\"\norg_id: \"\"\n";
        fs::write(&global_cfg_path, default_global)?;
        output::print_ok(&format!(
            "Global configuration created at {path}",
            path = global_cfg_path.display()
        ));
    }

    output::print_ok(&format!("Initialized workspace at {dir}"));
    if !args.wizard {
        output::print_info("Run `tld login` to authenticate with tldiagram.com");
    }

    Ok(())
}
