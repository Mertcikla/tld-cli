package cmd

import "github.com/spf13/cobra"

func newCreateCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "create",
		Short: "Create workspace resources",
	}
	c.AddCommand(newCreateDiagramCmd(wdir))
	c.AddCommand(newCreateObjectCmd(wdir))
	c.AddCommand(newCreateLinkCmd(wdir))
	return c
}

func newConnectCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "connect",
		Short: "Connect resources in the workspace",
	}
	c.AddCommand(newConnectObjectsCmd(wdir))
	return c
}

func newRemoveCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "remove",
		Short: "Remove workspace resources",
	}
	c.AddCommand(newRemoveDiagramCmd(wdir))
	c.AddCommand(newRemoveObjectCmd(wdir))
	c.AddCommand(newRemoveEdgeCmd(wdir))
	c.AddCommand(newRemoveLinkCmd(wdir))
	return c
}
