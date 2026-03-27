package workspace_test

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/mertcikla/tld-cli/workspace"
	"gopkg.in/yaml.v3"
)

// helpers

// ---- Slugify ----

func TestSlugify(t *testing.T) {
	cases := []struct {
		in   string
		want string
	}{
		{"API Service", "api-service"},
		{"Hello World", "hello-world"},
		{"My DB!", "my-db"},
		{"  leading spaces  ", "leading-spaces"},
		{"already-slug", "already-slug"},
		{"Special!@#$%Chars", "special-chars"},
		{"123 Numbers", "123-numbers"},
		{"ALL CAPS", "all-caps"},
		{"multiple---hyphens", "multiple-hyphens"}, // hyphens are non-alphanumeric, collapsed
	}
	for _, tc := range cases {
		got := workspace.Slugify(tc.in)
		if got != tc.want {
			t.Errorf("Slugify(%q) = %q, want %q", tc.in, got, tc.want)
		}
	}
}

// ---- WriteDiagram ----

func TestWriteDiagram_CreatesFile(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Diagram{Name: "System Overview", Description: "top level", LevelLabel: "System"}
	if err := workspace.WriteDiagram(dir, "system-overview", spec); err != nil {
		t.Fatalf("WriteDiagram: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if err != nil {
		t.Fatalf("read file: %v", err)
	}

	var got map[string]workspace.Diagram
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if len(got) != 1 {
		t.Fatalf("len = %d, want 1", len(got))
	}
	d := got["system-overview"]
	if d.Name != spec.Name {
		t.Errorf("Name: got %q, want %q", d.Name, spec.Name)
	}
	if d.Description != spec.Description {
		t.Errorf("Description: got %q, want %q", d.Description, spec.Description)
	}
	if d.LevelLabel != spec.LevelLabel {
		t.Errorf("LevelLabel: got %q, want %q", d.LevelLabel, spec.LevelLabel)
	}
}

func TestWriteDiagram_ErrorIfExists(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Diagram{Name: "My Diagram"}
	if err := workspace.WriteDiagram(dir, "my-diagram", spec); err != nil {
		t.Fatalf("first write: %v", err)
	}

	err := workspace.WriteDiagram(dir, "my-diagram", spec)
	if err == nil {
		t.Fatal("expected error on duplicate, got nil")
	}
	if !contains(err.Error(), "already exists") {
		t.Errorf("error %q does not contain 'already exists'", err.Error())
	}
}

// ---- WriteObject ----

func TestWriteObject_CreatesFile(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Object{
		Name:        "API Gateway",
		Type:        "service",
		Description: "entry point",
		Technology:  "Go",
		URL:         "https://example.com",
		Diagrams: []workspace.Placement{
			{Diagram: "system", PositionX: 100, PositionY: 200},
		},
	}
	if err := workspace.WriteObject(dir, "api-gateway", spec); err != nil {
		t.Fatalf("WriteObject: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	var got map[string]workspace.Object
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}

	if len(got) != 1 {
		t.Fatalf("len = %d, want 1", len(got))
	}
	o := got["api-gateway"]
	if o.Name != spec.Name {
		t.Errorf("Name: got %q", o.Name)
	}
	if o.Type != spec.Type {
		t.Errorf("Type: got %q", o.Type)
	}
	if len(o.Diagrams) != 1 || o.Diagrams[0].Diagram != "system" {
		t.Errorf("Diagrams: got %v", o.Diagrams)
	}
	if o.Diagrams[0].PositionX != 100 || o.Diagrams[0].PositionY != 200 {
		t.Errorf("Position: got %.0f,%.0f", o.Diagrams[0].PositionX, o.Diagrams[0].PositionY)
	}
}

func TestWriteObject_ErrorIfExists(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Object{Name: "DB", Type: "database"}
	if err := workspace.WriteObject(dir, "db", spec); err != nil {
		t.Fatalf("first write: %v", err)
	}
	err := workspace.WriteObject(dir, "db", spec)
	if err == nil {
		t.Fatal("expected error on duplicate")
	}
	if !contains(err.Error(), "already exists") {
		t.Errorf("error %q does not contain 'already exists'", err.Error())
	}
}

// ---- AppendEdge ----

func TestAppendEdge_CreatesFileWhenAbsent(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Edge{
		Diagram:      "system",
		SourceObject: "api",
		TargetObject: "db",
	}
	if err := workspace.AppendEdge(dir, spec); err != nil {
		t.Fatalf("AppendEdge: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "edges.yaml"))
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	var got []workspace.Edge
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(got) != 1 {
		t.Fatalf("len(got) = %d, want 1", len(got))
	}
	if got[0].Diagram != "system" || got[0].SourceObject != "api" || got[0].TargetObject != "db" {
		t.Errorf("unexpected edge: %+v", got[0])
	}
}

func TestAppendEdge_AppendsToExistingFile(t *testing.T) {
	dir := t.TempDir()

	e1 := &workspace.Edge{Diagram: "d1", SourceObject: "a", TargetObject: "b"}
	e2 := &workspace.Edge{Diagram: "d2", SourceObject: "c", TargetObject: "d"}

	if err := workspace.AppendEdge(dir, e1); err != nil {
		t.Fatalf("first append: %v", err)
	}
	if err := workspace.AppendEdge(dir, e2); err != nil {
		t.Fatalf("second append: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "edges.yaml"))
	var got []workspace.Edge
	_ = yaml.Unmarshal(data, &got)

	if len(got) != 2 {
		t.Fatalf("len(got) = %d, want 2", len(got))
	}
}

func TestAppendEdge_AllOptionalFields(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Edge{
		Diagram:          "diag",
		SourceObject:     "src",
		TargetObject:     "tgt",
		Label:            "calls",
		Description:      "HTTP call",
		RelationshipType: "sync",
		Direction:        "forward",
		EdgeType:         "bezier",
		URL:              "https://docs.example.com",
	}
	if err := workspace.AppendEdge(dir, spec); err != nil {
		t.Fatalf("AppendEdge: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "edges.yaml"))
	var got []workspace.Edge
	_ = yaml.Unmarshal(data, &got)

	if len(got) != 1 {
		t.Fatalf("len = %d", len(got))
	}
	e := got[0]
	if e.Label != "calls" || e.Description != "HTTP call" || e.RelationshipType != "sync" ||
		e.Direction != "forward" || e.EdgeType != "bezier" || e.URL != "https://docs.example.com" {
		t.Errorf("optional fields not round-tripped: %+v", e)
	}
}

// ---- AppendLink ----

func TestAppendLink_CreatesFileWhenAbsent(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Link{
		Object:      "api",
		FromDiagram: "system",
		ToDiagram:   "container",
	}
	if err := workspace.AppendLink(dir, spec); err != nil {
		t.Fatalf("AppendLink: %v", err)
	}

	data, err := os.ReadFile(filepath.Join(dir, "links.yaml"))
	if err != nil {
		t.Fatalf("read: %v", err)
	}
	var got []workspace.Link
	if err := yaml.Unmarshal(data, &got); err != nil {
		t.Fatalf("unmarshal: %v", err)
	}
	if len(got) != 1 {
		t.Fatalf("len = %d, want 1", len(got))
	}
	if got[0].Object != "api" || got[0].FromDiagram != "system" || got[0].ToDiagram != "container" {
		t.Errorf("unexpected link: %+v", got[0])
	}
}

func TestAppendLink_AppendsToExistingFile(t *testing.T) {
	dir := t.TempDir()

	l1 := &workspace.Link{Object: "a", FromDiagram: "d1", ToDiagram: "d2"}
	l2 := &workspace.Link{Object: "b", FromDiagram: "d3", ToDiagram: "d4"}

	_ = workspace.AppendLink(dir, l1)
	_ = workspace.AppendLink(dir, l2)

	data, _ := os.ReadFile(filepath.Join(dir, "links.yaml"))
	var got []workspace.Link
	_ = yaml.Unmarshal(data, &got)

	if len(got) != 2 {
		t.Fatalf("len = %d, want 2", len(got))
	}
}

// ---- UpsertObject ----

func TestUpsertObject_CreatesNew(t *testing.T) {
	dir := t.TempDir()

	spec := &workspace.Object{
		Name: "New Object",
		Type: "service",
		Diagrams: []workspace.Placement{
			{Diagram: "d1", PositionX: 10, PositionY: 20},
		},
	}
	if err := workspace.UpsertObject(dir, "new-obj", spec); err != nil {
		t.Fatalf("UpsertObject: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var got map[string]workspace.Object
	_ = yaml.Unmarshal(data, &got)

	if len(got) != 1 {
		t.Fatalf("len = %d", len(got))
	}
	if got["new-obj"].Name != "New Object" {
		t.Errorf("Name: got %q", got["new-obj"].Name)
	}
}

func TestUpsertObject_AppendsPlacement(t *testing.T) {
	dir := t.TempDir()

	o1 := &workspace.Object{
		Name: "Shared DB",
		Type: "database",
		Diagrams: []workspace.Placement{
			{Diagram: "d1", PositionX: 10, PositionY: 10},
		},
	}
	_ = workspace.UpsertObject(dir, "shared-db", o1)

	o2 := &workspace.Object{
		Name: "Shared DB",
		Type: "database",
		Diagrams: []workspace.Placement{
			{Diagram: "d2", PositionX: 50, PositionY: 50},
		},
	}
	if err := workspace.UpsertObject(dir, "shared-db", o2); err != nil {
		t.Fatalf("UpsertObject reuse: %v", err)
	}

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var got map[string]workspace.Object
	_ = yaml.Unmarshal(data, &got)

	obj := got["shared-db"]
	if len(obj.Diagrams) != 2 {
		t.Fatalf("expected 2 placements, got %d", len(obj.Diagrams))
	}
	if obj.Diagrams[0].Diagram != "d1" || obj.Diagrams[1].Diagram != "d2" {
		t.Errorf("unexpected diagrams: %v", obj.Diagrams)
	}
}

func TestUpsertObject_UpdatesExistingPlacement(t *testing.T) {
	dir := t.TempDir()

	o1 := &workspace.Object{
		Name: "Obj",
		Type: "service",
		Diagrams: []workspace.Placement{
			{Diagram: "d1", PositionX: 10, PositionY: 10},
		},
	}
	_ = workspace.UpsertObject(dir, "obj", o1)

	o2 := &workspace.Object{
		Name: "Obj",
		Type: "service",
		Diagrams: []workspace.Placement{
			{Diagram: "d1", PositionX: 99, PositionY: 99},
		},
	}
	_ = workspace.UpsertObject(dir, "obj", o2)

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var got map[string]workspace.Object
	_ = yaml.Unmarshal(data, &got)

	obj := got["obj"]
	if len(obj.Diagrams) != 1 {
		t.Fatalf("expected 1 placement, got %d", len(obj.Diagrams))
	}
	if obj.Diagrams[0].PositionX != 99 {
		t.Errorf("PositionX: got %v", obj.Diagrams[0].PositionX)
	}
}

func TestUpsertObject_ErrorsOnTypeMismatch(t *testing.T) {
	dir := t.TempDir()

	o1 := &workspace.Object{Name: "Shared", Type: "service"}
	_ = workspace.UpsertObject(dir, "shared", o1)

	o2 := &workspace.Object{Name: "Shared", Type: "database"}
	err := workspace.UpsertObject(dir, "shared", o2)
	if err == nil {
		t.Fatal("expected error on type mismatch")
	}
	if !contains(err.Error(), "already exists with type \"service\"") {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestUpsertObject_EnrichesMetadata(t *testing.T) {
	dir := t.TempDir()

	o1 := &workspace.Object{Name: "Obj", Type: "service"}
	_ = workspace.UpsertObject(dir, "obj", o1)

	o2 := &workspace.Object{
		Name:        "Obj",
		Type:        "service",
		Description: "New Desc",
		Technology:  "Go",
		Diagrams:    []workspace.Placement{{Diagram: "d1"}},
	}
	_ = workspace.UpsertObject(dir, "obj", o2)

	data, _ := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	var got map[string]workspace.Object
	_ = yaml.Unmarshal(data, &got)

	obj := got["obj"]
	if obj.Description != "New Desc" || obj.Technology != "Go" {
		t.Errorf("metadata not enriched: %+v", obj)
	}
}

// ---- Save ----

func TestSave(t *testing.T) {
	dir := t.TempDir()

	ws := &workspace.Workspace{
		Dir: dir,
		Diagrams: map[string]*workspace.Diagram{
			"d1": {Name: "D1"},
		},
		Objects: map[string]*workspace.Object{
			"o1": {Name: "O1", Type: "service"},
		},
		Edges: []workspace.Edge{
			{Diagram: "d1", SourceObject: "o1", TargetObject: "o2"},
		},
		Links: []workspace.Link{
			{Object: "o1", FromDiagram: "d1", ToDiagram: "d2"},
		},
		Meta: &workspace.Meta{
			Diagrams: map[string]*workspace.ResourceMetadata{
				"d1": {ID: 1, UpdatedAt: time.Now()},
			},
			Objects: map[string]*workspace.ResourceMetadata{
				"o1": {ID: 2, UpdatedAt: time.Now()},
			},
		},
	}

	if err := workspace.Save(ws); err != nil {
		t.Fatalf("Save: %v", err)
	}

	// Verify diagrams.yaml has _meta
	data, _ := os.ReadFile(filepath.Join(dir, "diagrams.yaml"))
	if !contains(string(data), "_meta") {
		t.Error("diagrams.yaml missing _meta")
	}

	// Verify objects.yaml has _meta
	data, _ = os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if !contains(string(data), "_meta") {
		t.Error("objects.yaml missing _meta")
	}

	// Verify edges.yaml
	data, _ = os.ReadFile(filepath.Join(dir, "edges.yaml"))
	if !contains(string(data), "source_object: o1") {
		t.Error("edges.yaml missing edge")
	}

	// Verify links.yaml
	data, _ = os.ReadFile(filepath.Join(dir, "links.yaml"))
	if !contains(string(data), "object: o1") {
		t.Error("links.yaml missing link")
	}
}

// ---- helpers ----

func contains(s, sub string) bool {
	return len(s) >= len(sub) && (s == sub || len(sub) == 0 ||
		func() bool {
			for i := 0; i <= len(s)-len(sub); i++ {
				if s[i:i+len(sub)] == sub {
					return true
				}
			}
			return false
		}())
}
