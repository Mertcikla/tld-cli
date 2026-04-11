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
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte(localContent), 0600); err != nil {
		t.Fatal(err)
	}
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
	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	content := string(data)

	// Check if comment is preserved
	if !strings.Contains(content, "# My important comment") {
		t.Error("Comment lost during merge")
	}

	var gotDiags map[string]workspace.Diagram
	if err := yaml.Unmarshal(data, &gotDiags); err != nil {
		t.Fatal(err)
	}

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
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte("d1: {name: Local}"), 0600); err != nil {
		t.Fatal(err)
	}
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
	if err := workspace.MergeWorkspace(dir, newWS, lastSyncMeta, currentMeta); err != nil {
		t.Fatal(err)
	}

	// Verify: Local should win on disk, but meta should mark conflict
	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	var gotDiags map[string]any
	if err := yaml.Unmarshal(data, &gotDiags); err != nil {
		t.Fatal(err)
	}

	nameValue := gotDiags["d1"].(map[string]any)["name"].(string)
	if !strings.Contains(nameValue, "<<< LOCAL") || !strings.Contains(nameValue, "Local") {
		t.Errorf("expected parseable conflict block, got %q", nameValue)
	}

	// Check _meta for conflict flag
	metaSection := gotDiags["_meta"].(map[string]any)
	d1Meta := metaSection["d1"].(map[string]any)
	if d1Meta["conflict"] != true {
		t.Error("Conflict flag not set in metadata")
	}
}

func TestMergeWorkspace_PositionChangesNeverConflict(t *testing.T) {
	dir := t.TempDir()
	lastSyncTime := time.Now().Add(-10 * time.Hour)
	lastSyncMeta := &workspace.Meta{Objects: map[string]*workspace.ResourceMetadata{"api": {ID: 1, UpdatedAt: lastSyncTime}}}
	if err := os.WriteFile(filepath.Join(dir, "objects.yaml"), []byte(`api:
  name: API
  type: service
  description: local description
  diagrams:
    - diagram: system
      position_x: 10
      position_y: 20
`), 0600); err != nil {
		t.Fatal(err)
	}
	currentMeta := &workspace.Meta{Objects: map[string]*workspace.ResourceMetadata{"api": {ID: 1, UpdatedAt: time.Now()}}}
	newWS := &workspace.Workspace{
		Dir: dir,
		Objects: map[string]*workspace.Object{
			"api": {
				Name:        "API",
				Type:        "service",
				Description: "",
				Diagrams:    []workspace.Placement{{Diagram: "system", PositionX: 55, PositionY: 66}},
			},
		},
		Meta: &workspace.Meta{Objects: map[string]*workspace.ResourceMetadata{"api": {ID: 1, UpdatedAt: time.Now().Add(time.Hour)}}},
	}

	if err := workspace.MergeWorkspace(dir, newWS, lastSyncMeta, currentMeta); err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(string(data), "<<< LOCAL") {
		t.Fatalf("unexpected conflict marker:\n%s", data)
	}
	var got map[string]workspace.Object
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatal(err)
	}
	if got["api"].Description != "local description" {
		t.Fatalf("description should stay local, got %q", got["api"].Description)
	}
	if got["api"].Diagrams[0].PositionX != 55 || got["api"].Diagrams[0].PositionY != 66 {
		t.Fatalf("positions should come from server, got %+v", got["api"].Diagrams[0])
	}
}

func TestMergeWorkspace_BothSidesChangePosition(t *testing.T) {
	dir := t.TempDir()
	lastSyncTime := time.Now().Add(-10 * time.Hour)
	lastSyncMeta := &workspace.Meta{Objects: map[string]*workspace.ResourceMetadata{"api": {ID: 1, UpdatedAt: lastSyncTime}}}
	if err := os.WriteFile(filepath.Join(dir, "objects.yaml"), []byte(`api:
  name: API
  type: service
  diagrams:
    - diagram: system
      position_x: 11
      position_y: 12
`), 0600); err != nil {
		t.Fatal(err)
	}
	currentMeta := &workspace.Meta{Objects: map[string]*workspace.ResourceMetadata{"api": {ID: 1, UpdatedAt: time.Now()}}}
	newWS := &workspace.Workspace{
		Dir:     dir,
		Objects: map[string]*workspace.Object{"api": {Name: "API", Type: "service", Diagrams: []workspace.Placement{{Diagram: "system", PositionX: 88, PositionY: 99}}}},
		Meta:    &workspace.Meta{Objects: map[string]*workspace.ResourceMetadata{"api": {ID: 1, UpdatedAt: time.Now().Add(time.Hour)}}},
	}
	if err := workspace.MergeWorkspace(dir, newWS, lastSyncMeta, currentMeta); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if strings.Contains(string(data), "<<< LOCAL") {
		t.Fatalf("unexpected conflict marker:\n%s", data)
	}
	var got map[string]workspace.Object
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatal(err)
	}
	if got["api"].Diagrams[0].PositionX != 88 || got["api"].Diagrams[0].PositionY != 99 {
		t.Fatalf("server position should win, got %+v", got["api"].Diagrams[0])
	}
}

func TestMergeWorkspace_SemanticConflictPreservesComment(t *testing.T) {
	dir := t.TempDir()
	lastSyncTime := time.Now().Add(-10 * time.Hour)
	lastSyncMeta := &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{"d1": {ID: 1, UpdatedAt: lastSyncTime}}}
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte("d1:\n  name: Local Name\n"), 0600); err != nil {
		t.Fatal(err)
	}
	currentMeta := &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{"d1": {ID: 1, UpdatedAt: time.Now()}}}
	newWS := &workspace.Workspace{
		Dir:      dir,
		Diagrams: map[string]*workspace.Diagram{"d1": {Name: "Server Name"}},
		Meta:     &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{"d1": {ID: 1, UpdatedAt: time.Now().Add(time.Hour)}}},
	}
	if err := workspace.MergeWorkspace(dir, newWS, lastSyncMeta, currentMeta); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	content := string(data)
	if !strings.Contains(content, "CONFLICT:") || !strings.Contains(content, "<<< LOCAL") {
		t.Fatalf("expected conflict block with comment:\n%s", content)
	}
	ws := &workspace.Workspace{Dir: dir, Diagrams: map[string]*workspace.Diagram{"d1": {Name: "<<< LOCAL\nLocal Name\n===\nServer Name\n>>> SERVER"}}}
	errs := ws.Validate()
	if len(errs) == 0 || !strings.Contains(errs[0].Error(), "unresolved merge conflict") {
		t.Fatalf("expected unresolved merge conflict error, got %v", errs)
	}
}

func TestMergeWorkspace_NewServerElement(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte("d1:\n  name: Existing\n"), 0600); err != nil {
		t.Fatal(err)
	}
	newWS := &workspace.Workspace{
		Dir:      dir,
		Diagrams: map[string]*workspace.Diagram{"d1": {Name: "Existing"}, "d2": {Name: "New Diagram"}},
		Meta:     &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{"d1": {}, "d2": {}}},
	}
	if err := workspace.MergeWorkspace(dir, newWS, &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{}}, &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{}}); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if !strings.Contains(string(data), "d2:") {
		t.Fatalf("new server element missing:\n%s", data)
	}
}

func TestMergeWorkspace_LocalOnlyElement(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte("local-only:\n  name: Local Only\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := workspace.MergeWorkspace(dir, &workspace.Workspace{Dir: dir, Diagrams: map[string]*workspace.Diagram{}, Meta: &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{}}}, &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{}}, &workspace.Meta{Diagrams: map[string]*workspace.ResourceMetadata{}}); err != nil {
		t.Fatal(err)
	}
	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if !strings.Contains(string(data), "local-only:") {
		t.Fatalf("local-only element should be preserved:\n%s", data)
	}
}
