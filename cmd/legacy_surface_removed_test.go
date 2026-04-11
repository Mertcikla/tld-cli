package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
)

func TestLegacyCommandsRemoved(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	testCases := [][]string{
		{"create", "diagram", "System"},
		{"create", "object", "system", "API", "service"},
		{"update", "object", "api"},
		{"update", "source", "api"},
		{"update", "diagram", "system"},
		{"update", "edge", "--diagram", "system", "--from", "api", "--to", "db"},
		{"connect", "objects", "system", "--from", "api", "--to", "db"},
		{"remove", "diagram", "system"},
		{"remove", "object", "api"},
		{"remove", "edge", "--diagram", "system", "--from", "api", "--to", "db"},
		{"remove", "link", "--from", "root", "--to", "api"},
		{"rename", "diagram", "system", "system-2"},
		{"rename", "object", "api", "api-2"},
	}

	for _, args := range testCases {
		_, stderr, err := runCmd(t, dir, args...)
		if err == nil {
			t.Fatalf("expected command %v to be unavailable", args)
		}
		combined := strings.ToLower(stderr + "\n" + err.Error())
		if !strings.Contains(combined, "unknown command") && !strings.Contains(combined, "unknown flag") && !strings.Contains(combined, "accepts 0 arg") {
			t.Fatalf("unexpected error for %v: err=%v stderr=%q", args, err, stderr)
		}
	}
}

func TestRenameElementCmd(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	mustRunCmd(t, dir, "create", "element", "Platform", "--ref", "platform", "--kind", "workspace", "--with-view")
	mustRunCmd(t, dir, "create", "element", "API", "--ref", "api", "--kind", "service", "--parent", "platform")
	mustRunCmd(t, dir, "connect", "elements", "--view", "platform", "--from", "api", "--to", "platform", "--label", "hosts")

	stdout, _, err := runCmd(t, dir, "rename", "element", "api", "api-service")
	if err != nil {
		t.Fatalf("rename element: %v", err)
	}
	if !strings.Contains(stdout, "Renamed element \"api\" to \"api-service\"") {
		t.Fatalf("unexpected output: %q", stdout)
	}

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("load workspace: %v", err)
	}
	if _, ok := ws.Elements["api"]; ok {
		t.Fatal("old element ref still present")
	}
	if _, ok := ws.Elements["api-service"]; !ok {
		t.Fatal("renamed element ref missing")
	}
	if _, ok := ws.Connectors["platform:api-service:platform:hosts"]; !ok {
		t.Fatalf("connector refs not updated: %+v", ws.Connectors)
	}
}

func TestRenameConnectorCmd(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	content := "\"platform:api:db:reads\":\n" +
		"  view: platform\n" +
		"  source: api\n" +
		"  target: db\n" +
		"  label: reads\n"
	if err := os.WriteFile(filepath.Join(dir, "connectors.yaml"), []byte(content), 0600); err != nil {
		t.Fatalf("write connectors.yaml: %v", err)
	}

	stdout, _, err := runCmd(t, dir, "rename", "connector", "platform:api:db:reads", "platform:api:db:queries")
	if err != nil {
		t.Fatalf("rename connector: %v", err)
	}
	if !strings.Contains(stdout, "Renamed connector \"platform:api:db:reads\" to \"platform:api:db:queries\"") {
		t.Fatalf("unexpected output: %q", stdout)
	}

	data, err := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if err != nil {
		t.Fatalf("read connectors.yaml: %v", err)
	}
	text := string(data)
	if strings.Contains(text, "platform:api:db:reads") {
		t.Fatalf("old connector ref still present:\n%s", text)
	}
	if !strings.Contains(text, "platform:api:db:queries") {
		t.Fatalf("new connector ref missing:\n%s", text)
	}
}
