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
	}

	c.AddCommand(newRenameDiagramCmd(wdir))
	c.AddCommand(newRenameObjectCmd(wdir))

	return c
}

func newRenameDiagramCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "diagram <old-ref> <new-ref>",
		Short: "Rename a diagram reference and all its usages",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			oldRef, newRef := args[0], args[1]
			if err := workspace.RenameDiagram(*wdir, oldRef, newRef); err != nil {
				return err
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Renamed diagram %q to %q and updated all references.\n", oldRef, newRef)
			return nil
		},
	}
}

func newRenameObjectCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "object <old-ref> <new-ref>",
		Short: "Rename an object reference and all its usages",
		Args:  cobra.ExactArgs(2),
		RunE: func(cmd *cobra.Command, args []string) error {
			oldRef, newRef := args[0], args[1]
			if err := workspace.RenameObject(*wdir, oldRef, newRef); err != nil {
				return err
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Renamed object %q to %q and updated all references.\n", oldRef, newRef)
			return nil
		},
	}
}
