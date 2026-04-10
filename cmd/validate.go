package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newValidateCmd(wdir *string) *cobra.Command {
	var strictness int

	c := &cobra.Command{
		Use:   "validate",
		Short: "Validate the workspace YAML files",
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			// Override strictness if flag is set
			if strictness > 0 {
				if ws.Config.Validation == nil {
					ws.Config.Validation = &workspace.ValidationConfig{}
				}
				ws.Config.Validation.Level = strictness
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
			if len(ws.Elements) > 0 || len(ws.Connectors) > 0 {
				viewCount := 0
				for _, element := range ws.Elements {
					if element.HasView {
						viewCount++
					}
				}
				fmt.Fprintf(cmd.OutOrStdout(), "Element workspace: %d elements, %d views, %d connectors\n", len(ws.Elements), viewCount, len(ws.Connectors))
			}

			// Evaluate Diagram warnings
			warnings := planner.AnalyzePlan(ws)
			if len(warnings) > 0 {
				level := 3
				if ws.Config.Validation != nil && ws.Config.Validation.Level > 0 {
					level = ws.Config.Validation.Level
				}
				levelNames := map[int]string{1: "Minimal", 2: "Standard", 3: "Strict"}
				fmt.Fprintf(cmd.OutOrStdout(), "\n⚠️  Architectural Warnings (Level %d: %s)\n\n", level, levelNames[level])
				for _, wg := range warnings {
					fmt.Fprintf(cmd.OutOrStdout(), "[%s]\n%s\n", wg.RuleName, wg.Mediation)
					for _, v := range wg.Violations {
						fmt.Fprintf(cmd.OutOrStdout(), "  * %s\n", v)
					}
					fmt.Fprintln(cmd.OutOrStdout())
				}
			}

			return nil
		},
	}

	c.Flags().IntVar(&strictness, "strictness", 0, "override validation strictness level [1-3]")
	return c
}
