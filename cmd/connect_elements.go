package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func elementParentRef(element *workspace.Element) (string, error) {
	if element == nil {
		return "", fmt.Errorf("element is required")
	}
	if len(element.Placements) != 1 {
		return "", fmt.Errorf("element %q must have exactly 1 placement, got %d", element.Name, len(element.Placements))
	}
	parentRef := element.Placements[0].ParentRef
	if parentRef == "" {
		parentRef = "root"
	}
	return parentRef, nil
}

func inferConnectorView(ws *workspace.Workspace, from, to string) (string, error) {
	if ws == nil {
		return "", fmt.Errorf("workspace is required")
	}
	fromElement, ok := ws.Elements[from]
	if !ok {
		return "", fmt.Errorf("source element %q not found", from)
	}
	toElement, ok := ws.Elements[to]
	if !ok {
		return "", fmt.Errorf("target element %q not found", to)
	}
	fromParent, err := elementParentRef(fromElement)
	if err != nil {
		return "", fmt.Errorf("source element %q: %w", from, err)
	}
	toParent, err := elementParentRef(toElement)
	if err != nil {
		return "", fmt.Errorf("target element %q: %w", to, err)
	}
	if fromParent != toParent {
		return "", fmt.Errorf("elements %q and %q must share the same parent diagram (got %q and %q)", from, to, fromParent, toParent)
	}
	return fromParent, nil
}

func newConnectElementsCmd(wdir *string) *cobra.Command {
	var (
		from         string
		to           string
		label        string
		description  string
		relationship string
		direction    string
		style        string
		url          string
		legacyView   string
	)

	c := &cobra.Command{
		Use:   "elements",
		Short: "Add a connector between two elements; owner diagram is inferred from their shared parent",
		Args:  cobra.NoArgs,
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}
			view := legacyView
			if view == "" {
				view, err = inferConnectorView(ws, from, to)
				if err != nil {
					return err
				}
			}
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

	c.Flags().StringVar(&from, "from", "", "source element ref (required)")
	c.Flags().StringVar(&to, "to", "", "target element ref (required)")
	c.Flags().StringVar(&label, "label", "", "connector label")
	c.Flags().StringVar(&description, "description", "", "connector description")
	c.Flags().StringVar(&relationship, "relationship", "", "semantic relationship type")
	c.Flags().StringVar(&direction, "direction", "forward", "forward|backward|both|none")
	c.Flags().StringVar(&style, "style", "bezier", "bezier|straight|step|smoothstep")
	c.Flags().StringVar(&url, "url", "", "external URL")
	c.Flags().StringVar(&legacyView, "view", "", "deprecated")
	_ = c.Flags().MarkHidden("view")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")
	return c
}
