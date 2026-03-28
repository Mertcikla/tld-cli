// Package planner converts a workspace into an ApplyPlanRequest and builds diagram execution order.
package planner

import (
	"sort"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/workspace"
	"google.golang.org/protobuf/types/known/timestamppb"
)

// Plan holds the resolved ApplyPlanRequest and a topologically-sorted list of
// diagram refs (parents before children).
type Plan struct {
	Request      *diagv1.ApplyPlanRequest
	DiagramOrder []string // sorted refs
}

// Build resolves workspace refs into an ApplyPlanRequest, ordering diagrams
// topologically (parents before children) so the server can resolve
// parent_diagram_ref in insertion order.
func Build(ws *workspace.Workspace, recreateIDs bool) (*Plan, error) {
	ordered := topoSortDiagrams(ws)

	req := &diagv1.ApplyPlanRequest{
		OrgId: ws.Config.OrgID,
	}

	// Diagrams
	for _, ref := range ordered {
		d := ws.Diagrams[ref]
		pd := &diagv1.PlanDiagram{
			Ref:  ref,
			Name: d.Name,
		}
		if d.Description != "" {
			pd.Description = &d.Description
		}
		if d.LevelLabel != "" {
			pd.LevelLabel = &d.LevelLabel
		}
		if d.ParentDiagram != "" {
			pd.ParentDiagramRef = &d.ParentDiagram
		}

		// Add metadata if available and not recreating IDs
		if !recreateIDs && ws.Meta != nil {
			if meta, ok := ws.Meta.Diagrams[ref]; ok {
				id := int32(meta.ID)
				pd.Id = &id
				pd.UpdatedAt = timestamppb.New(meta.UpdatedAt)
			}
		}

		req.Diagrams = append(req.Diagrams, pd)
	}

	// Objects
	for ref, o := range ws.Objects {
		po := &diagv1.PlanObject{
			Ref:  ref,
			Name: o.Name,
		}
		if o.Type != "" {
			po.Type = &o.Type
		}
		if o.Description != "" {
			po.Description = &o.Description
		}
		if o.Technology != "" {
			po.Technology = &o.Technology
		}
		if o.URL != "" {
			po.Url = &o.URL
		}
		if o.LogoURL != "" {
			po.LogoUrl = &o.LogoURL
		}
		for _, p := range o.Diagrams {
			pp := &diagv1.PlanObjectPlacement{DiagramRef: p.Diagram}
			if p.PositionX != 0 {
				pp.PositionX = &p.PositionX
			}
			if p.PositionY != 0 {
				pp.PositionY = &p.PositionY
			}
			po.Placements = append(po.Placements, pp)
		}

		// Add metadata if available and not recreating IDs
		if !recreateIDs && ws.Meta != nil {
			if meta, ok := ws.Meta.Objects[ref]; ok {
				id := int32(meta.ID)
				po.Id = &id
				po.UpdatedAt = timestamppb.New(meta.UpdatedAt)
			}
		}

		req.Objects = append(req.Objects, po)
	}

	// Edges
	for edgeRef, e := range ws.Edges {
		pe := &diagv1.PlanEdge{
			DiagramRef:      e.Diagram,
			SourceObjectRef: e.SourceObject,
			TargetObjectRef: e.TargetObject,
		}
		if e.Label != "" {
			pe.Label = &e.Label
		}
		if e.Description != "" {
			pe.Description = &e.Description
		}
		if e.RelationshipType != "" {
			pe.RelationshipType = &e.RelationshipType
		}
		if e.Direction != "" {
			pe.Direction = &e.Direction
		}
		if e.EdgeType != "" {
			pe.EdgeType = &e.EdgeType
		}
		if e.URL != "" {
			pe.Url = &e.URL
		}
		if e.SourceHandle != "" {
			pe.SourceHandle = &e.SourceHandle
		}
		if e.TargetHandle != "" {
			pe.TargetHandle = &e.TargetHandle
		}

		// Attach metadata for idempotent upserts using the map key as the stable ref
		if !recreateIDs && ws.Meta != nil {
			if meta, ok := ws.Meta.Edges[edgeRef]; ok {
				id := int32(meta.ID)
				pe.Id = &id
				pe.UpdatedAt = timestamppb.New(meta.UpdatedAt)
			}
		}

		req.Edges = append(req.Edges, pe)
	}

	// Links
	for _, l := range ws.Links {
		req.Links = append(req.Links, &diagv1.PlanLink{
			ObjectRef:      l.Object,
			FromDiagramRef: l.FromDiagram,
			ToDiagramRef:   l.ToDiagram,
		})
	}

	return &Plan{Request: req, DiagramOrder: ordered}, nil
}

// topoSortDiagrams returns diagram refs ordered so parents appear before
// children (stable: root nodes first in alphabetical order within each level).
func topoSortDiagrams(ws *workspace.Workspace) []string {
	inDegree := make(map[string]int)
	children := make(map[string][]string)

	for ref, d := range ws.Diagrams {
		if _, ok := inDegree[ref]; !ok {
			inDegree[ref] = 0
		}
		if d.ParentDiagram != "" {
			inDegree[ref]++
			children[d.ParentDiagram] = append(children[d.ParentDiagram], ref)
		}
	}

	// Kahn's algorithm
	var queue []string
	for ref, deg := range inDegree {
		if deg == 0 {
			queue = append(queue, ref)
		}
	}
	// Sort for determinism
	sort.Strings(queue)

	var result []string
	for len(queue) > 0 {
		cur := queue[0]
		queue = queue[1:]
		result = append(result, cur)
		ch := children[cur]
		sort.Strings(ch)
		for _, c := range ch {
			inDegree[c]--
			if inDegree[c] == 0 {
				queue = append(queue, c)
			}
		}
	}
	return result
}
