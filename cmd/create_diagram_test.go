package cmd_test

import (
	"strings"
	"testing"
)

func TestCreateDiagramCmd_Removed(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, stderr, err := runCmd(t, dir, "create", "diagram", "My System")
	if err == nil {
		t.Fatal("expected create diagram to be unavailable")
	}
	if !strings.Contains(stderr, "Unknown command") && !strings.Contains(err.Error(), "unknown command") {
		t.Fatalf("unexpected error: %v, stderr=%q", err, stderr)
	}
}
