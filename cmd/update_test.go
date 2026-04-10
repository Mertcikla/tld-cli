package cmd_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"gopkg.in/yaml.v3"

	"github.com/mertcikla/tld-cli/workspace"
)

func TestUpdateObjectCmd_SourceLinking(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	// Create an object first
	createLegacyObject(t, dir, "system", "api-gateway", "API Gateway", "service")
	var err error

	// Update with source linking flags
	_, _, err = runCmd(t, dir, "update", "object", "api-gateway",
		"--repo", "owner/repo",
		"--branch", "main",
		"--language", "go",
		"--file", "cmd/main.go")
	if err != nil {
		t.Fatalf("update object: %v", err)
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
	if spec.Repo != "owner/repo" || spec.Branch != "main" || spec.Language != "go" || spec.FilePath != "cmd/main.go" {
		t.Errorf("unexpected source linking: %+v", spec)
	}
}

func TestUpdateSourceCmd_SymbolLinking(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	// Create an object
	createLegacyObject(t, dir, "system", "api-gateway", "API Gateway", "service")
	var err error

	// Update source with symbol
	_, _, err = runCmd(t, dir, "update", "source", "api-gateway",
		"--repo", "owner/repo",
		"--branch", "develop",
		"--file", "pkg/api/handler.go",
		"--symbol", "Handler",
		"--symbol-type", "struct")
	if err != nil {
		t.Fatalf("update source: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var objects map[string]workspace.Object
	_ = yaml.Unmarshal(data, &objects)
	spec := objects["api-gateway"]

	if spec.Repo != "owner/repo" || spec.Branch != "develop" {
		t.Errorf("unexpected repo/branch: %s/%s", spec.Repo, spec.Branch)
	}

	// Check FilePath fragment
	parts := strings.Split(spec.FilePath, "#")
	if len(parts) != 2 {
		t.Fatalf("expected fragment in file_path: %s", spec.FilePath)
	}
	if parts[0] != "pkg/api/handler.go" {
		t.Errorf("unexpected base path: %s", parts[0])
	}

	var symbolInfo map[string]any
	if err := json.Unmarshal([]byte(parts[1]), &symbolInfo); err != nil {
		t.Fatalf("failed to unmarshal fragment: %v", err)
	}
	if symbolInfo["name"] != "Handler" || symbolInfo["type"] != "struct" {
		t.Errorf("unexpected symbol info: %+v", symbolInfo)
	}
}

func TestUpdateSourceCmd_LineLinking(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	// Create an object
	createLegacyObject(t, dir, "system", "api-gateway", "API Gateway", "service")

	// Update source with line
	_, _, err := runCmd(t, dir, "update", "source", "api-gateway",
		"--file", "README.md",
		"--start-line", "10",
		"--end-line", "15")
	if err != nil {
		t.Fatalf("update source: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var objects map[string]workspace.Object
	_ = yaml.Unmarshal(data, &objects)
	spec := objects["api-gateway"]

	parts := strings.Split(spec.FilePath, "#")
	if len(parts) != 2 {
		t.Fatalf("expected fragment in file_path: %s", spec.FilePath)
	}

	var lineInfo map[string]int
	_ = json.Unmarshal([]byte(parts[1]), &lineInfo)
	if lineInfo["startLine"] != 10 || lineInfo["endLine"] != 15 {
		t.Errorf("unexpected line info: %+v", lineInfo)
	}
}

func TestUpdateSourceCmd_DatabaseFormat(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	// Create an object
	createLegacyObject(t, dir, "system", "pytorchtest", "PytorchTest", "service")

	// Update source with the specific format
	_, _, err := runCmd(t, dir, "update", "source", "pytorchtest",
		"--file", "android/pytorch_android/src/androidTest/java/org/pytorch/PytorchTestBase.java",
		"--symbol", "testShuffleOps",
		"--symbol-type", "method_declaration")
	if err != nil {
		t.Fatalf("update source: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var objects map[string]workspace.Object
	_ = yaml.Unmarshal(data, &objects)
	spec := objects["pytorchtest"]

	expected := `android/pytorch_android/src/androidTest/java/org/pytorch/PytorchTestBase.java#{"name":"testShuffleOps","type":"method_declaration"}`
	if spec.FilePath != expected {
		t.Errorf("\nExpected: %s\nGot:      %s", expected, spec.FilePath)
	}
}

func TestUpdateSourceCmd_UnsupportedType(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	// Try to update source for a diagram (not supported)
	_, stderr, err := runCmd(t, dir, "update", "source", "system")
	if err == nil {
		t.Fatal("expected error for diagram")
	}
	if !strings.Contains(err.Error(), "source linking isnt supported for this object yet") &&
		!strings.Contains(stderr, "source linking isnt supported for this object yet") {
		t.Errorf("expected specific error message, got: %v (stderr: %s)", err, stderr)
	}
}
