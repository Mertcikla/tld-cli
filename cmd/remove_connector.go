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
				if wantsJSONOutput() {
					return writeCommandJSONError(cmd.OutOrStdout(), "remove connector", err)
				}
				return fmt.Errorf("remove connector: %w", err)
			}
			if wantsJSONOutput() {
				return writeMutationJSONOutput(cmd.OutOrStdout(), "remove connector", "remove", fmt.Sprintf("%s:%s:%s", view, from, to))
			}
			if n == 0 {
				fmt.Fprintln(cmd.OutOrStdout(), "No matching connectors found - nothing removed.")
			} else {
				fmt.Fprintf(cmd.OutOrStdout(), "Removed %d connector(s) from connectors.yaml\n", n)
				fmt.Fprintln(cmd.OutOrStdout(), "Change recorded locally in connectors.yaml. Run 'tld apply' to push to cloud.")
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
