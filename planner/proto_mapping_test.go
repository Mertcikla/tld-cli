package planner_test

import (
	"testing"

	"github.com/mertcikla/tldiagram-cli/planner"
	"github.com/mertcikla/tldiagram-cli/workspace"
)

// TestProtoMapping_DiagramFields verifies that each DiagramSpec field maps to the
// correct proto field with the correct nil/non-nil semantics.
func TestProtoMapping_DiagramRefMatchesMapKey(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("org-1", map[string]*workspace.Diagram{
		"my-ref": {Name: "My Diagram"},
	}, nil, nil, nil), false)
	if len(plan.Request.Diagrams) != 1 {
		t.Fatalf("expected 1 diagram")
	}
	if plan.Request.Diagrams[0].Ref != "my-ref" {
		t.Errorf("Ref = %q, want 'my-ref'", plan.Request.Diagrams[0].Ref)
	}
}

func TestProtoMapping_DiagramName(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"d": {Name: "Hello World"},
	}, nil, nil, nil), false)
	if plan.Request.Diagrams[0].Name != "Hello World" {
		t.Errorf("Name = %q", plan.Request.Diagrams[0].Name)
	}
}

func TestProtoMapping_DiagramDescriptionNilWhenEmpty(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"d": {Name: "D", Description: ""},
	}, nil, nil, nil), false)
	if plan.Request.Diagrams[0].Description != nil {
		t.Error("Description should be nil when empty string")
	}
}

func TestProtoMapping_DiagramDescriptionNonNilWhenSet(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"d": {Name: "D", Description: "a desc"},
	}, nil, nil, nil), false)
	d := plan.Request.Diagrams[0]
	if d.Description == nil || *d.Description != "a desc" {
		t.Errorf("Description = %v", d.Description)
	}
}

func TestProtoMapping_DiagramLevelLabelNilWhenEmpty(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"d": {Name: "D"},
	}, nil, nil, nil), false)
	if plan.Request.Diagrams[0].LevelLabel != nil {
		t.Error("LevelLabel should be nil when empty")
	}
}

func TestProtoMapping_DiagramLevelLabelNonNilWhenSet(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"d": {Name: "D", LevelLabel: "System"},
	}, nil, nil, nil), false)
	d := plan.Request.Diagrams[0]
	if d.LevelLabel == nil || *d.LevelLabel != "System" {
		t.Errorf("LevelLabel = %v", d.LevelLabel)
	}
}

func TestProtoMapping_DiagramParentDiagramRefNilWhenEmpty(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"d": {Name: "D"},
	}, nil, nil, nil), false)
	if plan.Request.Diagrams[0].ParentDiagramRef != nil {
		t.Error("ParentDiagramRef should be nil when empty")
	}
}

func TestProtoMapping_DiagramParentDiagramRefNonNilWhenSet(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", map[string]*workspace.Diagram{
		"parent": {Name: "Parent"},
		"child":  {Name: "Child", ParentDiagram: "parent"},
	}, nil, nil, nil), false)
	var child *struct{ ParentDiagramRef *string }
	for _, d := range plan.Request.Diagrams {
		if d.Ref == "child" {
			if d.ParentDiagramRef == nil || *d.ParentDiagramRef != "parent" {
				t.Errorf("ParentDiagramRef = %v, want 'parent'", d.ParentDiagramRef)
			}
			_ = child
			return
		}
	}
	t.Error("child diagram not found")
}

// ---- Object fields ----

func TestProtoMapping_ObjectRef(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"my-object": {Name: "N", Type: "T"},
	}, nil, nil), false)
	if plan.Request.Objects[0].Ref != "my-object" {
		t.Errorf("Ref = %q", plan.Request.Objects[0].Ref)
	}
}

func TestProtoMapping_ObjectDescriptionNilWhenEmpty(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"o": {Name: "N", Type: "T"},
	}, nil, nil), false)
	if plan.Request.Objects[0].Description != nil {
		t.Error("Description should be nil")
	}
}

func TestProtoMapping_ObjectURLMapsToUrl(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"o": {Name: "N", Type: "T", URL: "https://example.com"},
	}, nil, nil), false)
	o := plan.Request.Objects[0]
	if o.Url == nil || *o.Url != "https://example.com" {
		t.Errorf("Url = %v", o.Url)
	}
}

func TestProtoMapping_ObjectLogoURLMapsToLogoUrl(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"o": {Name: "N", Type: "T", LogoURL: "https://logo.example.com"},
	}, nil, nil), false)
	o := plan.Request.Objects[0]
	if o.LogoUrl == nil || *o.LogoUrl != "https://logo.example.com" {
		t.Errorf("LogoUrl = %v", o.LogoUrl)
	}
}

// ---- Placement fields ----

func TestProtoMapping_PlacementDiagramRef(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"o": {Name: "N", Type: "T", Diagrams: []workspace.Placement{{Diagram: "my-diag"}}},
	}, nil, nil), false)
	p := plan.Request.Objects[0].Placements[0]
	if p.DiagramRef != "my-diag" {
		t.Errorf("DiagramRef = %q", p.DiagramRef)
	}
}

func TestProtoMapping_PlacementPositionXNilWhenZero(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"o": {Name: "N", Type: "T", Diagrams: []workspace.Placement{{Diagram: "d", PositionX: 0}}},
	}, nil, nil), false)
	p := plan.Request.Objects[0].Placements[0]
	if p.PositionX != nil {
		t.Errorf("PositionX should be nil when 0, got %v", p.PositionX)
	}
}

func TestProtoMapping_PlacementPositionXNonNilWhenNonZero(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, map[string]*workspace.Object{
		"o": {Name: "N", Type: "T", Diagrams: []workspace.Placement{{Diagram: "d", PositionX: 42.0}}},
	}, nil, nil), false)
	p := plan.Request.Objects[0].Placements[0]
	if p.PositionX == nil || *p.PositionX != 42.0 {
		t.Errorf("PositionX = %v", p.PositionX)
	}
}

// ---- Edge fields ----

func TestProtoMapping_EdgeRequiredFields(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, nil, []workspace.Edge{
		{Diagram: "d", SourceObject: "src", TargetObject: "tgt"},
	}, nil), false)
	e := plan.Request.Edges[0]
	if e.DiagramRef != "d" {
		t.Errorf("DiagramRef = %q", e.DiagramRef)
	}
	if e.SourceObjectRef != "src" {
		t.Errorf("SourceObjectRef = %q", e.SourceObjectRef)
	}
	if e.TargetObjectRef != "tgt" {
		t.Errorf("TargetObjectRef = %q", e.TargetObjectRef)
	}
}

func TestProtoMapping_EdgeOptionalFieldsNilWhenEmpty(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, nil, []workspace.Edge{
		{Diagram: "d", SourceObject: "s", TargetObject: "t"},
	}, nil), false)
	e := plan.Request.Edges[0]
	if e.Label != nil || e.Description != nil || e.RelationshipType != nil ||
		e.Direction != nil || e.EdgeType != nil || e.Url != nil {
		t.Errorf("all optional fields should be nil: %+v", e)
	}
}

// ---- Link fields ----

func TestProtoMapping_LinkFields(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("", nil, nil, nil, []workspace.Link{
		{Object: "obj", FromDiagram: "from", ToDiagram: "to"},
	}), false)
	l := plan.Request.Links[0]
	if l.ObjectRef != "obj" {
		t.Errorf("ObjectRef = %q", l.ObjectRef)
	}
	if l.FromDiagramRef != "from" {
		t.Errorf("FromDiagramRef = %q", l.FromDiagramRef)
	}
	if l.ToDiagramRef != "to" {
		t.Errorf("ToDiagramRef = %q", l.ToDiagramRef)
	}
}

// ---- Request fields ----

func TestProtoMapping_RequestOrgID(t *testing.T) {
	plan, _ := planner.Build(wsWithOrgID("my-org-uuid", nil, nil, nil, nil), false)
	if plan.Request.OrgId != "my-org-uuid" {
		t.Errorf("OrgId = %q", plan.Request.OrgId)
	}
}

// ---- helpers ----

func wsWithOrgID(orgID string, diagrams map[string]*workspace.Diagram, objects map[string]*workspace.Object, edges []workspace.Edge, links []workspace.Link) *workspace.Workspace {
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
		Config:   workspace.Config{OrgID: orgID},
	}
}
