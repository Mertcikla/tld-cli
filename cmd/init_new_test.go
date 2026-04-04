package cmd_test

import (
	"os"
	"path/filepath"
	"testing"
)

func TestInitCmd_CreatesTldDirectory(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)

	// Change to temp dir to test default "tld" directory creation
	oldWd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	if err := os.Chdir(dir); err != nil {
		t.Fatal(err)
	}
	defer func() { _ = os.Chdir(oldWd) }()

	// Run init without args
	_, _, err = runCmd(t, ".", "init")
	if err != nil {
		t.Fatalf("init: %v", err)
	}

	// Check if "tld" directory was created
	tldDir := filepath.Join(dir, "tld")
	if stat, err := os.Stat(tldDir); err != nil || !stat.IsDir() {
		t.Fatalf("tld directory was not created: %v", err)
	}

	// Check if YAML files were created in tld/
	files := []string{"diagrams.yaml", "objects.yaml", "edges.yaml", "links.yaml"}
	for _, f := range files {
		path := filepath.Join(tldDir, f)
		if _, err := os.Stat(path); err != nil {
			t.Errorf("file %s was not created in tld directory", f)
		}
	}
}

func TestInitCmd_CustomDirectory(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)

	customDir := filepath.Join(dir, "my-diagrams")

	// Run init with custom dir
	_, _, err := runCmd(t, ".", "init", customDir)
	if err != nil {
		t.Fatalf("init: %v", err)
	}

	// Check if custom directory was created
	if stat, err := os.Stat(customDir); err != nil || !stat.IsDir() {
		t.Fatalf("custom directory %s was not created", customDir)
	}

	// Check if YAML files were created in customDir/
	if _, err := os.Stat(filepath.Join(customDir, "diagrams.yaml")); err != nil {
		t.Errorf("diagrams.yaml was not created in custom directory")
	}
}
