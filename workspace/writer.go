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
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write diagrams.yaml: %w", err)
	}
	return nil
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
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write objects.yaml: %w", err)
	}
	return nil
}

// UpsertObject adds an object to objects.yaml or updates an existing one by adding a placement.
func UpsertObject(dir, ref string, spec *Object) error {
	return upsertYAMLNodeKey(filepath.Join(dir, "objects.yaml"), ref, spec)
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
		return fmt.Errorf("write edges.yaml: %w", err)
	}
	return nil
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
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write edges.yaml: %w", err)
	}
	return nil
}

// AppendLink appends a Link to links.yaml (creates file if absent).
func AppendLink(dir string, spec *Link) error {
	path := filepath.Join(dir, "links.yaml")
	return appendYAMLList(path, spec)
}

// WriteElement adds an element to elements.yaml. Errors if ref already exists.
func WriteElement(dir, ref string, spec *Element) error {
	path := filepath.Join(dir, "elements.yaml")
	return updateYAMLMap(path, ref, spec)
}

// UpdateElement overwrites an element in elements.yaml.
func UpdateElement(dir, ref string, spec *Element) error {
	path := filepath.Join(dir, "elements.yaml")
	existing := make(map[string]*Element)
	if data, err := os.ReadFile(path); err == nil {
		_ = yaml.Unmarshal(data, &existing)
	}
	existing[ref] = spec
	data, err := yaml.Marshal(existing)
	if err != nil {
		return fmt.Errorf("marshal elements: %w", err)
	}
	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write elements.yaml: %w", err)
	}
	return nil
}

// UpsertElement adds an element to elements.yaml or updates placements on an existing one.
func UpsertElement(dir, ref string, spec *Element) error {
	return upsertYAMLNodeKey(filepath.Join(dir, "elements.yaml"), ref, spec)
}

// AppendConnector adds a connector to connectors.yaml keyed by view:source:target:label.
func AppendConnector(dir string, spec *Connector) error {
	return upsertYAMLNodeKey(filepath.Join(dir, "connectors.yaml"), connectorKey(spec), spec)
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

func upsertYAMLNodeKey(path, ref string, spec any) error {
	root, mapping, err := loadYAMLMappingNode(path)
	if err != nil {
		return err
	}

	for i := 0; i+1 < len(mapping.Content); i += 2 {
		if mapping.Content[i].Value != ref {
			continue
		}

		mergedSpec, err := mergeExistingSpec(ref, mapping.Content[i+1], spec)
		if err != nil {
			return err
		}
		newValue, err := encodeYAMLValueNode(mergedSpec)
		if err != nil {
			return fmt.Errorf("encode %s: %w", ref, err)
		}
		mapping.Content[i+1] = newValue
		return writeYAMLNode(path, root)
	}

	newValue, err := encodeYAMLValueNode(spec)
	if err != nil {
		return fmt.Errorf("encode %s: %w", ref, err)
	}
	mapping.Content = append(mapping.Content,
		&yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: ref},
		newValue,
	)
	return writeYAMLNode(path, root)
}

func loadYAMLMappingNode(path string) (*yaml.Node, *yaml.Node, error) {
	var root yaml.Node
	data, err := os.ReadFile(path)
	if err == nil {
		if err := yaml.Unmarshal(data, &root); err != nil {
			return nil, nil, fmt.Errorf("parse %s: %w", path, err)
		}
	} else if !os.IsNotExist(err) {
		return nil, nil, fmt.Errorf("read %s: %w", path, err)
	}

	if root.Kind == 0 {
		root = yaml.Node{Kind: yaml.DocumentNode, Content: []*yaml.Node{{Kind: yaml.MappingNode}}}
	}
	if root.Kind != yaml.DocumentNode || len(root.Content) == 0 || root.Content[0].Kind != yaml.MappingNode {
		return nil, nil, fmt.Errorf("%s must contain a YAML mapping", filepath.Base(path))
	}
	return &root, root.Content[0], nil
}

func encodeYAMLValueNode(spec any) (*yaml.Node, error) {
	var value yaml.Node
	if err := value.Encode(spec); err != nil {
		return nil, err
	}
	if value.Kind == yaml.DocumentNode {
		if len(value.Content) == 0 {
			return &yaml.Node{}, nil
		}
		return value.Content[0], nil
	}
	return &value, nil
}

func mergeExistingSpec(ref string, existingNode *yaml.Node, spec any) (any, error) {
	switch incoming := spec.(type) {
	case *Object:
		var existing Object
		if err := existingNode.Decode(&existing); err != nil {
			return nil, fmt.Errorf("decode existing object %q: %w", ref, err)
		}
		return mergeObjectFields(ref, &existing, incoming)
	case *Element:
		var existing Element
		if err := existingNode.Decode(&existing); err != nil {
			return nil, fmt.Errorf("decode existing element %q: %w", ref, err)
		}
		return mergeElementFields(ref, &existing, incoming)
	default:
		return spec, nil
	}
}

func mergeObjectFields(ref string, existing, incoming *Object) (*Object, error) {
	if existing.Type != incoming.Type {
		return nil, fmt.Errorf("object %q already exists with type %q (tried to reuse as %q)", ref, existing.Type, incoming.Type)
	}

	merged := *existing
	if merged.Name == "" {
		merged.Name = incoming.Name
	}
	if merged.Description == "" {
		merged.Description = incoming.Description
	}
	if merged.Technology == "" {
		merged.Technology = incoming.Technology
	}
	if merged.URL == "" {
		merged.URL = incoming.URL
	}
	for _, newPlacement := range incoming.Diagrams {
		found := false
		for index, placement := range merged.Diagrams {
			if placement.Diagram == newPlacement.Diagram {
				merged.Diagrams[index].PositionX = newPlacement.PositionX
				merged.Diagrams[index].PositionY = newPlacement.PositionY
				found = true
				break
			}
		}
		if !found {
			merged.Diagrams = append(merged.Diagrams, newPlacement)
		}
	}
	return &merged, nil
}

func mergeElementFields(ref string, existing, incoming *Element) (*Element, error) {
	if existing.Kind != incoming.Kind {
		return nil, fmt.Errorf("element %q already exists with kind %q (tried to reuse as %q)", ref, existing.Kind, incoming.Kind)
	}

	merged := *existing
	if merged.Name == "" {
		merged.Name = incoming.Name
	}
	if merged.Description == "" {
		merged.Description = incoming.Description
	}
	if merged.Technology == "" {
		merged.Technology = incoming.Technology
	}
	if merged.URL == "" {
		merged.URL = incoming.URL
	}
	if incoming.HasView {
		merged.HasView = true
		if merged.ViewLabel == "" {
			merged.ViewLabel = incoming.ViewLabel
		}
	}
	for _, newPlacement := range incoming.Placements {
		found := false
		for index, placement := range merged.Placements {
			if placement.ParentRef == newPlacement.ParentRef {
				merged.Placements[index].PositionX = newPlacement.PositionX
				merged.Placements[index].PositionY = newPlacement.PositionY
				found = true
				break
			}
		}
		if !found {
			merged.Placements = append(merged.Placements, newPlacement)
		}
	}
	return &merged, nil
}

func connectorKey(spec *Connector) string {
	return spec.View + ":" + spec.Source + ":" + spec.Target + ":" + spec.Label
}

func writeYAMLNode(path string, root *yaml.Node) error {
	f, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create %s: %w", path, err)
	}
	defer func() { _ = f.Close() }()

	enc := yaml.NewEncoder(f)
	enc.SetIndent(2)
	if err := enc.Encode(root); err != nil {
		return fmt.Errorf("encode %s: %w", path, err)
	}
	if err := f.Close(); err != nil {
		return fmt.Errorf("close %s: %w", path, err)
	}
	return nil
}

// WriteFullYAMLMap writes a map of items to a YAML file, including an optional _meta section.
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
			metaNode, err := EncodeMeta(meta)
			if err != nil {
				return fmt.Errorf("encode meta for %s: %w", path, err)
			}
			// Add _meta key
			mapping.Content = append(mapping.Content,
				&yaml.Node{Kind: yaml.ScalarNode, Value: "_meta"},
				metaNode,
			)
		}
	}

	// 3. Write back to file with specific indentation
	f, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create %s: %w", path, err)
	}
	defer func() {
		_ = f.Close()
	}()

	enc := yaml.NewEncoder(f)
	enc.SetIndent(2)
	if err := enc.Encode(&node); err != nil {
		return fmt.Errorf("encode %s: %w", path, err)
	}

	if err := f.Close(); err != nil {
		return fmt.Errorf("close %s: %w", path, err)
	}

	return nil
}

// EncodeMeta converts ResourceMetadata map into a yaml.Node.
func EncodeMeta(meta map[string]*ResourceMetadata) (*yaml.Node, error) {
	metaNode := &yaml.Node{Kind: yaml.MappingNode}
	// Sort keys for determinism (yaml.v3 does this anyway for maps, but we are building node)
	// Actually, let's just use Encode and then move it into our node
	data, err := yaml.Marshal(meta)
	if err != nil {
		return nil, fmt.Errorf("marshal meta: %w", err)
	}
	var n yaml.Node
	if err := yaml.Unmarshal(data, &n); err != nil {
		return nil, fmt.Errorf("unmarshal meta: %w", err)
	}
	if len(n.Content) > 0 {
		return n.Content[0], nil
	}
	return metaNode, nil
}

// RenameDiagram changes a diagram ref in diagrams.yaml and cascades to all references.
func RenameDiagram(dir, oldRef, newRef string) error {
	if oldRef == newRef {
		return nil
	}

	// 1. Update diagrams.yaml
	if err := updateDiagramsYamlForRename(dir, oldRef, newRef); err != nil {
		return err
	}

	// 2. Update objects.yaml (placements)
	if err := updateObjectsYamlForDiagramRename(dir, oldRef, newRef); err != nil {
		return err
	}

	// 3. Update edges.yaml (diagram field and keys)
	if err := updateEdgesYamlForDiagramRename(dir, oldRef, newRef); err != nil {
		return err
	}

	// 4. Update links.yaml
	if err := updateLinksYamlForDiagramRename(dir, oldRef, newRef); err != nil {
		return err
	}

	return nil
}

func updateDiagramsYamlForRename(dir, oldRef, newRef string) error {
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
	return nil
}

func updateObjectsYamlForDiagramRename(dir, oldRef, newRef string) error {
	objPath := filepath.Join(dir, "objects.yaml")
	data, err := os.ReadFile(objPath)
	if err != nil {
		return nil // File might not exist, which is fine
	}
	var objects map[string]*Object
	if err := yaml.Unmarshal(data, &objects); err != nil {
		return nil // Invalid YAML or not objects.yaml, skip
	}
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
		out, err := yaml.Marshal(objects)
		if err != nil {
			return fmt.Errorf("marshal objects: %w", err)
		}
		if err := os.WriteFile(objPath, out, 0600); err != nil {
			return fmt.Errorf("write objects.yaml: %w", err)
		}
	}
	return nil
}

func updateEdgesYamlForDiagramRename(dir, oldRef, newRef string) error {
	edgePath := filepath.Join(dir, "edges.yaml")
	data, err := os.ReadFile(edgePath)
	if err != nil {
		return nil
	}
	var edges map[string]any
	if err := yaml.Unmarshal(data, &edges); err != nil {
		return nil
	}
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
		out, err := yaml.Marshal(newEdges)
		if err != nil {
			return fmt.Errorf("marshal edges: %w", err)
		}
		if err := os.WriteFile(edgePath, out, 0600); err != nil {
			return fmt.Errorf("write edges.yaml: %w", err)
		}
	}
	return nil
}

func updateLinksYamlForDiagramRename(dir, oldRef, newRef string) error {
	linkPath := filepath.Join(dir, "links.yaml")
	data, err := os.ReadFile(linkPath)
	if err != nil {
		return nil
	}
	var links []map[string]any
	if err := yaml.Unmarshal(data, &links); err != nil {
		return nil
	}
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
		out, err := yaml.Marshal(links)
		if err != nil {
			return fmt.Errorf("marshal links: %w", err)
		}
		if err := os.WriteFile(linkPath, out, 0600); err != nil {
			return fmt.Errorf("write links.yaml: %w", err)
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
	if err := updateObjectsYamlForRename(dir, oldRef, newRef); err != nil {
		return err
	}

	// 2. Update edges.yaml (source_object and target_object fields and keys)
	if err := updateEdgesYamlForObjectRename(dir, oldRef, newRef); err != nil {
		return err
	}

	// 3. Update links.yaml
	if err := updateLinksYamlForObjectRename(dir, oldRef, newRef); err != nil {
		return err
	}

	return nil
}

// RenameElement changes an element ref in elements.yaml and cascades to connectors.
func RenameElement(dir, oldRef, newRef string) error {
	if oldRef == newRef {
		return nil
	}

	path := filepath.Join(dir, "elements.yaml")
	root, mapping, err := loadYAMLMappingNode(path)
	if err != nil {
		return err
	}

	for i := 0; i+1 < len(mapping.Content); i += 2 {
		keyNode := mapping.Content[i]
		valNode := mapping.Content[i+1]
		if keyNode.Value == "_meta_elements" || keyNode.Value == "_meta_views" {
			continue
		}
		if keyNode.Value == oldRef {
			keyNode.Value = newRef
		}
		updatePlacementParentRefs(valNode, oldRef, newRef)
	}

	for i := 0; i+1 < len(mapping.Content); i += 2 {
		keyNode := mapping.Content[i]
		valNode := mapping.Content[i+1]
		if keyNode.Value == "_meta_elements" || keyNode.Value == "_meta_views" {
			renameMappingKeys(valNode, map[string]string{oldRef: newRef})
		}
	}

	if err := writeYAMLNode(path, root); err != nil {
		return err
	}
	return RenameConnector(dir, oldRef, newRef)
}

func updatePlacementParentRefs(node *yaml.Node, oldRef, newRef string) {
	if node == nil || node.Kind != yaml.MappingNode {
		return
	}
	for i := 0; i+1 < len(node.Content); i += 2 {
		if node.Content[i].Value != "placements" {
			continue
		}
		placements := node.Content[i+1]
		if placements.Kind != yaml.SequenceNode {
			return
		}
		for _, placement := range placements.Content {
			updateScalarField(placement, "parent", oldRef, newRef)
		}
	}
}

func updateObjectsYamlForRename(dir, oldRef, newRef string) error {
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
	return nil
}

func updateEdgesYamlForObjectRename(dir, oldRef, newRef string) error {
	edgePath := filepath.Join(dir, "edges.yaml")
	data, err := os.ReadFile(edgePath)
	if err != nil {
		return nil
	}
	var edges map[string]any
	if err := yaml.Unmarshal(data, &edges); err != nil {
		return nil
	}
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
		out, err := yaml.Marshal(newEdges)
		if err != nil {
			return fmt.Errorf("marshal edges: %w", err)
		}
		if err := os.WriteFile(edgePath, out, 0600); err != nil {
			return fmt.Errorf("write edges.yaml: %w", err)
		}
	}
	return nil
}

func updateLinksYamlForObjectRename(dir, oldRef, newRef string) error {
	linkPath := filepath.Join(dir, "links.yaml")
	data, err := os.ReadFile(linkPath)
	if err != nil {
		return nil
	}
	var links []map[string]any
	if err := yaml.Unmarshal(data, &links); err != nil {
		return nil
	}
	changed := false
	for _, l := range links {
		if o, ok := l["object"].(string); ok && o == oldRef {
			l["object"] = newRef
			changed = true
		}
	}
	if changed {
		out, err := yaml.Marshal(links)
		if err != nil {
			return fmt.Errorf("marshal links: %w", err)
		}
		if err := os.WriteFile(linkPath, out, 0600); err != nil {
			return fmt.Errorf("write links.yaml: %w", err)
		}
	}
	return nil
}

// RenameConnector updates connector refs in connectors.yaml.
// It renames an explicit connector key and also cascades element ref changes
// into connector source/target fields and their derived keys.
func RenameConnector(dir, oldRef, newRef string) error {
	if oldRef == newRef {
		return nil
	}

	path := filepath.Join(dir, "connectors.yaml")
	if _, err := os.Stat(path); err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return fmt.Errorf("stat connectors.yaml: %w", err)
	}

	root, mapping, err := loadYAMLMappingNode(path)
	if err != nil {
		return err
	}

	renames := make(map[string]string)
	usedKeys := make(map[string]string)
	var metaKey *yaml.Node
	var metaValue *yaml.Node
	var newContent []*yaml.Node

	for i := 0; i+1 < len(mapping.Content); i += 2 {
		keyNode := mapping.Content[i]
		valNode := mapping.Content[i+1]

		if keyNode.Value == "_meta_connectors" || keyNode.Value == "_meta" {
			metaKey = keyNode
			metaValue = valNode
			continue
		}

		entryOldKey := keyNode.Value
		fieldChanged := false
		if updateScalarField(valNode, "view", oldRef, newRef) {
			fieldChanged = true
		}
		if updateScalarField(valNode, "source", oldRef, newRef) {
			fieldChanged = true
		}
		if updateScalarField(valNode, "target", oldRef, newRef) {
			fieldChanged = true
		}

		entryNewKey := entryOldKey
		if entryOldKey == oldRef {
			entryNewKey = newRef
		}
		if fieldChanged {
			var spec Connector
			if err := valNode.Decode(&spec); err != nil {
				return fmt.Errorf("decode connector %q: %w", entryOldKey, err)
			}
			entryNewKey = connectorKey(&spec)
		}

		if previous, exists := usedKeys[entryNewKey]; exists && previous != entryOldKey {
			return fmt.Errorf("connector %q already exists", entryNewKey)
		}
		usedKeys[entryNewKey] = entryOldKey
		if entryNewKey != entryOldKey {
			renames[entryOldKey] = entryNewKey
		}

		newContent = append(newContent,
			&yaml.Node{Kind: yaml.ScalarNode, Tag: "!!str", Value: entryNewKey},
			valNode,
		)
	}

	if metaKey != nil && metaValue != nil {
		renameMappingKeys(metaValue, renames)
		newContent = append(newContent, metaKey, metaValue)
	}

	mapping.Content = newContent
	return writeYAMLNode(path, root)
}

func updateScalarField(mapping *yaml.Node, fieldName, oldVal, newVal string) bool {
	if mapping.Kind != yaml.MappingNode {
		return false
	}
	for i := 0; i+1 < len(mapping.Content); i += 2 {
		if mapping.Content[i].Value == fieldName && mapping.Content[i+1].Value == oldVal {
			mapping.Content[i+1].Value = newVal
			return true
		}
	}
	return false
}

func renameMappingKeys(mapping *yaml.Node, renames map[string]string) {
	if mapping == nil || mapping.Kind != yaml.MappingNode || len(renames) == 0 {
		return
	}
	for i := 0; i+1 < len(mapping.Content); i += 2 {
		if newKey, ok := renames[mapping.Content[i].Value]; ok {
			mapping.Content[i].Value = newKey
		}
	}
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
		if err := yaml.Unmarshal(data, &existing); err != nil {
			return fmt.Errorf("parse %s: %w", path, err)
		}
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
		if err := yaml.Unmarshal(data, &existing); err != nil {
			return fmt.Errorf("parse %s: %w", path, err)
		}
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
