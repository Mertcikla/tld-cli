package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newUpdateCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "update",
		Short: "Update a resource field with a value",
		RunE: func(cmd *cobra.Command, args []string) error {
			if len(args) == 0 {
				return cmd.Help()
			}
			return cobra.NoArgs(cmd, args)
		},
	}

	c.AddCommand(newUpdateElementCmd(wdir))
	c.AddCommand(newUpdateConnectorCmd(wdir))

	return c
}

func newUpdateElementCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "element <ref> <field> <value>",
		Short: "Update an element field",
		Args:  cobra.ExactArgs(3),
		RunE: func(cmd *cobra.Command, args []string) error {
			ref, field, value := args[0], args[1], args[2]
			if err := workspace.UpdateElementField(*wdir, ref, field, value); err != nil {
				return fmt.Errorf("update element: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated element %q: %s=%q\n", ref, field, value)
			return nil
		},
	}
}

func newUpdateConnectorCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "connector <ref> <field> <value>",
		Short: "Update a connector field",
		Args:  cobra.ExactArgs(3),
		RunE: func(cmd *cobra.Command, args []string) error {
			ref, field, value := args[0], args[1], args[2]
			if err := workspace.UpdateConnectorField(*wdir, ref, field, value); err != nil {
				return fmt.Errorf("update connector: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated connector %q: %s=%q\n", ref, field, value)
			return nil
		},
	}
}
