package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newCreateElementCmd(wdir *string) *cobra.Command {
	var (
		description     string
		technology      string
		url             string
		positionX       float64
		positionY       float64
		ref             string
		kind            string
		parent          string
		diagramLabel    string
		legacyViewLabel string
		legacyWithView  bool
	)

	c := &cobra.Command{
		Use:   "element <name>",
		Short: "Create or update an element in elements.yaml",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			name := args[0]
			r := ref
			if r == "" {
				r = workspace.Slugify(name)
			}
			placementParent := parent
			if placementParent == "" {
				placementParent = "root"
			}
			if diagramLabel == "" {
				diagramLabel = legacyViewLabel
			}
			_ = legacyWithView
			spec := &workspace.Element{
				Name:        name,
				Kind:        kind,
				Description: description,
				Technology:  technology,
				URL:         url,
				HasView:     true,
				ViewLabel:   diagramLabel,
				Placements: []workspace.ViewPlacement{{
					ParentRef: placementParent,
					PositionX: positionX,
					PositionY: positionY,
				}},
			}
			if err := workspace.UpsertElement(*wdir, r, spec); err != nil {
				if wantsJSONOutput() {
					return writeCommandJSONError(cmd.OutOrStdout(), "add", err)
				}
				return fmt.Errorf("upsert element: %w", err)
			}
			if wantsJSONOutput() {
				return writeMutationJSONOutput(cmd.OutOrStdout(), "add", "add", r)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated elements.yaml (upserted %s)\n", r)
			fmt.Fprintln(cmd.OutOrStdout(), "Change recorded locally in elements.yaml. Run 'tld apply' to push to cloud.")
			return nil
		},
	}

	c.Flags().StringVar(&kind, "kind", "service", "element kind")
	c.Flags().StringVar(&description, "description", "", "description")
	c.Flags().StringVar(&technology, "technology", "", "primary technology")
	c.Flags().StringVar(&url, "url", "", "external URL")
	c.Flags().Float64Var(&positionX, "position-x", 0, "horizontal canvas position")
	c.Flags().Float64Var(&positionY, "position-y", 0, "vertical canvas position")
	c.Flags().StringVar(&ref, "ref", "", "override generated ref (default: slugified name)")
	c.Flags().StringVar(&parent, "parent", "root", "parent element ref or root")
	c.Flags().StringVar(&diagramLabel, "diagram-label", "", "optional label for the element's canonical diagram")
	c.Flags().BoolVar(&legacyWithView, "with-view", false, "deprecated")
	c.Flags().StringVar(&legacyViewLabel, "view-label", "", "deprecated")
	_ = c.Flags().MarkHidden("with-view")
	_ = c.Flags().MarkHidden("view-label")
	return c
}
