package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newRenameCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "rename",
		Short: "Rename resource references and cascade changes",
		RunE: func(cmd *cobra.Command, args []string) error {
			if len(args) == 0 {
				return cmd.Help()
			}
			return cobra.NoArgs(cmd, args)
		},
	}

	c.AddCommand(newRenameElementCmd(wdir))
	c.AddCommand(newRenameConnectorCmd(wdir))

	return c
}

func newRenameElementCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "element <old-ref> <new-ref>",
		Short: "Rename an element reference and all its usages",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			oldRef, newRef := args[0], args[1]
			if err := workspace.RenameElement(*wdir, oldRef, newRef); err != nil {
				return fmt.Errorf("rename element: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Renamed element %q to %q and updated all references.\n", oldRef, newRef)
			return nil
		},
	}
}

func newRenameConnectorCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "connector <old-ref> <new-ref>",
		Short: "Rename a connector reference",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			oldRef, newRef := args[0], args[1]
			if err := workspace.RenameConnector(*wdir, oldRef, newRef); err != nil {
				return fmt.Errorf("rename connector: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Renamed connector %q to %q.\n", oldRef, newRef)
			return nil
		},
	}
}
