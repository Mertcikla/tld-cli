use crate::error::TldError;
use crate::workspace::{
    Element, ViewPlacement, Workspace,
    types::{Repository, WorkspaceConfig},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRootBinding {
    pub repo_key: String,
    pub project_name: String,
    pub local_ref: String,
    pub workspace_changed: bool,
    pub config_changed: bool,
}

pub fn ensure_default_repository_root(
    ws: &mut Workspace,
) -> Result<Option<RepositoryRootBinding>, TldError> {
    let Some(spec) = select_repository_root_spec(ws) else {
        return Ok(None);
    };

    let mut workspace_changed = false;
    let mut config_changed = false;

    if let Some(existing) = ws.elements.get(&spec.local_ref)
        && existing.name != spec.project_name
    {
        return Err(TldError::Generic(format!(
            "Configured repository root ref '{}' already exists as '{}'.",
            spec.local_ref, existing.name
        )));
    }

    if let Some(config) = ws.ws_config.as_mut()
        && let Some(repo) = config.repositories.get_mut(&spec.repo_key)
        && repo.root != spec.local_ref
    {
        repo.root.clone_from(&spec.local_ref);
        config_changed = true;
    }

    let repo_url = ws
        .ws_config
        .as_ref()
        .and_then(|config| config.repositories.get(&spec.repo_key))
        .and_then(|repo| (!repo.url.is_empty()).then_some(repo.url.clone()));

    if let Some(element) = ws.elements.get_mut(&spec.local_ref) {
        if ensure_repository_root_defaults(element, &spec.project_name, repo_url.as_deref()) {
            workspace_changed = true;
        }
    } else {
        ws.upsert_element(
            spec.local_ref.clone(),
            repository_root_element(&spec.project_name, repo_url.as_deref()),
        );
        workspace_changed = true;
    }

    Ok(Some(RepositoryRootBinding {
        repo_key: spec.repo_key,
        project_name: spec.project_name,
        local_ref: spec.local_ref,
        workspace_changed,
        config_changed,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepositoryRootSpec {
    repo_key: String,
    project_name: String,
    local_ref: String,
}

fn select_repository_root_spec(ws: &Workspace) -> Option<RepositoryRootSpec> {
    let config = ws.ws_config.as_ref()?;
    let (repo_key, repo) = select_default_repository(config)?;
    let project_name = if config.project_name.trim().is_empty() {
        repo_key.clone()
    } else {
        config.project_name.trim().to_string()
    };
    let local_ref = infer_repository_root_ref(ws, repo, &project_name);

    Some(RepositoryRootSpec {
        repo_key,
        project_name,
        local_ref,
    })
}

fn select_default_repository(config: &WorkspaceConfig) -> Option<(String, &Repository)> {
    if config.repositories.is_empty() {
        return None;
    }

    let mut entries: Vec<_> = config.repositories.iter().collect();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));

    if entries.len() == 1 {
        let (key, repo) = entries.into_iter().next()?;
        return Some((key.clone(), repo));
    }

    entries
        .into_iter()
        .find(|(_, repo)| repo.local_dir.trim().is_empty() || repo.local_dir.trim() == ".")
        .map(|(key, repo)| (key.clone(), repo))
}

fn infer_repository_root_ref(ws: &Workspace, repo: &Repository, project_name: &str) -> String {
    let configured = repo.root.trim();
    if !configured.is_empty() {
        return configured.to_string();
    }

    if let Some(existing_ref) = find_existing_local_root_ref(ws, project_name) {
        return existing_ref;
    }

    let base_ref = crate::workspace::slugify(project_name);
    next_available_ref(ws, &base_ref, project_name).unwrap_or(base_ref)
}

fn next_available_ref(ws: &Workspace, base_ref: &str, project_name: &str) -> Option<String> {
    for suffix in 0..100 {
        let candidate = if suffix == 0 {
            base_ref.to_string()
        } else {
            format!("{base_ref}-{}", suffix + 1)
        };

        match ws.elements.get(&candidate) {
            None => return Some(candidate),
            Some(element) if element.name == project_name => return Some(candidate),
            Some(_) => {}
        }
    }

    None
}

fn find_existing_local_root_ref(ws: &Workspace, project_name: &str) -> Option<String> {
    let mut matches: Vec<_> = ws
        .elements
        .iter()
        .filter(|(_, element)| {
            element.name == project_name
                && (element.kind == "repository"
                    || element
                        .placements
                        .iter()
                        .any(|placement| placement.parent_ref == "root"))
        })
        .map(|(ref_name, _)| ref_name.clone())
        .collect();
    matches.sort();
    matches.into_iter().next()
}

fn repository_root_element(project_name: &str, repo_url: Option<&str>) -> Element {
    let mut element = Element {
        name: project_name.to_string(),
        kind: "repository".to_string(),
        technology: "Git Repository".to_string(),
        has_view: true,
        view_label: project_name.to_string(),
        placements: vec![ViewPlacement {
            parent_ref: "root".to_string(),
            ..Default::default()
        }],
        ..Default::default()
    };

    if let Some(url) = repo_url {
        element.repo = url.to_string();
    }

    element
}

fn ensure_repository_root_defaults(
    element: &mut Element,
    project_name: &str,
    repo_url: Option<&str>,
) -> bool {
    let mut changed = false;

    if element.kind.is_empty() {
        element.kind = "repository".to_string();
        changed = true;
    }
    if element.technology.is_empty() {
        element.technology = "Git Repository".to_string();
        changed = true;
    }
    if !element.has_view {
        element.has_view = true;
        changed = true;
    }
    if element.view_label.is_empty() {
        element.view_label = project_name.to_string();
        changed = true;
    }
    if element.name != project_name {
        element.name = project_name.to_string();
        changed = true;
    }
    if element.repo.is_empty()
        && let Some(url) = repo_url
    {
        element.repo = url.to_string();
        changed = true;
    }
    if !element
        .placements
        .iter()
        .any(|placement| placement.parent_ref == "root")
    {
        element.placements.push(ViewPlacement {
            parent_ref: "root".to_string(),
            ..Default::default()
        });
        changed = true;
    }

    changed
}

#[cfg(test)]
mod tests {
    use super::ensure_default_repository_root;
    use crate::workspace::{
        Config, Element, ViewPlacement, Workspace,
        types::{Repository, WorkspaceConfig},
    };
    use std::collections::HashMap;

    #[test]
    fn creates_workspace_repository_root_from_project_name() {
        let mut repositories = HashMap::new();
        repositories.insert(
            "origin".to_string(),
            Repository {
                local_dir: ".".to_string(),
                ..Default::default()
            },
        );

        let mut ws = Workspace {
            dir: ".".to_string(),
            config: Config::default(),
            ws_config: Some(WorkspaceConfig {
                project_name: "kafka".to_string(),
                repositories,
                ..Default::default()
            }),
            ..Default::default()
        };

        let binding = ensure_default_repository_root(&mut ws)
            .expect("ensure root")
            .expect("binding");

        assert_eq!(binding.local_ref, "kafka");
        assert!(binding.workspace_changed);
        assert!(binding.config_changed);

        let repo = ws.elements.get("kafka").expect("root element");
        assert_eq!(repo.name, "kafka");
        assert_eq!(repo.kind, "repository");
        assert!(repo.has_view);
        assert!(
            repo.placements
                .iter()
                .any(|placement| placement.parent_ref == "root")
        );
        assert_eq!(
            ws.ws_config
                .as_ref()
                .and_then(|config| config.repositories.get("origin"))
                .map(|repo| repo.root.as_str()),
            Some("kafka")
        );
    }

    #[test]
    fn reuses_existing_local_repository_root_ref() {
        let mut repositories = HashMap::new();
        repositories.insert(
            "origin".to_string(),
            Repository {
                local_dir: ".".to_string(),
                ..Default::default()
            },
        );

        let mut elements = HashMap::new();
        elements.insert(
            "workspace-root".to_string(),
            Element {
                name: "kafka".to_string(),
                kind: "repository".to_string(),
                placements: vec![ViewPlacement {
                    parent_ref: "root".to_string(),
                    ..Default::default()
                }],
                ..Default::default()
            },
        );

        let mut ws = Workspace {
            dir: ".".to_string(),
            config: Config::default(),
            ws_config: Some(WorkspaceConfig {
                project_name: "kafka".to_string(),
                repositories,
                ..Default::default()
            }),
            elements,
            ..Default::default()
        };

        let binding = ensure_default_repository_root(&mut ws)
            .expect("ensure root")
            .expect("binding");

        assert_eq!(binding.local_ref, "workspace-root");
        assert!(binding.workspace_changed);
        assert!(binding.config_changed);
        let repo = ws.elements.get("workspace-root").expect("root element");
        assert!(repo.has_view);
        assert_eq!(repo.view_label, "kafka");
    }

    #[test]
    fn avoids_ref_collisions_when_creating_repository_root() {
        let mut repositories = HashMap::new();
        repositories.insert(
            "origin".to_string(),
            Repository {
                local_dir: ".".to_string(),
                ..Default::default()
            },
        );

        let mut elements = HashMap::new();
        elements.insert(
            "kafka".to_string(),
            Element {
                name: "Broker".to_string(),
                kind: "service".to_string(),
                ..Default::default()
            },
        );

        let mut ws = Workspace {
            dir: ".".to_string(),
            config: Config::default(),
            ws_config: Some(WorkspaceConfig {
                project_name: "kafka".to_string(),
                repositories,
                ..Default::default()
            }),
            elements,
            ..Default::default()
        };

        let binding = ensure_default_repository_root(&mut ws)
            .expect("ensure root")
            .expect("binding");

        assert_eq!(binding.local_ref, "kafka-2");
        assert!(ws.elements.contains_key("kafka-2"));
    }
}
