package cmd_test

import (
	"os"
	"path/filepath"
	"testing"

	"gopkg.in/yaml.v3"

	"github.com/mertcikla/tldiagram-cli/workspace"
)

// setupWorkspaceWithObjects creates a workspace with "system" diagram and two objects: "svc", "db".
func setupWorkspaceWithObjects(t *testing.T, dir string) {
	t.Helper()
	setupDiagram(t, dir)
	if _, _, err := runCmd(t, dir, "create", "object", "system", "Service", "service"); err != nil {
		t.Fatalf("create svc: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "system", "Database", "database"); err != nil {
		t.Fatalf("create db: %v", err)
	}
}

func TestConnectObjectsCmd_AppendsEdge(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceWithObjects(t, dir)

	_, _, err := runCmd(t, dir, "connect", "objects", "system", "--from", "service", "--to", "database")
	if err != nil {
		t.Fatalf("connect objects: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "edges.yaml"))
	if err != nil {
		t.Fatalf("read edges.yaml: %v", err)
	}
	var edges []workspace.Edge
	if err := yaml.Unmarshal(data, &edges); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(edges) != 1 {
		t.Fatalf("len(edges) = %d, want 1", len(edges))
	}
	if edges[0].SourceObject != "service" || edges[0].TargetObject != "database" {
		t.Errorf("unexpected edge: %+v", edges[0])
	}
}

func TestConnectObjectsCmd_TwoCallsTwoEntries(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceWithObjects(t, dir)

	_, _, err := runCmd(t, dir, "connect", "objects", "system", "--from", "service", "--to", "database")
	if err != nil {
		t.Fatalf("first connect: %v", err)
	}
	_, _, err = runCmd(t, dir, "connect", "objects", "system", "--from", "database", "--to", "service")
	if err != nil {
		t.Fatalf("second connect: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "edges.yaml"))
	var edges []workspace.Edge
	_ = yaml.Unmarshal(data, &edges)

	if len(edges) != 2 {
		t.Fatalf("len(edges) = %d, want 2", len(edges))
	}
}

func TestConnectObjectsCmd_MissingFromFlag(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "connect", "objects", "system", "--to", "db")
	if err == nil {
		t.Fatal("expected error for missing --from")
	}
}

func TestConnectObjectsCmd_MissingToFlag(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "connect", "objects", "system", "--from", "svc")
	if err == nil {
		t.Fatal("expected error for missing --to")
	}
}
