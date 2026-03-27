package workspace

import "fmt"

// ValidationError describes a single validation failure.
type ValidationError struct {
	Location string
	Message  string
}

func (e ValidationError) Error() string {
	return fmt.Sprintf("%s: %s", e.Location, e.Message)
}

// Validate checks cross-reference integrity, required fields, and cycle
// detection in parent_diagram chains.
func (ws *Workspace) Validate() []ValidationError {
	var errs []ValidationError

	// Diagrams: required fields + parent ref integrity
	for ref, d := range ws.Diagrams {
		loc := fmt.Sprintf("diagrams.yaml[%s]", ref)
		if d.Name == "" {
			errs = append(errs, ValidationError{loc, "name is required"})
		}
		if d.ParentDiagram != "" {
			if _, ok := ws.Diagrams[d.ParentDiagram]; !ok {
				errs = append(errs, ValidationError{
					loc,
					fmt.Sprintf("parent_diagram %q not found", d.ParentDiagram),
				})
			}
		}
	}
	// Cycle detection in parent chains
	for ref := range ws.Diagrams {
		if cycle := detectParentCycle(ws, ref); cycle != "" {
			errs = append(errs, ValidationError{
				fmt.Sprintf("diagrams.yaml[%s]", ref),
				fmt.Sprintf("circular parent_diagram chain: %s", cycle),
			})
		}
	}

	// Objects: required fields + diagram placement refs
	for ref, o := range ws.Objects {
		loc := fmt.Sprintf("objects.yaml[%s]", ref)
		if o.Name == "" {
			errs = append(errs, ValidationError{loc, "name is required"})
		}
		if o.Type == "" {
			errs = append(errs, ValidationError{loc, "type is required"})
		}
		for i, p := range o.Diagrams {
			ploc := fmt.Sprintf("objects.yaml[%s][diagrams][%d]", ref, i)
			if p.Diagram == "" {
				errs = append(errs, ValidationError{ploc, "diagram is required"})
				continue
			}
			if _, ok := ws.Diagrams[p.Diagram]; !ok {
				errs = append(errs, ValidationError{
					ploc,
					fmt.Sprintf("diagram ref %q not found", p.Diagram),
				})
			}
		}
	}

	// Edges: required fields + ref integrity
	for i, e := range ws.Edges {
		loc := fmt.Sprintf("edges.yaml[%d]", i)
		if e.Diagram == "" {
			errs = append(errs, ValidationError{loc, "diagram is required"})
		} else if _, ok := ws.Diagrams[e.Diagram]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("diagram ref %q not found", e.Diagram)})
		}
		if e.SourceObject == "" {
			errs = append(errs, ValidationError{loc, "source_object is required"})
		} else if _, ok := ws.Objects[e.SourceObject]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("source_object ref %q not found", e.SourceObject)})
		}
		if e.TargetObject == "" {
			errs = append(errs, ValidationError{loc, "target_object is required"})
		} else if _, ok := ws.Objects[e.TargetObject]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("target_object ref %q not found", e.TargetObject)})
		}
	}

	// Links: required fields + ref integrity
	for i, l := range ws.Links {
		loc := fmt.Sprintf("links.yaml[%d]", i)
		if l.Object != "" {
			if _, ok := ws.Objects[l.Object]; !ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("object ref %q not found", l.Object)})
			}
		}
		if l.FromDiagram == "" {
			errs = append(errs, ValidationError{loc, "from_diagram is required"})
		} else if _, ok := ws.Diagrams[l.FromDiagram]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("from_diagram ref %q not found", l.FromDiagram)})
		}
		if l.ToDiagram == "" {
			errs = append(errs, ValidationError{loc, "to_diagram is required"})
		} else if _, ok := ws.Diagrams[l.ToDiagram]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("to_diagram ref %q not found", l.ToDiagram)})
		}
	}

	return errs
}

// detectParentCycle returns a string describing the cycle if one is found
// starting from startRef, or "" if no cycle.
func detectParentCycle(ws *Workspace, startRef string) string {
	visited := map[string]bool{}
	cur := startRef
	path := []string{cur}
	for {
		d, ok := ws.Diagrams[cur]
		if !ok || d.ParentDiagram == "" {
			return ""
		}
		if visited[d.ParentDiagram] {
			return fmt.Sprintf("%v -> %s", path, d.ParentDiagram)
		}
		visited[cur] = true
		cur = d.ParentDiagram
		path = append(path, cur)
	}
}
