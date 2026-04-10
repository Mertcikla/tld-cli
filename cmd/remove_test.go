package cmd_test

import (
	"strings"
	"testing"
)

func TestRemoveElementCmd_LocalOnly(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "element", "API", "--ref", "api", "--kind", "service")
	if err != nil {
		t.Fatalf("create element: %v", err)
	}

	stdout, _, err := runCmd(t, dir, "remove", "element", "api")
	if err != nil {
		t.Fatalf("remove element: %v", err)
	}
	if !strings.Contains(stdout, "Removed api from elements.yaml") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}
}

func TestRemoveConnectorCmd(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	mustRunCmd(t, dir, "create", "element", "Platform", "--ref", "platform", "--kind", "workspace", "--with-view")
	mustRunCmd(t, dir, "create", "element", "API", "--ref", "api", "--kind", "service", "--parent", "platform")
	mustRunCmd(t, dir, "create", "element", "DB", "--ref", "db", "--kind", "database", "--parent", "platform")
	mustRunCmd(t, dir, "connect", "elements", "--view", "platform", "--from", "api", "--to", "db", "--label", "reads")

	stdout, _, err := runCmd(t, dir, "remove", "connector", "--view", "platform", "--from", "api", "--to", "db")
	if err != nil {
		t.Fatalf("remove connector: %v", err)
	}
	if !strings.Contains(stdout, "Removed 1 connector(s)") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}
}
