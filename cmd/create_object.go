package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newCreateObjectCmd(wdir *string) *cobra.Command {
	var (
		description string
		technology  string
		url         string
		positionX   float64
		positionY   float64
		ref         string
	)

	c := &cobra.Command{
		Use:   "object <diagram_ref> <name> <type>",
		Short: "Create a new object YAML file with a placement",
		Args:  cobra.ExactArgs(3),
		RunE: func(cmd *cobra.Command, args []string) error {
			diagramRef, name, objType := args[0], args[1], args[2]
			r := ref
			if r == "" {
				r = workspace.Slugify(name)
			}
			spec := &workspace.Object{
				Name:        name,
				Type:        objType,
				Description: description,
				Technology:  technology,
				URL:         url,
				Diagrams: []workspace.Placement{
					{
						Diagram:   diagramRef,
						PositionX: positionX,
						PositionY: positionY,
					},
				},
			}
			if err := workspace.UpsertObject(*wdir, r, spec); err != nil {
				return fmt.Errorf("upsert object: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated objects.yaml (upserted %s)\n", r)
			return nil
		},
	}

	c.Flags().StringVar(&description, "description", "", "description")
	c.Flags().StringVar(&technology, "technology", "", "primary technology")
	c.Flags().StringVar(&url, "url", "", "external URL")
	c.Flags().Float64Var(&positionX, "position-x", 0, "horizontal canvas position")
	c.Flags().Float64Var(&positionY, "position-y", 0, "vertical canvas position")
	c.Flags().StringVar(&ref, "ref", "", "override generated ref (default: slugified name)")
	return c
}
