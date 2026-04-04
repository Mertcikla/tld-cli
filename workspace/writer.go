package workspace

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// WriteDiagram adds a diagram to diagrams.yaml. Errors if ref already exists.
func WriteDiagram(dir, ref string, spec *Diagram) error {
	path := filepath.Join(dir, "diagrams.yaml")
	return updateYAMLMap(path, ref, spec)
}

// UpdateDiagram overwrites a diagram in diagrams.yaml.
func UpdateDiagram(dir, ref string, spec *Diagram) error {
	path := filepath.Join(dir, "diagrams.yaml")
	existing := make(map[string]*Diagram)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}
	existing[ref] = spec
	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal diagrams: %w", err)
	}
	return os.WriteFile(path, data, 0600)
}

// WriteObject adds an object to objects.yaml. Errors if ref already exists.
func WriteObject(dir, ref string, spec *Object) error {
	path := filepath.Join(dir, "objects.yaml")
	return updateYAMLMap(path, ref, spec)
}

// UpdateObject overwrites an object in objects.yaml.
func UpdateObject(dir, ref string, spec *Object) error {
	path := filepath.Join(dir, "objects.yaml")
	existing := make(map[string]*Object)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}
	existing[ref] = spec
	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal objects: %w", err)
	}
	return os.WriteFile(path, data, 0600)
}

// UpsertObject adds an object to objects.yaml or updates an existing one by adding a placement.
func UpsertObject(dir, ref string, spec *Object) error {
	path := filepath.Join(dir, "objects.yaml")
	existing := make(map[string]*Object)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}

	if old, ok := existing[ref]; ok {
		if old.Type != spec.Type {
			return fmt.Errorf("object %q already exists with type %q (tried to reuse as %q)", ref, old.Type, spec.Type)
		}
		newPlacement := spec.Diagrams[0]
		found := false
		for i, p := range old.Diagrams {
			if p.Diagram == newPlacement.Diagram {
				old.Diagrams[i].PositionX = newPlacement.PositionX
				old.Diagrams[i].PositionY = newPlacement.PositionY
				found = true
				break
			}
		}
		if !found {
			old.Diagrams = append(old.Diagrams, newPlacement)
		}
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
		existing[ref] = spec
	}
	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal objects: %w", err)
	}
	return os.WriteFile(path, data, 0600)
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
	return os.WriteFile(path, data, 0600)
}

// UpdateEdge overwrites an edge in edges.yaml.
func UpdateEdge(dir, key string, spec *Edge) error {
	path := filepath.Join(dir, "edges.yaml")
	existing := make(map[string]*Edge)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}
	existing[key] = spec
	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal edges: %w", err)
	}
	return os.WriteFile(path, data, 0600)
}

// AppendLink appends a Link to links.yaml (creates file if absent).
func AppendLink(dir string, spec *Link) error {
	path := filepath.Join(dir, "links.yaml")
	return appendYAMLList(path, spec)
}

// Save writes the entire workspace state to YAML files in ws.Dir.
func Save(ws *Workspace) error {
	// Write diagrams
	if err := WriteFullYAMLMap(filepath.Join(ws.Dir, "diagrams.yaml"), ws.Diagrams, ws.Meta.Diagrams); err != nil {
		return fmt.Errorf("write diagrams: %w", err)
	}

	// Write objects
	if err := WriteFullYAMLMap(filepath.Join(ws.Dir, "objects.yaml"), ws.Objects, ws.Meta.Objects); err != nil {
		return fmt.Errorf("write objects: %w", err)
	}

	// Write edges
	if len(ws.Edges) > 0 {
		var edgesMeta map[string]*ResourceMetadata
		if ws.Meta != nil {
			edgesMeta = ws.Meta.Edges
		}
		if err := WriteFullYAMLMap(filepath.Join(ws.Dir, "edges.yaml"), ws.Edges, edgesMeta); err != nil {
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

func WriteFullYAMLMap(path string, items any, meta map[string]*ResourceMetadata) error {
	// 1. Marshal items to a node
	var node yaml.Node
	if err := node.Encode(items); err != nil {
		return fmt.Errorf("encode items for %s: %w", path, err)
	}

	// 2. Add _meta section if present
	if len(meta) > 0 {
		var mapping *yaml.Node
		if node.Kind == yaml.DocumentNode && len(node.Content) > 0 {
			mapping = node.Content[0]
		} else if node.Kind == yaml.MappingNode {
			mapping = &node
		}

		if mapping != nil && mapping.Kind == yaml.MappingNode {
			// Add _meta key
			mapping.Content = append(mapping.Content,
				&yaml.Node{Kind: yaml.ScalarNode, Value: "_meta"},
				EncodeMeta(meta),
			)
		}
	}

	// 3. Write back to file with specific indentation
	f, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create %s: %w", path, err)
	}
	defer f.Close()

	enc := yaml.NewEncoder(f)
	enc.SetIndent(2)
	if err := enc.Encode(&node); err != nil {
		return fmt.Errorf("encode %s: %w", path, err)
	}

	return nil
}

func EncodeMeta(meta map[string]*ResourceMetadata) *yaml.Node {
	metaNode := &yaml.Node{Kind: yaml.MappingNode}
	// Sort keys for determinism (yaml.v3 does this anyway for maps, but we are building node)
	// Actually, let's just use Encode and then move it into our node
	data, _ := yaml.Marshal(meta)
	var n yaml.Node
	_ = yaml.Unmarshal(data, &n)
	if len(n.Content) > 0 {
		return n.Content[0]
	}
	return metaNode
}

// RenameDiagram changes a diagram ref in diagrams.yaml and cascades to all references.
func RenameDiagram(dir, oldRef, newRef string) error {
	if oldRef == newRef {
		return nil
	}

	// 1. Update diagrams.yaml
	path := filepath.Join(dir, "diagrams.yaml")
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("read diagrams.yaml: %w", err)
	}
	var diagrams map[string]any
	if err := yaml.Unmarshal(data, &diagrams); err != nil {
		return fmt.Errorf("parse diagrams.yaml: %w", err)
	}
	spec, ok := diagrams[oldRef]
	if !ok {
		return fmt.Errorf("diagram %q not found", oldRef)
	}
	if _, exists := diagrams[newRef]; exists {
		return fmt.Errorf("diagram %q already exists", newRef)
	}
	diagrams[newRef] = spec
	delete(diagrams, oldRef)

	// Update _meta in diagrams.yaml
	if meta, ok := diagrams["_meta"].(map[string]any); ok {
		if m, ok := meta[oldRef]; ok {
			meta[newRef] = m
			delete(meta, oldRef)
		}
	}

	out, err := yaml.Marshal(diagrams)
	if err != nil {
		return fmt.Errorf("marshal diagrams: %w", err)
	}
	if err := os.WriteFile(path, out, 0600); err != nil {
		return fmt.Errorf("write diagrams.yaml: %w", err)
	}

	// 2. Update objects.yaml (placements)
	objPath := filepath.Join(dir, "objects.yaml")
	if data, err := os.ReadFile(objPath); err == nil {
		var objects map[string]*Object
		if err := yaml.Unmarshal(data, &objects); err == nil {
			changed := false
			for _, o := range objects {
				for i, p := range o.Diagrams {
					if p.Diagram == oldRef {
						o.Diagrams[i].Diagram = newRef
						changed = true
					}
				}
			}
			if changed {
				out, _ := yaml.Marshal(objects)
				_ = os.WriteFile(objPath, out, 0600)
			}
		}
	}

	// 3. Update edges.yaml (diagram field and keys)
	edgePath := filepath.Join(dir, "edges.yaml")
	if data, err := os.ReadFile(edgePath); err == nil {
		var edges map[string]any
		if err := yaml.Unmarshal(data, &edges); err == nil {
			newEdges := make(map[string]any)
			changed := false
			for k, v := range edges {
				if k == "_meta" {
					newEdges[k] = v
					// Update _meta keys if needed
					if meta, ok := v.(map[string]any); ok {
						for mk, mv := range meta {
							if strings.HasPrefix(mk, oldRef+":") {
								newMk := newRef + mk[len(oldRef):]
								meta[newMk] = mv
								delete(meta, mk)
								changed = true
							}
						}
					}
					continue
				}
				if m, ok := v.(map[string]any); ok {
					if d, ok := m["diagram"].(string); ok && d == oldRef {
						m["diagram"] = newRef
						newKey := newRef + k[len(oldRef):]
						newEdges[newKey] = m
						changed = true
					} else {
						newEdges[k] = v
					}
				} else {
					newEdges[k] = v
				}
			}
			if changed {
				out, _ := yaml.Marshal(newEdges)
				_ = os.WriteFile(edgePath, out, 0600)
			}
		}
	}

	// 4. Update links.yaml
	linkPath := filepath.Join(dir, "links.yaml")
	if data, err := os.ReadFile(linkPath); err == nil {
		var links []map[string]any
		if err := yaml.Unmarshal(data, &links); err == nil {
			changed := false
			for _, l := range links {
				if f, ok := l["from_diagram"].(string); ok && f == oldRef {
					l["from_diagram"] = newRef
					changed = true
				}
				if t, ok := l["to_diagram"].(string); ok && t == oldRef {
					l["to_diagram"] = newRef
					changed = true
				}
			}
			if changed {
				out, _ := yaml.Marshal(links)
				_ = os.WriteFile(linkPath, out, 0600)
			}
		}
	}

	return nil
}

// RenameObject changes an object ref in objects.yaml and cascades to all references.
func RenameObject(dir, oldRef, newRef string) error {
	if oldRef == newRef {
		return nil
	}

	// 1. Update objects.yaml
	path := filepath.Join(dir, "objects.yaml")
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("read objects.yaml: %w", err)
	}
	var objects map[string]any
	if err := yaml.Unmarshal(data, &objects); err != nil {
		return fmt.Errorf("parse objects.yaml: %w", err)
	}
	spec, ok := objects[oldRef]
	if !ok {
		return fmt.Errorf("object %q not found", oldRef)
	}
	if _, exists := objects[newRef]; exists {
		return fmt.Errorf("object %q already exists", newRef)
	}
	objects[newRef] = spec
	delete(objects, oldRef)

	// Update _meta in objects.yaml
	if meta, ok := objects["_meta"].(map[string]any); ok {
		if m, ok := meta[oldRef]; ok {
			meta[newRef] = m
			delete(meta, oldRef)
		}
	}

	out, err := yaml.Marshal(objects)
	if err != nil {
		return fmt.Errorf("marshal objects: %w", err)
	}
	if err := os.WriteFile(path, out, 0600); err != nil {
		return fmt.Errorf("write objects.yaml: %w", err)
	}

	// 2. Update edges.yaml (source_object and target_object fields and keys)
	edgePath := filepath.Join(dir, "edges.yaml")
	if data, err := os.ReadFile(edgePath); err == nil {
		var edges map[string]any
		if err := yaml.Unmarshal(data, &edges); err == nil {
			newEdges := make(map[string]any)
			changed := false
			for k, v := range edges {
				if k == "_meta" {
					newEdges[k] = v
					// Update _meta keys if needed
					if meta, ok := v.(map[string]any); ok {
						for mk, mv := range meta {
							parts := strings.Split(mk, ":")
							if len(parts) >= 3 {
								diag, src, tgt := parts[0], parts[1], parts[2]
								label := ""
								if len(parts) > 3 {
									label = strings.Join(parts[3:], ":")
								}
								newSrc, newTgt := src, tgt
								metaChanged := false
								if src == oldRef {
									newSrc = newRef
									metaChanged = true
								}
								if tgt == oldRef {
									newTgt = newRef
									metaChanged = true
								}
								if metaChanged {
									newMk := diag + ":" + newSrc + ":" + newTgt + ":" + label
									meta[newMk] = mv
									delete(meta, mk)
									changed = true
								}
							}
						}
					}
					continue
				}
				if m, ok := v.(map[string]any); ok {
					src, _ := m["source_object"].(string)
					tgt, _ := m["target_object"].(string)
					edgeChanged := false
					if src == oldRef {
						m["source_object"] = newRef
						edgeChanged = true
					}
					if tgt == oldRef {
						m["target_object"] = newRef
						edgeChanged = true
					}
					if edgeChanged {
						parts := strings.Split(k, ":")
						if len(parts) >= 3 {
							diag := parts[0]
							label := ""
							if len(parts) > 3 {
								label = strings.Join(parts[3:], ":")
							}
							newKey := diag + ":" + m["source_object"].(string) + ":" + m["target_object"].(string) + ":" + label
							newEdges[newKey] = m
							changed = true
						} else {
							newEdges[k] = m
						}
					} else {
						newEdges[k] = v
					}
				} else {
					newEdges[k] = v
				}
			}
			if changed {
				out, _ := yaml.Marshal(newEdges)
				_ = os.WriteFile(edgePath, out, 0600)
			}
		}
	}

	// 3. Update links.yaml
	linkPath := filepath.Join(dir, "links.yaml")
	if data, err := os.ReadFile(linkPath); err == nil {
		var links []map[string]any
		if err := yaml.Unmarshal(data, &links); err == nil {
			changed := false
			for _, l := range links {
				if o, ok := l["object"].(string); ok && o == oldRef {
					l["object"] = newRef
					changed = true
				}
			}
			if changed {
				out, _ := yaml.Marshal(links)
				_ = os.WriteFile(linkPath, out, 0600)
			}
		}
	}

	return nil
}

// Slugify converts "API Service" -> "api-service" for use as a ref/filename.
func Slugify(s string) string {
	s = strings.ToLower(s)
	// Replace all non-alphanumeric characters with hyphens
	var result strings.Builder
	for _, r := range s {
		if (r >= 'a' && r <= 'z') || (r >= '0' && r <= '9') {
			result.WriteRune(r)
		} else {
			result.WriteRune('-')
		}
	}
	s = result.String()
	// Clean up multiple hyphens
	for strings.Contains(s, "--") {
		s = strings.ReplaceAll(s, "--", "-")
	}
	return strings.Trim(s, "-")
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
