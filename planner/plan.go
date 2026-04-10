// Package planner converts a workspace into an ApplyPlanRequest and builds diagram execution order.
package planner

import (
	"fmt"
	"sort"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/workspace"
	"google.golang.org/protobuf/types/known/timestamppb"
)

const syntheticRootViewRef = "__workspace_root__"

// Plan holds the resolved ApplyPlanRequest and a topologically-sorted list of
// diagram refs (parents before children).
type Plan struct {
	Request      *diagv1.ApplyPlanRequest
	DiagramOrder []string // sorted refs
	Model        string
}

// Build resolves workspace refs into an ApplyPlanRequest, ordering diagrams
// topologically (parents before children) so the server can resolve
// parent_diagram_ref in insertion order.
func Build(ws *workspace.Workspace, recreateIDs bool) (*Plan, error) {
	if usesElementWorkspace(ws) {
		return buildFromElements(ws, recreateIDs)
	}

	if len(ws.Diagrams) > 0 || len(ws.Objects) > 0 || len(ws.Edges) > 0 || len(ws.Links) > 0 {
		return nil, fmt.Errorf("legacy diagram/object/edge/link workspaces are no longer supported; migrate to elements.yaml and connectors.yaml")
	}

	return &Plan{
		Request: &diagv1.ApplyPlanRequest{OrgId: ws.Config.OrgID},
		Model:   "workspace",
	}, nil
}

func buildFromElements(ws *workspace.Workspace, recreateIDs bool) (*Plan, error) {
	elementRefs := make([]string, 0, len(ws.Elements))
	for ref := range ws.Elements {
		elementRefs = append(elementRefs, ref)
	}
	sort.Strings(elementRefs)

	connectorRefs := make([]string, 0, len(ws.Connectors))
	for ref := range ws.Connectors {
		connectorRefs = append(connectorRefs, ref)
	}
	sort.Strings(connectorRefs)

	req := &diagv1.ApplyPlanRequest{OrgId: ws.Config.OrgID}

	for _, ref := range elementRefs {
		element := ws.Elements[ref]
		planElement := &diagv1.PlanElement{
			Ref:     ref,
			Name:    element.Name,
			HasView: element.HasView,
		}
		if element.Kind != "" {
			planElement.Kind = &element.Kind
		}
		if element.Description != "" {
			planElement.Description = &element.Description
		}
		if element.Technology != "" {
			planElement.Technology = &element.Technology
		}
		if element.URL != "" {
			planElement.Url = &element.URL
		}
		if element.LogoURL != "" {
			planElement.LogoUrl = &element.LogoURL
		}
		if element.Repo != "" {
			planElement.Repo = &element.Repo
		}
		if element.Branch != "" {
			planElement.Branch = &element.Branch
		}
		if element.Language != "" {
			planElement.Language = &element.Language
		}
		if element.FilePath != "" {
			planElement.FilePath = &element.FilePath
		}
		if element.ViewLabel != "" {
			planElement.ViewLabel = &element.ViewLabel
		}
		for _, placement := range element.Placements {
			parentRef := placement.ParentRef
			if parentRef == "" {
				parentRef = syntheticRootViewRef
			}
			planPlacement := &diagv1.PlanViewPlacement{ParentRef: parentRef}
			if placement.PositionX != 0 {
				planPlacement.PositionX = &placement.PositionX
			}
			if placement.PositionY != 0 {
				planPlacement.PositionY = &placement.PositionY
			}
			planElement.Placements = append(planElement.Placements, planPlacement)
		}

		if !recreateIDs && ws.Meta != nil {
			if meta, ok := ws.Meta.Elements[ref]; ok {
				id := int32(meta.ID)
				planElement.Id = &id
				planElement.UpdatedAt = timestamppb.New(meta.UpdatedAt)
			}
			if element.HasView {
				if meta, ok := ws.Meta.Views[ref]; ok {
					id := int32(meta.ID)
					planElement.ViewId = &id
					planElement.ViewUpdatedAt = timestamppb.New(meta.UpdatedAt)
				}
			}
		}

		req.Elements = append(req.Elements, planElement)
	}

	for _, ref := range connectorRefs {
		connector := ws.Connectors[ref]
		viewRef := connector.View
		if viewRef == "" {
			viewRef = syntheticRootViewRef
		}
		planConnector := &diagv1.PlanConnector{
			Ref:              ref,
			ViewRef:          viewRef,
			SourceElementRef: connector.Source,
			TargetElementRef: connector.Target,
		}
		if connector.Label != "" {
			planConnector.Label = &connector.Label
		}
		if connector.Description != "" {
			planConnector.Description = &connector.Description
		}
		if connector.Relationship != "" {
			planConnector.Relationship = &connector.Relationship
		}
		if connector.Direction != "" {
			planConnector.Direction = &connector.Direction
		}
		if connector.Style != "" {
			planConnector.Style = &connector.Style
		}
		if connector.URL != "" {
			planConnector.Url = &connector.URL
		}
		if connector.SourceHandle != "" {
			planConnector.SourceHandle = &connector.SourceHandle
		}
		if connector.TargetHandle != "" {
			planConnector.TargetHandle = &connector.TargetHandle
		}

		if !recreateIDs && ws.Meta != nil {
			if meta, ok := ws.Meta.Connectors[ref]; ok {
				id := int32(meta.ID)
				planConnector.Id = &id
				planConnector.UpdatedAt = timestamppb.New(meta.UpdatedAt)
			}
		}

		req.Connectors = append(req.Connectors, planConnector)
	}

	return &Plan{Request: req, Model: "workspace"}, nil
}

func usesElementWorkspace(ws *workspace.Workspace) bool {
	return len(ws.Elements) > 0 || len(ws.Connectors) > 0
}
