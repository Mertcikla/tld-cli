package reporter_test

import (
	"strings"
	"testing"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/reporter"
	"github.com/mertcikla/tld-cli/workspace"
)

func emptyPlan(t *testing.T) *planner.Plan {
	t.Helper()
	plan, err := planner.Build(&workspace.Workspace{
		Diagrams: map[string]*workspace.Diagram{},
		Objects:  map[string]*workspace.Object{},
	}, false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	return plan
}

func TestRenderExecutionMarkdown_SuccessHeader(t *testing.T) {
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), nil, true, false)
	if !strings.Contains(buf.String(), "## Status: SUCCESS") {
		t.Errorf("missing SUCCESS header: %q", buf.String())
	}
}

func TestRenderExecutionMarkdown_FailureHeader(t *testing.T) {
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), nil, false, false)
	if !strings.Contains(buf.String(), "## Status: ROLLED BACK") {
		t.Errorf("missing ROLLED BACK header: %q", buf.String())
	}
}

func TestRenderExecutionMarkdown_NilResponseNoCrash(t *testing.T) {
	var buf strings.Builder
	// Should not panic
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), nil, false, false)
	out := buf.String()
	if !strings.Contains(out, "## Status:") {
		t.Errorf("expected status line: %q", out)
	}
}

func TestRenderExecutionMarkdown_SummaryTable(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		Summary: &diagv1.PlanSummary{
			DiagramsPlanned: 2,
			DiagramsCreated: 2,
			ObjectsPlanned:  3,
			ObjectsCreated:  3,
			EdgesPlanned:    1,
			EdgesCreated:    1,
			LinksPlanned:    1,
			LinksCreated:    1,
		},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, false)
	out := buf.String()

	if !strings.Contains(out, "## Planned vs Created") {
		t.Errorf("missing summary section: %q", out)
	}
	if !strings.Contains(out, "| Diagrams | 2 | 2 |") {
		t.Errorf("wrong diagram count: %q", out)
	}
	if !strings.Contains(out, "| Objects  | 3 | 3 |") {
		t.Errorf("wrong object count: %q", out)
	}
}

func TestRenderExecutionMarkdown_CreatedDiagramsSection(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		CreatedDiagrams: []*diagv1.Diagram{
			{Id: 1, Name: "System"},
			{Id: 2, Name: "Container"},
		},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, true)
	out := buf.String()

	if !strings.Contains(out, "### Diagrams") {
		t.Errorf("missing Diagrams section: %q", out)
	}
	if !strings.Contains(out, "1") || !strings.Contains(out, "System") {
		t.Errorf("diagram 1 not in output: %q", out)
	}
	if !strings.Contains(out, "2") || !strings.Contains(out, "Container") {
		t.Errorf("diagram 2 not in output: %q", out)
	}
}

func TestRenderExecutionMarkdown_CreatedSectionsAbsentOnFailure(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		CreatedDiagrams: []*diagv1.Diagram{{Id: 1, Name: "System"}},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, false, true)
	out := buf.String()

	if strings.Contains(out, "## Created Resources") {
		t.Errorf("Created Resources should be absent on failure: %q", out)
	}
}

func ptrString(s string) *string { return &s }

func TestRenderExecutionMarkdown_CreatedObjectsSection(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		CreatedObjects: []*diagv1.Object{
			{Id: 10, Name: "API Gateway", Type: ptrString("service")},
		},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, true)
	out := buf.String()

	if !strings.Contains(out, "### Objects") {
		t.Errorf("missing Objects section: %q", out)
	}
	if !strings.Contains(out, "10") || !strings.Contains(out, "API Gateway") {
		t.Errorf("object not in output: %q", out)
	}
}

func TestRenderExecutionMarkdown_DriftSection(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		Drift: []*diagv1.PlanDriftItem{
			{ResourceType: "diagram", Ref: "old-diag", Reason: "name changed"},
		},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, false)
	out := buf.String()

	if !strings.Contains(out, "## Drift") {
		t.Errorf("missing Drift section: %q", out)
	}
	if !strings.Contains(out, "diagram") || !strings.Contains(out, "old-diag") || !strings.Contains(out, "name changed") {
		t.Errorf("drift item not in output: %q", out)
	}
}

func TestRenderExecutionMarkdown_DriftAbsent(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		Drift: []*diagv1.PlanDriftItem{},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, false)
	out := buf.String()

	if strings.Contains(out, "## Drift") {
		t.Errorf("Drift section should be absent when empty: %q", out)
	}
}

func TestRenderExecutionMarkdown_CreatedEdgesSection(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		CreatedEdges: []*diagv1.Edge{
			{Id: 100, SourceObjectId: 10, TargetObjectId: 20},
		},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, true)
	out := buf.String()

	if !strings.Contains(out, "### Edges") {
		t.Errorf("missing Edges section: %q", out)
	}
	if !strings.Contains(out, "100") {
		t.Errorf("edge ID not in output: %q", out)
	}
}

func TestRenderExecutionMarkdown_CreatedLinksSection(t *testing.T) {
	resp := &diagv1.ApplyPlanResponse{
		CreatedLinks: []*diagv1.ObjectLink{
			{Id: 1000, ObjectId: 10, FromDiagramId: 1, ToDiagramId: 2},
		},
	}
	var buf strings.Builder
	reporter.RenderExecutionMarkdown(&buf, emptyPlan(t), resp, true, true)
	out := buf.String()

	if !strings.Contains(out, "### Links") {
		t.Errorf("missing Links section: %q", out)
	}
	if !strings.Contains(out, "1000") {
		t.Errorf("link ID not in output: %q", out)
	}
}
