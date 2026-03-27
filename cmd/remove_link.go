package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newRemoveLinkCmd(wdir *string) *cobra.Command {
	var (
		object      string
		fromDiagram string
		toDiagram   string
	)

	c := &cobra.Command{
		Use:   "link",
		Short: "Remove matching link(s) from links.yaml",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			n, err := workspace.RemoveLink(*wdir, object, fromDiagram, toDiagram)
			if err != nil {
				return fmt.Errorf("remove link: %w", err)
			}
			if n == 0 {
				fmt.Fprintln(cmd.OutOrStdout(), "No matching links found — nothing removed.")
			} else {
				fmt.Fprintf(cmd.OutOrStdout(), "Removed %d link(s) from links.yaml\n", n)
			}
			return nil
		},
	}

	c.Flags().StringVar(&object, "object", "", "object ref (optional)")
	c.Flags().StringVar(&fromDiagram, "from", "", "from diagram ref (required)")
	c.Flags().StringVar(&toDiagram, "to", "", "to diagram ref (required)")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
