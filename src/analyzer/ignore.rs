use glob::Pattern;
use std::path::Path;

#[derive(Debug, Default, Clone)]
pub struct Rules {
    pub exclude: Vec<String>,
}

impl Rules {
    pub fn new(exclude: Vec<String>) -> Self {
        Self { exclude }
    }

    pub fn should_ignore_path(&self, path: &str) -> bool {
        let normalized_path = self.normalize_path(path);
        let base_name = Path::new(&normalized_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        for pattern in &self.exclude {
            if pattern.is_empty() {
                continue;
            }
            let normalized_pattern = self.normalize_pattern(pattern);

            if self.match_pattern(&normalized_pattern, &normalized_path)
                || self.match_pattern(&normalized_pattern, base_name)
            {
                return true;
            }

            // Handle trailing slash (directory matching)
            if let Some(trimmed) = normalized_pattern.strip_suffix('/') {
                if normalized_path == trimmed
                    || normalized_path.starts_with(&format!("{}/", trimmed))
                    || base_name == trimmed
                {
                    return true;
                }
            }
        }
        false
    }

    fn match_pattern(&self, pattern: &str, value: &str) -> bool {
        match Pattern::new(pattern) {
            Ok(p) => p.matches(value),
            Err(_) => pattern == value,
        }
    }

    fn normalize_path(&self, path: &str) -> String {
        path.trim()
            .replace('\\', "/")
            .trim_start_matches("./")
            .trim_start_matches('/')
            .to_string()
    }

    fn normalize_pattern(&self, pattern: &str) -> String {
        pattern
            .trim()
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string()
    }
}
