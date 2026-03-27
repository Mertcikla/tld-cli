// Package cmd implements the tld CLI command tree.
package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newAddLinkCmd(wdir *string) *cobra.Command {
	var (
		object      string
		fromDiagram string
		toDiagram   string
	)

	c := &cobra.Command{
		Use:   "link",
		Short: "Add a drill-down link between two diagrams (optionally anchored to an object)",
		RunE: func(cmd *cobra.Command, _ []string) error {
			spec := &workspace.Link{
				Object:      object,
				FromDiagram: fromDiagram,
				ToDiagram:   toDiagram,
			}
			if err := workspace.AppendLink(*wdir, spec); err != nil {
				return fmt.Errorf("append link: %w", err)
			}
			if object != "" {
				fmt.Fprintf(cmd.OutOrStdout(), "Appended link: object %s, %s -> %s to links.yaml\n",
					object, fromDiagram, toDiagram)
			} else {
				fmt.Fprintf(cmd.OutOrStdout(), "Appended link: %s -> %s to links.yaml\n",
					fromDiagram, toDiagram)
			}
			return nil
		},
	}

	c.Flags().StringVar(&object, "object", "", "object ref (optional)")
	c.Flags().StringVar(&fromDiagram, "from", "", "source diagram ref (required)")
	c.Flags().StringVar(&toDiagram, "to", "", "target diagram ref (required)")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
