package workspace_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
	"gopkg.in/yaml.v3"
)

func TestRenameDiagram(t *testing.T) {
	dir := t.TempDir()

	// Setup initial state
	diagrams := map[string]workspace.Diagram{
		"old-diag": {Name: "Old Diagram"},
	}
	diagData, err := yaml.Marshal(diagrams)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), diagData, 0600); err != nil {
		t.Fatal(err)
	}

	objects := map[string]workspace.Object{
		"obj1": {
			Name: "Object 1",
			Diagrams: []workspace.Placement{
				{Diagram: "old-diag", PositionX: 10, PositionY: 20},
			},
		},
	}
	objData, err := yaml.Marshal(objects)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "objects.yaml"), objData, 0600); err != nil {
		t.Fatal(err)
	}

	edges := map[string]workspace.Edge{
		"old-diag:src:tgt:": {
			Diagram:      "old-diag",
			SourceObject: "src",
			TargetObject: "tgt",
		},
	}
	edgeData, err := yaml.Marshal(edges)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "edges.yaml"), edgeData, 0600); err != nil {
		t.Fatal(err)
	}

	links := []workspace.Link{
		{FromDiagram: "old-diag", ToDiagram: "other"},
		{FromDiagram: "other", ToDiagram: "old-diag"},
	}
	linkData, err := yaml.Marshal(links)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "links.yaml"), linkData, 0600); err != nil {
		t.Fatal(err)
	}

	// Perform rename
	if err := workspace.RenameDiagram(dir, "old-diag", "new-diag"); err != nil {
		t.Fatalf("RenameDiagram failed: %v", err)
	}

	// Verify diagrams.yaml
	var gotDiags map[string]workspace.Diagram
	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotDiags); err != nil {
		t.Fatal(err)
	}
	if _, ok := gotDiags["old-diag"]; ok {
		t.Error("old-diag still exists in diagrams.yaml")
	}
	if _, ok := gotDiags["new-diag"]; !ok {
		t.Error("new-diag not found in diagrams.yaml")
	}

	// Verify objects.yaml
	var gotObjs map[string]workspace.Object
	data, err = os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotObjs); err != nil {
		t.Fatal(err)
	}
	if gotObjs["obj1"].Diagrams[0].Diagram != "new-diag" {
		t.Errorf("object placement not updated: got %q, want %q", gotObjs["obj1"].Diagrams[0].Diagram, "new-diag")
	}

	// Verify edges.yaml
	var gotEdges map[string]workspace.Edge
	data, err = os.ReadFile(filepath.Join(dir, "edges.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotEdges); err != nil {
		t.Fatal(err)
	}
	if _, ok := gotEdges["old-diag:src:tgt:"]; ok {
		t.Error("old edge key still exists in edges.yaml")
	}
	if e, ok := gotEdges["new-diag:src:tgt:"]; !ok {
		t.Error("new edge key not found in edges.yaml")
	} else if e.Diagram != "new-diag" {
		t.Errorf("edge diagram field not updated: got %q, want %q", e.Diagram, "new-diag")
	}

	// Verify links.yaml
	var gotLinks []workspace.Link
	data, err = os.ReadFile(filepath.Join(dir, "links.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotLinks); err != nil {
		t.Fatal(err)
	}
	if gotLinks[0].FromDiagram != "new-diag" {
		t.Errorf("link from_diagram not updated: got %q, want %q", gotLinks[0].FromDiagram, "new-diag")
	}
	if gotLinks[1].ToDiagram != "new-diag" {
		t.Errorf("link to_diagram not updated: got %q, want %q", gotLinks[1].ToDiagram, "new-diag")
	}
}

func TestRenameObject(t *testing.T) {
	dir := t.TempDir()

	// Setup initial state
	objects := map[string]workspace.Object{
		"old-obj": {Name: "Old Object", Type: "service"},
	}
	objData, err := yaml.Marshal(objects)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "objects.yaml"), objData, 0600); err != nil {
		t.Fatal(err)
	}

	edges := map[string]workspace.Edge{
		"diag:old-obj:tgt:": {
			Diagram:      "diag",
			SourceObject: "old-obj",
			TargetObject: "tgt",
		},
		"diag:src:old-obj:": {
			Diagram:      "diag",
			SourceObject: "src",
			TargetObject: "old-obj",
		},
	}
	edgeData, err := yaml.Marshal(edges)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "edges.yaml"), edgeData, 0600); err != nil {
		t.Fatal(err)
	}

	links := []workspace.Link{
		{Object: "old-obj", FromDiagram: "f", ToDiagram: "t"},
	}
	linkData, err := yaml.Marshal(links)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "links.yaml"), linkData, 0600); err != nil {
		t.Fatal(err)
	}

	// Perform rename
	if err := workspace.RenameObject(dir, "old-obj", "new-obj"); err != nil {
		t.Fatalf("RenameObject failed: %v", err)
	}

	// Verify objects.yaml
	var gotObjs map[string]workspace.Object
	data, err := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotObjs); err != nil {
		t.Fatal(err)
	}
	if _, ok := gotObjs["old-obj"]; ok {
		t.Error("old-obj still exists in objects.yaml")
	}
	if _, ok := gotObjs["new-obj"]; !ok {
		t.Error("new-obj not found in objects.yaml")
	}

	// Verify edges.yaml
	var gotEdges map[string]workspace.Edge
	data, err = os.ReadFile(filepath.Join(dir, "edges.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotEdges); err != nil {
		t.Fatal(err)
	}
	if _, ok := gotEdges["diag:old-obj:tgt:"]; ok {
		t.Error("old edge key (source) still exists")
	}
	if e, ok := gotEdges["diag:new-obj:tgt:"]; !ok {
		t.Error("new edge key (source) not found")
	} else if e.SourceObject != "new-obj" {
		t.Errorf("edge source_object not updated: got %q", e.SourceObject)
	}

	if _, ok := gotEdges["diag:src:old-obj:"]; ok {
		t.Error("old edge key (target) still exists")
	}
	if e, ok := gotEdges["diag:src:new-obj:"]; !ok {
		t.Error("new edge key (target) not found")
	} else if e.TargetObject != "new-obj" {
		t.Errorf("edge target_object not updated: got %q", e.TargetObject)
	}

	// Verify links.yaml
	var gotLinks []workspace.Link
	data, err = os.ReadFile(filepath.Join(dir, "links.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if err := yaml.Unmarshal(data, &gotLinks); err != nil {
		t.Fatal(err)
	}
	if gotLinks[0].Object != "new-obj" {
		t.Errorf("link object not updated: got %q", gotLinks[0].Object)
	}
}

func TestRename_Errors(t *testing.T) {
	dir := t.TempDir()

	// Test renaming non-existent
	err := workspace.RenameDiagram(dir, "missing", "new")
	if err == nil {
		t.Error("expected error for missing diagram, got nil")
	}

	// Test renaming to existing
	diagrams := map[string]workspace.Diagram{
		"diag1": {Name: "D1"},
		"diag2": {Name: "D2"},
	}
	data, err := yaml.Marshal(diagrams)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"), data, 0600); err != nil {
		t.Fatal(err)
	}

	err = workspace.RenameDiagram(dir, "diag1", "diag2")
	if err == nil {
		t.Error("expected error renaming to existing diagram, got nil")
	}
}
