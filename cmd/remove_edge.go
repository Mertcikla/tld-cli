package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newRemoveEdgeCmd(wdir *string) *cobra.Command {
	var (
		diagram string
		from    string
		to      string
	)

	c := &cobra.Command{
		Use:   "edge",
		Short: "Remove matching edge(s) from edges.yaml",
		RunE: func(cmd *cobra.Command, _ []string) error {
			n, err := workspace.RemoveEdge(*wdir, diagram, from, to)
			if err != nil {
				return fmt.Errorf("remove edge: %w", err)
			}
			if n == 0 {
				fmt.Fprintln(cmd.OutOrStdout(), "No matching edges found — nothing removed.")
			} else {
				fmt.Fprintf(cmd.OutOrStdout(), "Removed %d edge(s) from edges.yaml\n", n)
			}
			return nil
		},
	}

	c.Flags().StringVar(&diagram, "diagram", "", "diagram ref (required)")
	c.Flags().StringVar(&from, "from", "", "source object ref (required)")
	c.Flags().StringVar(&to, "to", "", "target object ref (required)")
	_ = c.MarkFlagRequired("diagram")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
