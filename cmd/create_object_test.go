package cmd_test

import (
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
)

// setupDiagram creates a legacy workspace fixture with one diagram named "system".
func setupDiagram(t *testing.T, dir string) {
	t.Helper()
	mustInitWorkspace(t, dir)
	if err := workspace.WriteDiagram(dir, "system", &workspace.Diagram{Name: "System"}); err != nil {
		t.Fatalf("write diagram: %v", err)
	}
}

func createLegacyObject(t *testing.T, dir, diagramRef, ref, name, objectType string) {
	t.Helper()
	if err := workspace.UpsertObject(dir, ref, &workspace.Object{
		Name: name,
		Type: objectType,
		Diagrams: []workspace.Placement{{
			Diagram: diagramRef,
		}},
	}); err != nil {
		t.Fatalf("upsert object: %v", err)
	}
}

func TestCreateObjectCmd_Removed(t *testing.T) {
	dir := t.TempDir()
	setupDiagram(t, dir)

	_, stderr, err := runCmd(t, dir, "create", "object", "system", "API Gateway", "service")
	if err == nil {
		t.Fatal("expected create object to be unavailable")
	}
	if !strings.Contains(stderr, "Unknown command") && !strings.Contains(err.Error(), "unknown command") {
		t.Fatalf("unexpected error: %v, stderr=%q", err, stderr)
	}
}
