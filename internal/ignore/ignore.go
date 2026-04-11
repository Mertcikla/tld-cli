// Package ignore provides rule-based filtering for repos, folders, files, and symbols
// used by tld analyze and tld check commands.
package ignore

import (
	"path/filepath"
	"strings"
)

// Rules holds ignore patterns loaded from tld/ignore.yaml.
type Rules struct {
	Repos   []string `yaml:"repos"`
	Folders []string `yaml:"folders"`
	Files   []string `yaml:"files"`
	Symbols []string `yaml:"symbols"`
}

// ShouldIgnoreRepo returns true if the given remote URL matches any repo pattern.
func (r *Rules) ShouldIgnoreRepo(url string) bool {
	if r == nil {
		return false
	}
	for _, pattern := range r.Repos {
		if matchPattern(pattern, url) {
			return true
		}
	}
	return false
}

// ShouldIgnoreFolder returns true if the given folder path matches any folder pattern.
// path may be relative (e.g. "vendor/foo") or just a folder name (e.g. "vendor").
func (r *Rules) ShouldIgnoreFolder(path string) bool {
	if r == nil {
		return false
	}
	for _, pattern := range r.Folders {
		// Trim trailing slash from pattern for consistent matching
		trimmed := strings.TrimSuffix(pattern, "/")
		if matchPattern(trimmed, path) {
			return true
		}
		// Also check if any path segment matches
		if matchPattern(trimmed, filepath.Base(path)) {
			return true
		}
		// Check if path starts with pattern (for prefix matching like "vendor/")
		if strings.HasPrefix(path, strings.TrimSuffix(pattern, "/")+"/") {
			return true
		}
		if path == strings.TrimSuffix(pattern, "/") {
			return true
		}
	}
	return false
}

// ShouldIgnoreFile returns true if the given file path matches any file pattern.
func (r *Rules) ShouldIgnoreFile(path string) bool {
	if r == nil {
		return false
	}
	base := filepath.Base(path)
	for _, pattern := range r.Files {
		if matchPattern(pattern, base) {
			return true
		}
		if matchPattern(pattern, path) {
			return true
		}
	}
	return false
}

// ShouldIgnoreSymbol returns true if the given symbol name matches any symbol pattern.
func (r *Rules) ShouldIgnoreSymbol(name string) bool {
	if r == nil {
		return false
	}
	for _, pattern := range r.Symbols {
		if matchPattern(pattern, name) {
			return true
		}
	}
	return false
}

// matchPattern matches a value against a pattern using filepath.Match glob syntax.
// Falls back to exact string equality if the pattern contains no glob characters.
func matchPattern(pattern, value string) bool {
	matched, err := filepath.Match(pattern, value)
	if err != nil {
		// Invalid pattern — treat as exact match
		return pattern == value
	}
	return matched
}
