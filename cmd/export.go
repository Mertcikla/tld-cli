package cmd

import (
	"fmt"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newExportCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "export [org-id]",
		Short: "Export all diagrams from an organization to the local workspace",
		Args:  cobra.MaximumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w (did you run 'tld init'?)", err)
			}

			targetOrg := ws.Config.OrgID
			if len(args) > 0 {
				targetOrg = args[0]
			}
			if targetOrg == "" {
				return fmt.Errorf("org-id required (either as argument or in .tld.yaml)")
			}

			c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
			resp, err := c.ExportOrganization(cmd.Context(), connect.NewRequest(&diagv1.ExportOrganizationRequest{
				OrgId: targetOrg,
			}))
			if err != nil {
				return fmt.Errorf("export failed: %w", err)
			}

			// Convert ExportOrganizationResponse to workspace types
			newWS := &workspace.Workspace{
				Dir:      *wdir,
				Config:   ws.Config,
				Diagrams: make(map[string]*workspace.Diagram),
				Objects:  make(map[string]*workspace.Object),
				Meta: &workspace.Meta{
					Diagrams: make(map[string]*workspace.ResourceMetadata),
					Objects:  make(map[string]*workspace.ResourceMetadata),
				},
			}

			// Map ID to Name for referencing
			diagramIDToName := make(map[int32]string)
			for _, d := range resp.Msg.Diagrams {
				ref := d.Name // Use name as ref
				diagramIDToName[d.Id] = ref
				newWS.Diagrams[ref] = &workspace.Diagram{
					Name:        d.Name,
					Description: d.GetDescription(),
					LevelLabel:  d.GetLevelLabel(),
				}
				newWS.Meta.Diagrams[ref] = &workspace.ResourceMetadata{
					ID:        workspace.ResourceID(d.Id),
					UpdatedAt: d.UpdatedAt.AsTime(),
				}
			}

			// Fix parent diagrams refs
			for _, d := range resp.Msg.Diagrams {
				if d.ParentDiagramId != nil && *d.ParentDiagramId != 0 {
					if parentRef, ok := diagramIDToName[*d.ParentDiagramId]; ok {
						newWS.Diagrams[d.Name].ParentDiagram = parentRef
					}
				}
			}

			objectIDToName := make(map[int32]string)
			for _, o := range resp.Msg.Objects {
				ref := o.Name
				objectIDToName[o.Id] = ref
				objType := ""
				if o.Type != nil {
					objType = *o.Type
				}
				newWS.Objects[ref] = &workspace.Object{
					Name:        o.Name,
					Type:        objType,
					Description: o.GetDescription(),
					Technology:  o.GetTechnology(),
					URL:         o.GetUrl(),
					LogoURL:     o.GetLogoUrl(),
				}
				newWS.Meta.Objects[ref] = &workspace.ResourceMetadata{
					ID:        workspace.ResourceID(o.Id),
					UpdatedAt: o.UpdatedAt.AsTime(),
				}
			}

			// Add placements to objects
			for _, p := range resp.Msg.Placements {
				objRef, ok1 := objectIDToName[p.ObjectId]
				diagRef, ok2 := diagramIDToName[p.DiagramId]
				if ok1 && ok2 {
					newWS.Objects[objRef].Diagrams = append(newWS.Objects[objRef].Diagrams, workspace.Placement{
						Diagram:   diagRef,
						PositionX: p.PositionX,
						PositionY: p.PositionY,
					})
				}
			}

			// Convert edges
			for _, e := range resp.Msg.Edges {
				diagRef, ok1 := diagramIDToName[e.DiagramId]
				srcRef, ok2 := objectIDToName[e.SourceObjectId]
				tgtRef, ok3 := objectIDToName[e.TargetObjectId]
				if ok1 && ok2 && ok3 {
					newWS.Edges = append(newWS.Edges, workspace.Edge{
						Diagram:          diagRef,
						SourceObject:     srcRef,
						TargetObject:     tgtRef,
						Label:            e.GetLabel(),
						Description:      e.GetDescription(),
						RelationshipType: e.GetRelationshipType(),
						Direction:        e.Direction,
						EdgeType:         e.EdgeType,
						URL:              e.GetUrl(),
						SourceHandle:     e.GetSourceHandle(),
						TargetHandle:     e.GetTargetHandle(),
					})
				}
			}

			// Convert links
			for _, l := range resp.Msg.Links {
				fromRef, ok1 := diagramIDToName[l.FromDiagramId]
				toRef, ok2 := diagramIDToName[l.ToDiagramId]
				if ok1 && ok2 {
					objRef := ""
					if l.ObjectId != 0 {
						objRef = objectIDToName[l.ObjectId]
					}
					newWS.Links = append(newWS.Links, workspace.Link{
						Object:      objRef,
						FromDiagram: fromRef,
						ToDiagram:   toRef,
					})
				}
			}

			// Write to YAML files
			if err := workspace.Save(newWS); err != nil {
				return fmt.Errorf("save workspace: %w", err)
			}

			fmt.Fprintf(cmd.OutOrStdout(), "Exported %d diagrams, %d objects, %d edges, %d links to %s\n",
				len(newWS.Diagrams), len(newWS.Objects), len(newWS.Edges), len(newWS.Links), *wdir)

			return nil
		},
	}

	return c
}
