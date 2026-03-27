package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"gopkg.in/yaml.v3"

	"github.com/mertcikla/tld-cli/workspace"
)

func TestCreateDiagramCmd_Basic(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	stdout, _, err := runCmd(t, dir, "create", "diagram", "My System")
	if err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if !strings.Contains(stdout, "Updated diagrams.yaml") {
		t.Errorf("stdout %q does not contain 'Updated diagrams.yaml'", stdout)
	}

	// Verify updated
	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatalf("read file: %v", err)
	}
	var diagrams map[string]workspace.Diagram
	if err := yaml.Unmarshal(data, &diagrams); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	spec, ok := diagrams["my-system"]
	if !ok {
		t.Fatal("missing 'my-system' in diagrams.yaml")
	}
	if spec.Name != "My System" {
		t.Errorf("Name = %q, want 'My System'", spec.Name)
	}
}

func TestCreateDiagramCmd_RefOverride(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "diagram", "My System", "--ref", "custom-ref")
	if err != nil {
		t.Fatalf("create diagram: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatalf("read file: %v", err)
	}
	var diagrams map[string]workspace.Diagram
	if err := yaml.Unmarshal(data, &diagrams); err != nil {
		t.Fatalf("unmarshal diagrams: %v", err)
	}
	if _, ok := diagrams["custom-ref"]; !ok {
		t.Error("expected custom-ref in diagrams.yaml")
	}
}

func TestCreateDiagramCmd_AllFlags(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	// Create parent first
	_, _, err := runCmd(t, dir, "create", "diagram", "Parent")
	if err != nil {
		t.Fatalf("create parent: %v", err)
	}

	_, _, err = runCmd(t, dir, "create", "diagram", "Child",
		"--description", "A child diagram",
		"--level-label", "Container",
		"--parent", "parent")
	if err != nil {
		t.Fatalf("create child: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	var diagrams map[string]workspace.Diagram
	_ = yaml.Unmarshal(data, &diagrams)
	spec := diagrams["child"]

	if spec.Description != "A child diagram" {
		t.Errorf("Description = %q", spec.Description)
	}
	if spec.LevelLabel != "Container" {
		t.Errorf("LevelLabel = %q", spec.LevelLabel)
	}
	if spec.ParentDiagram != "parent" {
		t.Errorf("ParentDiagram = %q", spec.ParentDiagram)
	}
}

func TestCreateDiagramCmd_DuplicateErrors(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "diagram", "System")
	if err != nil {
		t.Fatalf("first create: %v", err)
	}
	_, _, err = runCmd(t, dir, "create", "diagram", "System")
	if err == nil {
		t.Fatal("expected error for duplicate diagram")
	}
	if !strings.Contains(err.Error(), "already exists") {
		t.Errorf("error %q does not contain 'already exists'", err.Error())
	}
}

func TestCreateDiagramCmd_MissingArg(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "diagram")
	if err == nil {
		t.Fatal("expected error for missing arg")
	}
}
