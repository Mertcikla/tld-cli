package workspace

import (
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"gopkg.in/yaml.v3"
)

// WriteDiagram adds a diagram to diagrams.yaml. Errors if ref already exists.
func WriteDiagram(dir, ref string, spec *Diagram) error {
	path := filepath.Join(dir, "diagrams.yaml")
	return updateYAMLMap(path, ref, spec)
}

// WriteObject adds an object to objects.yaml. Errors if ref already exists.
func WriteObject(dir, ref string, spec *Object) error {
	path := filepath.Join(dir, "objects.yaml")
	return updateYAMLMap(path, ref, spec)
}

// UpsertObject adds an object to objects.yaml or updates an existing one by adding a placement.
func UpsertObject(dir, ref string, spec *Object) error {
	path := filepath.Join(dir, "objects.yaml")

	// Read existing objects
	existing := make(map[string]*Object)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}

	if old, ok := existing[ref]; ok {
		// Validate type match to prevent accidental reuse of same ref for different things
		if old.Type != spec.Type {
			return fmt.Errorf("object %q already exists with type %q (tried to reuse as %q)", ref, old.Type, spec.Type)
		}

		// Check if placement already exists for this diagram
		newPlacement := spec.Diagrams[0]
		found := false
		for i, p := range old.Diagrams {
			if p.Diagram == newPlacement.Diagram {
				// Update position if it's the same diagram
				old.Diagrams[i].PositionX = newPlacement.PositionX
				old.Diagrams[i].PositionY = newPlacement.PositionY
				found = true
				break
			}
		}
		if !found {
			old.Diagrams = append(old.Diagrams, newPlacement)
		}

		// Enrich metadata if currently empty
		if old.Description == "" {
			old.Description = spec.Description
		}
		if old.Technology == "" {
			old.Technology = spec.Technology
		}
		if old.URL == "" {
			old.URL = spec.URL
		}
	} else {
		// New object
		existing[ref] = spec
	}

	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal objects: %w", err)
	}
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

// AppendEdge adds an Edge to edges.yaml keyed by "diagram:source:target:label" (creates file if absent).
func AppendEdge(dir string, spec *Edge) error {
	path := filepath.Join(dir, "edges.yaml")
	existing := make(map[string]*Edge)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
		delete(existing, "_meta")
	}
	key := spec.Diagram + ":" + spec.SourceObject + ":" + spec.TargetObject + ":" + spec.Label
	existing[key] = spec
	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal edges: %w", err)
	}
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

// AppendLink appends a Link to links.yaml (creates file if absent).
func AppendLink(dir string, spec *Link) error {
	path := filepath.Join(dir, "links.yaml")
	return appendYAMLList(path, spec)
}

// Save writes the entire workspace state to YAML files in ws.Dir.
func Save(ws *Workspace) error {
	// Write diagrams
	if err := writeFullYAMLMap(filepath.Join(ws.Dir, "diagrams.yaml"), ws.Diagrams, ws.Meta.Diagrams); err != nil {
		return fmt.Errorf("write diagrams: %w", err)
	}

	// Write objects
	if err := writeFullYAMLMap(filepath.Join(ws.Dir, "objects.yaml"), ws.Objects, ws.Meta.Objects); err != nil {
		return fmt.Errorf("write objects: %w", err)
	}

	// Write edges
	if len(ws.Edges) > 0 {
		var edgesMeta map[string]*ResourceMetadata
		if ws.Meta != nil {
			edgesMeta = ws.Meta.Edges
		}
		if err := writeFullYAMLMap(filepath.Join(ws.Dir, "edges.yaml"), ws.Edges, edgesMeta); err != nil {
			return fmt.Errorf("write edges: %w", err)
		}
	}

	// Write links
	if len(ws.Links) > 0 {
		data, err := yaml.Marshal(ws.Links)
		if err != nil {
			return fmt.Errorf("marshal links: %w", err)
		}
		if err := os.WriteFile(filepath.Join(ws.Dir, "links.yaml"), data, 0600); err != nil {
			return fmt.Errorf("write links: %w", err)
		}
	}

	return nil
}

func writeFullYAMLMap(path string, items any, meta map[string]*ResourceMetadata) error {
	// Marshal items to a map
	data, err := yaml.Marshal(items)
	if err != nil {
		return fmt.Errorf("marshal items %s: %w", path, err)
	}
	var yamlMap map[string]any
	if err := yaml.Unmarshal(data, &yamlMap); err != nil {
		return fmt.Errorf("unmarshal %s: %w", path, err)
	}
	if len(meta) > 0 {
		metaSection := make(map[string]any)
		for ref, m := range meta {
			metaSection[ref] = map[string]any{
				"id":         m.ID,
				"updated_at": m.UpdatedAt.Format(time.RFC3339),
			}
		}
		yamlMap["_meta"] = metaSection
	}

	// Write back to file
	finalData, err := yaml.Marshal(yamlMap)
	if err != nil {
		return fmt.Errorf("re-marshal %s: %w", path, err)
	}
	if err := os.WriteFile(path, finalData, 0600); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

var slugifyRegex = regexp.MustCompile(`[^a-z0-9]+`)

// Slugify converts "API Service" -> "api-service" for use as a ref/filename.
func Slugify(s string) string {
	s = strings.ToLower(s)
	s = slugifyRegex.ReplaceAllString(s, "-")
	s = strings.Trim(s, "-")
	return s
}

func updateYAMLMap(path, ref string, item any) error {
	// Read existing map (if any)
	existing := make(map[string]any)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}

	if _, ok := existing[ref]; ok {
		return fmt.Errorf("item %q already exists in %s", ref, filepath.Base(path))
	}

	// Marshal item to a map so we can insert it
	itemData, err := yaml.Marshal(item)
	if err != nil {
		return fmt.Errorf("marshal item: %w", err)
	}
	var itemMap map[string]any
	if err := yaml.Unmarshal(itemData, &itemMap); err != nil {
		return fmt.Errorf("unmarshal item: %w", err)
	}
	existing[ref] = itemMap

	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal %s: %w", path, err)
	}
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

func appendYAMLList(path string, item any) error {
	// Read existing list (if any)
	var existing []map[string]any
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}

	// Marshal item to a map so we can append it
	itemData, err := yaml.Marshal(item)
	if err != nil {
		return fmt.Errorf("marshal item for append: %w", err)
	}
	var itemMap map[string]any
	if err := yaml.Unmarshal(itemData, &itemMap); err != nil {
		return fmt.Errorf("unmarshal item for append: %w", err)
	}
	existing = append(existing, itemMap)

	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal list %s: %w", path, err)
	}
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}
