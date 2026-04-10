package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newRemoveElementCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "element <ref>",
		Short: "Remove an element from elements.yaml",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			ref := args[0]
			if err := workspace.RemoveElement(*wdir, ref); err != nil {
				return fmt.Errorf("remove element: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Removed %s from elements.yaml\n", ref)
			return nil
		},
	}
}
