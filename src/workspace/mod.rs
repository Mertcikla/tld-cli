pub mod conversion;
pub mod git;
pub mod lockfile;
pub mod merger;
pub mod mutations;
pub mod types;
pub mod validator;
pub mod workspace_builder;
pub use git::*;
pub use lockfile::*;
pub use types::*;
pub use validator::*;

use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::TldError;

// ── Global config ─────────────────────────────────────────────────────────────

/// Returns the path to the global config directory.
/// Respects `TLD_CONFIG_DIR` and `XDG_CONFIG_HOME`.
pub fn config_dir() -> Result<PathBuf, TldError> {
    if let Some(v) = std::env::var_os("TLD_CONFIG_DIR") {
        return Ok(PathBuf::from(v));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(xdg).join("tldiagram"));
    }
    let home = dirs::home_dir().ok_or_else(|| {
        TldError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cannot determine home directory",
        ))
    })?;
    Ok(home.join(".config").join("tldiagram"))
}

/// Returns the path to the global config file (`tld.yaml`).
pub fn config_path() -> Result<PathBuf, TldError> {
    Ok(config_dir()?.join("tld.yaml"))
}

/// Loads the global user config. Returns a default `Config` if the file
/// does not exist yet (e.g. before first `tld login`).
/// Falls back to TLD_SERVER_URL and TLD_API_KEY environment variables.
pub fn load_config() -> Result<Config, TldError> {
    let path = config_path()?;
    let mut cfg = if path.exists() {
        let data = fs::read_to_string(&path)?;
        serde_yaml::from_str::<Config>(&data).map_err(|e| TldError::Yaml(e.to_string()))?
    } else {
        Config::default()
    };

    // Apply environment variable overrides
    if cfg.server_url.is_empty()
        && let Some(server_url) = std::env::var_os("TLD_SERVER_URL")
        && let Some(s) = server_url.to_str()
    {
        cfg.server_url = s.to_string();
    }
    if cfg.api_key.is_empty()
        && let Some(api_key) = std::env::var_os("TLD_API_KEY")
        && let Some(k) = api_key.to_str()
    {
        cfg.api_key = k.to_string();
    }
    if cfg.org_id.is_empty()
        && let Some(org_id) = std::env::var_os("TLD_ORG_ID")
        && let Some(o) = org_id.to_str()
    {
        cfg.org_id = o.to_string();
    }

    Ok(cfg)
}

/// Write the global config, merging auth fields.
pub fn write_config(server_url: &str, api_key: &str, org_id: &str) -> Result<PathBuf, TldError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cfg = load_config().unwrap_or_default();
    cfg.server_url = server_url.to_string();
    cfg.api_key = api_key.to_string();
    cfg.org_id = org_id.to_string();

    let data = serde_yaml::to_string(&cfg).map_err(|e| TldError::Yaml(e.to_string()))?;
    fs::write(&path, data)?;
    Ok(path)
}

// ── Workspace loading ─────────────────────────────────────────────────────────

/// Resolves a workspace directory, falling back to well-known names.
pub fn resolve_workspace_dir(wdir: Option<&str>) -> String {
    if let Some(d) = wdir {
        return d.to_string();
    }
    for candidate in &[".tld", "tld"] {
        if Path::new(candidate).is_dir() {
            return candidate.to_string();
        }
    }
    ".".to_string()
}

/// Loads the workspace from the given workspace directory.
pub fn load(wdir: &str) -> Result<Workspace, TldError> {
    let global_cfg = load_config()?;

    // Optional workspace-local config
    let ws_config_path = Path::new(wdir).join(".tld.yaml");
    let workspace_config = if ws_config_path.exists() {
        let data = fs::read_to_string(&ws_config_path)?;
        Some(
            serde_yaml::from_str::<WorkspaceConfig>(&data)
                .map_err(|e| TldError::Yaml(e.to_string()))?,
        )
    } else {
        None
    };

    // Elements
    let elements_path = Path::new(wdir).join("elements.yaml");
    let elements = if elements_path.exists() {
        let data = fs::read_to_string(&elements_path)?;
        serde_yaml::from_str::<std::collections::HashMap<String, Element>>(&data)
            .map_err(|e| TldError::Yaml(e.to_string()))?
    } else {
        std::collections::HashMap::new()
    };

    // Connectors
    let connectors_path = Path::new(wdir).join("connectors.yaml");
    let connectors = if connectors_path.exists() {
        let data = fs::read_to_string(&connectors_path)?;
        serde_yaml::from_str::<std::collections::HashMap<String, Connector>>(&data)
            .map_err(|e| TldError::Yaml(e.to_string()))?
    } else {
        std::collections::HashMap::new()
    };

    // Metadata
    let meta = lockfile::load_metadata(wdir).ok();

    Ok(Workspace {
        dir: wdir.to_string(),
        config: global_cfg,
        ws_config: workspace_config,
        elements,
        connectors,
        meta,
    })
}

/// Saves `elements.yaml`, `connectors.yaml`, and metadata to `.tld.lock`.
pub fn save(ws: &Workspace) -> Result<(), TldError> {
    let elements_path = Path::new(&ws.dir).join("elements.yaml");
    let data = serde_yaml::to_string(&ws.elements).map_err(|e| TldError::Yaml(e.to_string()))?;
    fs::write(&elements_path, data)?;

    if !ws.connectors.is_empty() {
        let connectors_path = Path::new(&ws.dir).join("connectors.yaml");
        let data =
            serde_yaml::to_string(&ws.connectors).map_err(|e| TldError::Yaml(e.to_string()))?;
        fs::write(&connectors_path, data)?;
    }

    if let Some(meta) = &ws.meta {
        let lock_file = lockfile::load_lock_file(&ws.dir)?.unwrap_or_else(|| lockfile::LockFile {
            version: "v1".to_string(),
            ..Default::default()
        });
        let mut new_lock = lock_file;
        new_lock.current_elements.clone_from(&meta.elements);
        new_lock.current_views.clone_from(&meta.views);
        new_lock.current_connectors.clone_from(&meta.connectors);
        lockfile::save_lock_file(&ws.dir, &new_lock)?;
    }

    Ok(())
}

/// Converts "API Service" -> "api-service" for use as a ref/filename.
pub fn slugify(s: &str) -> String {
    let s = s.to_lowercase();
    let mut result = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            result.push(c);
        } else {
            result.push('-');
        }
    }
    // Clean up multiple hyphens and trim
    let mut s = result;
    while s.contains("--") {
        s = s.replace("--", "-");
    }
    s.trim_matches('-').to_string()
}
