package cmd_test

import (
	"os"
	"path/filepath"
	"testing"

	"gopkg.in/yaml.v3"

	"github.com/mertcikla/tldiagram-cli/workspace"
)

// setupWorkspaceForLinks creates a workspace with two diagrams and one object on the first.
func setupWorkspaceForLinks(t *testing.T, dir string) {
	t.Helper()
	mustInitWorkspace(t, dir)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System"); err != nil {
		t.Fatalf("create system: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "diagram", "Container"); err != nil {
		t.Fatalf("create container: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "system", "API", "service"); err != nil {
		t.Fatalf("create api: %v", err)
	}
}

func TestAddLinkCmd_AppendsLink(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceForLinks(t, dir)

	_, _, err := runCmd(t, dir, "add", "link", "--object", "api", "--from", "system", "--to", "container")
	if err != nil {
		t.Fatalf("add link: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "links.yaml"))
	if err != nil {
		t.Fatalf("read links.yaml: %v", err)
	}
	var links []workspace.Link
	if err := yaml.Unmarshal(data, &links); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(links) != 1 {
		t.Fatalf("len(links) = %d, want 1", len(links))
	}
	if links[0].Object != "api" || links[0].FromDiagram != "system" || links[0].ToDiagram != "container" {
		t.Errorf("unexpected link: %+v", links[0])
	}
}

func TestAddLinkCmd_WithoutObject(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceForLinks(t, dir)

	_, _, err := runCmd(t, dir, "add", "link", "--from", "system", "--to", "container")
	if err != nil {
		t.Fatalf("add link without object: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "links.yaml"))
	if err != nil {
		t.Fatalf("read links.yaml: %v", err)
	}
	var links []workspace.Link
	if err := yaml.Unmarshal(data, &links); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(links) != 1 {
		t.Fatalf("len(links) = %d, want 1", len(links))
	}
	if links[0].Object != "" || links[0].FromDiagram != "system" || links[0].ToDiagram != "container" {
		t.Errorf("unexpected link: %+v", links[0])
	}
}

func TestAddLinkCmd_TwoCallsTwoEntries(t *testing.T) {
	dir := t.TempDir()
	setupWorkspaceForLinks(t, dir)

	_, _, err := runCmd(t, dir, "add", "link", "--object", "api", "--from", "system", "--to", "container")
	if err != nil {
		t.Fatalf("first add link: %v", err)
	}
	_, _, err = runCmd(t, dir, "add", "link", "--from", "container", "--to", "system")
	if err != nil {
		t.Fatalf("second add link: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "links.yaml"))
	var links []workspace.Link
	_ = yaml.Unmarshal(data, &links)

	if len(links) != 2 {
		t.Fatalf("len(links) = %d, want 2", len(links))
	}
}

func TestAddLinkCmd_MissingFromFlag(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "add", "link", "--object", "obj", "--to", "con")
	if err == nil {
		t.Fatal("expected error for missing --from")
	}
}

func TestAddLinkCmd_MissingToFlag(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	_, _, err := runCmd(t, dir, "add", "link", "--object", "obj", "--from", "sys")
	if err == nil {
		t.Fatal("expected error for missing --to")
	}
}
