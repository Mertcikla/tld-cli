package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newConnectElementsCmd(wdir *string) *cobra.Command {
	var (
		view         string
		from         string
		to           string
		label        string
		description  string
		relationship string
		direction    string
		style        string
		url          string
	)

	c := &cobra.Command{
		Use:   "elements",
		Short: "Add a connector between two elements inside a parent element view",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			spec := &workspace.Connector{
				View:         view,
				Source:       from,
				Target:       to,
				Label:        label,
				Description:  description,
				Relationship: relationship,
				Direction:    direction,
				Style:        style,
				URL:          url,
			}
			if err := workspace.AppendConnector(*wdir, spec); err != nil {
				return fmt.Errorf("append connector: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Appended connector %s -> %s in view %s to connectors.yaml\n", from, to, view)
			return nil
		},
	}

	c.Flags().StringVar(&view, "view", "", "owner element ref of the view (required)")
	c.Flags().StringVar(&from, "from", "", "source element ref (required)")
	c.Flags().StringVar(&to, "to", "", "target element ref (required)")
	c.Flags().StringVar(&label, "label", "", "connector label")
	c.Flags().StringVar(&description, "description", "", "connector description")
	c.Flags().StringVar(&relationship, "relationship", "", "semantic relationship type")
	c.Flags().StringVar(&direction, "direction", "forward", "forward|backward|both|none")
	c.Flags().StringVar(&style, "style", "bezier", "bezier|straight|step|smoothstep")
	c.Flags().StringVar(&url, "url", "", "external URL")
	_ = c.MarkFlagRequired("view")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
