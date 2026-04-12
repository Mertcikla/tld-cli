package workspace_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/mertcikla/tld-cli/workspace"
	"gopkg.in/yaml.v3"
)

func TestSlugify(t *testing.T) {
	cases := []struct {
		in   string
		want string
	}{
		{"API Service", "api-service"},
		{"My DB!", "my-db"},
		{"  leading spaces  ", "leading-spaces"},
		{"multiple---hyphens", "multiple-hyphens"},
	}
	for _, tc := range cases {
		if got := workspace.Slugify(tc.in); got != tc.want {
			t.Fatalf("Slugify(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}

func TestUpsertElement_CreatesAndMergesPlacements(t *testing.T) {
	dir := t.TempDir()
	if err := workspace.UpsertElement(dir, "api", &workspace.Element{
		Name:       "API",
		Kind:       "service",
		Placements: []workspace.ViewPlacement{{ParentRef: "root", PositionX: 10, PositionY: 20}},
	}); err != nil {
		t.Fatalf("first UpsertElement: %v", err)
	}
	if err := workspace.UpsertElement(dir, "api", &workspace.Element{
		Name:        "API",
		Kind:        "service",
		Description: "Handles traffic",
		HasView:     true,
		ViewLabel:   "Container",
		Placements: []workspace.ViewPlacement{
			{ParentRef: "root", PositionX: 30, PositionY: 40},
			{ParentRef: "platform", PositionX: 50, PositionY: 60},
		},
	}); err != nil {
		t.Fatalf("second UpsertElement: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	var got map[string]workspace.Element
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatal(err)
	}
	element := got["api"]
	if element.Description != "Handles traffic" || !element.HasView || element.ViewLabel != "Container" {
		t.Fatalf("unexpected merged element: %+v", element)
	}
	if len(element.Placements) != 2 {
		t.Fatalf("expected 2 placements, got %d", len(element.Placements))
	}
	if element.Placements[0].ParentRef != "root" || element.Placements[0].PositionX != 30 || element.Placements[0].PositionY != 40 {
		t.Fatalf("root placement not updated: %+v", element.Placements[0])
	}
}

func TestUpsertElement_ErrorsOnKindMismatch(t *testing.T) {
	dir := t.TempDir()
	if err := workspace.UpsertElement(dir, "shared", &workspace.Element{Name: "Shared", Kind: "service"}); err != nil {
		t.Fatalf("first UpsertElement: %v", err)
	}
	err := workspace.UpsertElement(dir, "shared", &workspace.Element{Name: "Shared", Kind: "database"})
	if err == nil {
		t.Fatal("expected kind mismatch error")
	}
	if !strings.Contains(err.Error(), "already exists with kind") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestUpsertElement_PreservesElementAndViewMetaSections(t *testing.T) {
	dir := t.TempDir()
	content := `api:
  name: API
  kind: service
  placements:
    - parent: root
      position_x: 10
      position_y: 20
_meta_elements:
  api:
    id: elem123
    updated_at: 2024-01-01T00:00:00Z
_meta_views:
  api:
    id: view123
    updated_at: 2024-01-02T00:00:00Z
`
	if err := os.WriteFile(filepath.Join(dir, "elements.yaml"), []byte(content), 0600); err != nil {
		t.Fatal(err)
	}

	if err := workspace.UpsertElement(dir, "api", &workspace.Element{
		Name:        "API",
		Kind:        "service",
		Description: "Updated",
		Placements:  []workspace.ViewPlacement{{ParentRef: "root", PositionX: 50, PositionY: 60}},
	}); err != nil {
		t.Fatalf("UpsertElement: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	text := string(data)
	if !strings.Contains(text, "_meta_elements:") || !strings.Contains(text, "_meta_views:") {
		t.Fatalf("elements.yaml lost metadata sections:\n%s", text)
	}
	if !strings.Contains(text, "id: elem123") || !strings.Contains(text, "id: view123") {
		t.Fatalf("elements.yaml lost metadata values:\n%s", text)
	}
}

func TestAppendConnector_PreservesMetaSection(t *testing.T) {
	dir := t.TempDir()
	content := `system:api:db:reads:
  view: system
  source: api
  target: db
  label: reads
_meta_connectors:
  system:api:db:reads:
    id: conn123
    updated_at: 2024-01-01T00:00:00Z
`
	if err := os.WriteFile(filepath.Join(dir, "connectors.yaml"), []byte(content), 0600); err != nil {
		t.Fatal(err)
	}

	if err := workspace.AppendConnector(dir, &workspace.Connector{
		View:   "system",
		Source: "api",
		Target: "queue",
		Label:  "publishes",
	}); err != nil {
		t.Fatalf("AppendConnector: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	text := string(data)
	if !strings.Contains(text, "_meta_connectors:") {
		t.Fatalf("connectors.yaml lost _meta_connectors:\n%s", text)
	}
	if strings.Contains(text, "{view:") {
		t.Fatalf("connectors.yaml should use block-style YAML, got:\n%s", text)
	}
	if !strings.Contains(text, "\n  view: system\n") || !strings.Contains(text, "\n  source: api\n") {
		t.Fatalf("connectors.yaml did not render in a readable block style:\n%s", text)
	}
	if !strings.Contains(text, "id: conn123") {
		t.Fatalf("connectors.yaml lost connector metadata:\n%s", text)
	}
}

func TestSave_WritesElementsAndConnectorsAndRemovesLegacyFiles(t *testing.T) {
	dir := t.TempDir()
	for _, legacyFile := range []string{"diagrams.yaml", "objects.yaml", "edges.yaml", "links.yaml"} {
		if err := os.WriteFile(filepath.Join(dir, legacyFile), []byte("legacy: true\n"), 0600); err != nil {
			t.Fatal(err)
		}
	}

	ws := &workspace.Workspace{
		Dir: dir,
		Elements: map[string]*workspace.Element{
			"api": {
				Name:       "API",
				Kind:       "service",
				HasView:    true,
				Placements: []workspace.ViewPlacement{{ParentRef: "root"}},
			},
			"db": {
				Name:       "DB",
				Kind:       "database",
				Placements: []workspace.ViewPlacement{{ParentRef: "api"}},
			},
		},
		Connectors: map[string]*workspace.Connector{
			"api:api:db:reads": {View: "api", Source: "api", Target: "db", Label: "reads"},
		},
		Meta: &workspace.Meta{
			Elements: map[string]*workspace.ResourceMetadata{
				"api": {ID: 1, UpdatedAt: time.Now()},
			},
			Views: map[string]*workspace.ResourceMetadata{
				"api": {ID: 2, UpdatedAt: time.Now()},
			},
			Connectors: map[string]*workspace.ResourceMetadata{
				"api:api:db:reads": {ID: 3, UpdatedAt: time.Now()},
			},
		},
	}

	if err := workspace.Save(ws); err != nil {
		t.Fatalf("Save: %v", err)
	}

	elementsData, _ := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if !strings.Contains(string(elementsData), "_meta_elements:") || !strings.Contains(string(elementsData), "_meta_views:") {
		t.Fatalf("elements.yaml missing metadata sections:\n%s", elementsData)
	}
	connectorsData, _ := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if !strings.Contains(string(connectorsData), "_meta_connectors:") || !strings.Contains(string(connectorsData), "label: reads") {
		t.Fatalf("connectors.yaml missing connector data or metadata:\n%s", connectorsData)
	}
	for _, legacyFile := range []string{"diagrams.yaml", "objects.yaml", "edges.yaml", "links.yaml"} {
		if _, err := os.Stat(filepath.Join(dir, legacyFile)); !os.IsNotExist(err) {
			t.Fatalf("expected %s to be removed, err=%v", legacyFile, err)
		}
	}
}
