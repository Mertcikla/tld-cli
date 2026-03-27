// Package reporter renders execution result summaries for apply operations.
package reporter

import (
	"fmt"
	"io"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/planner"
)

// RenderExecutionMarkdown writes an apply execution report comparing plan vs result.
func RenderExecutionMarkdown(w io.Writer, _ *planner.Plan, resp *diagv1.ApplyPlanResponse, success bool, verbose bool) {
	status := "SUCCESS"
	if !success {
		status = "ROLLED BACK"
	}

	fmt.Fprintf(w, "# Apply Report\n\n")
	fmt.Fprintf(w, "## Status: %s\n\n", status)

	if resp == nil {
		return
	}

	s := resp.GetSummary()
	if s != nil {
		fmt.Fprintln(w, "## Planned vs Created")
		fmt.Fprintln(w)
		fmt.Fprintln(w, "| Resource | Planned | Created |")
		fmt.Fprintln(w, "|----------|---------|---------|")
		fmt.Fprintf(w, "| Diagrams | %d | %d |\n", s.DiagramsPlanned, s.DiagramsCreated)
		fmt.Fprintf(w, "| Objects  | %d | %d |\n", s.ObjectsPlanned, s.ObjectsCreated)
		fmt.Fprintf(w, "| Edges    | %d | %d |\n", s.EdgesPlanned, s.EdgesCreated)
		fmt.Fprintf(w, "| Links    | %d | %d |\n", s.LinksPlanned, s.LinksCreated)
		fmt.Fprintln(w)
	}

	if success && verbose {
		fmt.Fprintln(w, "## Created Resources")
		fmt.Fprintln(w)

		if len(resp.CreatedDiagrams) > 0 {
			fmt.Fprintln(w, "### Diagrams")
			fmt.Fprintln(w, "| ID | Name |")
			fmt.Fprintln(w, "|----|------|")
			for _, d := range resp.CreatedDiagrams {
				fmt.Fprintf(w, "| %d | %s |\n", d.Id, d.Name)
			}
			fmt.Fprintln(w)
		}

		if len(resp.CreatedObjects) > 0 {
			fmt.Fprintln(w, "### Objects")
			fmt.Fprintln(w, "| ID | Name | Type |")
			fmt.Fprintln(w, "|----|------|------|")
			for _, o := range resp.CreatedObjects {
				objType := ""
				if o.Type != nil {
					objType = *o.Type
				}
				fmt.Fprintf(w, "| %d | %s | %s |\n", o.Id, o.Name, objType)
			}
			fmt.Fprintln(w)
		}

		if len(resp.CreatedEdges) > 0 {
			fmt.Fprintln(w, "### Edges")
			fmt.Fprintln(w, "| ID | Source -> Target |")
			fmt.Fprintln(w, "|----|-----------------|")
			for _, e := range resp.CreatedEdges {
				fmt.Fprintf(w, "| %d | %d -> %d |\n", e.Id, e.SourceObjectId, e.TargetObjectId)
			}
			fmt.Fprintln(w)
		}

		if len(resp.CreatedLinks) > 0 {
			fmt.Fprintln(w, "### Links")
			fmt.Fprintln(w, "| ID | Object | From -> To |")
			fmt.Fprintln(w, "|----|--------|-----------|")
			for _, l := range resp.CreatedLinks {
				fmt.Fprintf(w, "| %d | %d | %d -> %d |\n", l.Id, l.ObjectId, l.FromDiagramId, l.ToDiagramId)
			}
			fmt.Fprintln(w)
		}
	}

	if len(resp.Drift) > 0 {
		fmt.Fprintln(w, "## Drift")
		fmt.Fprintln(w)
		fmt.Fprintln(w, "| Resource | Ref | Reason |")
		fmt.Fprintln(w, "|----------|-----|--------|")
		for _, d := range resp.Drift {
			fmt.Fprintf(w, "| %s | %s | %s |\n", d.ResourceType, d.Ref, d.Reason)
		}
		fmt.Fprintln(w)
	}
}
