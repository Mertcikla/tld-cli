package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newRemoveConnectorCmd(wdir *string) *cobra.Command {
	var (
		view string
		from string
		to   string
	)

	c := &cobra.Command{
		Use:   "connector",
		Short: "Remove matching connector(s) from connectors.yaml",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			n, err := workspace.RemoveConnector(*wdir, view, from, to)
			if err != nil {
				return fmt.Errorf("remove connector: %w", err)
			}
			if n == 0 {
				fmt.Fprintln(cmd.OutOrStdout(), "No matching connectors found - nothing removed.")
			} else {
				fmt.Fprintf(cmd.OutOrStdout(), "Removed %d connector(s) from connectors.yaml\n", n)
			}
			return nil
		},
	}

	c.Flags().StringVar(&view, "view", "", "view ref (required)")
	c.Flags().StringVar(&from, "from", "", "source element ref (required)")
	c.Flags().StringVar(&to, "to", "", "target element ref (required)")
	_ = c.MarkFlagRequired("view")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
