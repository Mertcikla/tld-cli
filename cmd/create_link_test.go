package cmd_test

import (
	"os"
	"path/filepath"
	"testing"

	"gopkg.in/yaml.v3"

	"github.com/mertcikla/tld-cli/workspace"
)

// setupWorkspaceForLinks creates an element workspace with two children on the same parent diagram.
func setupWorkspaceForLinks(t *testing.T, dir string) {
	t.Helper()
	mustInitWorkspace(t, dir)
	mustRunCmd(t, dir, "create", "element", "Platform", "--ref", "platform", "--kind", "workspace")
	mustRunCmd(t, dir, "create", "element", "API", "--ref", "api", "--parent", "platform", "--kind", "service")
	mustRunCmd(t, dir, "create", "element", "DB", "--ref", "db", "--parent", "platform", "--kind", "database")
}

func TestAddLinkCmd_AppendsLink(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceForLinks(t, dir)

	_, _, err := runCmd(t, dir, "create", "link", "--from", "api", "--to", "db")
	if err != nil {
		t.Fatalf("create link: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if err != nil {
		t.Fatalf("read connectors.yaml: %v", err)
	}
	var connectors map[string]*workspace.Connector
	if err := yaml.Unmarshal(data, &connectors); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(connectors) != 1 {
		t.Fatalf("len(connectors) = %d, want 1", len(connectors))
	}
	connector := connectors["platform:api:db:"]
	if connector == nil || connector.View != "platform" || connector.Source != "api" || connector.Target != "db" {
		t.Errorf("unexpected connector: %+v", connector)
	}
}

func TestAddLinkCmd_RootElementsInferRootDiagram(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	mustRunCmd(t, dir, "create", "element", "API", "--ref", "api", "--kind", "service")
	mustRunCmd(t, dir, "create", "element", "DB", "--ref", "db", "--kind", "database")

	_, _, err := runCmd(t, dir, "create", "link", "--from", "api", "--to", "db")
	if err != nil {
		t.Fatalf("create link: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	if err != nil {
		t.Fatalf("read connectors.yaml: %v", err)
	}
	var connectors map[string]*workspace.Connector
	if err := yaml.Unmarshal(data, &connectors); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(connectors) != 1 {
		t.Fatalf("len(connectors) = %d, want 1", len(connectors))
	}
	connector := connectors["root:api:db:"]
	if connector == nil || connector.View != "root" {
		t.Errorf("unexpected connector: %+v", connector)
	}
}

func TestAddLinkCmd_TwoCallsTwoEntries(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceForLinks(t, dir)

	_, _, err := runCmd(t, dir, "create", "link", "--from", "api", "--to", "db")
	if err != nil {
		t.Fatalf("first create link: %v", err)
	}
	_, _, err = runCmd(t, dir, "create", "link", "--from", "db", "--to", "api")
	if err != nil {
		t.Fatalf("second create link: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	var connectors map[string]*workspace.Connector
	_ = yaml.Unmarshal(data, &connectors)

	if len(connectors) != 2 {
		t.Fatalf("len(connectors) = %d, want 2", len(connectors))
	}
}

func TestAddLinkCmd_ElementsInDifferentDiagramsSucceeds(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	mustRunCmd(t, dir, "create", "element", "Parent1", "--ref", "parent1", "--kind", "workspace")
	mustRunCmd(t, dir, "create", "element", "Parent2", "--ref", "parent2", "--kind", "workspace")
	mustRunCmd(t, dir, "create", "element", "API", "--ref", "api", "--parent", "parent1", "--kind", "service")
	mustRunCmd(t, dir, "create", "element", "DB", "--ref", "db", "--parent", "parent2", "--kind", "database")

	_, _, err := runCmd(t, dir, "create", "link", "--from", "api", "--to", "db")
	if err != nil {
		t.Fatalf("create link: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	var connectors map[string]*workspace.Connector
	_ = yaml.Unmarshal(data, &connectors)
	connector := connectors["root:api:db:"]
	if connector == nil || connector.View != "root" {
		t.Errorf("expected connector in root view, got %+v", connector)
	}
}

func TestAddLinkCmd_ElementsWithMultiplePlacementsSucceeds(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	// Create an element with 2 placements manually in elements.yaml
	elements := map[string]*workspace.Element{
		"api": {
			Name: "API", Kind: "service",
			Placements: []workspace.ViewPlacement{
				{ParentRef: "root"},
				{ParentRef: "other"},
			},
		},
		"db": {
			Name: "DB", Kind: "database",
			Placements: []workspace.ViewPlacement{
				{ParentRef: "other"},
			},
		},
	}
	data, _ := yaml.Marshal(elements)
	os.WriteFile(filepath.Join(dir, "elements.yaml"), data, 0600)

	_, _, err := runCmd(t, dir, "create", "link", "--from", "api", "--to", "db")
	if err != nil {
		t.Fatalf("create link: %v", err)
	}

	raw, _ := os.ReadFile(filepath.Join(dir, "connectors.yaml"))
	var connectors map[string]*workspace.Connector
	_ = yaml.Unmarshal(raw, &connectors)
	connector := connectors["other:api:db:"]
	if connector == nil || connector.View != "other" {
		t.Errorf("expected connector in 'other' view (shared parent), got %+v", connector)
	}
}

func TestAddLinkCmd_MissingFromFlag(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "link", "--to", "db")
	if err == nil {
		t.Fatal("expected error for missing --from")
	}
}

func TestAddLinkCmd_MissingToFlag(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "create", "link", "--from", "api")
	if err == nil {
		t.Fatal("expected error for missing --to")
	}
}
