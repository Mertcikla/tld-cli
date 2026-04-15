use crate::analyzer::TreeSitterService;
use crate::workspace::types::*;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub location: String,
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

pub struct ValidationOptions {
    pub skip_symbols: bool,
    pub strictness: i32,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            skip_symbols: false,
            strictness: 0,
        }
    }
}

impl Workspace {
    pub fn validate(&self, opts: &ValidationOptions) -> Vec<ValidationError> {
        let mut errs = Vec::new();

        // Check for duplicate names
        let mut names = std::collections::HashMap::new();

        for (ref_name, element) in &self.elements {
            let loc = format!("elements.yaml[{}]", ref_name);

            if element.name.is_empty() {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: "name is required".to_string(),
                });
            } else {
                if let Some(existing_ref) = names.get(&element.name) {
                    errs.push(ValidationError {
                        location: loc.clone(),
                        message: format!(
                            "duplicate element name \"{}\" (also used by \"{}\")",
                            element.name, existing_ref
                        ),
                    });
                }
                names.insert(element.name.clone(), ref_name.clone());
            }

            if element.kind.is_empty() {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: "kind is required".to_string(),
                });
            }

            for (index, placement) in element.placements.iter().enumerate() {
                let ploc = format!("{}[placements][{}]", loc, index);
                if placement.parent_ref.is_empty() {
                    errs.push(ValidationError {
                        location: ploc,
                        message: "parent is required".to_string(),
                    });
                } else if placement.parent_ref != "root"
                    && !self.elements.contains_key(&placement.parent_ref)
                {
                    errs.push(ValidationError {
                        location: ploc,
                        message: format!("parent ref \"{}\" not found", placement.parent_ref),
                    });
                }
            }
        }

        for (ref_name, connector) in &self.connectors {
            let loc = format!("connectors.yaml[{}]", ref_name);

            if connector.view.is_empty() {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: "view is required".to_string(),
                });
            } else if connector.view != "root" && !self.elements.contains_key(&connector.view) {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: format!("view ref \"{}\" not found", connector.view),
                });
            }

            if connector.source.is_empty() {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: "source is required".to_string(),
                });
            } else if !self.elements.contains_key(&connector.source) {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: format!("source ref \"{}\" not found", connector.source),
                });
            }

            if connector.target.is_empty() {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: "target is required".to_string(),
                });
            } else if !self.elements.contains_key(&connector.target) {
                errs.push(ValidationError {
                    location: loc.clone(),
                    message: format!("target ref \"{}\" not found", connector.target),
                });
            }
        }

        if !opts.skip_symbols {
            errs.extend(self.validate_symbols());
        }

        errs.extend(self.validate_conflict_markers());

        errs
    }

    fn validate_conflict_markers(&self) -> Vec<ValidationError> {
        let mut errs = Vec::new();
        let markers = ["<<< LOCAL", ">>> SERVER"];

        for (ref_name, element) in &self.elements {
            let loc = format!("elements.yaml[{}]", ref_name);
            for marker in markers {
                if element.name.contains(marker)
                    || element.description.contains(marker)
                    || element.technology.contains(marker)
                {
                    errs.push(ValidationError {
                        location: loc.clone(),
                        message: "unresolved merge conflict".to_string(),
                    });
                    break;
                }
            }
        }

        for (ref_name, connector) in &self.connectors {
            let loc = format!("connectors.yaml[{}]", ref_name);
            for marker in markers {
                if connector.label.contains(marker)
                    || connector.description.contains(marker)
                    || connector.relationship.contains(marker)
                {
                    errs.push(ValidationError {
                        location: loc.clone(),
                        message: "unresolved merge conflict".to_string(),
                    });
                    break;
                }
            }
        }

        errs
    }

    fn validate_symbols(&self) -> Vec<ValidationError> {
        let mut errs = Vec::new();
        let service = TreeSitterService::new();

        for (ref_name, element) in &self.elements {
            if element.file_path.is_empty() || element.symbol.is_empty() {
                continue;
            }

            let abs_path = std::path::Path::new(&self.dir).join(&element.file_path);
            if !abs_path.exists() {
                continue;
            }

            match service.extract_file(abs_path.to_str().unwrap_or("")) {
                Ok(result) => {
                    let found = result.symbols.iter().any(|s| s.name == element.symbol);
                    if !found {
                        errs.push(ValidationError {
                            location: format!("elements.yaml[{}]", ref_name),
                            message: format!(
                                "symbol \"{}\" not found in {}",
                                element.symbol, element.file_path
                            ),
                        });
                    }
                }
                Err(e) => {
                    // Ignore unsupported languages or other parsing errors for now to match Go behavior
                    if !e.to_string().contains("Unsupported language")
                        && !e.to_string().contains("Parser logic not yet implemented")
                    {
                        errs.push(ValidationError {
                            location: format!("elements.yaml[{}]", ref_name),
                            message: format!("symbol verification failed: {}", e),
                        });
                    }
                }
            }
        }

        errs
    }

    pub fn check_outdated(&self) -> Vec<String> {
        let mut outdated = Vec::new();
        let meta = match &self.meta {
            Some(m) => m,
            None => return Vec::new(),
        };

        for (ref_name, element) in &self.elements {
            if element.file_path.is_empty() {
                continue;
            }

            let m = match meta.elements.get(ref_name) {
                Some(m) => m,
                None => continue,
            };

            if let Ok(commit_time) =
                crate::workspace::get_file_last_commit_at(&self.dir, &element.file_path)
            {
                if commit_time > m.updated_at {
                    outdated.push(format!(
                        "elements.yaml[{}]: file {} changed {}, diagram last synced {}",
                        ref_name,
                        element.file_path,
                        commit_time.format("%Y-%m-%d %H:%M:%S"),
                        m.updated_at.format("%Y-%m-%d %H:%M:%S")
                    ));
                }
            }
        }
        outdated
    }
}
