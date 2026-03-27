package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newValidateCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "validate",
		Short: "Validate the workspace YAML files",
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}
			errs := ws.Validate()
			if len(errs) > 0 {
				fmt.Fprintln(cmd.ErrOrStderr(), "Validation errors:")
				for _, e := range errs {
					fmt.Fprintf(cmd.ErrOrStderr(), "  - %s\n", e)
				}
				return fmt.Errorf("%d validation error(s)", len(errs))
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Workspace valid: %d diagrams, %d objects, %d edges, %d links\n",
				len(ws.Diagrams), len(ws.Objects), len(ws.Edges), len(ws.Links))
			return nil
		},
	}
}
