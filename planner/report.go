package planner

import (
	"fmt"
	"io"
	"sort"
	"strings"

	"github.com/mertcikla/tld-cli/workspace"
)

// RenderPlanMarkdown writes a human-readable plan report to w.
func RenderPlanMarkdown(w io.Writer, plan *Plan, ws *workspace.Workspace, verbose bool) {
	renderElementPlanMarkdown(w, plan, ws, verbose)
}

func renderElementPlanMarkdown(w io.Writer, plan *Plan, ws *workspace.Workspace, verbose bool) {
	fmt.Fprintln(w, "# Element Plan")
	fmt.Fprintln(w)

	fmt.Fprintln(w, "## View Structure")
	fmt.Fprintln(w)
	renderElementTree(w, ws)
	fmt.Fprintln(w)

	viewCount := 0
	for _, element := range ws.Elements {
		if element.HasView {
			viewCount++
		}
	}

	fmt.Fprintln(w, "## Summary")
	fmt.Fprintln(w)
	fmt.Fprintln(w, "| Resource | Count |")
	fmt.Fprintln(w, "|----------|-------|")
	fmt.Fprintf(w, "| Elements | %d |\n", len(ws.Elements))
	fmt.Fprintf(w, "| Views    | %d |\n", viewCount)
	fmt.Fprintf(w, "| Connectors | %d |\n", len(ws.Connectors))
	fmt.Fprintln(w)

	if !verbose {
		fmt.Fprintln(w, "Use '-v' or '--verbose' for detailed element placement and connector reporting.")
		return
	}

	fmt.Fprintln(w, "## Elements")
	fmt.Fprintln(w)
	fmt.Fprintln(w, "| Ref | Name | Kind | Has View | Placements |")
	fmt.Fprintln(w, "|-----|------|------|----------|------------|")
	for ref, element := range ws.Elements {
		parents := make([]string, 0, len(element.Placements))
		for _, placement := range element.Placements {
			parents = append(parents, placement.ParentRef)
		}
		fmt.Fprintf(w, "| %s | %s | %s | %t | %s |\n", ref, element.Name, element.Kind, element.HasView, strings.Join(parents, ", "))
	}
	fmt.Fprintln(w)

	if len(ws.Connectors) > 0 {
		fmt.Fprintln(w, "## Connectors")
		fmt.Fprintln(w)
		fmt.Fprintln(w, "| View | Source -> Target | Label | Direction |")
		fmt.Fprintln(w, "|------|------------------|-------|-----------|")
		for _, connector := range ws.Connectors {
			direction := connector.Direction
			if direction == "" {
				direction = "forward"
			}
			fmt.Fprintf(w, "| %s | %s -> %s | %s | %s |\n", connector.View, connector.Source, connector.Target, connector.Label, direction)
		}
		fmt.Fprintln(w)
	}
}

func renderElementTree(w io.Writer, ws *workspace.Workspace) {
	children := make(map[string][]string)
	roots := []string{}
	for ref, element := range ws.Elements {
		if len(element.Placements) == 0 {
			roots = append(roots, ref)
			continue
		}
		rooted := false
		for _, placement := range element.Placements {
			if placement.ParentRef == "" || placement.ParentRef == "root" || placement.ParentRef == syntheticRootViewRef {
				rooted = true
				continue
			}
			children[placement.ParentRef] = append(children[placement.ParentRef], ref)
		}
		if rooted {
			roots = append(roots, ref)
		}
	}
	sort.Strings(roots)
	for parent := range children {
		sort.Strings(children[parent])
	}
	visited := make(map[string]bool)
	for _, root := range roots {
		printElementNode(w, ws, children, root, 0, visited)
	}
}

func printElementNode(w io.Writer, ws *workspace.Workspace, children map[string][]string, ref string, depth int, visited map[string]bool) {
	if visited[ref] {
		return
	}
	visited[ref] = true
	element := ws.Elements[ref]
	indent := strings.Repeat("  ", depth)
	viewSuffix := ""
	if element.HasView {
		viewSuffix = " [view]"
	}
	fmt.Fprintf(w, "%s- **%s**%s: %s\n", indent, ref, viewSuffix, element.Name)
	for _, child := range children[ref] {
		printElementNode(w, ws, children, child, depth+1, visited)
	}
}
