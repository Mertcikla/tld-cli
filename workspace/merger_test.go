package workspace_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/mertcikla/tld-cli/workspace"
	"gopkg.in/yaml.v3"
)

func TestMergeWorkspace_ThreeWay(t *testing.T) {
	dir := t.TempDir()

	// 1. Last sync state
	lastSyncTime := time.Now().Add(-2 * time.Hour)
	lastSyncMeta := &workspace.Meta{
		Diagrams: map[string]*workspace.ResourceMetadata{
			"d1": {ID: 1, UpdatedAt: lastSyncTime},
			"d2": {ID: 2, UpdatedAt: lastSyncTime},
		},
	}

	// 2. Current local state (with comments and one local change)
	localContent := `
# My important comment
d1:
  name: Local D1
  description: local
d2:
  name: D2
  description: original
`
	os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte(localContent), 0600)
	currentMeta := &workspace.Meta{
		Diagrams: map[string]*workspace.ResourceMetadata{
			"d1": {ID: 1, UpdatedAt: time.Now()}, // Local change
			"d2": {ID: 2, UpdatedAt: lastSyncTime},
		},
	}

	// 3. New server state (d2 changed on server, d3 added)
	serverUpdateTime := time.Now()
	newWS := &workspace.Workspace{
		Dir: dir,
		Diagrams: map[string]*workspace.Diagram{
			"d1": {Name: "Original D1", Description: "original"},
			"d2": {Name: "Server D2", Description: "server update"},
			"d3": {Name: "New D3"},
		},
		Meta: &workspace.Meta{
			Diagrams: map[string]*workspace.ResourceMetadata{
				"d1": {ID: 1, UpdatedAt: lastSyncTime},
				"d2": {ID: 2, UpdatedAt: serverUpdateTime},
				"d3": {ID: 3, UpdatedAt: serverUpdateTime},
			},
		},
	}

	// Perform merge
	err := workspace.MergeWorkspace(dir, newWS, lastSyncMeta, currentMeta)
	if err != nil {
		t.Fatalf("MergeWorkspace failed: %v", err)
	}

	// Verify results
	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	content := string(data)

	// Check if comment is preserved
	if !strings.Contains(content, "# My important comment") {
		t.Error("Comment lost during merge")
	}

	var gotDiags map[string]workspace.Diagram
	yaml.Unmarshal(data, &gotDiags)

	// d1 should keep local change (Server didn't change it since last sync)
	if gotDiags["d1"].Name != "Local D1" {
		t.Errorf("d1 should keep local name, got %q", gotDiags["d1"].Name)
	}

	// d2 should take server update (Local didn't change it)
	if gotDiags["d2"].Name != "Server D2" {
		t.Errorf("d2 should take server name, got %q", gotDiags["d2"].Name)
	}

	// d3 should be added
	if _, ok := gotDiags["d3"]; !ok {
		t.Error("d3 was not added")
	}
}

func TestMergeWorkspace_ConflictDetection(t *testing.T) {
	dir := t.TempDir()

	lastSyncTime := time.Now().Add(-10 * time.Hour)
	lastSyncMeta := &workspace.Meta{
		Diagrams: map[string]*workspace.ResourceMetadata{
			"d1": {ID: 1, UpdatedAt: lastSyncTime},
		},
	}

	// Local change
	os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte("d1: {name: Local}"), 0600)
	currentMeta := &workspace.Meta{
		Diagrams: map[string]*workspace.ResourceMetadata{
			"d1": {ID: 1, UpdatedAt: time.Now()},
		},
	}

	// Server change
	newWS := &workspace.Workspace{
		Dir: dir,
		Diagrams: map[string]*workspace.Diagram{
			"d1": {Name: "Server"},
		},
		Meta: &workspace.Meta{
			Diagrams: map[string]*workspace.ResourceMetadata{
				"d1": {ID: 1, UpdatedAt: time.Now()},
			},
		},
	}

	// Merge
	workspace.MergeWorkspace(dir, newWS, lastSyncMeta, currentMeta)

	// Verify: Local should win on disk, but meta should mark conflict
	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	var gotDiags map[string]any
	yaml.Unmarshal(data, &gotDiags)

	if gotDiags["d1"].(map[string]any)["name"] != "Local" {
		t.Error("Local change should be preserved on conflict")
	}

	// Check _meta for conflict flag
	metaSection := gotDiags["_meta"].(map[string]any)
	d1Meta := metaSection["d1"].(map[string]any)
	if d1Meta["conflict"] != true {
		t.Error("Conflict flag not set in metadata")
	}
}
