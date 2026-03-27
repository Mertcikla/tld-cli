package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"gopkg.in/yaml.v3"

	"github.com/mertcikla/tldiagram-cli/workspace"
)

// setupDiagram creates a workspace with one diagram named "system".
func setupDiagram(t *testing.T, dir string) {
	t.Helper()
	mustInitWorkspace(t, dir)
	_, _, err := runCmd(t, dir, "create", "diagram", "System")
	if err != nil {
		t.Fatalf("create diagram: %v", err)
	}
}

func TestCreateObjectCmd_Basic(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	stdout, _, err := runCmd(t, dir, "create", "object", "system", "API Gateway", "service")
	if err != nil {
		t.Fatalf("create object: %v", err)
	}
	if !strings.Contains(stdout, "Updated objects.yaml") {
		t.Errorf("stdout %q does not contain 'Updated objects.yaml'", stdout)
	}

	data, err := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatalf("read file: %v", err)
	}
	var objects map[string]workspace.Object
	if err := yaml.Unmarshal(data, &objects); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	spec, ok := objects["api-gateway"]
	if !ok {
		t.Fatal("missing 'api-gateway' in objects.yaml")
	}
	if spec.Name != "API Gateway" || spec.Type != "service" {
		t.Errorf("unexpected spec: %+v", spec)
	}
	if len(spec.Diagrams) != 1 || spec.Diagrams[0].Diagram != "system" {
		t.Errorf("unexpected placements: %+v", spec.Diagrams)
	}
}

func TestCreateObjectCmd_RefOverride(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	_, _, err := runCmd(t, dir, "create", "object", "system", "My DB", "database", "--ref", "main-db")
	if err != nil {
		t.Fatalf("create object: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatalf("read file: %v", err)
	}
	var objects map[string]workspace.Object
	if err := yaml.Unmarshal(data, &objects); err != nil {
		t.Fatalf("unmarshal objects: %v", err)
	}
	if _, ok := objects["main-db"]; !ok {
		t.Error("expected main-db in objects.yaml")
	}
}

func TestCreateObjectCmd_PositionStored(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	_, _, err := runCmd(t, dir, "create", "object", "system", "DB", "database",
		"--position-x", "150.5", "--position-y", "300.0")
	if err != nil {
		t.Fatalf("create object: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var objects map[string]workspace.Object
	_ = yaml.Unmarshal(data, &objects)
	spec := objects["db"]

	if len(spec.Diagrams) != 1 {
		t.Fatalf("unexpected placements: %+v", spec.Diagrams)
	}
	if spec.Diagrams[0].PositionX != 150.5 || spec.Diagrams[0].PositionY != 300.0 {
		t.Errorf("position: got (%.1f, %.1f)", spec.Diagrams[0].PositionX, spec.Diagrams[0].PositionY)
	}
}

func TestCreateObjectCmd_DuplicateErrors(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	_, _, err := runCmd(t, dir, "create", "object", "system", "API", "service")
	if err != nil {
		t.Fatalf("first create: %v", err)
	}
	_, _, err = runCmd(t, dir, "create", "object", "system", "API", "database")
	if err == nil {
		t.Fatal("expected error for duplicate")
	}
}

func TestCreateObjectCmd_MissingArgs(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "object", "system", "API") // missing type
	if err == nil {
		t.Fatal("expected error for missing arg")
	}
}
