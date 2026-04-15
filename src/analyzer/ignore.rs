use glob::Pattern;
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
        let path_segments: Vec<&str> = normalized_path.split('/').collect();
        let base_name = path_segments.last().copied().unwrap_or("");

        for pattern in &self.exclude {
            if pattern.is_empty() {
                continue;
            }
            let normalized_pattern = self.normalize_pattern(pattern);
            let trimmed_pattern = normalized_pattern.strip_suffix('/').unwrap_or(&normalized_pattern);
            let is_dir_pattern = normalized_pattern.ends_with('/');

            // 1. Direct match or glob match on the full normalized path
            if self.match_pattern(&normalized_pattern, &normalized_path) {
                return true;
            }

            // 2. Match against any segment of the path (for directory patterns or base names)
            for segment in &path_segments {
                if is_dir_pattern {
                    if *segment == trimmed_pattern {
                        return true;
                    }
                } else if self.match_pattern(&normalized_pattern, segment) {
                    return true;
                }
            }

            // 3. Fallback for the base name itself
            if self.match_pattern(&normalized_pattern, base_name) {
                return true;
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
