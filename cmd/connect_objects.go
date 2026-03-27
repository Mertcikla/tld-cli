package cmd

import (
	"fmt"

	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
)

func newConnectObjectsCmd(wdir *string) *cobra.Command {
	var (
		from             string
		to               string
		label            string
		description      string
		relationshipType string
		direction        string
		edgeType         string
		url              string
	)

	c := &cobra.Command{
		Use:   "objects <diagram_ref>",
		Short: "Add an edge between two objects on a diagram",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			diagramRef := args[0]
			spec := &workspace.Edge{
				Diagram:          diagramRef,
				SourceObject:     from,
				TargetObject:     to,
				Label:            label,
				Description:      description,
				RelationshipType: relationshipType,
				Direction:        direction,
				EdgeType:         edgeType,
				URL:              url,
			}
			if err := workspace.AppendEdge(*wdir, spec); err != nil {
				return fmt.Errorf("append edge: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Appended edge %s -> %s on diagram %s to edges.yaml\n",
				from, to, diagramRef)
			return nil
		},
	}

	c.Flags().StringVar(&from, "from", "", "source object ref (required)")
	c.Flags().StringVar(&to, "to", "", "target object ref (required)")
	c.Flags().StringVar(&label, "label", "", "edge label")
	c.Flags().StringVar(&description, "description", "", "edge description")
	c.Flags().StringVar(&relationshipType, "relationship-type", "", "semantic relationship type")
	c.Flags().StringVar(&direction, "direction", "forward", "forward|backward|both|none")
	c.Flags().StringVar(&edgeType, "edge-type", "bezier", "bezier|straight|step|smoothstep")
	c.Flags().StringVar(&url, "url", "", "external URL")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
