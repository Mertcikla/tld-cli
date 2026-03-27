package workspace

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// DeleteDiagram removes a diagram from diagrams.yaml and cascades:
//   - removes edges where diagram == ref
//   - removes links where from_diagram or to_diagram == ref
//   - removes placements of this diagram from objects.yaml
//
// Returns (edgesRemoved, linksRemoved, placementsRemoved, error).
// Errors if diagram does not exist.
func DeleteDiagram(dir, ref string) (int, int, int, error) {
	err := filterYAMLMap(filepath.Join(dir, "diagrams.yaml"), func(k string, _ any) bool { return k != ref })
	if err != nil {
		return 0, 0, 0, err
	}

	edgesRemoved, err := filterYAMLList(
		filepath.Join(dir, "edges.yaml"),
		func(m map[string]any) bool { return strVal(m, "diagram") != ref },
	)
	if err != nil {
		return 0, 0, 0, err
	}

	linksRemoved, err := filterYAMLList(
		filepath.Join(dir, "links.yaml"),
		func(m map[string]any) bool {
			return strVal(m, "from_diagram") != ref && strVal(m, "to_diagram") != ref
		},
	)
	if err != nil {
		return 0, 0, edgesRemoved, err
	}

	placementsRemoved, err := removePlacementsForDiagram(dir, ref)
	if err != nil {
		return edgesRemoved, linksRemoved, 0, err
	}

	return edgesRemoved, linksRemoved, placementsRemoved, nil
}

// DeleteObject removes an object from objects.yaml and cascades:
//   - removes edges where source_object or target_object == ref
//   - removes links where object == ref
//
// Returns (edgesRemoved, linksRemoved, error).
// Errors if object does not exist.
func DeleteObject(dir, ref string) (int, int, error) {
	err := filterYAMLMap(filepath.Join(dir, "objects.yaml"), func(k string, _ any) bool { return k != ref })
	if err != nil {
		return 0, 0, err
	}

	edgesRemoved, err := filterYAMLList(
		filepath.Join(dir, "edges.yaml"),
		func(m map[string]any) bool {
			return strVal(m, "source_object") != ref && strVal(m, "target_object") != ref
		},
	)
	if err != nil {
		return 0, 0, err
	}

	linksRemoved, err := filterYAMLList(
		filepath.Join(dir, "links.yaml"),
		func(m map[string]any) bool { return strVal(m, "object") != ref },
	)
	if err != nil {
		return edgesRemoved, 0, err
	}

	return edgesRemoved, linksRemoved, nil
}

// RemoveEdge removes all edges from edges.yaml where
// diagram == diagram AND source_object == source AND target_object == target.
// Safe: no error if file is absent or no matches found.
// Returns count of removed edges.
func RemoveEdge(dir, diagram, source, target string) (int, error) {
	return filterYAMLList(
		filepath.Join(dir, "edges.yaml"),
		func(m map[string]any) bool {
			return strVal(m, "diagram") != diagram ||
				strVal(m, "source_object") != source ||
				strVal(m, "target_object") != target
		},
	)
}

// RemoveLink removes all links from links.yaml where
// from_diagram == fromDiagram AND to_diagram == toDiagram and,
// when object is non-empty, object == object.
// Safe: no error if file is absent or no matches found.
// Returns count of removed links.
func RemoveLink(dir, object, fromDiagram, toDiagram string) (int, error) {
	return filterYAMLList(
		filepath.Join(dir, "links.yaml"),
		func(m map[string]any) bool {
			if strVal(m, "from_diagram") != fromDiagram || strVal(m, "to_diagram") != toDiagram {
				return true // keep
			}
			if object != "" && strVal(m, "object") != object {
				return true // keep
			}
			return false // remove
		},
	)
}

// filterYAMLList reads path as []map[string]any, keeps only items where keep(item)==true,
// writes back, and returns the count removed. Safe: returns 0,nil if file absent.
func filterYAMLList(path string, keep func(map[string]any) bool) (int, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return 0, nil // file absent is fine
	}

	var items []map[string]any
	if err := yaml.Unmarshal(data, &items); err != nil {
		return 0, fmt.Errorf("parse %s: %w", path, err)
	}

	var kept []map[string]any
	for _, item := range items {
		if keep(item) {
			kept = append(kept, item)
		}
	}
	removed := len(items) - len(kept)
	if removed == 0 {
		return 0, nil
	}

	out, err := yaml.Marshal(kept)
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

// removePlacementsForDiagram reads objects.yaml, strips placements where
// diagram == ref, and writes back if changed. Returns total placements removed.
func removePlacementsForDiagram(dir, ref string) (int, error) {
	path := filepath.Join(dir, "objects.yaml")
	data, err := os.ReadFile(path)
	if err != nil {
		return 0, nil // objects.yaml absent
	}

	var objects map[string]*Object
	if err := yaml.Unmarshal(data, &objects); err != nil {
		return 0, fmt.Errorf("parse objects.yaml: %w", err)
	}

	total := 0
	changed := false
	for _, o := range objects {
		before := len(o.Diagrams)
		var filtered []Placement
		for _, p := range o.Diagrams {
			if p.Diagram != ref {
				filtered = append(filtered, p)
			}
		}
		removed := before - len(filtered)
		if removed > 0 {
			o.Diagrams = filtered
			total += removed
			changed = true
		}
	}

	if changed {
		out, err := yaml.Marshal(objects)
		if err != nil {
			return total, fmt.Errorf("marshal objects: %w", err)
		}
		if err := os.WriteFile(path, out, 0600); err != nil {
			return total, fmt.Errorf("write %s: %w", path, err)
		}
	}
	return total, nil
}

func strVal(m map[string]any, key string) string {
	if v, ok := m[key]; ok {
		if s, ok := v.(string); ok {
			return s
		}
	}
	return ""
}
