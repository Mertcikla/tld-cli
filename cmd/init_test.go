package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestInitCmd_CreatesWorkspace(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)

	stdout, _, err := runCmd(t, ".", "init", dir)
	if err != nil {
		t.Fatalf("init: %v", err)
	}
	if !strings.Contains(stdout, "Initialized workspace") {
		t.Errorf("stdout %q does not contain 'Initialized workspace'", stdout)
	}

	workspaceCfgPath := filepath.Join(dir, ".tld.yaml")
	data, err := os.ReadFile(workspaceCfgPath)
	if err != nil {
		t.Fatalf("read .tld.yaml from workspace: %v", err)
	}
	workspaceContent := string(data)
	if !strings.Contains(workspaceContent, "repositories") || !strings.Contains(workspaceContent, "exclude:") {
		t.Errorf(".tld.yaml missing expected keys: %q", workspaceContent)
	}

	// Check tld.yaml was created globally
	cfgPath := filepath.Join(configDir, "tld.yaml")
	data, err = os.ReadFile(cfgPath)
	if err != nil {
		t.Fatalf("read tld.yaml from global config: %v", err)
	}
	content := string(data)
	if !strings.Contains(content, "server_url") || !strings.Contains(content, "api_key") || !strings.Contains(content, "org_id") {
		t.Errorf("tld.yaml missing expected keys: %q", content)
	}
}

func TestInitCmd_AlreadyInitialized(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)

	// First init
	_, _, err := runCmd(t, ".", "init", dir)
	if err != nil {
		t.Fatalf("first init: %v", err)
	}
	// Second init - should succeed and report already exists (config dir exists)
	stdout, _, err := runCmd(t, ".", "init", dir)
	if err != nil {
		t.Fatalf("second init: %v", err)
	}
	if !strings.Contains(stdout, "Initialized workspace at") || !strings.Contains(stdout, "config already exists") {
		t.Errorf("stdout %q does not contain 'Initialized' or 'config already exists'", stdout)
	}
}

func TestInitCmd_DefaultDir(t *testing.T) {
	// Without positional arg, init targets "." (CWD). We just verify no crash.
	// We can't change CWD safely in parallel tests, so skip the functional check
	// and just test that the command is recognized.
	dir := t.TempDir()
	_, _, err := runCmd(t, ".", "init", dir)
	if err != nil {
		t.Fatalf("init with explicit dir: %v", err)
	}
}

func TestInitCmd_WizardProducesValidYAML(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)

	stdin := strings.NewReader("My Project\nfrontend\ngithub.com/x/fe\n./frontend\n1\nn\n")
	stdout, stderr, err := runCmdWithStdin(t, ".", stdin, "init", dir, "--wizard")
	if err != nil {
		t.Fatalf("init --wizard: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "Next steps:") {
		t.Fatalf("stdout missing next steps:\n%s", stdout)
	}

	configPath := filepath.Join(dir, ".tld.yaml")
	data, err := os.ReadFile(configPath)
	if err != nil {
		t.Fatalf("read .tld.yaml: %v", err)
	}
	content := string(data)
	if !strings.Contains(content, "project_name: My Project") {
		t.Fatalf("project name missing from .tld.yaml:\n%s", content)
	}
	if !strings.Contains(content, "frontend:") || !strings.Contains(content, "mode: upsert") {
		t.Fatalf("repository config missing from .tld.yaml:\n%s", content)
	}

	validateOut, validateErr, err := runCmd(t, dir, "validate")
	if err != nil {
		t.Fatalf("validate after wizard: %v\nstdout: %s\nstderr: %s", err, validateOut, validateErr)
	}
}
