package workspace

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestDeleteDiagram(t *testing.T) {
	tmpDir := t.TempDir()

	diagramsYAML := `
diag1:
  name: Diagram 1
diag2:
  name: Diagram 2
`
	edgesYAML := `
- diagram: diag1
  source_object: obj1
  target_object: obj2
- diagram: diag2
  source_object: obj1
  target_object: obj3
`
	linksYAML := `
- from_diagram: diag1
  to_diagram: diag2
- from_diagram: diag2
  to_diagram: diag3
`
	objectsYAML := `
obj1:
  name: Object 1
  diagrams:
    - diagram: diag1
    - diagram: diag2
obj2:
  name: Object 2
  diagrams:
    - diagram: diag1
`

	if err := os.WriteFile(filepath.Join(tmpDir, "diagrams.yaml"), []byte(diagramsYAML), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tmpDir, "edges.yaml"), []byte(edgesYAML), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tmpDir, "links.yaml"), []byte(linksYAML), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tmpDir, "objects.yaml"), []byte(objectsYAML), 0600); err != nil {
		t.Fatal(err)
	}

	edgesRemoved, linksRemoved, placementsRemoved, err := DeleteDiagram(tmpDir, "diag1")
	if err != nil {
		t.Errorf("DeleteDiagram failed: %v", err)
	}
	if edgesRemoved != 1 {
		t.Errorf("expected 1 edge removed, got %d", edgesRemoved)
	}
	if linksRemoved != 1 {
		t.Errorf("expected 1 link removed, got %d", linksRemoved)
	}
	if placementsRemoved != 2 {
		t.Errorf("expected 2 placements removed, got %d", placementsRemoved)
	}

	// Verify files
	data, _ := os.ReadFile(filepath.Join(tmpDir, "diagrams.yaml"))
	if strings.Contains(string(data), "diag1") {
		t.Errorf("diagrams.yaml still contains diag1")
	}
	if !strings.Contains(string(data), "diag2") {
		t.Errorf("diagrams.yaml lost diag2")
	}

	data, _ = os.ReadFile(filepath.Join(tmpDir, "edges.yaml"))
	if strings.Contains(string(data), "diag1") {
		t.Errorf("edges.yaml still contains diag1")
	}

	data, _ = os.ReadFile(filepath.Join(tmpDir, "links.yaml"))
	if strings.Contains(string(data), "diag1") {
		t.Errorf("links.yaml still contains diag1")
	}

	data, _ = os.ReadFile(filepath.Join(tmpDir, "objects.yaml"))
	if strings.Contains(string(data), "- diagram: diag1") {
		t.Errorf("objects.yaml still contains diag1 placement")
	}
}

func TestDeleteObject(t *testing.T) {
	tmpDir := t.TempDir()

	objectsYAML := `
obj1:
  name: Object 1
obj2:
  name: Object 2
`
	edgesYAML := `
- diagram: diag1
  source_object: obj1
  target_object: obj2
- diagram: diag1
  source_object: obj2
  target_object: obj3
`
	linksYAML := `
- object: obj1
  from_diagram: diag1
  to_diagram: diag2
- object: obj2
  from_diagram: diag1
  to_diagram: diag3
`

	if err := os.WriteFile(filepath.Join(tmpDir, "objects.yaml"), []byte(objectsYAML), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tmpDir, "edges.yaml"), []byte(edgesYAML), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(tmpDir, "links.yaml"), []byte(linksYAML), 0600); err != nil {
		t.Fatal(err)
	}

	edgesRemoved, linksRemoved, err := DeleteObject(tmpDir, "obj1")
	if err != nil {
		t.Errorf("DeleteObject failed: %v", err)
	}
	if edgesRemoved != 1 {
		t.Errorf("expected 1 edge removed, got %d", edgesRemoved)
	}
	if linksRemoved != 1 {
		t.Errorf("expected 1 link removed, got %d", linksRemoved)
	}

	// Verify files
	data, _ := os.ReadFile(filepath.Join(tmpDir, "objects.yaml"))
	if strings.Contains(string(data), "obj1") {
		t.Errorf("objects.yaml still contains obj1")
	}

	data, _ = os.ReadFile(filepath.Join(tmpDir, "edges.yaml"))
	if strings.Contains(string(data), "source_object: obj1") {
		t.Errorf("edges.yaml still contains obj1")
	}

	data, _ = os.ReadFile(filepath.Join(tmpDir, "links.yaml"))
	if strings.Contains(string(data), "object: obj1") {
		t.Errorf("links.yaml still contains obj1")
	}
}

func TestRemoveEdge(t *testing.T) {
	tmpDir := t.TempDir()

	edgesYAML := `
- diagram: diag1
  source_object: obj1
  target_object: obj2
- diagram: diag1
  source_object: obj1
  target_object: obj3
`
	if err := os.WriteFile(filepath.Join(tmpDir, "edges.yaml"), []byte(edgesYAML), 0600); err != nil {
		t.Fatal(err)
	}

	removed, err := RemoveEdge(tmpDir, "diag1", "obj1", "obj2")
	if err != nil {
		t.Errorf("RemoveEdge failed: %v", err)
	}
	if removed != 1 {
		t.Errorf("expected 1 edge removed, got %d", removed)
	}

	data, _ := os.ReadFile(filepath.Join(tmpDir, "edges.yaml"))
	if strings.Contains(string(data), "target_object: obj2") {
		t.Errorf("edges.yaml still contains edge to obj2")
	}
}

func TestRemoveLink(t *testing.T) {
	tmpDir := t.TempDir()

	linksYAML := `
- from_diagram: diag1
  to_diagram: diag2
  object: obj1
- from_diagram: diag1
  to_diagram: diag2
  object: obj2
`
	if err := os.WriteFile(filepath.Join(tmpDir, "links.yaml"), []byte(linksYAML), 0600); err != nil {
		t.Fatal(err)
	}

	removed, err := RemoveLink(tmpDir, "obj1", "diag1", "diag2")
	if err != nil {
		t.Errorf("RemoveLink failed: %v", err)
	}
	if removed != 1 {
		t.Errorf("expected 1 link removed, got %d", removed)
	}

	data, _ := os.ReadFile(filepath.Join(tmpDir, "links.yaml"))
	if strings.Contains(string(data), "object: obj1") {
		t.Errorf("links.yaml still contains link with obj1")
	}
}
