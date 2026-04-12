package workspace_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
	"gopkg.in/yaml.v3"
)

func TestRenameElement_CascadesPlacementsConnectorsAndMetadata(t *testing.T) {
	dir := t.TempDir()
	if err := os.WriteFile(filepath.Join(dir, "elements.yaml"), []byte(`platform:
  name: Platform
  kind: workspace
  has_view: true
  placements:
    - parent: root
api:
  name: API
  kind: service
  placements:
    - parent: platform
_meta_elements:
  platform:
    id: 11
    updated_at: 2024-01-01T00:00:00Z
  api:
    id: 12
    updated_at: 2024-01-01T00:00:00Z
_meta_views:
  platform:
    id: 21
    updated_at: 2024-01-01T00:00:00Z
`), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(dir, "connectors.yaml"), []byte(`platform:platform:api:contains:
  view: platform
  source: platform
  target: api
  label: contains
system:web:platform:calls:
  view: system
  source: web
  target: platform
  label: calls
_meta_connectors:
  platform:platform:api:contains:
    id: 31
    updated_at: 2024-01-01T00:00:00Z
  system:web:platform:calls:
    id: 32
    updated_at: 2024-01-01T00:00:00Z
`), 0600); err != nil {
		t.Fatal(err)
	}

	if err := workspace.RenameElement(dir, "platform", "platform-core"); err != nil {
		t.Fatalf("RenameElement failed: %v", err)
	}

	elementsData, err := os.ReadFile(filepath.Join(dir, "elements.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	elementsText := string(elementsData)
	if strings.Contains(elementsText, "\nplatform:\n") || strings.HasPrefix(elementsText, "platform:\n") || !strings.Contains(elementsText, "platform-core:\n") {
		t.Fatalf("elements key was not renamed:\n%s", elementsText)
	}
	if !strings.Contains(elementsText, "parent: platform-core") {
		t.Fatalf("placement parent was not updated:\n%s", elementsText)
	}
	if !strings.Contains(elementsText, "_meta_elements:") || !strings.Contains(elementsText, "_meta_views:") || !strings.Contains(elementsText, "platform-core:") {
		t.Fatalf("element metadata was not updated:\n%s", elementsText)
	}

	connectorsData, err := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if err != nil {
		t.Fatal(err)
	}
	connectorsText := string(connectorsData)
	if strings.Contains(connectorsText, "platform:platform:api:contains") || strings.Contains(connectorsText, "system:web:platform:calls") {
		t.Fatalf("old connector keys still exist:\n%s", connectorsText)
	}
	if !strings.Contains(connectorsText, "platform-core:platform-core:api:contains") || !strings.Contains(connectorsText, "system:web:platform-core:calls") {
		t.Fatalf("renamed connector keys missing:\n%s", connectorsText)
	}
	if !strings.Contains(connectorsText, "view: platform-core") || !strings.Contains(connectorsText, "source: platform-core") || !strings.Contains(connectorsText, "target: platform-core") {
		t.Fatalf("connector fields were not updated:\n%s", connectorsText)
	}
	if !strings.Contains(connectorsText, "_meta_connectors:") || !strings.Contains(connectorsText, "platform-core:platform-core:api:contains") {
		t.Fatalf("connector metadata was not renamed:\n%s", connectorsText)
	}
}

func TestRenameConnector(t *testing.T) {
	dir := t.TempDir()
	content := `system:api-handler:db:reads:
  view: system
  source: api-handler
  target: db
  label: reads
system:web:api-handler:calls:
  view: system
  source: web
  target: api-handler
  label: calls
_meta_connectors:
  system:api-handler:db:reads:
    id: c1
    updated_at: 2024-01-01T00:00:00Z
  system:web:api-handler:calls:
    id: c2
    updated_at: 2024-01-01T00:00:00Z
`
	if err := os.WriteFile(filepath.Join(dir, "connectors.yaml"), []byte(content), 0600); err != nil {
		t.Fatal(err)
	}

	if err := workspace.RenameConnector(dir, "api-handler", "api-handler-2"); err != nil {
		t.Fatalf("RenameConnector failed: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if err != nil {
		t.Fatal(err)
	}

	var got map[string]workspace.Connector
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatal(err)
	}
	if _, ok := got["system:api-handler:db:reads"]; ok {
		t.Fatal("old connector key still exists")
	}
	if _, ok := got["system:web:api-handler:calls"]; ok {
		t.Fatal("old target connector key still exists")
	}
	if conn, ok := got["system:api-handler-2:db:reads"]; !ok {
		t.Fatal("renamed source connector key missing")
	} else if conn.Source != "api-handler-2" {
		t.Fatalf("source not updated: %+v", conn)
	}
	if conn, ok := got["system:web:api-handler-2:calls"]; !ok {
		t.Fatal("renamed target connector key missing")
	} else if conn.Target != "api-handler-2" {
		t.Fatalf("target not updated: %+v", conn)
	}

	text := string(data)
	if !strings.Contains(text, "_meta_connectors:") {
		t.Fatalf("metadata section missing:\n%s", text)
	}
	if !strings.Contains(text, "system:api-handler-2:db:reads") || !strings.Contains(text, "system:web:api-handler-2:calls") {
		t.Fatalf("metadata keys were not renamed:\n%s", text)
	}
}
