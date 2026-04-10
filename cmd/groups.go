package cmd

import "github.com/spf13/cobra"

func newCreateCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "create",
		Short: "Create workspace resources",
		RunE: func(cmd *cobra.Command, args []string) error {
			if len(args) == 0 {
				return cmd.Help()
			}
			return cobra.NoArgs(cmd, args)
		},
	}
	c.AddCommand(newCreateElementCmd(wdir))
	c.AddCommand(newCreateLinkCmd(wdir))
	return c
}

func newConnectCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "connect",
		Short: "Connect resources in the workspace",
	}
	c.AddCommand(newConnectElementsCmd(wdir))
	c.AddCommand(newConnectObjectsCmd(wdir))
	return c
}

func newRemoveCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "remove",
		Short: "Remove workspace resources",
	}
	c.AddCommand(newRemoveElementCmd(wdir))
	c.AddCommand(newRemoveConnectorCmd(wdir))
	c.AddCommand(newRemoveDiagramCmd(wdir))
	c.AddCommand(newRemoveObjectCmd(wdir))
	c.AddCommand(newRemoveEdgeCmd(wdir))
	c.AddCommand(newRemoveLinkCmd(wdir))
	return c
}
