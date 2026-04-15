use clap::Args;
use std::fs;
use std::path::Path;
use crate::error::TldError;
use crate::output;
use crate::workspace;

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Workspace directory to initialise (default: .tld)
    pub dir: Option<String>,
    /// Run interactive setup wizard
    #[arg(long, default_value = "false")]
    pub wizard: bool,
}

pub async fn exec(args: InitArgs, _wdir: String) -> Result<(), TldError> {
    let dir = args.dir.unwrap_or_else(|| ".tld".to_string());
    let path = Path::new(&dir);

    if !path.exists() {
        fs::create_dir_all(path)?;
    }

    // Create empty YAML files if they don't exist
    let files = [
        ("elements.yaml", "{}\n"),
        ("connectors.yaml", "{}\n"),
    ];

    for (filename, content) in files {
        let file_path = path.join(filename);
        if !file_path.exists() {
            fs::write(&file_path, content)?;
        }
    }

    let ws_config_path = path.join(".tld.yaml");
    if !ws_config_path.exists() {
        // Simple default for now, skipping git detection for this step
        let default_config = "project_name: \"\"\nexclude: []\nrepositories: {}\n";
        fs::write(ws_config_path, default_config)?;
    }

    // Initialize global config if it doesn't exist
    let global_cfg_path = workspace::config_path()?;
    if !global_cfg_path.exists() {
        if let Some(parent) = global_cfg_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let default_global = "server_url: https://tldiagram.com\napi_key: \"\"\norg_id: \"\"\n";
        fs::write(&global_cfg_path, default_global)?;
        output::print_ok(&format!("Global configuration created at {:?}", global_cfg_path));
    }

    output::print_ok(&format!("Initialized workspace at {}", dir));
    if !args.wizard {
        output::print_info("Run `tld login` to authenticate with tldiagram.com");
    }

    Ok(())
}
