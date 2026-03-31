package cmd

import (
	"fmt"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newUpdateCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "update",
		Short: "Update workspace resources (local YAML only)",
		Long: `Update modifies existing resources in your local YAML files.
Run 'tld apply' after updating to sync these changes with the server.`,
	}
	c.AddCommand(
		newUpdateObjectCmd(wdir),
		newUpdateDiagramCmd(wdir),
		newUpdateEdgeCmd(wdir),
	)
	return c
}

func newUpdateObjectCmd(wdir *string) *cobra.Command {
	var (
		name        string
		objType     string
		description string
		technology  string
		url         string
	)

	c := &cobra.Command{
		Use:   "object <ref>",
		Short: "Update an object's properties in objects.yaml",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			ref := args[0]

			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			obj, ok := ws.Objects[ref]
			if !ok {
				return fmt.Errorf("object %q not found in workspace", ref)
			}

			// Update fields if flags are provided
			if name != "" {
				obj.Name = name
			}
			if objType != "" {
				obj.Type = objType
			}
			if description != "" {
				obj.Description = description
			}
			if technology != "" {
				obj.Technology = technology
			}
			if url != "" {
				obj.URL = url
			}

			// Update local YAML
			if err := workspace.UpdateObject(*wdir, ref, obj); err != nil {
				return fmt.Errorf("update object: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated %s in objects.yaml\n", ref)
			fmt.Fprintln(cmd.OutOrStdout(), "  hint: run 'tld apply' to sync changes with the server")

			return nil
		},
	}

	c.Flags().StringVar(&name, "name", "", "new name")
	c.Flags().StringVar(&objType, "type", "", "new type")
	c.Flags().StringVar(&description, "description", "", "new description")
	c.Flags().StringVar(&technology, "technology", "", "new primary technology")
	c.Flags().StringVar(&url, "url", "", "new external URL")
	return c
}

func newUpdateDiagramCmd(wdir *string) *cobra.Command {
	var (
		name        string
		description string
		levelLabel  string
	)

	c := &cobra.Command{
		Use:   "diagram <ref>",
		Short: "Update a diagram's properties in diagrams.yaml",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			ref := args[0]

			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			diag, ok := ws.Diagrams[ref]
			if !ok {
				return fmt.Errorf("diagram %q not found in workspace", ref)
			}

			// Update fields if flags are provided
			if name != "" {
				diag.Name = name
			}
			if description != "" {
				diag.Description = description
			}
			if levelLabel != "" {
				diag.LevelLabel = levelLabel
			}

			// Update local YAML
			if err := workspace.UpdateDiagram(*wdir, ref, diag); err != nil {
				return fmt.Errorf("update diagram: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated %s in diagrams.yaml\n", ref)
			fmt.Fprintln(cmd.OutOrStdout(), "  hint: run 'tld apply' to sync changes with the server")

			return nil
		},
	}

	c.Flags().StringVar(&name, "name", "", "new name")
	c.Flags().StringVar(&description, "description", "", "new description")
	c.Flags().StringVar(&levelLabel, "level-label", "", "new level label")
	return c
}

func newUpdateEdgeCmd(wdir *string) *cobra.Command {
	var (
		diagram     string
		from        string
		to          string
		label       string
		newLabel    string
		description string
		direction   string
		edgeType    string
	)

	c := &cobra.Command{
		Use:   "edge",
		Short: "Update an edge's properties in edges.yaml",
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			// Identify edge in local workspace
			key := fmt.Sprintf("%s:%s:%s:%s", diagram, from, to, label)
			edge, ok := ws.Edges[key]
			if !ok {
				return fmt.Errorf("edge not found in workspace: %s", key)
			}

			// Update fields
			if newLabel != "" {
				edge.Label = newLabel
			}
			if description != "" {
				edge.Description = description
			}
			if direction != "" {
				edge.Direction = direction
			}
			if edgeType != "" {
				edge.EdgeType = edgeType
			}

			// Update local YAML
			if err := workspace.UpdateEdge(*wdir, key, edge); err != nil {
				return fmt.Errorf("update edge: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Updated edge in edges.yaml\n")
			fmt.Fprintln(cmd.OutOrStdout(), "  hint: run 'tld apply' to sync changes with the server")

			return nil
		},
	}

	c.Flags().StringVar(&diagram, "diagram", "", "diagram ref (required)")
	c.Flags().StringVar(&from, "from", "", "source object ref (required)")
	c.Flags().StringVar(&to, "to", "", "target object ref (required)")
	c.Flags().StringVar(&label, "label", "", "current label (required if multiple edges exist)")
	c.Flags().StringVar(&newLabel, "new-label", "", "new label")
	c.Flags().StringVar(&description, "description", "", "new description")
	c.Flags().StringVar(&direction, "direction", "", "new direction (forward, backward, both, none)")
	c.Flags().StringVar(&edgeType, "edge-type", "", "new edge type (bezier, straight, step, smoothstep)")

	_ = c.MarkFlagRequired("diagram")
	_ = c.MarkFlagRequired("from")
	_ = c.MarkFlagRequired("to")

	return c
}
