package planner

import (
	"fmt"
	"strings"

	"github.com/mertcikla/tld-cli/workspace"
)

// WarningGroup represents a collection of similar architectural warnings.
type WarningGroup struct {
	RuleName    string
	Description string
	Mediation   string
	Violations  []string
}

// AnalyzePlan evaluates the workspace against architectural best practices and
// returns grouped warnings based on the configured strictness level.
func AnalyzePlan(ws *workspace.Workspace) []WarningGroup {
	level := 3
	allowLowInsight := false
	if ws.Config.Validation != nil {
		if ws.Config.Validation.Level > 0 {
			level = ws.Config.Validation.Level
		}
		allowLowInsight = ws.Config.Validation.AllowLowInsight
	}

	warnings := make(map[string]*WarningGroup)
	addWarning := func(rule, desc, mediation, violation string) {
		if _, ok := warnings[rule]; !ok {
			warnings[rule] = &WarningGroup{
				RuleName:    rule,
				Description: desc,
				Mediation:   mediation,
				Violations:  []string{},
			}
		}
		warnings[rule].Violations = append(warnings[rule].Violations, violation)
	}

	// Helpers
	isGenericName := func(name string) bool {
		lower := strings.ToLower(name)
		if strings.Contains(lower, "module") || strings.Contains(lower, "stuff") || strings.Contains(lower, "thing") {
			return true
		}
		return false
	}
	isGenericLabel := func(label string) bool {
		lower := strings.ToLower(label)
		return lower == "calls" || lower == "uses" || lower == "connects" || lower == "links" || lower == ""
	}

	// Prepare data structures
	diagramObjects := make(map[string][]string) // diagramRef -> objectRefs
	objectEdges := make(map[string]map[string]int) // objectRef -> diagramRef -> edgeCount
	diagramEdges := make(map[string]int) // diagramRef -> edgeCount

	for objRef, o := range ws.Objects {
		for _, p := range o.Diagrams {
			diagramObjects[p.Diagram] = append(diagramObjects[p.Diagram], objRef)
		}
		objectEdges[objRef] = make(map[string]int)
	}

	for _, e := range ws.Edges {
		diagramEdges[e.Diagram]++
		if objectEdges[e.SourceObject] != nil {
			objectEdges[e.SourceObject][e.Diagram]++
		}
		if objectEdges[e.TargetObject] != nil {
			objectEdges[e.TargetObject][e.Diagram]++
		}
	}

	// 1. Workspace-wide checks
	maxDepth := 0
	for ref := range ws.Diagrams {
		depth := 0
		cur := ref
		visited := make(map[string]bool)
		for cur != "" {
			if visited[cur] {
				break // cycle, handled by validator
			}
			visited[cur] = true
			if d, ok := ws.Diagrams[cur]; ok {
				cur = d.ParentDiagram
				if cur != "" {
					depth++
				}
			} else {
				break
			}
		}
		if depth > maxDepth {
			maxDepth = depth
		}
	}

	// Level 1 rules (always applied if level >= 1)
	if level >= 1 {
		if maxDepth < 1 && len(ws.Diagrams) > 1 {
			// maxDepth < 1 means max depth is 0, so < 2 levels deep
			addWarning("Depth Mismatch", "Diagram hierarchy is flat", "Create sub-diagrams to establish a zoomable hierarchy.", "Workspace")
		}

		for diagRef, objs := range diagramObjects {
			densityLimit := 15
			if level >= 2 {
				densityLimit = 12
			}
			if len(objs) > densityLimit {
				addWarning("High Density", fmt.Sprintf("> %d objects on one diagram", densityLimit), "Split the diagram into sub-diagrams to reduce cognitive load.", fmt.Sprintf("Diagram %q has %d objects", diagRef, len(objs)))
			}

			if !allowLowInsight && len(objs) > 0 {
				edgesCount := diagramEdges[diagRef]
				if edgesCount < len(objs) {
					addWarning("Low Insight Ratio", "Edges < Objects", "Add more edges to illustrate how components interact, rather than just listing them.", fmt.Sprintf("Diagram %q (Objects: %d, Edges: %d)", diagRef, len(objs), edgesCount))
				}
			}
		}

		for objRef, o := range ws.Objects {
			isRootLevel := false
			for _, p := range o.Diagrams {
				if d, ok := ws.Diagrams[p.Diagram]; ok && d.ParentDiagram == "" {
					isRootLevel = true
				}
				
				// Isolated Object & Shared Context
				edgesInDiag := objectEdges[objRef][p.Diagram]
				if edgesInDiag == 0 {
					if len(o.Diagrams) == 1 {
						addWarning("Isolated Object", "Object has 0 edges in a diagram", "explore its relationships further and link it using \"tld add link --from --to\" etc.", fmt.Sprintf("Object %q in Diagram %q", objRef, p.Diagram))
					} else {
						addWarning("Shared Context", "Shared object has no edges in sub-view", "Add edges to the shared object in this specific diagram or remove it from this view.", fmt.Sprintf("Object %q in Diagram %q", objRef, p.Diagram))
					}
				}
			}

			if isRootLevel {
				lowerType := strings.ToLower(o.Type)
				if lowerType == "function" || lowerType == "class" {
					addWarning("Abstraction Leak", "Implementation types at Root level", "Move functions and classes into sub-diagrams. Keep root views at the Service/Subsystem level.", fmt.Sprintf("Object %q (Type: %s)", objRef, o.Type))
				}
			}
		}

		for _, l := range ws.Links {
			if targetObjects := diagramObjects[l.ToDiagram]; len(targetObjects) == 0 {
				addWarning("Dead-End Drilldown", "Object has a link but no sub-diagram content", "Ensure the linked diagram exists and contains relevant details.", fmt.Sprintf("Link to Diagram %q", l.ToDiagram))
			}
		}
	}

	// Level 2 rules
	if level >= 2 {
		for objRef, o := range ws.Objects {
			if o.Technology == "" {
				addWarning("Missing Tech", "No `technology` field", "Add a 'technology' field (e.g. Go, React) to clarify the stack.", fmt.Sprintf("Object %q", objRef))
			}
		}
		for edgeRef, e := range ws.Edges {
			if isGenericLabel(e.Label) {
				addWarning("Generic Labels", "Label is overly generic", "Replace generic labels with domain-specific verbs like 'validates JWT' or 'SQL Query'.", fmt.Sprintf("Edge %q in Diagram %q (Label: %q)", edgeRef, e.Diagram, e.Label))
			}
		}
	}

	// Level 3 rules
	if level >= 3 {
		for diagRef, d := range ws.Diagrams {
			if d.Description == "" {
				addWarning("Missing Desc", "`description` field is empty", "Add a one-sentence summary to help readers understand the responsibility.", fmt.Sprintf("Diagram %q", diagRef))
			}
			if isGenericName(d.Name) {
				addWarning("Generic Naming", "Vague names make the map harder to understand", "Rename the element with a domain-specific, descriptive name.", fmt.Sprintf("Diagram %q (Name: %q)", diagRef, d.Name))
			}
		}
		for objRef, o := range ws.Objects {
			if o.Description == "" {
				addWarning("Missing Desc", "`description` field is empty", "Add a one-sentence summary to help readers understand the responsibility.", fmt.Sprintf("Object %q", objRef))
			}
			if isGenericName(o.Name) {
				addWarning("Generic Naming", "Vague names make the map harder to understand", "Rename the element with a domain-specific, descriptive name.", fmt.Sprintf("Object %q (Name: %q)", objRef, o.Name))
			}
		}
		for edgeRef, e := range ws.Edges {
			if e.Label == "" {
				addWarning("Missing Label", "Edge has no `label`", "An edge without a label is just a line. Add a 'label' field to tell what it does.", fmt.Sprintf("Edge %q in Diagram %q", edgeRef, e.Diagram))
			}
		}
	}

	// Convert map to slice
	var result []WarningGroup
	// Fixed order for determinism in tests/output
	order := []string{
		"High Density", "Isolated Object", "Shared Context", "Depth Mismatch", "Low Insight Ratio", "Dead-End Drilldown", "Abstraction Leak",
		"Generic Labels", "Missing Tech",
		"Missing Desc", "Generic Naming", "Missing Label",
	}
	for _, rule := range order {
		if wg, ok := warnings[rule]; ok {
			result = append(result, *wg)
		}
	}

	return result
}
