package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPlanCmd_OutputsMarkdown(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	stdout, _, err := runCmd(t, dir, "plan")
	if err != nil {
		t.Fatalf("plan: %v", err)
	}
	if !strings.Contains(stdout, "# Element Plan") {
		t.Errorf("stdout %q does not contain '# Element Plan'", stdout)
	}
}

func TestPlanCmd_VerboseFlag(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	// Without verbose
	stdout, _, err := runCmd(t, dir, "plan")
	if err != nil {
		t.Fatalf("plan: %v", err)
	}
	if strings.Contains(stdout, "## Elements") {
		t.Errorf("stdout contains verbose section when it shouldn't: %q", stdout)
	}
	if !strings.Contains(stdout, "Use '-v' or '--verbose' for detailed element placement and connector reporting") {
		t.Errorf("stdout missing verbose hint: %q", stdout)
	}

	// With verbose
	stdout, _, err = runCmd(t, dir, "plan", "-v")
	if err != nil {
		t.Fatalf("plan -v: %v", err)
	}
	if !strings.Contains(stdout, "## Elements") {
		t.Errorf("stdout missing verbose section when -v is used: %q", stdout)
	}
	if strings.Contains(stdout, "Use '-v' or '--verbose' for detailed resource reporting") {
		t.Errorf("stdout contains verbose hint when -v is used: %q", stdout)
	}
}

func TestPlanCmd_OutputToFile(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	outFile := filepath.Join(dir, "plan.md")
	stdout, _, err := runCmd(t, dir, "plan", "--output", outFile)
	if err != nil {
		t.Fatalf("plan --output: %v", err)
	}
	if stdout != "" {
		t.Errorf("stdout should be empty when --output used, got: %q", stdout)
	}

	data, err := os.ReadFile(outFile)
	if err != nil {
		t.Fatalf("read output file: %v", err)
	}
	if !strings.Contains(string(data), "# Element Plan") {
		t.Errorf("file content %q does not contain '# Element Plan'", string(data))
	}
}

func TestPlanCmd_InvalidWorkspaceErrors(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	if err := os.WriteFile(filepath.Join(dir, "elements.yaml"),
		[]byte("child:\n  name: Child\n  kind: service\n  placements:\n    - parent: nonexistent\n"), 0600); err != nil {
		t.Fatalf("write elements: %v", err)
	}
	if err := os.WriteFile(filepath.Join(dir, ".tld.yaml"), []byte("org_id: test-org\nserver_url: http://localhost\napi_key: key\n"), 0600); err != nil {
		t.Fatalf("write config: %v", err)
	}

	_, _, err := runCmd(t, dir, "plan")
	if err == nil {
		t.Fatal("expected error for invalid workspace")
	}
}

func TestPlanCmd_MissingConfig(t *testing.T) {
	dir := t.TempDir()
	_, _, err := runCmd(t, dir, "plan")
	if err == nil {
		t.Fatal("expected error for missing config")
	}
	if !strings.Contains(err.Error(), "load workspace") {
		t.Errorf("error %q does not contain 'load workspace'", err.Error())
	}
}
