package cmd

import (
	"strings"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/workspace"
)

// convertExportResponse converts an ExportOrganizationResponse into a fully-populated
// Workspace. baseWS supplies the Dir and Config fields and is used to preserve existing refs.
func convertExportResponse(baseWS *workspace.Workspace, msg *diagv1.ExportOrganizationResponse) *workspace.Workspace {
	newWS := &workspace.Workspace{
		Dir:        baseWS.Dir,
		Config:     baseWS.Config,
		Diagrams:   make(map[string]*workspace.Diagram),
		Elements:   make(map[string]*workspace.Element),
		Connectors: make(map[string]*workspace.Connector),
		Meta: &workspace.Meta{
			Elements:   make(map[string]*workspace.ResourceMetadata),
			Views:      make(map[string]*workspace.ResourceMetadata),
			Connectors: make(map[string]*workspace.ResourceMetadata),
		},
	}

	// Reverse lookup for existing refs by ID.
	existingElementRefs := make(map[int32]string)
	if baseWS.Meta != nil {
		for ref, m := range baseWS.Meta.Elements {
			existingElementRefs[int32(m.ID)] = ref
		}
	}
	existingConnectorRefs := make(map[int32]string)
	if baseWS.Meta != nil {
		for ref, m := range baseWS.Meta.Connectors {
			existingConnectorRefs[int32(m.ID)] = ref
		}
	}

	objectIDToRef := make(map[int32]string)
	for _, e := range msg.Elements {
		ref, ok := existingElementRefs[e.Id]
		if !ok {
			ref = workspace.Slugify(e.Name)
		}
		objectIDToRef[e.Id] = ref
		kind := ""
		if e != nil {
			kind = *e.Kind
		}
		newWS.Elements[ref] = &workspace.Element{
			Name:        e.Name,
			Kind:        kind,
			Description: e.GetDescription(),
			Technology:  e.GetTechnology(),
			URL:         e.GetUrl(),
			LogoURL:     e.GetLogoUrl(),
			Repo:        e.GetRepo(),
			Branch:      e.GetBranch(),
			Language:    e.GetLanguage(),
			FilePath:    e.GetFilePath(),
		}
		newWS.Meta.Elements[ref] = &workspace.ResourceMetadata{
			ID:        workspace.ResourceID(e.Id),
			UpdatedAt: e.UpdatedAt.AsTime(),
		}
	}

	ownerByDiagramID := make(map[int32]string)
	parentByDiagramID := make(map[int32]int32)
	for _, link := range msg.Connectors {
		ownerRef, ok := objectIDToRef[link.TargetElementId]
		if !ok || link.TargetElementId == 0 {
			continue
		}
		ownerByDiagramID[link.TargetElementId] = ownerRef
		parentByDiagramID[link.TargetElementId] = link.SourceElementId
	}

	diagramIDToViewRef := make(map[int32]string)
	for _, d := range msg.Diagrams {
		if ownerRef, ok := ownerByDiagramID[d.Id]; ok {
			diagramIDToViewRef[d.Id] = ownerRef
			element := newWS.Elements[ownerRef]
			element.HasView = true
			if label := exportedDiagramLabel(d, element.Name); label != "" {
				element.ViewLabel = label
			}
			newWS.Diagrams[ownerRef] = &workspace.Diagram{
				Name:        d.Name,
				Description: d.GetDescription(),
				LevelLabel:  element.ViewLabel,
			}
			newWS.Meta.Views[ownerRef] = &workspace.ResourceMetadata{
				ID:        workspace.ResourceID(d.Id),
				UpdatedAt: d.UpdatedAt.AsTime(),
			}
			continue
		}

		diagramIDToViewRef[d.Id] = "root"
		newWS.Diagrams["root"] = &workspace.Diagram{
			Name:        d.Name,
			Description: d.GetDescription(),
			LevelLabel:  exportedDiagramLabel(d, "Workspace Root"),
		}
	}

	for childID, parentID := range parentByDiagramID {
		childRef := diagramIDToViewRef[childID]
		parentRef := diagramIDToViewRef[parentID]
		if childRef == "" || childRef == "root" || parentRef == "" {
			continue
		}
		if diagram := newWS.Diagrams[childRef]; diagram != nil {
			diagram.ParentDiagram = parentRef
		}
	}

	for _, p := range msg.Placements {
		elementRef, ok := objectIDToRef[p.ElementId]
		if !ok {
			continue
		}
		parentRef := diagramIDToViewRef[p.ViewId]
		if parentRef == "" {
			parentRef = "root"
		}
		newWS.Elements[elementRef].Placements = append(newWS.Elements[elementRef].Placements, workspace.ViewPlacement{
			ParentRef: parentRef,
			PositionX: p.PositionX,
			PositionY: p.PositionY,
		})
	}

	for _, e := range msg.Connectors {
		viewRef := diagramIDToViewRef[e.ViewId]
		if viewRef == "" {
			viewRef = "root"
		}
		srcRef, ok2 := objectIDToRef[e.SourceElementId]
		tgtRef, ok3 := objectIDToRef[e.TargetElementId]
		if !ok2 || !ok3 {
			continue
		}

		key, ok := existingConnectorRefs[e.Id]
		if !ok {
			key = viewRef + ":" + srcRef + ":" + tgtRef + ":" + e.GetLabel()
		}

		newWS.Connectors[key] = &workspace.Connector{
			View:         viewRef,
			Source:       srcRef,
			Target:       tgtRef,
			Label:        e.GetLabel(),
			Description:  e.GetDescription(),
			Relationship: e.GetRelationship(),
			Direction:    e.Direction,
			Style:        e.Style,
			URL:          e.GetUrl(),
			SourceHandle: e.GetSourceHandle(),
			TargetHandle: e.GetTargetHandle(),
		}
		newWS.Meta.Connectors[key] = &workspace.ResourceMetadata{
			ID:        workspace.ResourceID(e.Id),
			UpdatedAt: e.UpdatedAt.AsTime(),
		}
	}

	return newWS
}

func exportedDiagramLabel(diagram *diagv1.Diagram, elementName string) string {
	if label := strings.TrimSpace(diagram.GetLevelLabel()); label != "" {
		return label
	}
	name := strings.TrimSpace(diagram.Name)
	if name != "" && !strings.EqualFold(name, strings.TrimSpace(elementName)) {
		return name
	}
	return ""
}

func countElementDiagrams(ws *workspace.Workspace) int {
	count := 0
	for _, element := range ws.Elements {
		if element.HasView {
			count++
		}
	}
	return count
}
