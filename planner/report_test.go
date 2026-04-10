package planner_test

import (
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
)

func reportWorkspace() *workspace.Workspace {
	return &workspace.Workspace{
		Elements: map[string]*workspace.Element{
			"platform": {Name: "Platform", Kind: "workspace", HasView: true, ViewLabel: "System"},
			"api": {
				Name:       "API",
				Kind:       "service",
				HasView:    true,
				Placements: []workspace.ViewPlacement{{ParentRef: "platform"}},
			},
			"db": {
				Name:       "DB",
				Kind:       "database",
				Placements: []workspace.ViewPlacement{{ParentRef: "platform"}},
			},
		},
		Connectors: map[string]*workspace.Connector{
			"platform:api:db:reads": {View: "platform", Source: "api", Target: "db", Label: "reads", Direction: "forward"},
		},
	}
}

func buildPlan(t *testing.T, w *workspace.Workspace) *planner.Plan {
	t.Helper()
	plan, err := planner.Build(w, false)
	if err != nil {
		t.Fatalf("Build: %v", err)
	}
	return plan
}

func TestRenderPlanMarkdown_Header(t *testing.T) {
	w := reportWorkspace()
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if !strings.Contains(out, "# Element Plan") {
		t.Fatalf("missing header: %q", out)
	}
	if !strings.Contains(out, "## View Structure") || !strings.Contains(out, "## Summary") {
		t.Fatalf("missing sections: %q", out)
	}
}

func TestRenderPlanMarkdown_SummaryTableCounts(t *testing.T) {
	w := reportWorkspace()
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if !strings.Contains(out, "| Elements | 3 |") {
		t.Fatalf("wrong element count: %q", out)
	}
	if !strings.Contains(out, "| Views    | 2 |") {
		t.Fatalf("wrong view count: %q", out)
	}
	if !strings.Contains(out, "| Connectors | 1 |") {
		t.Fatalf("wrong connector count: %q", out)
	}
}

func TestRenderPlanMarkdown_ViewStructure(t *testing.T) {
	w := reportWorkspace()
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if !strings.Contains(out, "platform") || !strings.Contains(out, "api") || !strings.Contains(out, "db") {
		t.Fatalf("missing element tree entries: %q", out)
	}
	if !strings.Contains(out, "[view]") {
		t.Fatalf("missing view marker: %q", out)
	}
}

func TestRenderPlanMarkdown_VerboseSections(t *testing.T) {
	w := reportWorkspace()
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, true)
	out := buf.String()
	if !strings.Contains(out, "## Elements") {
		t.Fatalf("missing elements section: %q", out)
	}
	if !strings.Contains(out, "## Connectors") {
		t.Fatalf("missing connectors section: %q", out)
	}
}

func TestRenderPlanMarkdown_VerboseHint(t *testing.T) {
	w := reportWorkspace()
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if strings.Contains(out, "## Elements") {
		t.Fatalf("verbose elements section should be hidden: %q", out)
	}
	if !strings.Contains(out, "Use '-v' or '--verbose' for detailed element placement and connector reporting.") {
		t.Fatalf("missing verbose hint: %q", out)
	}
}

func TestRenderPlanMarkdown_EmptyWorkspace(t *testing.T) {
	w := &workspace.Workspace{Elements: map[string]*workspace.Element{}, Connectors: map[string]*workspace.Connector{}}
	plan := buildPlan(t, w)
	var buf strings.Builder
	planner.RenderPlanMarkdown(&buf, plan, w, false)
	out := buf.String()
	if !strings.Contains(out, "# Element Plan") {
		t.Fatalf("expected header for empty workspace: %q", out)
	}
}
