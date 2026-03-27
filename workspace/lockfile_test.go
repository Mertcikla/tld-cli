package workspace

import (
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestLockFile(t *testing.T) {
	tmpDir := t.TempDir()

	versionID := "v123"
	appliedBy := "tester"
	parentVersion := "v122"

	lockFile, err := CreateLockFile(versionID, appliedBy, 1, 2, 3, 4, &parentVersion)
	if err != nil {
		t.Errorf("CreateLockFile failed: %v", err)
	}
	if lockFile.VersionID != versionID {
		t.Errorf("expected version ID %s, got %s", versionID, lockFile.VersionID)
	}
	if lockFile.AppliedBy != appliedBy {
		t.Errorf("expected applied by %s, got %s", appliedBy, lockFile.AppliedBy)
	}
	if lockFile.Resources.Diagrams != 1 {
		t.Errorf("expected 1 diagram, got %d", lockFile.Resources.Diagrams)
	}

	err = WriteLockFile(tmpDir, lockFile)
	if err != nil {
		t.Errorf("WriteLockFile failed: %v", err)
	}

	loaded, err := LoadLockFile(tmpDir)
	if err != nil {
		t.Errorf("LoadLockFile failed: %v", err)
	}
	if loaded.VersionID != versionID {
		t.Errorf("expected version ID %s, got %s", versionID, loaded.VersionID)
	}

	// Test Update
	newVersionID := "v124"
	UpdateLockFile(loaded, newVersionID, appliedBy, 2, 3, 4, 5, "hash123", &versionID)
	if loaded.VersionID != newVersionID {
		t.Errorf("expected new version ID %s, got %s", newVersionID, loaded.VersionID)
	}
	if loaded.WorkspaceHash != "hash123" {
		t.Errorf("expected hash123, got %s", loaded.WorkspaceHash)
	}
}

func TestCalculateWorkspaceHash(t *testing.T) {
	tmpDir := t.TempDir()

	if err := os.WriteFile(filepath.Join(tmpDir, "diagrams.yaml"), []byte("diagrams: {}"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tmpDir, "objects.yaml"), []byte("objects: {}"), 0600); err != nil {
		t.Fatal(err)
	}

	hash1, err := CalculateWorkspaceHash(tmpDir)
	if err != nil {
		t.Errorf("CalculateWorkspaceHash failed: %v", err)
	}
	if hash1 == "" {
		t.Error("expected non-empty hash")
	}

	// Change a file
	if err := os.WriteFile(filepath.Join(tmpDir, "diagrams.yaml"), []byte("diagrams: {d1: {}}"), 0600); err != nil {
		t.Fatal(err)
	}
	hash2, err := CalculateWorkspaceHash(tmpDir)
	if err != nil {
		t.Errorf("CalculateWorkspaceHash failed: %v", err)
	}
	if hash1 == hash2 {
		t.Error("expected different hashes after file change")
	}
}

func TestMetadata(t *testing.T) {
	tmpDir := t.TempDir()

	diagramsYAML := `
diag1:
  name: D1
_meta:
  diag1:
    id: "hash1"
    updated_at: "2024-03-24T10:00:00Z"
`
	if err := os.WriteFile(filepath.Join(tmpDir, "diagrams.yaml"), []byte(diagramsYAML), 0600); err != nil {
		t.Fatal(err)
	}

	meta, err := LoadMetadata(tmpDir)
	if err != nil {
		t.Errorf("LoadMetadata failed: %v", err)
	}
	if _, ok := meta.Diagrams["diag1"]; !ok {
		t.Error("expected diag1 in metadata")
	}

	newMeta := map[string]*ResourceMetadata{
		"diag2": {
			ID:        123,
			UpdatedAt: time.Now().UTC().Truncate(time.Second),
		},
	}
	err = WriteMetadata(tmpDir, "objects.yaml", newMeta)
	if err != nil {
		t.Errorf("WriteMetadata failed: %v", err)
	}

	meta2, err := LoadMetadata(tmpDir)
	if err != nil {
		t.Errorf("LoadMetadata failed: %v", err)
	}
	if _, ok := meta2.Objects["diag2"]; !ok {
		t.Error("expected diag2 in metadata")
	}
	if meta2.Objects["diag2"].ID != ResourceID(123) {
		t.Errorf("expected ID 123, got %d", meta2.Objects["diag2"].ID)
	}
}
