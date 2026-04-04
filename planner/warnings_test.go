package planner_test

import (
	"testing"

	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
)

func TestAnalyzePlan_TechnologyValidation(t *testing.T) {
	tests := []struct {
		name          string
		level         int
		technology    string
		wantWarningCount int
		wantRuleName  string
	}{
		{
			name:       "valid technology",
			level:      2,
			technology: "Go, React",
			wantWarningCount: 0,
		},
		{
			name:       "invalid technology",
			level:      2,
			technology: "UnknownTech",
			wantWarningCount: 1,
			wantRuleName: "Unknown Technology",
		},
		{
			name:       "mixed valid and invalid",
			level:      2,
			technology: "Go, NonExistentTech",
			wantWarningCount: 1,
			wantRuleName: "Unknown Technology",
		},
		{
			name:       "multiple invalid",
			level:      2,
			technology: "TechA / TechB",
			wantWarningCount: 1,
			wantRuleName: "Unknown Technology",
		},
		{
			name:       "empty technology level 2",
			level:      2,
			technology: "",
			wantWarningCount: 1,
			wantRuleName: "Missing Tech",
		},
		{
			name:       "invalid technology level 1",
			level:      1,
			technology: "UnknownTech",
			wantWarningCount: 0, // level 2 only
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			ws := &workspace.Workspace{
				Objects: map[string]*workspace.Object{
					"obj1": {
						Name:       "Object 1",
						Technology: tt.technology,
					},
				},
				Config: workspace.Config{
					Validation: &workspace.ValidationConfig{
						Level: tt.level,
					},
				},
			}

			warnings := planner.AnalyzePlan(ws)
			count := 0
			found := false
			for _, g := range warnings {
				count += len(g.Violations)
				if g.RuleName == tt.wantRuleName {
					found = true
				}
			}

			if count != tt.wantWarningCount {
				t.Errorf("got %d warnings, want %d", count, tt.wantWarningCount)
			}
			if tt.wantWarningCount > 0 && !found {
				t.Errorf("did not find warning rule %q", tt.wantRuleName)
			}
		})
	}
}
