package workspace_test

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/mertcikla/tld-cli/workspace"
	"strings"
)

func TestSave_Deterministic(t *testing.T) {
	dir := t.TempDir()
	
	ws := &workspace.Workspace{
		Dir: dir,
		Diagrams: map[string]*workspace.Diagram{
			"z-ref": {Name: "Z Diagram"},
			"a-ref": {Name: "A Diagram"},
		},
		Meta: &workspace.Meta{
			Diagrams: map[string]*workspace.ResourceMetadata{
				"a-ref": {ID: 1, UpdatedAt: time.Date(2023, 1, 1, 0, 0, 0, 0, time.UTC)},
				"z-ref": {ID: 2, UpdatedAt: time.Date(2023, 1, 1, 0, 0, 0, 0, time.UTC)},
			},
		},
	}

	if err := workspace.Save(ws); err != nil {
		t.Fatal(err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatal(err)
	}

	// Check if a-ref comes before z-ref
	s := string(data)
	aIdx := strings.Index(s, "a-ref:")
	zIdx := strings.Index(s, "z-ref:")
	metaIdx := strings.Index(s, "_meta:")

	if aIdx == -1 || zIdx == -1 || metaIdx == -1 {
		t.Fatalf("missing keys in YAML:\n%s", s)
	}

	if aIdx > zIdx {
		t.Errorf("a-ref should come before z-ref in sorted YAML:\n%s", s)
	}
	
	if metaIdx < zIdx {
		t.Errorf("_meta should come after resources in YAML:\n%s", s)
	}
}
