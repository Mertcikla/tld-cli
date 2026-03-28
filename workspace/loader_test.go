package workspace_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
)

// writeFile writes content to path, creating dirs as needed.
func writeFile(t *testing.T, path, content string) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0750); err != nil {
		t.Fatalf("MkdirAll: %v", err)
	}
	if err := os.WriteFile(path, []byte(content), 0600); err != nil {
		t.Fatalf("WriteFile %s: %v", path, err)
	}
}

// setupConfig creates a temporary config directory and sets TLD_CONFIG_DIR.
// It returns the path to the global tld.yaml file.
func setupConfig(t *testing.T) string {
	t.Helper()
	configDir := t.TempDir()
	_ = os.Setenv("TLD_CONFIG_DIR", configDir)
	t.Cleanup(func() { _ = os.Unsetenv("TLD_CONFIG_DIR") })
	return filepath.Join(configDir, "tld.yaml")
}

// minimalConfig returns minimal tld.yaml content.
func minimalConfig() string {
	return "server_url: https://tldiagram.com\napi_key: \"\"\norg_id: \"\"\n"
}

func TestLoad_MinimalWorkspace(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(ws.Diagrams) != 0 || len(ws.Objects) != 0 || len(ws.Edges) != 0 || len(ws.Links) != 0 {
		t.Errorf("expected empty maps, got diagrams=%d objects=%d edges=%d links=%d",
			len(ws.Diagrams), len(ws.Objects), len(ws.Edges), len(ws.Links))
	}
}

func TestLoad_MissingConfigFile(t *testing.T) {
	dir := t.TempDir()
	setupConfig(t)
	_, err := workspace.Load(dir)
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !strings.Contains(err.Error(), "read tld.yaml") {
		t.Errorf("error %q does not contain 'read tld.yaml'", err.Error())
	}
}

func TestLoad_MalformedConfigYAML(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, ":\t:\n")
	_, err := workspace.Load(dir)
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(err.Error(), "parse tld.yaml") {
		t.Errorf("error %q does not contain 'parse tld.yaml'", err.Error())
	}
}

func TestLoad_APIKeyFromEnv(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())

	_ = os.Setenv("TLD_API_KEY", "env-test-key")
	t.Cleanup(func() { _ = os.Unsetenv("TLD_API_KEY") })

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if ws.Config.APIKey != "env-test-key" {
		t.Errorf("APIKey = %q, want 'env-test-key'", ws.Config.APIKey)
	}
}

func TestLoad_APIKeyFileOverridesEnv(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, "server_url: http://localhost\napi_key: file-key\norg_id: \"\"\n")

	_ = os.Setenv("TLD_API_KEY", "env-key")
	t.Cleanup(func() { _ = os.Unsetenv("TLD_API_KEY") })

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if ws.Config.APIKey != "file-key" {
		t.Errorf("APIKey = %q, want 'file-key'", ws.Config.APIKey)
	}
}

func TestLoad_DiagramsLoaded(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "diagrams.yaml"), "system: {name: System}\ncontainer: {name: Container}\n")

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(ws.Diagrams) != 2 {
		t.Fatalf("len(Diagrams) = %d, want 2", len(ws.Diagrams))
	}
	if _, ok := ws.Diagrams["system"]; !ok {
		t.Error("missing 'system' diagram")
	}
	if _, ok := ws.Diagrams["container"]; !ok {
		t.Error("missing 'container' diagram")
	}
}

func TestLoad_MalformedDiagramYAML(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "diagrams.yaml"), ":\t:\n")

	_, err := workspace.Load(dir)
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(err.Error(), "parse diagrams.yaml") {
		t.Errorf("error %q does not contain 'parse diagrams.yaml'", err.Error())
	}
}

func TestLoad_ObjectsLoaded(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "objects.yaml"),
		"api:\n  name: API\n  type: service\n  diagrams:\n    - diagram: system\n      position_x: 100\n      position_y: 50\n")

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(ws.Objects) != 1 {
		t.Fatalf("len(Objects) = %d, want 1", len(ws.Objects))
	}
	api, ok := ws.Objects["api"]
	if !ok {
		t.Fatal("missing 'api' object")
	}
	if api.Name != "API" || api.Type != "service" {
		t.Errorf("unexpected object: %+v", api)
	}
	if len(api.Diagrams) != 1 || api.Diagrams[0].PositionX != 100 || api.Diagrams[0].PositionY != 50 {
		t.Errorf("unexpected placements: %+v", api.Diagrams)
	}
}

func TestLoad_MalformedObjectYAML(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "objects.yaml"), ":\t:\n")

	_, err := workspace.Load(dir)
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(err.Error(), "parse objects.yaml") {
		t.Errorf("error %q does not contain 'parse objects.yaml'", err.Error())
	}
}

func TestLoad_EdgesLoaded(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "edges.yaml"),
		"\"sys:a:b:\":\n  diagram: sys\n  source_object: a\n  target_object: b\n\"sys:c:d:\":\n  diagram: sys\n  source_object: c\n  target_object: d\n")

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(ws.Edges) != 2 {
		t.Fatalf("len(Edges) = %d, want 2", len(ws.Edges))
	}
}

func TestLoad_LinksLoaded(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "links.yaml"),
		"- object: api\n  from_diagram: system\n  to_diagram: container\n")

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(ws.Links) != 1 {
		t.Fatalf("len(Links) = %d, want 1", len(ws.Links))
	}
	if ws.Links[0].Object != "api" {
		t.Errorf("Link.Object = %q", ws.Links[0].Object)
	}
}

func TestLoad_MalformedEdgesYAML(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	writeFile(t, filepath.Join(dir, "edges.yaml"), "- diagram: sys\n  source_object: a\n  target_object: b\n")

	_, err := workspace.Load(dir)
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(err.Error(), "parse edges.yaml") {
		t.Errorf("error %q does not contain expected message", err.Error())
	}
}

func TestLoad_DiagramsFileAbsent(t *testing.T) {
	dir := t.TempDir()
	cfgPath := setupConfig(t)
	writeFile(t, cfgPath, minimalConfig())
	// No diagrams.yaml created - should not be an error

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	if len(ws.Diagrams) != 0 {
		t.Errorf("expected 0 diagrams, got %d", len(ws.Diagrams))
	}
}
