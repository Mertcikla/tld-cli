package workspace

import (
	"crypto/sha256"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"time"

	hashidlib "github.com/mertcikla/tld-cli/internal/hashids"
	"gopkg.in/yaml.v3"
)

// LoadLockFile reads the .tld.lock file from the workspace directory
func LoadLockFile(dir string) (*LockFile, error) {
	path := filepath.Join(dir, ".tld.lock")
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil, nil // No lock file exists yet
		}
		return nil, fmt.Errorf("read lock file: %w", err)
	}

	var lockFile LockFile
	if err := yaml.Unmarshal(data, &lockFile); err != nil {
		return nil, fmt.Errorf("parse lock file: %w", err)
	}

	return &lockFile, nil
}

// WriteLockFile writes the .tld.lock file to the workspace directory
func WriteLockFile(dir string, lockFile *LockFile) error {
	path := filepath.Join(dir, ".tld.lock")
	data, err := yaml.Marshal(lockFile)
	if err != nil {
		return fmt.Errorf("marshal lock file: %w", err)
	}

	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write lock file: %w", err)
	}

	return nil
}

// CalculateWorkspaceHash computes a hash of all YAML files in the workspace
func CalculateWorkspaceHash(dir string) (string, error) {
	hash := sha256.New()

	files := workspaceHashFiles(dir)

	for _, filename := range files {
		path := filepath.Join(dir, filename)
		if _, err := os.Stat(path); os.IsNotExist(err) {
			continue // File doesn't exist, skip
		}

		normalized, err := normalizedHashContent(path)
		if err != nil {
			return "", fmt.Errorf("hash %s: %w", filename, err)
		}
		if _, err := io.WriteString(hash, filename+"\n"); err != nil {
			return "", fmt.Errorf("hash %s: %w", filename, err)
		}
		if _, err := hash.Write(normalized); err != nil {
			return "", fmt.Errorf("hash %s: %w", filename, err)
		}
	}

	return fmt.Sprintf("sha256:%x", hash.Sum(nil)), nil
}

func workspaceHashFiles(dir string) []string {
	for _, filename := range []string{"elements.yaml", "connectors.yaml"} {
		if _, err := os.Stat(filepath.Join(dir, filename)); err == nil {
			return []string{"elements.yaml", "connectors.yaml"}
		}
	}
	// Fall back to the removed file set when hashing a workspace that has not been migrated yet.
	return []string{"diagrams.yaml", "objects.yaml", "edges.yaml", "links.yaml"}
}

func normalizedHashContent(path string) ([]byte, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var raw any
	if err := yaml.Unmarshal(data, &raw); err != nil {
		return data, nil
	}
	return yaml.Marshal(stripPositionFields(raw))
}

func stripPositionFields(value any) any {
	switch typed := value.(type) {
	case map[string]any:
		result := make(map[string]any, len(typed))
		for key, child := range typed {
			if positionKeys[key] {
				continue
			}
			result[key] = stripPositionFields(child)
		}
		return result
	case []any:
		result := make([]any, len(typed))
		for i, child := range typed {
			result[i] = stripPositionFields(child)
		}
		return result
	default:
		return value
	}
}

// CreateLockFile creates a new lock file with the given parameters
func CreateLockFile(versionID, appliedBy string, diagramCount, objectCount, edgeCount, linkCount int, parentVersion *string) (*LockFile, error) {
	return &LockFile{
		Version:   "v1",
		VersionID: versionID,
		LastApply: time.Now(),
		AppliedBy: appliedBy,
		Resources: &ResourceCounts{
			Diagrams: diagramCount,
			Objects:  objectCount,
			Edges:    edgeCount,
			Links:    linkCount,
		},
		ParentVersion: parentVersion,
	}, nil
}

// UpdateLockFile updates an existing lock file with new resource counts and hash
func UpdateLockFile(lockFile *LockFile, versionID, appliedBy string, diagramCount, objectCount, edgeCount, linkCount int, workspaceHash string, parentVersion *string, metadata *Meta) {
	lockFile.VersionID = versionID
	lockFile.LastApply = time.Now()
	lockFile.AppliedBy = appliedBy
	lockFile.Resources = &ResourceCounts{
		Diagrams: diagramCount,
		Objects:  objectCount,
		Edges:    edgeCount,
		Links:    linkCount,
	}
	lockFile.WorkspaceHash = workspaceHash
	lockFile.ParentVersion = parentVersion
	lockFile.Metadata = metadata
}

// LoadMetadata loads metadata from current files and, when present, legacy files for backward compatibility.
func LoadMetadata(dir string) (*Meta, error) {
	meta := &Meta{
		Elements:   make(map[string]*ResourceMetadata),
		Views:      make(map[string]*ResourceMetadata),
		Connectors: make(map[string]*ResourceMetadata),
	}

	if err := loadYAMLMetadataSection(filepath.Join(dir, "elements.yaml"), "_meta_elements", meta.Elements); err != nil {
		return nil, fmt.Errorf("load elements metadata: %w", err)
	}

	if err := loadYAMLMetadataSection(filepath.Join(dir, "elements.yaml"), "_meta_views", meta.Views); err != nil {
		return nil, fmt.Errorf("load view metadata: %w", err)
	}

	if err := loadYAMLMetadataSection(filepath.Join(dir, "connectors.yaml"), "_meta_connectors", meta.Connectors); err != nil {
		return nil, fmt.Errorf("load connector metadata: %w", err)
	}

	// Also extract inline metadata from connectors list if present
	if err := extractInlineMetadata(filepath.Join(dir, "connectors.yaml"), meta.Connectors); err != nil {
		return nil, fmt.Errorf("load inline connector metadata: %w", err)
	}

	return meta, nil
}

func extractInlineMetadata(path string, target map[string]*ResourceMetadata) error {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var list []*Connector
	if err := yaml.Unmarshal(data, &list); err == nil && len(list) > 0 {
		for _, c := range list {
			if c.ID != 0 {
				target[ConnectorKey(c)] = &ResourceMetadata{
					ID:        c.ID,
					UpdatedAt: c.UpdatedAt,
				}
			}
		}
	}
	return nil
}

// loadYAMLMetadataSection loads a metadata section from a YAML file.
func loadYAMLMetadataSection(filepath, sectionName string, target map[string]*ResourceMetadata) error {
	data, err := os.ReadFile(filepath)
	if err != nil {
		if os.IsNotExist(err) {
			return nil // File doesn't exist, that's ok
		}
		return fmt.Errorf("read %s: %w", filepath, err)
	}

	var yamlMap map[string]any
	if err := yaml.Unmarshal(data, &yamlMap); err != nil {
		return fmt.Errorf("parse %s: %w", filepath, err)
	}

	if metaSection, ok := yamlMap[sectionName].(map[string]any); ok {
		for ref, metaData := range metaSection {
			if metaMap, ok := metaData.(map[string]any); ok {
				metadata := &ResourceMetadata{}
				switch idValue := metaMap["id"].(type) {
				case string:
					if decoded, err := hashidlib.Decode(idValue); err == nil {
						metadata.ID = ResourceID(decoded)
					}
				case int:
					metadata.ID = ResourceID(idValue)
				case int64:
					metadata.ID = ResourceID(idValue)
				case float64:
					metadata.ID = ResourceID(idValue)
				}
				switch updatedAtValue := metaMap["updated_at"].(type) {
				case string:
					if updatedAt, err := time.Parse(time.RFC3339, updatedAtValue); err == nil {
						metadata.UpdatedAt = updatedAt
					}
				case time.Time:
					metadata.UpdatedAt = updatedAtValue
				}
				if conflict, ok := metaMap["conflict"].(bool); ok {
					metadata.Conflict = conflict
				}
				target[ref] = metadata
			}
		}
	}

	return nil
}

// WriteMetadata writes the _meta section to a YAML file
func WriteMetadata(dir, filename string, metadata map[string]*ResourceMetadata) error {
	return WriteMetadataSection(dir, filename, "_meta", metadata)
}

// WriteMetadataSection writes a named metadata section to a YAML file.
func WriteMetadataSection(dir, filename, sectionName string, metadata map[string]*ResourceMetadata) error {
	path := filepath.Join(dir, filename)

	if filename == "connectors.yaml" {
		return writeConnectorListWithMetadata(path, metadata)
	}

	// Read existing file
	var yamlMap map[string]any
	if data, err := os.ReadFile(path); err == nil {
		if err := yaml.Unmarshal(data, &yamlMap); err != nil {
			return fmt.Errorf("parse existing %s: %w", filename, err)
		}
	} else if !os.IsNotExist(err) {
		return fmt.Errorf("read %s: %w", filename, err)
	} else {
		yamlMap = make(map[string]any)
	}

	// Update _meta section
	metaSection := make(map[string]any)
	for ref, meta := range metadata {
		metaMap := map[string]any{
			"id":         meta.ID,
			"updated_at": meta.UpdatedAt.Format(time.RFC3339),
		}
		if meta.Conflict {
			metaMap["conflict"] = true
		}
		metaSection[ref] = metaMap
	}
	yamlMap[sectionName] = metaSection

	// Write back to file
	data, err := yaml.Marshal(yamlMap)
	if err != nil {
		return fmt.Errorf("marshal %s: %w", filename, err)
	}

	if err := os.WriteFile(path, data, 0600); err != nil {
		return fmt.Errorf("write %s: %w", filename, err)
	}

	return nil
}

func writeConnectorListWithMetadata(path string, metadata map[string]*ResourceMetadata) error {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	var list []*Connector
	if err := yaml.Unmarshal(data, &list); err != nil {
		return err
	}
	for _, c := range list {
		if m, ok := metadata[ConnectorKey(c)]; ok {
			c.ID = m.ID
			c.UpdatedAt = m.UpdatedAt
		}
	}
	return WriteFullYAMLList(path, list)
}

