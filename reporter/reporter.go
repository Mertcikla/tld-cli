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
		fmt.Fprintf(w, "| Elements | %d | %d |\n", s.ElementsPlanned, s.ElementsCreated)
		fmt.Fprintf(w, "| Diagrams | %d | %d |\n", s.DiagramsPlanned, s.DiagramsCreated)
		fmt.Fprintf(w, "| Connectors | %d | %d |\n", s.ConnectorsPlanned, s.ConnectorsCreated)
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

		if len(resp.CreatedElements) > 0 {
			fmt.Fprintln(w, "### Elements")
			fmt.Fprintln(w, "| ID | Name | Kind |")
			fmt.Fprintln(w, "|----|------|------|")
			for _, o := range resp.CreatedElements {
				kind := ""
				if o.Kind != nil {
					kind = *o.Kind
				}
				fmt.Fprintf(w, "| %d | %s | %s |\n", o.Id, o.Name, kind)
			}
			fmt.Fprintln(w)
		}

		if len(resp.CreatedConnectors) > 0 {
			fmt.Fprintln(w, "### Connectors")
			fmt.Fprintln(w, "| ID | Source -> Target |")
			fmt.Fprintln(w, "|----|-----------------|")
			for _, e := range resp.CreatedConnectors {
				fmt.Fprintf(w, "| %d | %d -> %d |\n", e.Id, e.SourceElementId, e.TargetElementId)
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
