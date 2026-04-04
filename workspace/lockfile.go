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

	// Hash each YAML file in order for deterministic results
	files := []string{"diagrams.yaml", "objects.yaml", "edges.yaml", "links.yaml"}

	for _, filename := range files {
		path := filepath.Join(dir, filename)
		if _, err := os.Stat(path); os.IsNotExist(err) {
			continue // File doesn't exist, skip
		}

		file, err := os.Open(path)
		if err != nil {
			return "", fmt.Errorf("open %s: %w", filename, err)
		}

		if _, err := io.Copy(hash, file); err != nil {
			_ = file.Close()
			return "", fmt.Errorf("hash %s: %w", filename, err)
		}
		_ = file.Close()
	}

	return fmt.Sprintf("sha256:%x", hash.Sum(nil)), nil
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

// LoadMetadata loads metadata from the _meta sections of YAML files
func LoadMetadata(dir string) (*Meta, error) {
	meta := &Meta{
		Diagrams: make(map[string]*ResourceMetadata),
		Objects:  make(map[string]*ResourceMetadata),
		Edges:    make(map[string]*ResourceMetadata),
	}

	// Load diagrams metadata
	if err := loadYAMLMetadata(filepath.Join(dir, "diagrams.yaml"), meta.Diagrams); err != nil {
		return nil, fmt.Errorf("load diagrams metadata: %w", err)
	}

	// Load objects metadata
	if err := loadYAMLMetadata(filepath.Join(dir, "objects.yaml"), meta.Objects); err != nil {
		return nil, fmt.Errorf("load objects metadata: %w", err)
	}

	// Load edges metadata
	if err := loadYAMLMetadata(filepath.Join(dir, "edges.yaml"), meta.Edges); err != nil {
		return nil, fmt.Errorf("load edges metadata: %w", err)
	}

	return meta, nil
}

// loadYAMLMetadata loads the _meta section from a YAML file
func loadYAMLMetadata(filepath string, target map[string]*ResourceMetadata) error {
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

	// Look for _meta section
	if metaSection, ok := yamlMap["_meta"].(map[string]any); ok {
		for ref, metaData := range metaSection {
			if metaMap, ok := metaData.(map[string]any); ok {
				metadata := &ResourceMetadata{}
				if idStr, ok := metaMap["id"].(string); ok {
					if decoded, err := hashidlib.Decode(idStr); err == nil {
						metadata.ID = ResourceID(decoded)
					}
				}
				if updatedAtStr, ok := metaMap["updated_at"].(string); ok {
					if updatedAt, err := time.Parse(time.RFC3339, updatedAtStr); err == nil {
						metadata.UpdatedAt = updatedAt
					}
				}
				target[ref] = metadata
			}
		}
	}

	return nil
}

// WriteMetadata writes the _meta section to a YAML file
func WriteMetadata(dir, filename string, metadata map[string]*ResourceMetadata) error {
	path := filepath.Join(dir, filename)

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
		metaSection[ref] = metaMap
	}
	yamlMap["_meta"] = metaSection

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
