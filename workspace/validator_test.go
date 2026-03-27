package workspace_test

import (
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
)

// buildWorkspace creates a minimal valid Workspace from the given components.
func buildWorkspace(diagrams map[string]*workspace.Diagram, objects map[string]*workspace.Object, edges []workspace.Edge, links []workspace.Link) *workspace.Workspace {
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
	}
}

func TestValidate_EmptyWorkspace(t *testing.T) {
	ws := buildWorkspace(nil, nil, nil, nil)
	errs := ws.Validate()
	if len(errs) != 0 {
		t.Errorf("expected 0 errors, got %d: %v", len(errs), errs)
	}
}

func TestValidate_DiagramNameRequired(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{
		"x": {Name: ""},
	}, nil, nil, nil)
	errs := ws.Validate()
	if !containsError(errs, "diagrams.yaml[x]", "name is required") {
		t.Errorf("expected name-required error, got: %v", errs)
	}
}

func TestValidate_DiagramParentRefNotFound(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{
		"child": {Name: "Child", ParentDiagram: "missing-parent"},
	}, nil, nil, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_DiagramCycleDirectSelf(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{
		"a": {Name: "A", ParentDiagram: "a"},
	}, nil, nil, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "circular") {
		t.Errorf("expected cycle error, got: %v", errs)
	}
}

func TestValidate_DiagramCycleTwoNodes(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{
		"a": {Name: "A", ParentDiagram: "b"},
		"b": {Name: "B", ParentDiagram: "a"},
	}, nil, nil, nil)
	errs := ws.Validate()
	cycleErrs := filterErrors(errs, "circular")
	if len(cycleErrs) == 0 {
		t.Errorf("expected cycle errors, got: %v", errs)
	}
}

func TestValidate_DiagramCycleThreeNodes(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{
		"a": {Name: "A", ParentDiagram: "b"},
		"b": {Name: "B", ParentDiagram: "c"},
		"c": {Name: "C", ParentDiagram: "a"},
	}, nil, nil, nil)
	errs := ws.Validate()
	cycleErrs := filterErrors(errs, "circular")
	if len(cycleErrs) == 0 {
		t.Errorf("expected cycle errors, got: %v", errs)
	}
}

func TestValidate_DiagramNoCycle(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{
		"a": {Name: "A"},
		"b": {Name: "B", ParentDiagram: "a"},
		"c": {Name: "C", ParentDiagram: "b"},
	}, nil, nil, nil)
	errs := ws.Validate()
	cycleErrs := filterErrors(errs, "circular")
	if len(cycleErrs) != 0 {
		t.Errorf("unexpected cycle errors: %v", cycleErrs)
	}
}

func TestValidate_ObjectNameRequired(t *testing.T) {
	ws := buildWorkspace(nil, map[string]*workspace.Object{
		"x": {Name: "", Type: "service"},
	}, nil, nil)
	errs := ws.Validate()
	if !containsError(errs, "objects.yaml[x]", "name is required") {
		t.Errorf("expected name-required error, got: %v", errs)
	}
}

func TestValidate_ObjectTypeRequired(t *testing.T) {
	ws := buildWorkspace(nil, map[string]*workspace.Object{
		"x": {Name: "X", Type: ""},
	}, nil, nil)
	errs := ws.Validate()
	if !containsError(errs, "objects.yaml[x]", "type is required") {
		t.Errorf("expected type-required error, got: %v", errs)
	}
}

func TestValidate_ObjectPlacementDiagramRequired(t *testing.T) {
	ws := buildWorkspace(nil, map[string]*workspace.Object{
		"svc": {Name: "Svc", Type: "service", Diagrams: []workspace.Placement{{Diagram: ""}}},
	}, nil, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "diagram is required") {
		t.Errorf("expected diagram-required error, got: %v", errs)
	}
}

func TestValidate_ObjectPlacementDiagramRefNotFound(t *testing.T) {
	ws := buildWorkspace(nil, map[string]*workspace.Object{
		"svc": {Name: "Svc", Type: "service", Diagrams: []workspace.Placement{{Diagram: "nonexistent"}}},
	}, nil, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_EdgeDiagramRequired(t *testing.T) {
	ws := buildWorkspace(nil, nil, []workspace.Edge{{Diagram: "", SourceObject: "a", TargetObject: "b"}}, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "diagram is required") {
		t.Errorf("expected diagram-required error, got: %v", errs)
	}
}

func TestValidate_EdgeDiagramRefNotFound(t *testing.T) {
	ws := buildWorkspace(nil, nil, []workspace.Edge{{Diagram: "missing", SourceObject: "a", TargetObject: "b"}}, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_EdgeSourceObjectRequired(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{"d": {Name: "D"}}, nil,
		[]workspace.Edge{{Diagram: "d", SourceObject: "", TargetObject: "b"}}, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "source_object is required") {
		t.Errorf("expected source_object-required error, got: %v", errs)
	}
}

func TestValidate_EdgeSourceObjectRefNotFound(t *testing.T) {
	ws := buildWorkspace(map[string]*workspace.Diagram{"d": {Name: "D"}}, nil,
		[]workspace.Edge{{Diagram: "d", SourceObject: "missing", TargetObject: "b"}}, nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_EdgeTargetObjectRequired(t *testing.T) {
	ws := buildWorkspace(
		map[string]*workspace.Diagram{"d": {Name: "D"}},
		map[string]*workspace.Object{"a": {Name: "A", Type: "service"}},
		[]workspace.Edge{{Diagram: "d", SourceObject: "a", TargetObject: ""}},
		nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "target_object is required") {
		t.Errorf("expected target_object-required error, got: %v", errs)
	}
}

func TestValidate_EdgeTargetObjectRefNotFound(t *testing.T) {
	ws := buildWorkspace(
		map[string]*workspace.Diagram{"d": {Name: "D"}},
		map[string]*workspace.Object{"a": {Name: "A", Type: "service"}},
		[]workspace.Edge{{Diagram: "d", SourceObject: "a", TargetObject: "missing"}},
		nil)
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_LinkWithoutObject(t *testing.T) {
	// A link without an object is valid; from/to diagram refs are still checked.
	diagrams := map[string]*workspace.Diagram{
		"d1": {Name: "D1"},
		"d2": {Name: "D2"},
	}
	ws := buildWorkspace(diagrams, nil, nil, []workspace.Link{{Object: "", FromDiagram: "d1", ToDiagram: "d2"}})
	errs := ws.Validate()
	if len(errs) != 0 {
		t.Errorf("expected no errors for object-less link, got: %v", errs)
	}
}

func TestValidate_LinkObjectRefNotFound(t *testing.T) {
	ws := buildWorkspace(nil, nil, nil, []workspace.Link{{Object: "missing", FromDiagram: "d1", ToDiagram: "d2"}})
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_LinkFromDiagramRequired(t *testing.T) {
	ws := buildWorkspace(nil,
		map[string]*workspace.Object{"obj": {Name: "Obj", Type: "service"}},
		nil,
		[]workspace.Link{{Object: "obj", FromDiagram: "", ToDiagram: "d2"}})
	errs := ws.Validate()
	if !containsErrorMsg(errs, "from_diagram is required") {
		t.Errorf("expected from_diagram-required error, got: %v", errs)
	}
}

func TestValidate_LinkFromDiagramRefNotFound(t *testing.T) {
	ws := buildWorkspace(nil,
		map[string]*workspace.Object{"obj": {Name: "Obj", Type: "service"}},
		nil,
		[]workspace.Link{{Object: "obj", FromDiagram: "missing", ToDiagram: "d2"}})
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_LinkToDiagramRequired(t *testing.T) {
	ws := buildWorkspace(
		map[string]*workspace.Diagram{"d1": {Name: "D1"}},
		map[string]*workspace.Object{"obj": {Name: "Obj", Type: "service"}},
		nil,
		[]workspace.Link{{Object: "obj", FromDiagram: "d1", ToDiagram: ""}})
	errs := ws.Validate()
	if !containsErrorMsg(errs, "to_diagram is required") {
		t.Errorf("expected to_diagram-required error, got: %v", errs)
	}
}

func TestValidate_LinkToDiagramRefNotFound(t *testing.T) {
	ws := buildWorkspace(
		map[string]*workspace.Diagram{"d1": {Name: "D1"}},
		map[string]*workspace.Object{"obj": {Name: "Obj", Type: "service"}},
		nil,
		[]workspace.Link{{Object: "obj", FromDiagram: "d1", ToDiagram: "missing"}})
	errs := ws.Validate()
	if !containsErrorMsg(errs, "not found") {
		t.Errorf("expected not-found error, got: %v", errs)
	}
}

func TestValidate_MultipleErrorsAccumulated(t *testing.T) {
	ws := buildWorkspace(nil, map[string]*workspace.Object{
		"a": {Name: "", Type: ""},
		"b": {Name: "", Type: ""},
		"c": {Name: "", Type: ""},
	}, nil, nil)
	errs := ws.Validate()
	// Each object is missing name AND type: at least 3 errors
	if len(errs) < 3 {
		t.Errorf("expected at least 3 errors (no early return), got %d: %v", len(errs), errs)
	}
}

// ---- helpers ----

func containsError(errs []workspace.ValidationError, location, message string) bool {
	for _, e := range errs {
		if strings.Contains(e.Location, location) && strings.Contains(e.Message, message) {
			return true
		}
	}
	return false
}

func containsErrorMsg(errs []workspace.ValidationError, message string) bool {
	for _, e := range errs {
		if strings.Contains(e.Message, message) {
			return true
		}
	}
	return false
}

func filterErrors(errs []workspace.ValidationError, msg string) []workspace.ValidationError {
	var out []workspace.ValidationError
	for _, e := range errs {
		if strings.Contains(e.Message, msg) || strings.Contains(e.Error(), msg) {
			out = append(out, e)
		}
	}
	return out
}
