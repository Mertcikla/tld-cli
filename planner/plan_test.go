package planner_test

import (
	"sort"
	"testing"

	"github.com/mertcikla/tldiagram-cli/planner"
	"github.com/mertcikla/tldiagram-cli/workspace"
)

func ws(diagrams map[string]*workspace.Diagram, objects map[string]*workspace.Object, edges []workspace.Edge, links []workspace.Link) *workspace.Workspace {
	if diagrams == nil {
		diagrams = map[string]*workspace.Diagram{}
	}
	if objects == nil {
		objects = map[string]*workspace.Object{}
	}
	return &workspace.Workspace{
		Diagrams: diagrams,
		Objects:  objects,
		Edges:    edges,
		Links:    links,
		Config:   workspace.Config{OrgID: "test-org-id"},
	}
}

func TestBuild_EmptyWorkspace(t *testing.T) {
	plan, err := planner.Build(ws(nil, nil, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	if plan == nil || plan.Request == nil {
		t.Fatal("plan or request is nil")
	}
	if plan.Request.OrgId != "test-org-id" {
		t.Errorf("OrgId = %q, want 'test-org-id'", plan.Request.OrgId)
	}
	if len(plan.Request.Diagrams) != 0 || len(plan.Request.Objects) != 0 {
		t.Errorf("expected empty request")
	}
}

func TestBuild_DiagramOptionalFieldsAbsent(t *testing.T) {
	plan, err := planner.Build(ws(map[string]*workspace.Diagram{
		"a": {Name: "A"}, // no Description, LevelLabel, ParentDiagram
	}, nil, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	if len(plan.Request.Diagrams) != 1 {
		t.Fatalf("expected 1 diagram")
	}
	d := plan.Request.Diagrams[0]
	if d.Description != nil {
		t.Errorf("Description should be nil, got %v", d.Description)
	}
	if d.LevelLabel != nil {
		t.Errorf("LevelLabel should be nil, got %v", d.LevelLabel)
	}
	if d.ParentDiagramRef != nil {
		t.Errorf("ParentDiagramRef should be nil, got %v", d.ParentDiagramRef)
	}
}

func TestBuild_DiagramOptionalFieldsPresent(t *testing.T) {
	plan, err := planner.Build(ws(map[string]*workspace.Diagram{
		"a": {Name: "A", Description: "desc", LevelLabel: "System", ParentDiagram: ""},
	}, nil, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	d := plan.Request.Diagrams[0]
	if d.Description == nil || *d.Description != "desc" {
		t.Errorf("Description: got %v", d.Description)
	}
	if d.LevelLabel == nil || *d.LevelLabel != "System" {
		t.Errorf("LevelLabel: got %v", d.LevelLabel)
	}
}

func TestTopoSort_LinearChain(t *testing.T) {
	plan, err := planner.Build(ws(map[string]*workspace.Diagram{
		"a": {Name: "A"},
		"b": {Name: "B", ParentDiagram: "a"},
		"c": {Name: "C", ParentDiagram: "b"},
	}, nil, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	order := plan.DiagramOrder
	if len(order) != 3 {
		t.Fatalf("len(order) = %d, want 3", len(order))
	}
	indexA, indexB, indexC := indexOf(order, "a"), indexOf(order, "b"), indexOf(order, "c")
	if indexA > indexB || indexB > indexC {
		t.Errorf("wrong order: %v (expected a before b before c)", order)
	}
}

func TestTopoSort_TwoRootsAlphabetical(t *testing.T) {
	plan, err := planner.Build(ws(map[string]*workspace.Diagram{
		"z": {Name: "Z"},
		"a": {Name: "A"},
	}, nil, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	order := plan.DiagramOrder
	if len(order) < 2 || order[0] != "a" {
		t.Errorf("expected 'a' first (alphabetical), got: %v", order)
	}
}

func TestTopoSort_DiamondParents(t *testing.T) {
	// A is root; B and C both parent A; D parents B and C (diamond).
	// In this codebase, parent_diagram is a single ref (no multi-parent),
	// so model B→A, C→A, D→B (linear chain, not true diamond).
	plan, err := planner.Build(ws(map[string]*workspace.Diagram{
		"a": {Name: "A"},
		"b": {Name: "B", ParentDiagram: "a"},
		"c": {Name: "C", ParentDiagram: "a"},
		"d": {Name: "D", ParentDiagram: "b"},
	}, nil, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	order := plan.DiagramOrder
	// a must come before b and c; b must come before d
	if indexOf(order, "a") > indexOf(order, "b") {
		t.Errorf("a must come before b: %v", order)
	}
	if indexOf(order, "a") > indexOf(order, "c") {
		t.Errorf("a must come before c: %v", order)
	}
	if indexOf(order, "b") > indexOf(order, "d") {
		t.Errorf("b must come before d: %v", order)
	}
}

func TestBuild_ObjectPlacementZeroPosition(t *testing.T) {
	plan, err := planner.Build(ws(nil, map[string]*workspace.Object{
		"svc": {
			Name: "Svc",
			Type: "service",
			Diagrams: []workspace.Placement{
				{Diagram: "d", PositionX: 0, PositionY: 0},
			},
		},
	}, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	if len(plan.Request.Objects) != 1 {
		t.Fatalf("expected 1 object")
	}
	if len(plan.Request.Objects[0].Placements) != 1 {
		t.Fatalf("expected 1 placement")
	}
	p := plan.Request.Objects[0].Placements[0]
	if p.PositionX != nil {
		t.Errorf("PositionX should be nil when 0, got %v", p.PositionX)
	}
	if p.PositionY != nil {
		t.Errorf("PositionY should be nil when 0, got %v", p.PositionY)
	}
}

func TestBuild_ObjectPlacementNonZeroPosition(t *testing.T) {
	plan, err := planner.Build(ws(nil, map[string]*workspace.Object{
		"svc": {
			Name: "Svc",
			Type: "service",
			Diagrams: []workspace.Placement{
				{Diagram: "d", PositionX: 100.5, PositionY: 200.0},
			},
		},
	}, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	p := plan.Request.Objects[0].Placements[0]
	if p.PositionX == nil || *p.PositionX != 100.5 {
		t.Errorf("PositionX: got %v", p.PositionX)
	}
	if p.PositionY == nil || *p.PositionY != 200.0 {
		t.Errorf("PositionY: got %v", p.PositionY)
	}
}

func TestBuild_EdgeOptionalFieldsRoundTrip(t *testing.T) {
	plan, err := planner.Build(ws(nil, nil, []workspace.Edge{
		{Diagram: "d", SourceObject: "a", TargetObject: "b"}, // all optional empty
	}, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	e := plan.Request.Edges[0]
	if e.Label != nil || e.Description != nil || e.RelationshipType != nil ||
		e.Direction != nil || e.EdgeType != nil || e.Url != nil {
		t.Errorf("all optional fields should be nil when empty: %+v", e)
	}

	// With all fields set
	plan2, _ := planner.Build(ws(nil, nil, []workspace.Edge{
		{
			Diagram:          "d",
			SourceObject:     "a",
			TargetObject:     "b",
			Label:            "calls",
			Description:      "desc",
			RelationshipType: "sync",
			Direction:        "forward",
			EdgeType:         "bezier",
			URL:              "https://x.com",
		},
	}, nil), false)
	e2 := plan2.Request.Edges[0]
	if e2.Label == nil || *e2.Label != "calls" {
		t.Errorf("Label: %v", e2.Label)
	}
	if e2.Direction == nil || *e2.Direction != "forward" {
		t.Errorf("Direction: %v", e2.Direction)
	}
	if e2.Url == nil || *e2.Url != "https://x.com" {
		t.Errorf("Url: %v", e2.Url)
	}
}

func TestBuild_MultipleObjects(t *testing.T) {
	plan, err := planner.Build(ws(nil, map[string]*workspace.Object{
		"a": {Name: "A", Type: "service"},
		"b": {Name: "B", Type: "database"},
		"c": {Name: "C", Type: "person"},
	}, nil, nil), false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	if len(plan.Request.Objects) != 3 {
		t.Errorf("expected 3 objects, got %d", len(plan.Request.Objects))
	}
	// Collect refs and sort for comparison
	var refs []string
	for _, o := range plan.Request.Objects {
		refs = append(refs, o.Ref)
	}
	sort.Strings(refs)
	want := []string{"a", "b", "c"}
	for i, r := range refs {
		if r != want[i] {
			t.Errorf("refs[%d] = %q, want %q", i, r, want[i])
		}
	}
}

func indexOf(s []string, v string) int {
	for i, x := range s {
		if x == v {
			return i
		}
	}
	return -1
}
