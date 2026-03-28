package cmd

import (
	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/workspace"
)

// convertExportResponse converts an ExportOrganizationResponse into a fully-populated
// Workspace. baseWS supplies the Dir and Config fields.
func convertExportResponse(baseWS *workspace.Workspace, msg *diagv1.ExportOrganizationResponse) *workspace.Workspace {
	newWS := &workspace.Workspace{
		Dir:    baseWS.Dir,
		Config: baseWS.Config,
		Diagrams: make(map[string]*workspace.Diagram),
		Objects:  make(map[string]*workspace.Object),
		Edges:    make(map[string]*workspace.Edge),
		Meta: &workspace.Meta{
			Diagrams: make(map[string]*workspace.ResourceMetadata),
			Objects:  make(map[string]*workspace.ResourceMetadata),
			Edges:    make(map[string]*workspace.ResourceMetadata),
		},
	}

	// Build ID → ref maps using slugified names
	diagramIDToRef := make(map[int32]string)
	for _, d := range msg.Diagrams {
		ref := workspace.Slugify(d.Name)
		diagramIDToRef[d.Id] = ref
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

	// Second pass: resolve parent diagram refs
	for _, d := range msg.Diagrams {
		if d.ParentDiagramId != nil && *d.ParentDiagramId != 0 {
			if parentRef, ok := diagramIDToRef[*d.ParentDiagramId]; ok {
				newWS.Diagrams[workspace.Slugify(d.Name)].ParentDiagram = parentRef
			}
		}
	}

	objectIDToRef := make(map[int32]string)
	for _, o := range msg.Objects {
		ref := workspace.Slugify(o.Name)
		objectIDToRef[o.Id] = ref
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

	// Placements
	for _, p := range msg.Placements {
		objRef, ok1 := objectIDToRef[p.ObjectId]
		diagRef, ok2 := diagramIDToRef[p.DiagramId]
		if ok1 && ok2 {
			newWS.Objects[objRef].Diagrams = append(newWS.Objects[objRef].Diagrams, workspace.Placement{
				Diagram:   diagRef,
				PositionX: p.PositionX,
				PositionY: p.PositionY,
			})
		}
	}

	// Edges (keyed by "diagram:src:tgt:label")
	for _, e := range msg.Edges {
		diagRef, ok1 := diagramIDToRef[e.DiagramId]
		srcRef, ok2 := objectIDToRef[e.SourceObjectId]
		tgtRef, ok3 := objectIDToRef[e.TargetObjectId]
		if !ok1 || !ok2 || !ok3 {
			continue
		}
		key := diagRef + ":" + srcRef + ":" + tgtRef + ":" + e.GetLabel()
		newWS.Edges[key] = &workspace.Edge{
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
		}
		newWS.Meta.Edges[key] = &workspace.ResourceMetadata{
			ID:        workspace.ResourceID(e.Id),
			UpdatedAt: e.UpdatedAt.AsTime(),
		}
	}

	// Links
	for _, l := range msg.Links {
		fromRef, ok1 := diagramIDToRef[l.FromDiagramId]
		toRef, ok2 := diagramIDToRef[l.ToDiagramId]
		if !ok1 || !ok2 {
			continue
		}
		objRef := ""
		if l.ObjectId != 0 {
			objRef = objectIDToRef[l.ObjectId]
		}
		newWS.Links = append(newWS.Links, workspace.Link{
			Object:      objRef,
			FromDiagram: fromRef,
			ToDiagram:   toRef,
		})
	}

	return newWS
}
