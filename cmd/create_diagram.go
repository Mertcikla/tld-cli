package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newCreateDiagramCmd(wdir *string) *cobra.Command {
	var (
		description string
		levelLabel  string
		parent      string
		ref         string
	)

	c := &cobra.Command{
		Use:   "diagram <name>",
		Short: "Create a new diagram YAML file",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			name := args[0]
			r := ref
			if r == "" {
				r = workspace.Slugify(name)
			}
			spec := &workspace.Diagram{
				Name:          name,
				Description:   description,
				LevelLabel:    levelLabel,
				ParentDiagram: parent,
			}
			if err := workspace.WriteDiagram(*wdir, r, spec); err != nil {
				return fmt.Errorf("write diagram: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated diagrams.yaml (added %s)\n", r)
			return nil
		},
	}

	c.Flags().StringVar(&description, "description", "", "description")
	c.Flags().StringVar(&levelLabel, "level-label", "", "abstraction level label (e.g. System, Container)")
	c.Flags().StringVar(&parent, "parent", "", "parent diagram ref")
	c.Flags().StringVar(&ref, "ref", "", "override generated ref (default: slugified name)")
	return c
}
