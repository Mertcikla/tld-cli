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

type warningContext struct {
	ws              *workspace.Workspace
	level           int
	allowLowInsight bool
	warnings        map[string]*WarningGroup
	diagramObjects  map[string][]string
	objectEdges     map[string]map[string]int
	diagramEdges    map[string]int
	maxDepth        int
}

// AnalyzePlan evaluates the workspace against architectural best practices and
// returns grouped warnings based on the configured strictness level.
func AnalyzePlan(ws *workspace.Workspace) []WarningGroup {
	ctx := newWarningContext(ws)
	ctx.prepareData()
	ctx.checkAll()

	return ctx.toSlice()
}

func newWarningContext(ws *workspace.Workspace) *warningContext {
	level := 3
	allowLowInsight := false
	if ws.Config.Validation != nil {
		if ws.Config.Validation.Level > 0 {
			level = ws.Config.Validation.Level
		}
		allowLowInsight = ws.Config.Validation.AllowLowInsight
	}

	return &warningContext{
		ws:              ws,
		level:           level,
		allowLowInsight: allowLowInsight,
		warnings:        make(map[string]*WarningGroup),
		diagramObjects:  make(map[string][]string),
		objectEdges:     make(map[string]map[string]int),
		diagramEdges:    make(map[string]int),
	}
}

func (ctx *warningContext) addWarning(rule, desc, mediation, violation string) {
	if _, ok := ctx.warnings[rule]; !ok {
		ctx.warnings[rule] = &WarningGroup{
			RuleName:    rule,
			Description: desc,
			Mediation:   mediation,
			Violations:  []string{},
		}
	}
	ctx.warnings[rule].Violations = append(ctx.warnings[rule].Violations, violation)
}

func (ctx *warningContext) prepareData() {
	for objRef, o := range ctx.ws.Objects {
		for _, p := range o.Diagrams {
			ctx.diagramObjects[p.Diagram] = append(ctx.diagramObjects[p.Diagram], objRef)
		}
		ctx.objectEdges[objRef] = make(map[string]int)
	}

	for _, e := range ctx.ws.Edges {
		ctx.diagramEdges[e.Diagram]++
		if objectEdges, ok := ctx.objectEdges[e.SourceObject]; ok {
			objectEdges[e.Diagram]++
		}
		if objectEdges, ok := ctx.objectEdges[e.TargetObject]; ok {
			objectEdges[e.Diagram]++
		}
	}

	ctx.calculateMaxDepth()
}

func (ctx *warningContext) calculateMaxDepth() {
	for ref := range ctx.ws.Diagrams {
		depth := 0
		cur := ref
		visited := make(map[string]bool)
		for cur != "" {
			if visited[cur] {
				break
			}
			visited[cur] = true
			if d, ok := ctx.ws.Diagrams[cur]; ok {
				cur = d.ParentDiagram
				if cur != "" {
					depth++
				}
			} else {
				break
			}
		}
		if depth > ctx.maxDepth {
			ctx.maxDepth = depth
		}
	}
}

func (ctx *warningContext) checkAll() {
	if ctx.level >= 1 {
		ctx.checkLevel1()
	}
	if ctx.level >= 2 {
		ctx.checkLevel2()
	}
	if ctx.level >= 3 {
		ctx.checkLevel3()
	}
}

func (ctx *warningContext) checkLevel1() {
	if ctx.maxDepth < 1 && len(ctx.ws.Diagrams) > 1 {
		ctx.addWarning("Depth Mismatch", "Diagram hierarchy is flat", "Create sub-diagrams to establish a zoomable hierarchy.", "Workspace")
	}

	for diagRef, objs := range ctx.diagramObjects {
		densityLimit := 15
		if ctx.level >= 2 {
			densityLimit = 12
		}
		if len(objs) > densityLimit {
			ctx.addWarning("High Density", fmt.Sprintf("> %d objects on one diagram", densityLimit), "Split the diagram into sub-diagrams to reduce cognitive load.", fmt.Sprintf("Diagram %q has %d objects", diagRef, len(objs)))
		}

		if !ctx.allowLowInsight && len(objs) > 0 {
			edgesCount := ctx.diagramEdges[diagRef]
			if edgesCount < len(objs) {
				ctx.addWarning("Low Insight Ratio", "Edges < Objects", "Add more edges to illustrate how components interact, rather than just listing them.", fmt.Sprintf("Diagram %q (Objects: %d, Edges: %d)", diagRef, len(objs), edgesCount))
			}
		}
	}

	ctx.checkObjectsL1()

	for _, l := range ctx.ws.Links {
		if targetObjects := ctx.diagramObjects[l.ToDiagram]; len(targetObjects) == 0 {
			ctx.addWarning("Dead-End Drilldown", "Object has a link but no sub-diagram content", "Ensure the linked diagram exists and contains relevant details.", fmt.Sprintf("Link to Diagram %q", l.ToDiagram))
		}
	}
}

func (ctx *warningContext) checkObjectsL1() {
	for objRef, o := range ctx.ws.Objects {
		isRootLevel := false
		for _, p := range o.Diagrams {
			if d, ok := ctx.ws.Diagrams[p.Diagram]; ok && d.ParentDiagram == "" {
				isRootLevel = true
			}

			if ctx.objectEdges[objRef][p.Diagram] == 0 {
				if len(o.Diagrams) == 1 {
					ctx.addWarning("Isolated Object", "Object has 0 edges in a diagram", "explore its relationships further and link it using \"tld create link --from --to\" etc.", fmt.Sprintf("Object %q in Diagram %q", objRef, p.Diagram))
				} else {
					ctx.addWarning("Shared Context", "Shared object has no edges in sub-view", "Add edges to the shared object in this specific diagram or remove it from this view.", fmt.Sprintf("Object %q in Diagram %q", objRef, p.Diagram))
				}
			}
		}

		if isRootLevel {
			lowerType := strings.ToLower(o.Type)
			if lowerType == "function" || lowerType == "class" {
				ctx.addWarning("Abstraction Leak", "Implementation types at Root level", "Move functions and classes into sub-diagrams. Keep root views at the Service/Subsystem level.", fmt.Sprintf("Object %q (Type: %s)", objRef, o.Type))
			}
		}
	}
}

func (ctx *warningContext) checkLevel2() {
	for objRef, o := range ctx.ws.Objects {
		if o.Technology == "" {
			ctx.addWarning("Missing Tech", "No `technology` field", "Add a 'technology' field (e.g. Go, React) to clarify the stack.", fmt.Sprintf("Object %q", objRef))
		}
	}
	for edgeRef, e := range ctx.ws.Edges {
		if isGenericLabel(e.Label) {
			ctx.addWarning("Generic Labels", "Label is overly generic", "Replace generic labels with domain-specific verbs like 'validates JWT' or 'SQL Query'.", fmt.Sprintf("Edge %q in Diagram %q (Label: %q)", edgeRef, e.Diagram, e.Label))
		}
	}
}

func (ctx *warningContext) checkLevel3() {
	for diagRef, d := range ctx.ws.Diagrams {
		if d.Description == "" {
			ctx.addWarning("Missing Desc", "`description` field is empty", "Add a one-sentence summary to help readers understand the responsibility.", fmt.Sprintf("Diagram %q", diagRef))
		}
		if isGenericName(d.Name) {
			ctx.addWarning("Generic Naming", "Vague names make the map harder to understand", "Rename the element with a domain-specific, descriptive name.", fmt.Sprintf("Diagram %q (Name: %q)", diagRef, d.Name))
		}
	}
	for objRef, o := range ctx.ws.Objects {
		if o.Description == "" {
			ctx.addWarning("Missing Desc", "`description` field is empty", "Add a one-sentence summary to help readers understand the responsibility.", fmt.Sprintf("Object %q", objRef))
		}
		if isGenericName(o.Name) {
			ctx.addWarning("Generic Naming", "Vague names make the map harder to understand", "Rename the element with a domain-specific, descriptive name.", fmt.Sprintf("Object %q (Name: %q)", objRef, o.Name))
		}
	}
	for edgeRef, e := range ctx.ws.Edges {
		if e.Label == "" {
			ctx.addWarning("Missing Label", "Edge has no `label`", "An edge without a label is just a line. Add a 'label' field to tell what it does.", fmt.Sprintf("Edge %q in Diagram %q", edgeRef, e.Diagram))
		}
	}
}

func (ctx *warningContext) toSlice() []WarningGroup {
	var result []WarningGroup
	order := []string{
		"High Density", "Isolated Object", "Shared Context", "Depth Mismatch", "Low Insight Ratio", "Dead-End Drilldown", "Abstraction Leak",
		"Generic Labels", "Missing Tech",
		"Missing Desc", "Generic Naming", "Missing Label",
	}
	for _, rule := range order {
		if wg, ok := ctx.warnings[rule]; ok {
			result = append(result, *wg)
		}
	}
	return result
}

func isGenericName(name string) bool {
	lower := strings.ToLower(name)
	return strings.Contains(lower, "module") || strings.Contains(lower, "stuff") || strings.Contains(lower, "thing")
}

func isGenericLabel(label string) bool {
	lower := strings.ToLower(label)
	return lower == "calls" || lower == "uses" || lower == "connects" || lower == "links" || lower == ""
}
