package workspace_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/mertcikla/tld-cli/workspace"
)

func TestSave_Deterministic(t *testing.T) {
	dir := t.TempDir()

	ws := &workspace.Workspace{
		Dir: dir,
		Elements: map[string]*workspace.Element{
			"z-ref": {Name: "Z Element", Kind: "service", Placements: []workspace.ViewPlacement{{ParentRef: "root"}}},
			"a-ref": {Name: "A Element", Kind: "service", Placements: []workspace.ViewPlacement{{ParentRef: "root"}}},
		},
		Meta: &workspace.Meta{
			Elements: map[string]*workspace.ResourceMetadata{
				"a-ref": {ID: 1, UpdatedAt: time.Date(2023, 1, 1, 0, 0, 0, 0, time.UTC)},
				"z-ref": {ID: 2, UpdatedAt: time.Date(2023, 1, 1, 0, 0, 0, 0, time.UTC)},
			},
			Views: map[string]*workspace.ResourceMetadata{
				"a-ref": {ID: 3, UpdatedAt: time.Date(2023, 1, 2, 0, 0, 0, 0, time.UTC)},
				"z-ref": {ID: 4, UpdatedAt: time.Date(2023, 1, 2, 0, 0, 0, 0, time.UTC)},
			},
		},
	}

	if err := workspace.Save(ws); err != nil {
		t.Fatal(err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if err != nil {
		t.Fatal(err)
	}

	s := string(data)
	aIdx := strings.Index(s, "a-ref:")
	zIdx := strings.Index(s, "z-ref:")
	metaIdx := strings.Index(s, "_meta_elements:")

	if aIdx == -1 || zIdx == -1 || metaIdx == -1 {
		t.Fatalf("missing keys in YAML:\n%s", s)
	}
	if aIdx > zIdx {
		t.Fatalf("a-ref should come before z-ref in sorted YAML:\n%s", s)
	}
	if metaIdx < zIdx {
		t.Fatalf("_meta_elements should come after resources in YAML:\n%s", s)
	}
}
