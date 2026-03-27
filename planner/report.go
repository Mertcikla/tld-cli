package planner

import (
	"fmt"
	"io"
	"strings"

	"github.com/mertcikla/tldiagram-cli/workspace"
)

// RenderPlanMarkdown writes a human-readable plan report to w.
func RenderPlanMarkdown(w io.Writer, plan *Plan, ws *workspace.Workspace) {
	fmt.Fprintln(w, "# Diagram Plan")
	fmt.Fprintln(w)

	// Diagram hierarchy
	fmt.Fprintln(w, "## Diagram Hierarchy")
	fmt.Fprintln(w)
	renderDiagramTree(w, ws, plan.DiagramOrder)
	fmt.Fprintln(w)

	fmt.Fprintln(w, "## Summary")
	fmt.Fprintln(w)
	fmt.Fprintln(w, "| Resource | Count |")
	fmt.Fprintln(w, "|----------|-------|")
	fmt.Fprintf(w, "| Diagrams | %d |\n", len(plan.Request.Diagrams))
	fmt.Fprintf(w, "| Objects  | %d |\n", len(plan.Request.Objects))
	fmt.Fprintf(w, "| Edges    | %d |\n", len(plan.Request.Edges))
	fmt.Fprintf(w, "| Links    | %d |\n", len(plan.Request.Links))
	fmt.Fprintln(w)

	// Objects per diagram
	fmt.Fprintln(w, "## Objects per Diagram")
	fmt.Fprintln(w)
	for _, diagRef := range plan.DiagramOrder {
		fmt.Fprintf(w, "### %s\n\n", diagRef)
		fmt.Fprintln(w, "| Ref | Name | Type | Position |")
		fmt.Fprintln(w, "|-----|------|------|----------|")
		for _, obj := range plan.Request.Objects {
			for _, p := range obj.Placements {
				if p.DiagramRef == diagRef {
					objType := ""
					if obj.Type != nil {
						objType = *obj.Type
					}
					fmt.Fprintf(w, "| %s | %s | %s | (%.0f, %.0f) |\n",
						obj.Ref, obj.Name, objType,
						p.GetPositionX(), p.GetPositionY())
				}
			}
		}
		fmt.Fprintln(w)
	}

	// Edges
	if len(plan.Request.Edges) > 0 {
		fmt.Fprintln(w, "## Edges")
		fmt.Fprintln(w)
		fmt.Fprintln(w, "| Diagram | Source -> Target | Label | Direction |")
		fmt.Fprintln(w, "|---------|-----------------|-------|-----------|")
		for _, e := range plan.Request.Edges {
			label := e.GetLabel()
			direction := e.GetDirection()
			if direction == "" {
				direction = "forward"
			}
			fmt.Fprintf(w, "| %s | %s -> %s | %s | %s |\n",
				e.DiagramRef, e.SourceObjectRef, e.TargetObjectRef, label, direction)
		}
		fmt.Fprintln(w)
	}

	// Links
	if len(plan.Request.Links) > 0 {
		fmt.Fprintln(w, "## Links")
		fmt.Fprintln(w)
		fmt.Fprintln(w, "| Object | From -> To |")
		fmt.Fprintln(w, "|--------|-----------|")
		for _, l := range plan.Request.Links {
			fmt.Fprintf(w, "| %s | %s -> %s |\n", l.ObjectRef, l.FromDiagramRef, l.ToDiagramRef)
		}
		fmt.Fprintln(w)
	}
}

func renderDiagramTree(w io.Writer, ws *workspace.Workspace, order []string) {
	// Build children map
	children := make(map[string][]string)
	roots := []string{}
	for _, ref := range order {
		d := ws.Diagrams[ref]
		if d.ParentDiagram == "" {
			roots = append(roots, ref)
		} else {
			children[d.ParentDiagram] = append(children[d.ParentDiagram], ref)
		}
	}
	for _, r := range roots {
		printDiagramNode(w, ws, children, r, 0)
	}
}

func printDiagramNode(w io.Writer, ws *workspace.Workspace, children map[string][]string, ref string, depth int) {
	d := ws.Diagrams[ref]
	indent := strings.Repeat("  ", depth)
	label := ""
	if d.LevelLabel != "" {
		label = " (" + d.LevelLabel + ")"
	}
	fmt.Fprintf(w, "%s- **%s**%s: %s\n", indent, ref, label, d.Name)
	for _, child := range children[ref] {
		printDiagramNode(w, ws, children, child, depth+1)
	}
}
