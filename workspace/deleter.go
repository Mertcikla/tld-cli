package workspace

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// RemoveElement removes an element from elements.yaml.
func RemoveElement(dir, ref string) error {
	return filterYAMLMap(filepath.Join(dir, "elements.yaml"), func(k string, _ any) bool { return k != ref })
}

// RemoveConnector removes connectors from connectors.yaml where view == view AND source == source AND target == target.
func RemoveConnector(dir, view, source, target string) (int, error) {
	keep := func(m map[string]any) bool {
		return strVal(m, "view") != view ||
			strVal(m, "source") != source ||
			strVal(m, "target") != target
	}
	path := filepath.Join(dir, "connectors.yaml")
	data, err := os.ReadFile(path)
	if err != nil {
		return 0, nil // file absent is fine
	}
	var items map[string]any
	if err := yaml.Unmarshal(data, &items); err != nil {
		return 0, fmt.Errorf("parse %s: %w", path, err)
	}
	removed := 0
	for k, v := range items {
		if k == "_meta" {
			continue
		}
		if m, ok := v.(map[string]any); ok {
			if !keep(m) {
				delete(items, k)
				removed++
			}
		}
	}
	if removed == 0 {
		return 0, nil
	}
	out, err := yaml.Marshal(items)
	if err != nil {
		return 0, fmt.Errorf("marshal %s: %w", path, err)
	}
	if err := os.WriteFile(path, out, 0600); err != nil {
		return 0, fmt.Errorf("write %s: %w", path, err)
	}
	return removed, nil
}

// filterYAMLMap reads path as map[string]any, keeps only items where keep(key, val)==true,
// writes back, and returns error if key not found or write fails.
func filterYAMLMap(path string, keep func(string, any) bool) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil // file absent is fine
	}

	var items map[string]any
	if err := yaml.Unmarshal(data, &items); err != nil {
		return fmt.Errorf("parse %s: %w", path, err)
	}

	before := len(items)
	kept := make(map[string]any)
	for k, v := range items {
		if keep(k, v) {
			kept[k] = v
		}
	}

	if len(kept) == before {
		return nil
	}

	out, err := yaml.Marshal(kept)
	if err != nil {
		return fmt.Errorf("marshal %s: %w", path, err)
	}
	if err := os.WriteFile(path, out, 0600); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

func strVal(m map[string]any, key string) string {
	if v, ok := m[key]; ok {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}
