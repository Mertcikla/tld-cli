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
    ffi::OsString,
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
/// Falls back to TLD_SERVER_URL and TLD_API_KEY environment variables when
/// the overrides are present and usable.
pub fn load_config() -> Result<Config, TldError> {
    let path = config_path()?;
    let cfg = load_config_from_disk(&path)?;
    Ok(apply_config_env_overrides(cfg, ConfigEnv::current()))
}

/// Write the global config, merging auth fields.
pub fn write_config(server_url: &str, api_key: &str, org_id: &str) -> Result<PathBuf, TldError> {
    let path = config_path()?;
    write_config_to_path(&path, server_url, api_key, org_id)?;
    Ok(path)
}

fn load_config_from_disk(path: &Path) -> Result<Config, TldError> {
    if path.exists() {
        let data = fs::read_to_string(path)?;
        serde_yaml::from_str::<Config>(&data).map_err(|e| TldError::Yaml(e.to_string()))
    } else {
        Ok(Config::default())
    }
}

fn write_config_to_path(
    path: &Path,
    server_url: &str,
    api_key: &str,
    org_id: &str,
) -> Result<(), TldError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cfg = load_config_from_disk(path)?;
    cfg.server_url = server_url.to_string();
    cfg.api_key = api_key.to_string();
    cfg.org_id = org_id.to_string();

    let data = serde_yaml::to_string(&cfg).map_err(|e| TldError::Yaml(e.to_string()))?;
    fs::write(path, data)?;
    Ok(())
}

#[derive(Debug, Default, Clone)]
struct ConfigEnv {
    server_url: Option<OsString>,
    api_key: Option<OsString>,
    org_id: Option<OsString>,
}

impl ConfigEnv {
    fn current() -> Self {
        Self {
            server_url: std::env::var_os("TLD_SERVER_URL"),
            api_key: std::env::var_os("TLD_API_KEY"),
            org_id: std::env::var_os("TLD_ORG_ID"),
        }
    }
}

fn env_value(value: Option<OsString>) -> Option<String> {
    let value = value?;
    let value = value.to_str()?.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn apply_config_env_overrides(mut cfg: Config, env: ConfigEnv) -> Config {
    if let Some(server_url) = env_value(env.server_url) {
        cfg.server_url = server_url;
    }
    if let Some(api_key) = env_value(env.api_key) {
        cfg.api_key = api_key;
    }
    if let Some(org_id) = env_value(env.org_id) {
        cfg.org_id = org_id;
    }

    cfg
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

#[cfg(test)]
mod tests {
    use super::{
        Config, ConfigEnv, apply_config_env_overrides, load_config_from_disk, write_config_to_path,
    };
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn load_config_reads_example_shape_and_ignores_unknown_fields() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("tld.yaml");
        fs::write(
            &path,
            concat!(
                "# tld workspace configuration\n",
                "server_url: http://localhost:8080\n",
                "api_key: \"tld_2a28f35bdd665b37e75233285a88cbbaf5429482930bedd9298a759eb7176089\"\n",
                "org_id: \"019d8caf-c445-76de-894d-2e72a53829e6\"\n",
                "strictness: 1\n"
            ),
        )
        .expect("write config");

        let cfg = load_config_from_disk(&path).expect("load config");

        assert_eq!(cfg.server_url, "http://localhost:8080");
        assert_eq!(
            cfg.api_key,
            "tld_2a28f35bdd665b37e75233285a88cbbaf5429482930bedd9298a759eb7176089"
        );
        assert_eq!(cfg.org_id, "019d8caf-c445-76de-894d-2e72a53829e6");
        assert!(cfg.validation.is_none());
    }

    #[test]
    fn load_config_applies_env_overrides_when_present() {
        let cfg = Config {
            server_url: "http://localhost:8080".to_string(),
            api_key: "from-config".to_string(),
            org_id: "from-config-org".to_string(),
            validation: None,
        };

        let overridden = apply_config_env_overrides(
            cfg,
            ConfigEnv {
                server_url: Some("http://env-host:9999".into()),
                api_key: Some("from-env".into()),
                org_id: Some("env-org".into()),
            },
        );

        assert_eq!(overridden.server_url, "http://env-host:9999");
        assert_eq!(overridden.api_key, "from-env");
        assert_eq!(overridden.org_id, "env-org");
    }

    #[test]
    fn load_config_ignores_blank_or_corrupted_env_values() {
        let cfg = Config {
            server_url: "http://localhost:8080".to_string(),
            api_key: "from-config".to_string(),
            org_id: "from-config-org".to_string(),
            validation: None,
        };

        let overridden = apply_config_env_overrides(
            cfg,
            ConfigEnv {
                server_url: Some("   ".into()),
                api_key: Some("".into()),
                org_id: None,
            },
        );

        assert_eq!(overridden.server_url, "http://localhost:8080");
        assert_eq!(overridden.api_key, "from-config");
        assert_eq!(overridden.org_id, "from-config-org");
    }

    #[cfg(unix)]
    #[test]
    fn load_config_ignores_non_utf8_env_values() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let cfg = Config {
            server_url: "http://localhost:8080".to_string(),
            api_key: "from-config".to_string(),
            org_id: "from-config-org".to_string(),
            validation: None,
        };

        let overridden = apply_config_env_overrides(
            cfg,
            ConfigEnv {
                server_url: Some(OsString::from_vec(vec![0xff, 0xfe])),
                api_key: Some(OsString::from("from-env")),
                org_id: None,
            },
        );

        assert_eq!(overridden.server_url, "http://localhost:8080");
        assert_eq!(overridden.api_key, "from-env");
        assert_eq!(overridden.org_id, "from-config-org");
    }

    #[test]
    fn load_config_errors_on_invalid_yaml() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("tld.yaml");
        fs::write(&path, "server_url: [not valid yaml").expect("write config");

        let err = load_config_from_disk(&path).expect_err("invalid config should fail");

        match err {
            crate::error::TldError::Yaml(_) => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn write_config_errors_when_existing_config_is_invalid() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("tld.yaml");
        fs::write(&path, "server_url: [not valid yaml").expect("write config");

        let err = write_config_to_path(&path, "https://tldiagram.com", "api-key", "org-id")
            .expect_err("invalid config should stop write");

        match err {
            crate::error::TldError::Yaml(_) => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn write_config_creates_expected_login_shape() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("tld.yaml");

        write_config_to_path(
            &path,
            "https://tldiagram.com",
            "tld_2a28f35bdd665b37e75233285a88cbbaf5429482930bedd9298a759eb7176089",
            "019d8caf-c445-76de-894d-2e72a53829e6",
        )
        .expect("write config");

        let written = fs::read_to_string(&path).expect("read written config");
        assert!(written.contains("server_url: https://tldiagram.com"));

        let cfg = load_config_from_disk(&path).expect("reload written config");
        assert_eq!(cfg.server_url, "https://tldiagram.com");
        assert_eq!(
            cfg.api_key,
            "tld_2a28f35bdd665b37e75233285a88cbbaf5429482930bedd9298a759eb7176089"
        );
        assert_eq!(cfg.org_id, "019d8caf-c445-76de-894d-2e72a53829e6");
    }
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
