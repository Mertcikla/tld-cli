package cmd

import "github.com/spf13/cobra"

func newAddCmd(wdir *string) *cobra.Command {
	c := newCreateElementCmd(wdir)
	c.Use = "add <name>"
	c.Short = "Add or update an element in elements.yaml"
	return c
}

func newConnectCmd(wdir *string) *cobra.Command {
	c := newConnectElementsCmd(wdir)
	c.Use = "connect"
	c.Short = "Add a connector between two elements"
	return c
}

func newRemoveCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "remove",
		Short: "Remove workspace resources",
		RunE: func(cmd *cobra.Command, args []string) error {
			if len(args) == 0 {
				return cmd.Help()
			}
			return cobra.NoArgs(cmd, args)
		},
	}
	c.AddCommand(newRemoveElementCmd(wdir))
	c.AddCommand(newRemoveConnectorCmd(wdir))
	return c
}
