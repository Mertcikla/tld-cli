package planner_test

import (
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
)

func buildPlan(t *testing.T, w *workspace.Workspace) *planner.Plan {
	t.Helper()
	plan, err := planner.Build(w, false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	return plan
}

func TestRenderPlanMarkdown_Header(t *testing.T) {
	w := ws(nil, nil, nil, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if !strings.Contains(out, "# Diagram Plan") {
		t.Errorf("missing header: %q", out)
	}
	if !strings.Contains(out, "## Summary") {
		t.Errorf("missing Summary section: %q", out)
	}
}

func TestRenderPlanMarkdown_SummaryTableCounts(t *testing.T) {
	w := ws(map[string]*workspace.Diagram{
		"a": {Name: "A"},
		"b": {Name: "B"},
	}, map[string]*workspace.Object{
		"svc": {Name: "Svc", Type: "service"},
	}, []workspace.Edge{
		{Diagram: "a", SourceObject: "svc", TargetObject: "svc"},
	}, []workspace.Link{
		{Object: "svc", FromDiagram: "a", ToDiagram: "b"},
	})
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()

	if !strings.Contains(out, "| Diagrams | 2 |") {
		t.Errorf("wrong diagram count in output: %q", out)
	}
	if !strings.Contains(out, "| Objects  | 1 |") {
		t.Errorf("wrong object count in output: %q", out)
	}
	if !strings.Contains(out, "| Edges    | 1 |") {
		t.Errorf("wrong edge count in output: %q", out)
	}
	if !strings.Contains(out, "| Links    | 1 |") {
		t.Errorf("wrong link count in output: %q", out)
	}
}

func TestRenderPlanMarkdown_DiagramHierarchySection(t *testing.T) {
	w := ws(map[string]*workspace.Diagram{
		"parent": {Name: "Parent System", LevelLabel: "System"},
		"child":  {Name: "Child Container", ParentDiagram: "parent"},
	}, nil, nil, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()

	if !strings.Contains(out, "## Diagram Hierarchy") {
		t.Errorf("missing hierarchy section: %q", out)
	}
	if !strings.Contains(out, "parent") {
		t.Errorf("parent ref not in output: %q", out)
	}
	if !strings.Contains(out, "child") {
		t.Errorf("child ref not in output: %q", out)
	}
	// LevelLabel should appear
	if !strings.Contains(out, "System") {
		t.Errorf("LevelLabel not in output: %q", out)
	}
}

func TestRenderPlanMarkdown_EdgesSection(t *testing.T) {
	w := ws(nil, nil, []workspace.Edge{
		{Diagram: "d", SourceObject: "src", TargetObject: "tgt", Label: "HTTP"},
	}, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, true)
	out := buf.String()

	if !strings.Contains(out, "## Edges") {
		t.Errorf("missing edges section: %q", out)
	}
	if !strings.Contains(out, "src") || !strings.Contains(out, "tgt") {
		t.Errorf("source/target missing from edges: %q", out)
	}
}

func TestRenderPlanMarkdown_LinksSection(t *testing.T) {
	w := ws(nil, nil, nil, []workspace.Link{
		{Object: "api", FromDiagram: "sys", ToDiagram: "con"},
	})
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, true)
	out := buf.String()

	if !strings.Contains(out, "## Links") {
		t.Errorf("missing links section: %q", out)
	}
	if !strings.Contains(out, "api") {
		t.Errorf("object ref missing from links: %q", out)
	}
}

func TestRenderPlanMarkdown_VerboseHint(t *testing.T) {
	w := ws(nil, nil, []workspace.Edge{
		{Diagram: "d", SourceObject: "src", TargetObject: "tgt", Label: "HTTP"},
	}, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()

	if strings.Contains(out, "## Edges") {
		t.Errorf("edges section should be hidden when verbose=false: %q", out)
	}
	if !strings.Contains(out, "💡 Use '-v' or '--verbose' for detailed resource reporting") {
		t.Errorf("missing verbose hint: %q", out)
	}
}


func TestRenderPlanMarkdown_EmptyWorkspace(t *testing.T) {
	w := ws(nil, nil, nil, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	// Should not panic
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if !strings.Contains(out, "# Diagram Plan") {
		t.Errorf("expected header even for empty workspace: %q", out)
	}
}

func TestRenderPlanMarkdown_NoEdgesNoEdgesSection(t *testing.T) {
	w := ws(nil, nil, nil, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if strings.Contains(out, "## Edges") {
		t.Errorf("edges section should be absent when no edges: %q", out)
	}
}

func TestRenderPlanMarkdown_NoLinksNoLinksSection(t *testing.T) {
	w := ws(nil, nil, nil, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if strings.Contains(out, "## Links") {
		t.Errorf("links section should be absent when no links: %q", out)
	}
}

func TestRenderPlanMarkdown_Order(t *testing.T) {
	w := ws(map[string]*workspace.Diagram{
		"a": {Name: "A"},
	}, nil, nil, nil)
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()

	hierarchyIdx := strings.Index(out, "## Diagram Hierarchy")
	summaryIdx := strings.Index(out, "## Summary")

	if hierarchyIdx == -1 {
		t.Fatal("missing Diagram Hierarchy section")
	}
	if summaryIdx == -1 {
		t.Fatal("missing Summary section")
	}

	if summaryIdx < hierarchyIdx {
		t.Errorf("Summary section should be after Diagram Hierarchy section, but it is before (Summary: %d, Hierarchy: %d)", summaryIdx, hierarchyIdx)
	}
}
