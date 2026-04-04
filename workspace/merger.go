package workspace

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// MergeWorkspace merges changes from a new workspace (from server) into the current on-disk state.
// It uses yaml.Node to preserve comments and formatting.
// lastSyncMeta is the metadata from the .tld.lock file (state at last pull/apply).
// currentMeta is the metadata loaded from local YAML files (current state on disk).
func MergeWorkspace(dir string, newWS *Workspace, lastSyncMeta *Meta, currentMeta *Meta) error {
	// Merge diagrams
	if err := mergeYAMLMapWithConflicts(filepath.Join(dir, "diagrams.yaml"), newWS.Diagrams, newWS.Meta.Diagrams, lastSyncMeta.Diagrams, currentMeta.Diagrams); err != nil {
		return fmt.Errorf("merge diagrams: %w", err)
	}

	// Merge objects
	if err := mergeYAMLMapWithConflicts(filepath.Join(dir, "objects.yaml"), newWS.Objects, newWS.Meta.Objects, lastSyncMeta.Objects, currentMeta.Objects); err != nil {
		return fmt.Errorf("merge objects: %w", err)
	}

	// Merge edges
	if err := mergeYAMLMapWithConflicts(filepath.Join(dir, "edges.yaml"), newWS.Edges, newWS.Meta.Edges, lastSyncMeta.Edges, currentMeta.Edges); err != nil {
		return fmt.Errorf("merge edges: %w", err)
	}

	// For links, we just overwrite for now
	if len(newWS.Links) > 0 {
		data, err := yaml.Marshal(newWS.Links)
		if err != nil {
			return fmt.Errorf("marshal links: %w", err)
		}
		if err := os.WriteFile(filepath.Join(dir, "links.yaml"), data, 0600); err != nil {
			return fmt.Errorf("write links: %w", err)
		}
	}

	return nil
}

func mergeYAMLMapWithConflicts(path string, serverItems any, serverMeta map[string]*ResourceMetadata, lastSyncMeta map[string]*ResourceMetadata, currentMeta map[string]*ResourceMetadata) error {
	// Load existing file into a Node
	var root yaml.Node
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return WriteFullYAMLMap(path, serverItems, serverMeta)
		}
		return err
	}

	if err := yaml.Unmarshal(data, &root); err != nil {
		return err
	}

	if root.Kind != yaml.DocumentNode || len(root.Content) == 0 || root.Content[0].Kind != yaml.MappingNode {
		return WriteFullYAMLMap(path, serverItems, serverMeta)
	}

	mapping := root.Content[0]

	// Convert serverItems to a map for easy lookup
	serverItemsData, _ := yaml.Marshal(serverItems)
	var serverItemsMap map[string]any
	_ = yaml.Unmarshal(serverItemsData, &serverItemsMap)

	seenKeys := make(map[string]bool)
	var newContent []*yaml.Node

	// Iterate through existing mapping (local state)
	for i := 0; i < len(mapping.Content); i += 2 {
		keyNode := mapping.Content[i]
		valNode := mapping.Content[i+1]
		key := keyNode.Value

		if key == "_meta" {
			continue
		}

		serverItem, onServer := serverItemsMap[key]
		sMeta := serverMeta[key]
		lMeta := lastSyncMeta[key]
		cMeta := currentMeta[key]

		if onServer {
			// Resource exists locally and on server
			seenKeys[key] = true

			localChanged := lMeta != nil && cMeta != nil && cMeta.UpdatedAt.After(lMeta.UpdatedAt)
			serverChanged := lMeta != nil && sMeta != nil && sMeta.UpdatedAt.After(lMeta.UpdatedAt)

			if localChanged && serverChanged {
				// CONFLICT: Both changed. Keep local for now but mark as conflict in meta.
				// The user will see this in 'tld status' or 'tld apply'.
				if sMeta != nil {
					sMeta.Conflict = true
				}
				newContent = append(newContent, keyNode, valNode)
			} else if serverChanged {
				// Server changed, local did not: Update local with server content.
				var newValNode yaml.Node
				_ = newValNode.Encode(serverItem)
				newContent = append(newContent, keyNode, &newValNode)
			} else {
				// No changes or only local changes: Keep local.
				newContent = append(newContent, keyNode, valNode)
			}
		} else {
			// Resource exists locally but not on server.
			// Was it deleted on server or created locally?
			if lMeta != nil {
				// Was on server before (last sync), so it was deleted on server.
				// Remove locally too.
			} else {
				// Was never on server, so it's a new local resource. Keep it.
				newContent = append(newContent, keyNode, valNode)
			}
		}
	}

	// Add new keys from server that weren't in the local file
	for key, val := range serverItemsMap {
		if !seenKeys[key] {
			var keyNode yaml.Node
			_ = keyNode.Encode(key)
			var valNode yaml.Node
			_ = valNode.Encode(val)
			newContent = append(newContent, &keyNode, &valNode)
		}
	}

	// Rebuild _meta with merged metadata
	if len(serverMeta) > 0 {
		var metaKeyNode yaml.Node
		_ = metaKeyNode.Encode("_meta")
		metaValNode := EncodeMeta(serverMeta)
		newContent = append(newContent, &metaKeyNode, metaValNode)
	}

	mapping.Content = newContent

	// Write back
	f, err := os.Create(path)
	if err != nil {
		return err
	}
	defer f.Close()

	enc := yaml.NewEncoder(f)
	enc.SetIndent(2)
	return enc.Encode(&root)
}
