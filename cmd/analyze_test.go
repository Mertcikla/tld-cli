package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
)

func TestAnalyzeCmd_DryRun_NoWrite(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	file := filepath.Join(dir, "service.go")
	if err := os.WriteFile(file, []byte("package main\nfunc Service() {}\n"), 0600); err != nil {
		t.Fatal(err)
	}
	before, err := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "analyze", file, "--dry-run")
	if err != nil {
		t.Fatalf("analyze --dry-run: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	after, err := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	if string(before) != string(after) {
		t.Fatalf("elements.yaml changed during dry-run")
	}
	if !strings.Contains(stdout, "[dry-run]   OK  1 elements written to elements.yaml") {
		t.Fatalf("unexpected stdout: %s", stdout)
	}
}

func TestAnalyzeCmd_WritesElements(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	file := filepath.Join(dir, "service.go")
	if err := os.WriteFile(file, []byte("package main\nfunc Foo() {}\nfunc Bar() {}\n"), 0600); err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "analyze", file)
	if err != nil {
		t.Fatalf("analyze: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatal(err)
	}
	if len(ws.Elements) != 2 {
		t.Fatalf("elements = %d, want 2", len(ws.Elements))
	}
}

func TestAnalyzeCmd_WritesConnectors(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	if err := os.WriteFile(filepath.Join(dir, "foo.go"), []byte("package main\nfunc Foo() {}\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "bar.go"), []byte("package main\nfunc Bar() { Foo() }\n"), 0600); err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "analyze", dir)
	if err != nil {
		t.Fatalf("analyze: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatal(err)
	}
	if len(ws.Connectors) == 0 {
		t.Fatalf("expected at least one connector, stdout=%s stderr=%s", stdout, stderr)
	}
}

func TestAnalyzeCmd_ExcludeRules(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	config := "project_name: Demo\nexclude:\n  - '*_test.go'\n"
	if err := os.WriteFile(filepath.Join(dir, ".tld.yaml"), []byte(config), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "prod.go"), []byte("package main\nfunc Prod() {}\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "prod_test.go"), []byte("package main\nfunc TestOnly() {}\n"), 0600); err != nil {
		t.Fatal(err)
	}

	if _, _, err := runCmd(t, dir, "analyze", dir); err != nil {
		t.Fatalf("analyze: %v", err)
	}
	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatal(err)
	}
	for _, element := range ws.Elements {
		if element.Name == "TestOnly" {
			t.Fatalf("unexpected test symbol in elements.yaml: %+v", ws.Elements)
		}
	}
}

func TestAnalyzeCmd_PathNotExist(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	_, _, err := runCmd(t, dir, "analyze", filepath.Join(dir, "missing.go"))
	if err == nil {
		t.Fatal("expected error for missing path")
	}
}
